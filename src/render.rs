//! Rendering abstraction - trait-based backend system for terminal and GUI targets
//!
//! The `Renderer` trait defines the interface that all rendering backends implement.
//! Backend implementations live in `tui::TerminalRenderer` and `gui::WgpuRenderer`.

use crate::style::Style;
use anyhow::Result;

/// Parameters for rendering an image
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

    /// Fill a rectangle with a solid color
    fn fill_rect(&mut self, bounds: crate::layout::Rect, color: crate::theme::Color) -> Result<()>;

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
