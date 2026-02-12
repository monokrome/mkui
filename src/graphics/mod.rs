//! Graphics backend abstraction - supports multiple rendering methods
//!
//! Performance optimizations:
//! - Pre-allocated buffers for escape sequence building
//! - Batched character writes for block rendering
//! - Efficient base64 encoding with pre-sized buffers

mod blocks;
mod framebuffer;
mod kitty;
mod sixel;

use anyhow::Result;
use std::io::Write;

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

/// Default capacity for line buffer in block rendering
pub(super) const LINE_BUFFER_CAPACITY: usize = 512;

/// Default capacity for escape sequence building
pub(super) const ESCAPE_BUFFER_CAPACITY: usize = 256;

/// Unicode placeholder character for Kitty graphics protocol (U+10EEEE)
const PLACEHOLDER_CHAR: char = '\u{10EEEE}';

/// Row/column diacritics for Unicode placeholders in Kitty graphics protocol.
/// These are combining characters (class 230) used to encode row and column indices.
const DIACRITICS: &[u32] = &[
    0x0305, 0x030D, 0x030E, 0x0310, 0x0312, 0x033D, 0x033E, 0x033F, 0x0346, 0x034A, 0x034B, 0x034C,
    0x0350, 0x0351, 0x0352, 0x0357, 0x035B, 0x0363, 0x0364, 0x0365, 0x0366, 0x0367, 0x0368, 0x0369,
    0x036A, 0x036B, 0x036C, 0x036D, 0x036E, 0x036F, 0x0483, 0x0484, 0x0485, 0x0486, 0x0487, 0x0592,
    0x0593, 0x0594, 0x0595, 0x0597, 0x0598, 0x0599, 0x059C, 0x059D, 0x059E, 0x059F, 0x05A0, 0x05A1,
    0x05A8, 0x05A9, 0x05AB, 0x05AC, 0x05AF, 0x05C4, 0x0610, 0x0611, 0x0612, 0x0613, 0x0614, 0x0615,
    0x0616, 0x0617, 0x0657, 0x0658, 0x0659, 0x065A, 0x065B, 0x065D, 0x065E, 0x06D6, 0x06D7, 0x06D8,
    0x06D9, 0x06DA, 0x06DB, 0x06DC, 0x06DF, 0x06E0, 0x06E1, 0x06E2, 0x06E4, 0x06E7, 0x06E8, 0x06EB,
    0x06EC, 0x0730, 0x0732, 0x0733, 0x0735, 0x0736, 0x073A, 0x073D, 0x073F, 0x0740, 0x0741, 0x0743,
    0x0745, 0x0747, 0x0749, 0x074A, 0x07EB, 0x07EC, 0x07ED, 0x07EE, 0x07EF, 0x07F0, 0x07F1, 0x07F3,
    0x0816, 0x0817, 0x0818, 0x0819, 0x081B, 0x081C, 0x081D, 0x081E, 0x081F, 0x0820, 0x0821, 0x0822,
    0x0823, 0x0825, 0x0826, 0x0827, 0x0829, 0x082A, 0x082B, 0x082C, 0x082D, 0x0951, 0x0953, 0x0954,
    0x0F82, 0x0F83, 0x0F86, 0x0F87, 0x135D, 0x135E, 0x135F, 0x17DD, 0x193A, 0x1A17, 0x1A75, 0x1A76,
    0x1A77, 0x1A78, 0x1A79, 0x1A7A, 0x1A7B, 0x1A7C, 0x1B6B, 0x1B6D, 0x1B6E, 0x1B6F, 0x1B70, 0x1B71,
    0x1B72, 0x1B73, 0x1CD0, 0x1CD1, 0x1CD2, 0x1CDA, 0x1CDB, 0x1CE0, 0x1DC0, 0x1DC1, 0x1DC3, 0x1DC4,
    0x1DC5, 0x1DC6, 0x1DC7, 0x1DC8, 0x1DC9, 0x1DCB, 0x1DCC, 0x1DD1, 0x1DD2, 0x1DD3, 0x1DD4, 0x1DD5,
    0x1DD6, 0x1DD7, 0x1DD8, 0x1DD9, 0x1DDA, 0x1DDB, 0x1DDC, 0x1DDD, 0x1DDE, 0x1DDF, 0x1DE0, 0x1DE1,
    0x1DE2, 0x1DE3, 0x1DE4, 0x1DE5, 0x1DE6, 0x1DFE, 0x20D0, 0x20D1, 0x20D4, 0x20D5, 0x20D6, 0x20D7,
    0x20DB, 0x20DC, 0x20E1, 0x20E7, 0x20E9, 0x20F0, 0x2CEF, 0x2CF0, 0x2CF1, 0x2DE0, 0x2DE1, 0x2DE2,
    0x2DE3, 0x2DE4, 0x2DE5, 0x2DE6, 0x2DE7, 0x2DE8, 0x2DE9, 0x2DEA, 0x2DEB, 0x2DEC, 0x2DED, 0x2DEE,
    0x2DEF, 0x2DF0, 0x2DF1, 0x2DF2, 0x2DF3, 0x2DF4, 0x2DF5, 0x2DF6, 0x2DF7, 0x2DF8, 0x2DF9, 0x2DFA,
    0x2DFB, 0x2DFC, 0x2DFD, 0x2DFE, 0x2DFF, 0xA66F, 0xA67C, 0xA67D, 0xA6F0, 0xA6F1, 0xA8E0, 0xA8E1,
    0xA8E2, 0xA8E3, 0xA8E4, 0xA8E5, 0xA8E6, 0xA8E7, 0xA8E8, 0xA8E9, 0xA8EA, 0xA8EB, 0xA8EC, 0xA8ED,
    0xA8EE, 0xA8EF, 0xA8F0, 0xA8F1, 0xAAB0, 0xAAB2, 0xAAB3, 0xAAB7, 0xAAB8, 0xAABE, 0xAABF, 0xAAC1,
    0xFE20, 0xFE21, 0xFE22, 0xFE23, 0xFE24, 0xFE25, 0xFE26,
];

