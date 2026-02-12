//! Rendering backend - terminal output, multi-backend graphics, and cursor management
//!
//! Performance optimizations:
//! - Write buffering to minimize syscalls
//! - Dirty region tracking to avoid unnecessary redraws
//! - Pre-allocated buffers to reduce allocations

use crate::graphics::{GraphicsBackend, ImageRenderer};
use crate::terminal::TerminalContext;
use anyhow::Result;
use std::io::{self, BufWriter, Write};

/// Default buffer capacity for write batching (16KB)
const WRITE_BUFFER_CAPACITY: usize = 16 * 1024;

/// Dirty region for optimized rendering
#[derive(Debug, Clone, Copy, Default)]
pub struct DirtyRegion {
    /// Minimum column that needs redraw
    pub min_col: u16,
    /// Minimum row that needs redraw
    pub min_row: u16,
    /// Maximum column that needs redraw
    pub max_col: u16,
    /// Maximum row that needs redraw
    pub max_row: u16,
    /// Whether any region is dirty
    pub is_dirty: bool,
}

impl DirtyRegion {
    /// Create a new empty (clean) region
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the entire screen as dirty
    pub fn mark_all(&mut self, cols: u16, rows: u16) {
        self.min_col = 0;
        self.min_row = 0;
        self.max_col = cols;
        self.max_row = rows;
        self.is_dirty = true;
    }

    /// Mark a specific region as dirty
    pub fn mark_region(&mut self, col: u16, row: u16, width: u16, height: u16) {
        if !self.is_dirty {
            self.min_col = col;
            self.min_row = row;
            self.max_col = col + width;
            self.max_row = row + height;
            self.is_dirty = true;
        } else {
            self.min_col = self.min_col.min(col);
            self.min_row = self.min_row.min(row);
            self.max_col = self.max_col.max(col + width);
            self.max_row = self.max_row.max(row + height);
        }
    }

    /// Clear the dirty region (mark as clean)
    pub fn clear(&mut self) {
        self.min_col = 0;
        self.min_row = 0;
        self.max_col = 0;
        self.max_row = 0;
        self.is_dirty = false;
    }

    /// Check if a region intersects with the dirty area
    pub fn intersects(&self, col: u16, row: u16, width: u16, height: u16) -> bool {
        if !self.is_dirty {
            return false;
        }
        !(col + width < self.min_col
            || col > self.max_col
            || row + height < self.min_row
            || row > self.max_row)
    }
}

/// Raw terminal renderer handling output and graphics
///
/// Uses internal write buffering to minimize syscalls and improve performance.
/// Call `flush()` after a batch of operations to ensure output is displayed.
pub struct Renderer {
    /// Buffered writer for batching terminal output
    writer: BufWriter<io::Stdout>,
    context: TerminalContext,
    image_renderer: ImageRenderer,
    in_alt_screen: bool,
    /// Dirty region tracking for optimized redraws
    dirty: DirtyRegion,
    /// Scratch buffer for building escape sequences (reduces allocations)
    scratch: String,
}

impl Renderer {
    /// Create a new renderer with detected terminal context and graphics backend
    pub fn new() -> Result<Self> {
        let context = TerminalContext::detect()?;
        let backend = GraphicsBackend::detect();
        let in_tmux = context.capabilities.in_multiplexer;

        eprintln!("Graphics backend: {}", backend.name());

        let stdout = io::stdout();
        let writer = BufWriter::with_capacity(WRITE_BUFFER_CAPACITY, stdout);

        Ok(Renderer {
            writer,
            context,
            image_renderer: ImageRenderer::new(backend, in_tmux),
            in_alt_screen: false,
            dirty: DirtyRegion::new(),
            scratch: String::with_capacity(256),
        })
    }

    /// Create a new renderer with a specific graphics backend
    pub fn with_backend(backend: GraphicsBackend) -> Result<Self> {
        let context = TerminalContext::detect()?;
        let in_tmux = context.capabilities.in_multiplexer;

        eprintln!("Graphics backend: {} (forced)", backend.name());

        let stdout = io::stdout();
        let writer = BufWriter::with_capacity(WRITE_BUFFER_CAPACITY, stdout);

        Ok(Renderer {
            writer,
            context,
            image_renderer: ImageRenderer::new(backend, in_tmux),
            in_alt_screen: false,
            dirty: DirtyRegion::new(),
            scratch: String::with_capacity(256),
        })
    }

    /// Get the current graphics backend
    pub fn graphics_backend(&self) -> GraphicsBackend {
        self.image_renderer.backend()
    }

    /// Enter alternative screen buffer
    ///
    /// Note: This immediately flushes to ensure the screen switch happens.
    pub fn enter_alt_screen(&mut self) -> Result<()> {
        if !self.in_alt_screen {
            write!(self.writer, "\x1b[?1049h")?;
            self.writer.flush()?; // Immediate flush for screen mode changes
            self.in_alt_screen = true;
            // Mark entire screen as dirty after entering alt screen
            let (cols, rows) = (self.context.geometry.cols, self.context.geometry.rows);
            self.dirty.mark_all(cols, rows);
        }
        Ok(())
    }

