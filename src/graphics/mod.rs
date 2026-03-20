//! Graphics backend abstraction - supports multiple rendering methods
//!
//! Each backend implements the `GraphicsRenderer` trait, providing a uniform
//! interface for image rendering across different terminal capabilities.

mod blocks;
mod framebuffer;
mod kitty;
mod sixel;

use anyhow::Result;
use std::io::Write;

/// Parameters for rendering an image at a terminal position
#[derive(Debug, Clone, Copy)]
pub struct ImageParams<'a> {
    /// Raw pixel data (RGB or RGBA depending on the render method)
    pub data: &'a [u8],
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Column position (0-indexed)
    pub col: u16,
    /// Row position (0-indexed)
    pub row: u16,
    /// Width in character cells (estimated from pixel dimensions if not provided)
    pub width_cells: Option<u16>,
    /// Height in character cells (estimated from pixel dimensions if not provided)
    pub height_cells: Option<u16>,
}

/// Graphics rendering backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsBackend {
    /// Linux framebuffer direct rendering
    Framebuffer,
    /// Kitty graphics protocol
    Kitty,
    /// Sixel graphics
    Sixel,
    /// Unicode block characters (universal fallback)
    Blocks,
}

impl GraphicsBackend {
    /// Detect the best available graphics backend
    pub fn detect() -> Self {
        if Self::has_framebuffer() {
            return GraphicsBackend::Framebuffer;
        }

        if Self::has_kitty() {
            return GraphicsBackend::Kitty;
        }

        if Self::has_sixel() {
            return GraphicsBackend::Sixel;
        }

        GraphicsBackend::Blocks
    }

    /// Check if Linux framebuffer is available
    fn has_framebuffer() -> bool {
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            return false;
        }

        std::path::Path::new("/dev/fb0").exists()
    }

    /// Check if Kitty graphics protocol is supported
    fn has_kitty() -> bool {
        std::env::var("KITTY_WINDOW_ID").is_ok()
            || std::env::var("TERM").unwrap_or_default().contains("kitty")
    }

    /// Check if Sixel is supported
    fn has_sixel() -> bool {
        let term = std::env::var("TERM").unwrap_or_default();
        term.contains("mlterm")
            || term.contains("xterm")
            || std::env::var("TERM_PROGRAM")
                .unwrap_or_default()
                .contains("iTerm")
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            GraphicsBackend::Framebuffer => "Linux Framebuffer",
            GraphicsBackend::Kitty => "Kitty Graphics",
            GraphicsBackend::Sixel => "Sixel",
            GraphicsBackend::Blocks => "Unicode Blocks",
        }
    }
}

/// Default capacity for line buffer
const LINE_BUFFER_CAPACITY: usize = 512;

/// Default capacity for escape sequence building
const ESCAPE_BUFFER_CAPACITY: usize = 256;

/// Trait for graphics rendering backends
pub trait GraphicsRenderer {
    /// Render an RGB image
    fn render_rgb(&mut self, writer: &mut dyn Write, params: &ImageParams) -> Result<()>;

    /// Render an RGBA image (default: strip alpha and delegate to render_rgb)
    fn render_rgba(&mut self, writer: &mut dyn Write, params: &ImageParams) -> Result<()> {
        let rgb = rgba_to_rgb(params.data);
        self.render_rgb(writer, &ImageParams { data: &rgb, ..*params })
    }

    /// Delete all tracked images
    fn delete_all_images(&mut self, _writer: &mut dyn Write) -> Result<()> {
        Ok(())
    }

    /// Reset animation state
    fn reset_animation(&mut self) {}

    /// Refresh pane position info (for multiplexer support)
    fn refresh_pane_info(&mut self) {}

    /// Enable or disable Unicode placeholder mode
    fn set_unicode_placeholders(&mut self, _enabled: bool) {}

    /// Get the backend type
    fn backend_type(&self) -> GraphicsBackend;
}

/// Strip alpha channel from RGBA data, producing RGB
fn rgba_to_rgb(data: &[u8]) -> Vec<u8> {
    data.chunks(4)
        .flat_map(|c| [c[0], c[1], c[2]])
        .collect()
}

/// Image renderer wrapping a graphics backend
///
/// Delegates to the appropriate `GraphicsRenderer` implementation based on
/// detected or specified terminal capabilities.
pub struct ImageRenderer {
    inner: Box<dyn GraphicsRenderer>,
}

impl ImageRenderer {
    /// Create a new image renderer with the specified backend
    pub fn new(backend: GraphicsBackend, in_tmux: bool) -> Self {
        let inner: Box<dyn GraphicsRenderer> = match backend {
            GraphicsBackend::Kitty => Box::new(kitty::KittyRenderer::new(in_tmux)),
            GraphicsBackend::Sixel => Box::new(sixel::SixelRenderer::new(in_tmux)),
            GraphicsBackend::Blocks => Box::new(blocks::BlocksRenderer::new()),
            GraphicsBackend::Framebuffer => Box::new(framebuffer::FramebufferRenderer),
        };
        ImageRenderer { inner }
    }

    /// Reset animation state (call when clearing images or starting fresh)
    pub fn reset_animation(&mut self) {
        self.inner.reset_animation();
    }

    /// Refresh pane info - call when pane position may have changed
    pub fn refresh_pane_info(&mut self) {
        self.inner.refresh_pane_info();
    }

    /// Enable or disable Unicode placeholder mode
    pub fn set_unicode_placeholders(&mut self, enabled: bool) {
        self.inner.set_unicode_placeholders(enabled);
    }

    /// Delete all images and reset animation state
    pub fn delete_all_images<W: Write>(&mut self, writer: &mut W) -> Result<()> {
        self.inner.delete_all_images(writer)
    }

    /// Get the current backend
    pub fn backend(&self) -> GraphicsBackend {
        self.inner.backend_type()
    }

    /// Render an RGB image at the specified terminal position
    pub fn render_image<W: Write>(
        &mut self,
        writer: &mut W,
        params: &ImageParams,
    ) -> Result<()> {
        self.inner.render_rgb(writer, params)
    }

    /// Render an RGBA image with alpha transparency support
    pub fn render_image_rgba<W: Write>(
        &mut self,
        writer: &mut W,
        params: &ImageParams,
    ) -> Result<()> {
        self.inner.render_rgba(writer, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_detection() {
        let backend = GraphicsBackend::detect();
        assert!(matches!(
            backend,
            GraphicsBackend::Framebuffer
                | GraphicsBackend::Kitty
                | GraphicsBackend::Sixel
                | GraphicsBackend::Blocks
        ));
    }

    #[test]
    fn test_backend_names() {
        assert_eq!(GraphicsBackend::Kitty.name(), "Kitty Graphics");
        assert_eq!(GraphicsBackend::Sixel.name(), "Sixel");
        assert_eq!(GraphicsBackend::Blocks.name(), "Unicode Blocks");
        assert_eq!(GraphicsBackend::Framebuffer.name(), "Linux Framebuffer");
    }
}