/// Get the diacritic character for a given index (0-255)
fn get_diacritic(index: u8) -> char {
    let idx = index as usize;
    if idx < DIACRITICS.len() {
        char::from_u32(DIACRITICS[idx]).unwrap_or('\u{0305}')
    } else {
        '\u{0305}'
    }
}

/// Tmux pane position (cached)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default)]
struct TmuxPaneOffset {
    top: u16,
    left: u16,
}

impl TmuxPaneOffset {
    fn query() -> Option<Self> {
        use std::process::Command;

        let output = Command::new("tmux")
            .args(["display-message", "-p", "#{pane_top} #{pane_left}"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.split_whitespace().collect();

        if parts.len() >= 2 {
            Some(TmuxPaneOffset {
                top: parts[0].parse().ok()?,
                left: parts[1].parse().ok()?,
            })
        } else {
            None
        }
    }
}

/// Image renderer for the selected backend
///
/// Uses pre-allocated buffers to minimize allocations during rendering.
pub struct ImageRenderer {
    pub(super) backend: GraphicsBackend,
    pub(super) in_tmux: bool,
    /// Pre-allocated buffer for building escape sequences
    pub(super) line_buffer: String,
    /// Pre-allocated buffer for command parameters
    pub(super) escape_buffer: String,
    /// Current animation frame number (for Kitty animation protocol)
    pub(super) animation_image_id: Option<u32>,
    /// Whether the animation has been initialized (first frame sent)
    pub(super) animation_initialized: bool,
    /// Cached tmux pane offset (refreshed on demand)
    tmux_pane_offset: Option<TmuxPaneOffset>,
}

impl ImageRenderer {
    /// Create a new image renderer with detected backend
    pub fn new(backend: GraphicsBackend, in_tmux: bool) -> Self {
        let tmux_pane_offset = if in_tmux {
            TmuxPaneOffset::query()
        } else {
            None
        };

        ImageRenderer {
            backend,
            in_tmux,
            line_buffer: String::with_capacity(LINE_BUFFER_CAPACITY),
            escape_buffer: String::with_capacity(ESCAPE_BUFFER_CAPACITY),
            animation_image_id: None,
            animation_initialized: false,
            tmux_pane_offset,
        }
    }

    /// Reset animation state (call when clearing images or starting fresh)
    pub fn reset_animation(&mut self) {
        self.animation_image_id = None;
        self.animation_initialized = false;
    }

    /// Refresh pane info - call when pane position may have changed
    pub fn refresh_pane_info(&mut self) {
        if self.in_tmux {
            self.tmux_pane_offset = TmuxPaneOffset::query();
        }
    }

    /// Enable or disable Unicode placeholder mode (no-op, kept for API compatibility)
    pub fn set_unicode_placeholders(&mut self, _enabled: bool) {}

    /// Delete all images and reset animation state
    pub fn delete_all_images<W: Write>(&mut self, writer: &mut W) -> Result<()> {
        self.reset_animation();

        if self.backend != GraphicsBackend::Kitty {
            return Ok(());
        }

        let delete_cmd = "\x1b_Ga=d,d=I,i=1,q=2\x1b\\";

        if self.in_tmux {
            let escaped = delete_cmd.replace('\x1b', "\x1b\x1b");
            write!(writer, "\x1bPtmux;{}\x1b\\", escaped)?;
        } else {
            write!(writer, "{}", delete_cmd)?;
        }

        Ok(())
    }

    /// Get the current backend
    pub fn backend(&self) -> GraphicsBackend {
        self.backend
    }

    /// Render an image at the specified terminal position
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    pub fn render_image<W: Write>(
        &mut self,
        writer: &mut W,
        image_data: &[u8],
        width: u32,
        height: u32,
        col: u16,
        row: u16,
        width_cells: Option<u16>,
        height_cells: Option<u16>,
    ) -> Result<()> {
        match self.backend {
            GraphicsBackend::Framebuffer => self.render_framebuffer(image_data, width, height),
            GraphicsBackend::Kitty => self.render_kitty(
                writer,
                image_data,
                width,
                height,
                col,
                row,
                width_cells,
                height_cells,
            ),
            GraphicsBackend::Sixel => {
                self.render_sixel(writer, image_data, width, height, col, row)
            }
            GraphicsBackend::Blocks => self.render_blocks(
                writer,
                image_data,
                width,
                height,
                col,
                row,
                width_cells,
                height_cells,
            ),
        }
    }

    /// Render an RGBA image with alpha transparency support
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    pub fn render_image_rgba<W: Write>(
        &mut self,
        writer: &mut W,
        image_data: &[u8],
        width: u32,
        height: u32,
        col: u16,
        row: u16,
        width_cells: Option<u16>,
        height_cells: Option<u16>,
    ) -> Result<()> {
        match self.backend {
            GraphicsBackend::Framebuffer => self.render_framebuffer(image_data, width, height),
            GraphicsBackend::Kitty => self.render_kitty_rgba(
                writer,
                image_data,
                width,
                height,
                col,
                row,
                width_cells,
                height_cells,
            ),
            GraphicsBackend::Sixel => {
                let rgb: Vec<u8> = image_data
                    .chunks(4)
                    .flat_map(|c| [c[0], c[1], c[2]])
                    .collect();
                self.render_sixel(writer, &rgb, width, height, col, row)
            }
            GraphicsBackend::Blocks => {
                let rgb: Vec<u8> = image_data
                    .chunks(4)
                    .flat_map(|c| [c[0], c[1], c[2]])
                    .collect();
                self.render_blocks(
                    writer,
                    &rgb,
                    width,
                    height,
                    col,
                    row,
                    width_cells,
                    height_cells,
                )
            }
        }
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