    /// Exit alternative screen buffer
    ///
    /// Note: This immediately flushes to ensure the screen switch happens.
    pub fn exit_alt_screen(&mut self) -> Result<()> {
        if self.in_alt_screen {
            write!(self.writer, "\x1b[?1049l")?;
            self.writer.flush()?; // Immediate flush for screen mode changes
            self.in_alt_screen = false;
            self.dirty.clear();
        }
        Ok(())
    }

    /// Clear the screen
    ///
    /// Marks the entire screen as dirty for subsequent rendering.
    pub fn clear(&mut self) -> Result<()> {
        write!(self.writer, "\x1b[2J")?;
        // Mark entire screen as dirty after clear
        let (cols, rows) = (self.context.geometry.cols, self.context.geometry.rows);
        self.dirty.mark_all(cols, rows);
        Ok(())
    }

    /// Move cursor to position (0-indexed)
    #[inline]
    pub fn move_cursor(&mut self, col: u16, row: u16) -> Result<()> {
        write!(self.writer, "\x1b[{};{}H", row + 1, col + 1)?;
        Ok(())
    }

    /// Hide cursor
    ///
    /// Note: Buffered - call flush() to ensure it takes effect immediately.
    pub fn hide_cursor(&mut self) -> Result<()> {
        write!(self.writer, "\x1b[?25l")?;
        Ok(())
    }

    /// Show cursor
    ///
    /// Note: Buffered - call flush() to ensure it takes effect immediately.
    pub fn show_cursor(&mut self) -> Result<()> {
        write!(self.writer, "\x1b[?25h")?;
        Ok(())
    }

    /// Write text at current cursor position
    #[inline]
    pub fn write_text(&mut self, text: &str) -> Result<()> {
        write!(self.writer, "{}", text)?;
        Ok(())
    }

    /// Write text with ANSI color/style codes
    #[inline]
    pub fn write_styled(&mut self, text: &str, style: &str) -> Result<()> {
        write!(self.writer, "{}{}\x1b[0m", style, text)?;
        Ok(())
    }

    /// Write a repeated character (more efficient than multiple write_text calls)
    #[inline]
    pub fn write_repeated(&mut self, ch: char, count: usize) -> Result<()> {
        for _ in 0..count {
            write!(self.writer, "{}", ch)?;
        }
        Ok(())
    }

    /// Flush output buffer to terminal
    ///
    /// Call this after a batch of rendering operations to display the output.
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    /// Get current dirty region
    pub fn dirty_region(&self) -> &DirtyRegion {
        &self.dirty
    }

    /// Mark a region as needing redraw
    pub fn mark_dirty(&mut self, col: u16, row: u16, width: u16, height: u16) {
        self.dirty.mark_region(col, row, width, height);
    }

    /// Clear dirty region tracking (call after full render)
    pub fn clear_dirty(&mut self) {
        self.dirty.clear();
    }

    /// Get current terminal context
    pub fn context(&self) -> &TerminalContext {
        &self.context
    }

    /// Refresh terminal geometry (call after resize)
    pub fn refresh_geometry(&mut self) -> Result<()> {
        self.context.refresh_geometry()?;
        // Also refresh tmux pane info if applicable
        self.image_renderer.refresh_pane_info();
        Ok(())
    }

    /// Force refresh tmux pane position (call on pane switch)
    pub fn refresh_pane_info(&mut self) {
        self.image_renderer.refresh_pane_info();
    }

    /// Render an image using the selected graphics backend
    ///
    /// # Arguments
    /// * `image_data` - Raw RGB image data
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `col` - Column position (0-indexed)
    /// * `row` - Row position (0-indexed)
    /// * `width_cells` - Width in character cells (optional)
    /// * `height_cells` - Height in character cells (optional)
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    pub fn render_image(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
        col: u16,
        row: u16,
        width_cells: Option<u16>,
        height_cells: Option<u16>,
    ) -> Result<()> {
        // Mark the image region as dirty
        let w = width_cells.unwrap_or((width / 10) as u16); // Estimate cells if not provided
        let h = height_cells.unwrap_or((height / 20) as u16);
        self.dirty.mark_region(col, row, w, h);

        self.image_renderer.render_image(
            &mut self.writer,
            image_data,
            width,
            height,
            col,
            row,
            width_cells,
            height_cells,
        )
    }

    /// Render an RGBA image with alpha transparency support
    ///
    /// # Arguments
    /// * `image_data` - Raw RGBA8 pixel data (4 bytes per pixel)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `col` - Column position (0-indexed)
    /// * `row` - Row position (0-indexed)
    /// * `width_cells` - Width in character cells (optional)
    /// * `height_cells` - Height in character cells (optional)
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    pub fn render_image_rgba(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
        col: u16,
        row: u16,
        width_cells: Option<u16>,
        height_cells: Option<u16>,
    ) -> Result<()> {
        // Mark the image region as dirty
        let w = width_cells.unwrap_or((width / 10) as u16);
        let h = height_cells.unwrap_or((height / 20) as u16);
        self.dirty.mark_region(col, row, w, h);

        self.image_renderer.render_image_rgba(
            &mut self.writer,
            image_data,
            width,
            height,
            col,
            row,
            width_cells,
            height_cells,
        )
    }

