//! Rendering abstraction - trait-based backend system for terminal and GUI targets
//!
//! The `Renderer` trait defines the interface that all rendering backends implement.
//! `TerminalRenderer` is the terminal backend using ANSI escape sequences.

use crate::graphics::{GraphicsBackend, ImageParams, ImageRenderer};
use crate::style::Style;
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

/// Backend-agnostic rendering interface
///
/// Components render through this trait, making them portable across
/// terminal and GUI backends.
pub trait Renderer {
    /// Write text at current cursor position
    fn write_text(&mut self, text: &str) -> Result<()>;

    /// Write text with visual style applied
    fn write_styled(&mut self, text: &str, style: &Style) -> Result<()>;

    /// Write a repeated character
    fn write_repeated(&mut self, ch: char, count: usize) -> Result<()>;

    /// Move cursor to position (0-indexed)
    fn move_cursor(&mut self, col: u16, row: u16) -> Result<()>;

    /// Hide cursor
    fn hide_cursor(&mut self) -> Result<()>;

    /// Show cursor
    fn show_cursor(&mut self) -> Result<()>;

    /// Clear the screen
    fn clear(&mut self) -> Result<()>;

    /// Flush output buffer
    fn flush(&mut self) -> Result<()>;

    /// Render an RGB image
    fn render_image(&mut self, params: &ImageParams) -> Result<()>;

    /// Render an RGBA image with alpha transparency
    fn render_image_rgba(&mut self, params: &ImageParams) -> Result<()>;

    /// Delete all tracked images
    fn clear_images(&mut self) -> Result<()>;

    /// Get the rendering surface dimensions (cols, rows)
    fn dimensions(&self) -> (u16, u16);

    /// Get current dirty region
    fn dirty_region(&self) -> &DirtyRegion;

    /// Mark a region as needing redraw
    fn mark_dirty(&mut self, col: u16, row: u16, width: u16, height: u16);

    /// Clear dirty region tracking (call after full render)
    fn clear_dirty(&mut self);

    /// Begin a render frame - hides cursor and prepares for rendering
    fn begin_frame(&mut self) -> Result<()>;

    /// End a render frame - shows cursor and flushes output
    fn end_frame(&mut self) -> Result<()>;
}

/// Terminal rendering backend using ANSI escape sequences
///
/// Uses internal write buffering to minimize syscalls and improve performance.
/// Call `flush()` after a batch of operations to ensure output is displayed.
pub struct TerminalRenderer {
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

impl TerminalRenderer {
    /// Create a new renderer with detected terminal context and graphics backend
    pub fn new() -> Result<Self> {
        let context = TerminalContext::detect()?;
        let backend = GraphicsBackend::detect();
        let in_tmux = context.capabilities.in_multiplexer;

        let stdout = io::stdout();
        let writer = BufWriter::with_capacity(WRITE_BUFFER_CAPACITY, stdout);

        Ok(TerminalRenderer {
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

        let stdout = io::stdout();
        let writer = BufWriter::with_capacity(WRITE_BUFFER_CAPACITY, stdout);

        Ok(TerminalRenderer {
            writer,
            context,
            image_renderer: ImageRenderer::new(backend, in_tmux),
            in_alt_screen: false,
            dirty: DirtyRegion::new(),
            scratch: String::with_capacity(256),
        })
    }

    /// Create a renderer with a fake terminal context for headless/test environments.
    ///
    /// Provides an 80x24 terminal with 10x20 character cell size and no real
    /// terminal I/O. Useful for unit testing components without a live terminal.
    pub fn headless() -> Self {
        use crate::terminal::{TerminalContext, TerminalGeometry};

        let context =
            TerminalContext::with_geometry(TerminalGeometry::with_char_size(80, 24, 10, 20));
        let backend = GraphicsBackend::detect();
        let stdout = io::stdout();
        let writer = BufWriter::with_capacity(WRITE_BUFFER_CAPACITY, stdout);

        TerminalRenderer {
            writer,
            context,
            image_renderer: ImageRenderer::new(backend, false),
            in_alt_screen: false,
            dirty: DirtyRegion::new(),
            scratch: String::with_capacity(256),
        }
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
            self.writer.flush()?;
            self.in_alt_screen = true;
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
            self.writer.flush()?;
            self.in_alt_screen = false;
            self.dirty.clear();
        }
        Ok(())
    }

    /// Get current terminal context
    pub fn context(&self) -> &TerminalContext {
        &self.context
    }

    /// Refresh terminal geometry (call after resize)
    pub fn refresh_geometry(&mut self) -> Result<()> {
        self.context.refresh_geometry()?;
        self.image_renderer.refresh_pane_info();
        Ok(())
    }

    /// Force refresh tmux pane position (call on pane switch)
    pub fn refresh_pane_info(&mut self) {
        self.image_renderer.refresh_pane_info();
    }

    /// Enable or disable Unicode placeholder mode for graphics
    pub fn set_unicode_placeholders(&mut self, enabled: bool) {
        self.image_renderer.set_unicode_placeholders(enabled);
    }

    /// Check if running inside a terminal multiplexer (tmux/screen)
    pub fn in_multiplexer(&self) -> bool {
        self.context.capabilities.in_multiplexer
    }

    /// Begin a render frame with options
    ///
    /// # Arguments
    /// * `clear_graphics` - If true, clears all tracked graphics images before rendering.
    pub fn begin_frame_with_options(&mut self, clear_graphics: bool) -> Result<()> {
        self.hide_cursor()?;
        if clear_graphics {
            self.clear_images()?;
        }
        Ok(())
    }

    /// Check if the renderer is in alternative screen mode
    pub fn in_alt_screen(&self) -> bool {
        self.in_alt_screen
    }

    /// Get mutable access to the scratch buffer for building strings
    pub fn scratch_buffer(&mut self) -> &mut String {
        self.scratch.clear();
        &mut self.scratch
    }

    /// Render a PNG image using Kitty graphics protocol (legacy method)
    #[deprecated(note = "Use render_image() instead for multi-backend support")]
    #[allow(clippy::too_many_arguments)]
    pub fn render_kitty_image(
        &mut self,
        png_data: &[u8],
        col: u16,
        row: u16,
        width_cells: Option<u16>,
        height_cells: Option<u16>,
    ) -> Result<()> {
        let img = image::load_from_memory(png_data)?;
        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();

        self.render_image(&ImageParams {
            data: &rgb,
            width,
            height,
            col,
            row,
            width_cells,
            height_cells,
        })
    }

    /// Mark the image region as dirty using provided or estimated cell dimensions.
    fn mark_image_dirty(&mut self, params: &ImageParams) {
        let w = params.width_cells.unwrap_or((params.width / 10) as u16);
        let h = params.height_cells.unwrap_or((params.height / 20) as u16);
        self.dirty.mark_region(params.col, params.row, w, h);
    }
}

impl Renderer for TerminalRenderer {
    #[inline]
    fn write_text(&mut self, text: &str) -> Result<()> {
        write!(self.writer, "{}", text)?;
        Ok(())
    }

