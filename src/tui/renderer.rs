//! Terminal rendering backend using ANSI escape sequences
//!
//! All terminal output goes through a background writer thread to prevent
//! blocking the main thread on slow connections (SSH). The renderer builds
//! frames into a buffer, then sends completed frames to the writer.

use crate::graphics::{GraphicsBackend, ImageRenderer};
use crate::render::{DirtyRegion, ImageParams, Renderer};
use crate::style::Style;
use crate::terminal::TerminalContext;
use anyhow::Result;
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;

/// Terminal rendering backend using ANSI escape sequences
///
/// Writes are buffered per-frame and flushed to a background writer thread
/// to prevent blocking the main thread on slow connections.
pub struct TerminalRenderer {
    /// Frame buffer — all writes go here during a frame
    buffer: Vec<u8>,
    /// Channel to send completed frames to the writer thread
    frame_tx: mpsc::SyncSender<Vec<u8>>,
    /// Handle to the writer thread
    _writer_thread: thread::JoinHandle<()>,
    context: TerminalContext,
    image_renderer: ImageRenderer,
    in_alt_screen: bool,
    dirty: DirtyRegion,
    scratch: String,
}

impl TerminalRenderer {
    /// Create a new renderer with detected terminal context and graphics backend
    pub fn new() -> Result<Self> {
        let context = TerminalContext::detect()?;
        let backend = GraphicsBackend::detect();
        let in_tmux = context.capabilities.in_multiplexer;

        // Bounded channel with 1 slot — writer takes latest frame, sender
        // drops the old frame if writer is still busy
        let (frame_tx, frame_rx) = mpsc::sync_channel::<Vec<u8>>(2);

        let writer_thread = thread::spawn(move || {
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            while let Ok(frame) = frame_rx.recv() {
                let _ = stdout.write_all(&frame);
                let _ = stdout.flush();
            }
        });

        Ok(TerminalRenderer {
            buffer: Vec::with_capacity(64 * 1024),
            frame_tx,
            _writer_thread: writer_thread,
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

        let (frame_tx, frame_rx) = mpsc::sync_channel::<Vec<u8>>(2);

        let writer_thread = thread::spawn(move || {
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            while let Ok(frame) = frame_rx.recv() {
                let _ = stdout.write_all(&frame);
                let _ = stdout.flush();
            }
        });

        Ok(TerminalRenderer {
            buffer: Vec::with_capacity(64 * 1024),
            frame_tx,
            _writer_thread: writer_thread,
            context,
            image_renderer: ImageRenderer::new(backend, in_tmux),
            in_alt_screen: false,
            dirty: DirtyRegion::new(),
            scratch: String::with_capacity(256),
        })
    }

    /// Create a renderer for headless/test environments.
    pub fn headless() -> Self {
        use crate::terminal::{TerminalContext, TerminalGeometry};

        let context =
            TerminalContext::with_geometry(TerminalGeometry::with_char_size(80, 24, 10, 20));
        let backend = GraphicsBackend::detect();

        let (frame_tx, frame_rx) = mpsc::sync_channel::<Vec<u8>>(2);

        let writer_thread = thread::spawn(move || {
            // Headless — discard all output
            while frame_rx.recv().is_ok() {}
        });

        TerminalRenderer {
            buffer: Vec::with_capacity(64 * 1024),
            frame_tx,
            _writer_thread: writer_thread,
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
    pub fn enter_alt_screen(&mut self) -> Result<()> {
        if !self.in_alt_screen {
            self.write_direct(b"\x1b[?1049h")?;
            self.in_alt_screen = true;
            let (cols, rows) = (self.context.geometry.cols, self.context.geometry.rows);
            self.dirty.mark_all(cols, rows);
        }
        Ok(())
    }

    /// Exit alternative screen buffer
    pub fn exit_alt_screen(&mut self) -> Result<()> {
        if self.in_alt_screen {
            self.write_direct(b"\x1b[?1049l\x1b[?25h")?;
            self.in_alt_screen = false;
            self.dirty.clear();
        }
        Ok(())
    }

    /// Write directly to stdout (bypasses frame buffer — for screen mode changes)
    fn write_direct(&self, data: &[u8]) -> Result<()> {
        let mut stdout = io::stdout().lock();
        stdout.write_all(data)?;
        stdout.flush()?;
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

    /// Force refresh tmux pane position
    pub fn refresh_pane_info(&mut self) {
        self.image_renderer.refresh_pane_info();
    }

    /// Enable or disable Unicode placeholder mode for graphics
    pub fn set_unicode_placeholders(&mut self, enabled: bool) {
        self.image_renderer.set_unicode_placeholders(enabled);
    }

    /// Check if running inside a terminal multiplexer
    pub fn in_multiplexer(&self) -> bool {
        self.context.capabilities.in_multiplexer
    }

    /// Begin a render frame with options
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

    /// Get the current frame buffer size (for checking if anything was rendered)
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Discard the current frame without sending it
    pub fn discard_frame(&mut self) {
        self.buffer.clear();
    }

    /// Get mutable access to the scratch buffer
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

    fn mark_image_dirty(&mut self, params: &ImageParams) {
        let w = params.width_cells.unwrap_or((params.width / 10) as u16);
        let h = params.height_cells.unwrap_or((params.height / 20) as u16);
        self.dirty.mark_region(params.col, params.row, w, h);
    }
}

impl Renderer for TerminalRenderer {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    #[inline]
    fn write_text(&mut self, text: &str) -> Result<()> {
        write!(self.buffer, "{}", text)?;
        Ok(())
    }

    #[inline]
    fn write_styled(&mut self, text: &str, style: &Style) -> Result<()> {
        let ansi = style.to_ansi();
        if ansi.is_empty() {
            write!(self.buffer, "{}", text)?;
        } else {
            write!(self.buffer, "{}{}\x1b[0m", ansi, text)?;
        }
        Ok(())
    }

    #[inline]
    fn write_repeated(&mut self, ch: char, count: usize) -> Result<()> {
        for _ in 0..count {
            write!(self.buffer, "{}", ch)?;
        }
        Ok(())
    }

    #[inline]
    fn move_cursor(&mut self, col: u16, row: u16) -> Result<()> {
        write!(self.buffer, "\x1b[{};{}H", row + 1, col + 1)?;
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<()> {
        write!(self.buffer, "\x1b[?25l")?;
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<()> {
        write!(self.buffer, "\x1b[?25h")?;
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        write!(self.buffer, "\x1b[2J")?;
        let (cols, rows) = (self.context.geometry.cols, self.context.geometry.rows);
        self.dirty.mark_all(cols, rows);
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if !self.buffer.is_empty() {
            let frame = std::mem::replace(&mut self.buffer, Vec::with_capacity(64 * 1024));
            // Blocking send — guarantees frame delivery. The bounded channel
            // (capacity 2) provides backpressure without dropping UI frames.
            let _ = self.frame_tx.send(frame);
        }
        Ok(())
    }

    fn fill_rect(&mut self, bounds: crate::layout::Rect, color: crate::theme::Color) -> Result<()> {
        let style = Style::new().bg(color);
        let ansi = style.to_ansi();
        let spaces: String = std::iter::repeat_n(' ', bounds.width as usize).collect();

        for row in 0..bounds.height {
            write!(self.buffer, "\x1b[{};{}H", bounds.y + row + 1, bounds.x + 1)?;
            write!(self.buffer, "{}{}\x1b[0m", ansi, spaces)?;
        }

        Ok(())
    }

    fn render_image(&mut self, params: &ImageParams) -> Result<()> {
        self.mark_image_dirty(params);
        self.image_renderer.render_image(&mut self.buffer, params)
    }

    fn render_image_rgba(&mut self, params: &ImageParams) -> Result<()> {
        self.mark_image_dirty(params);
        self.image_renderer
            .render_image_rgba(&mut self.buffer, params)
    }

    fn clear_images(&mut self) -> Result<()> {
        self.image_renderer.delete_all_images(&mut self.buffer)?;
        Ok(())
    }

    fn dimensions(&self) -> (u16, u16) {
        self.context.char_dimensions()
    }

    fn cell_aspect(&self) -> f32 {
        let geom = &self.context.geometry;
        if geom.char_width > 0 && geom.char_height > 0 {
            geom.char_height as f32 / geom.char_width as f32
        } else {
            2.0
        }
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
        self.buffer.clear();
        self.hide_cursor()?;
        Ok(())
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