    /// Render a PNG image using Kitty graphics protocol (legacy method)
    ///
    /// # Arguments
    /// * `png_data` - PNG encoded image data
    /// * `col` - Column position (0-indexed)
    /// * `row` - Row position (0-indexed)
    /// * `width_cells` - Width in character cells (optional)
    /// * `height_cells` - Height in character cells (optional)
    #[deprecated(note = "Use render_image() instead for multi-backend support")]
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    pub fn render_kitty_image(
        &mut self,
        png_data: &[u8],
        col: u16,
        row: u16,
        width_cells: Option<u16>,
        height_cells: Option<u16>,
    ) -> Result<()> {
        // Decode PNG to raw RGB
        let img = image::load_from_memory(png_data)?;
        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();

        self.render_image(&rgb, width, height, col, row, width_cells, height_cells)
    }

    /// Delete all graphics images tracked by this renderer
    ///
    /// This uses image ID tracking to reliably delete images even in tmux.
    /// Call this before rendering new images to prevent persistence issues.
    pub fn clear_images(&mut self) -> Result<()> {
        self.image_renderer.delete_all_images(&mut self.writer)?;
        Ok(())
    }

    /// Enable or disable Unicode placeholder mode for graphics
    ///
    /// When enabled (default in tmux), images use Unicode placeholder characters
    /// which makes them behave like text - respecting pane boundaries and scrolling.
    /// This is slower but more reliable in terminal multiplexers.
    ///
    /// When disabled, images are placed directly at screen coordinates which is
    /// faster but can escape pane boundaries in tmux.
    pub fn set_unicode_placeholders(&mut self, enabled: bool) {
        self.image_renderer.set_unicode_placeholders(enabled);
    }

    /// Check if running inside a terminal multiplexer (tmux/screen)
    pub fn in_multiplexer(&self) -> bool {
        self.context.capabilities.in_multiplexer
    }

    /// Begin a render frame - hides cursor and prepares for rendering
    ///
    /// Call this at the start of each frame for optimal performance.
    /// If `clear_graphics` is true, also clears all tracked images to prevent
    /// persistence issues (recommended for dynamic content).
    pub fn begin_frame(&mut self) -> Result<()> {
        self.begin_frame_with_options(true)
    }

    /// Begin a render frame with options
    ///
    /// # Arguments
    /// * `clear_graphics` - If true, clears all tracked graphics images before rendering.
    ///   Set to true for dynamic content that changes each frame.
    ///   Set to false for static images that should persist.
    pub fn begin_frame_with_options(&mut self, clear_graphics: bool) -> Result<()> {
        self.hide_cursor()?;
        if clear_graphics {
            self.clear_images()?;
        }
        Ok(())
    }

    /// End a render frame - shows cursor and flushes output
    ///
    /// Call this at the end of each frame to display all buffered output.
    pub fn end_frame(&mut self) -> Result<()> {
        self.show_cursor()?;
        self.flush()?;
        self.clear_dirty();
        Ok(())
    }

    /// Check if the renderer is in alternative screen mode
    pub fn in_alt_screen(&self) -> bool {
        self.in_alt_screen
    }

    /// Get mutable access to the scratch buffer for building strings
    ///
    /// This is useful for components that need to build escape sequences
    /// without allocating new strings each time.
    pub fn scratch_buffer(&mut self) -> &mut String {
        self.scratch.clear();
        &mut self.scratch
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // Ensure we exit alt screen and show cursor on drop
        // Use explicit flush after each critical operation to ensure
        // terminal state is properly restored even during panics
        let _ = self.exit_alt_screen();
        let _ = self.show_cursor();
        let _ = self.writer.flush();
    }
}

/// Helper to create PNG images from RGB/RGBA buffers
pub mod image_helpers {
    use image::{ImageBuffer, Rgb, Rgba};
    use std::io::Cursor;

    /// Encode RGB8 buffer to PNG
    pub fn rgb_to_png(width: u32, height: u32, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, data)
            .ok_or_else(|| anyhow::anyhow!("Invalid RGB buffer dimensions"))?;

        let mut png_data = Vec::new();
        img.write_to(&mut Cursor::new(&mut png_data), image::ImageFormat::Png)?;
        Ok(png_data)
    }

    /// Encode RGBA8 buffer to PNG
    pub fn rgba_to_png(width: u32, height: u32, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, data)
            .ok_or_else(|| anyhow::anyhow!("Invalid RGBA buffer dimensions"))?;

        let mut png_data = Vec::new();
        img.write_to(&mut Cursor::new(&mut png_data), image::ImageFormat::Png)?;
        Ok(png_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        // Should be able to create renderer
        let result = Renderer::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_image_helpers() {
        // Create simple 2x2 red image
        let data = vec![255, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0];

        let result = image_helpers::rgb_to_png(2, 2, &data);
        assert!(result.is_ok());

        let png = result.unwrap();
        // PNG header should start with magic bytes
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