    #[inline]
    fn write_styled(&mut self, text: &str, style: &Style) -> Result<()> {
        let ansi = style.to_ansi();
        if ansi.is_empty() {
            write!(self.writer, "{}", text)?;
        } else {
            write!(self.writer, "{}{}\x1b[0m", ansi, text)?;
        }
        Ok(())
    }

    #[inline]
    fn write_repeated(&mut self, ch: char, count: usize) -> Result<()> {
        for _ in 0..count {
            write!(self.writer, "{}", ch)?;
        }
        Ok(())
    }

    #[inline]
    fn move_cursor(&mut self, col: u16, row: u16) -> Result<()> {
        write!(self.writer, "\x1b[{};{}H", row + 1, col + 1)?;
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<()> {
        write!(self.writer, "\x1b[?25l")?;
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<()> {
        write!(self.writer, "\x1b[?25h")?;
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        write!(self.writer, "\x1b[2J")?;
        let (cols, rows) = (self.context.geometry.cols, self.context.geometry.rows);
        self.dirty.mark_all(cols, rows);
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    fn render_image(&mut self, params: &ImageParams) -> Result<()> {
        self.mark_image_dirty(params);
        self.image_renderer.render_image(&mut self.writer, params)
    }

    fn render_image_rgba(&mut self, params: &ImageParams) -> Result<()> {
        self.mark_image_dirty(params);
        self.image_renderer
            .render_image_rgba(&mut self.writer, params)
    }

    fn clear_images(&mut self) -> Result<()> {
        self.image_renderer.delete_all_images(&mut self.writer)?;
        Ok(())
    }

    fn dimensions(&self) -> (u16, u16) {
        self.context.char_dimensions()
    }

    fn dirty_region(&self) -> &DirtyRegion {
        &self.dirty
    }

    fn mark_dirty(&mut self, col: u16, row: u16, width: u16, height: u16) {
        self.dirty.mark_region(col, row, width, height);
    }

    fn clear_dirty(&mut self) {
        self.dirty.clear();
    }

    fn begin_frame(&mut self) -> Result<()> {
        self.begin_frame_with_options(true)
    }

    fn end_frame(&mut self) -> Result<()> {
        self.show_cursor()?;
        self.flush()?;
        self.clear_dirty();
        Ok(())
    }
}

impl Drop for TerminalRenderer {
    fn drop(&mut self) {
        let _ = self.exit_alt_screen();
        let _ = self.show_cursor();
        let _ = self.writer.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = TerminalRenderer::headless();
        assert_eq!(renderer.context.geometry.cols, 80);
        assert_eq!(renderer.context.geometry.rows, 24);
    }
}
