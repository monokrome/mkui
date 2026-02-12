//! Unicode block character rendering backend (universal fallback)

use super::ImageRenderer;
use anyhow::Result;
use std::io::Write;

impl ImageRenderer {
    /// Render using Unicode block characters
    ///
    /// Optimized to batch character writes per line to reduce syscalls.
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    pub(super) fn render_blocks<W: Write>(
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
        let cell_width = width_cells.unwrap_or(width as u16 / 2) as u32;
        let cell_height = height_cells.unwrap_or(height as u16 / 4) as u32;

        if cell_width == 0 || cell_height == 0 {
            return Ok(());
        }

        let pixels_per_cell_x = width / cell_width;
        let pixels_per_cell_y = height / cell_height;

        const BLOCKS: [char; 8] = [' ', '░', '░', '▒', '▒', '▓', '▓', '█'];

        for cy in 0..cell_height {
            self.line_buffer.clear();

            for cx in 0..cell_width {
                let px = (cx * pixels_per_cell_x) as usize;
                let py = (cy * pixels_per_cell_y) as usize;

                if px < width as usize && py < height as usize {
                    let idx = (py * width as usize + px) * 3;

                    if idx + 2 < image_data.len() {
                        let r = image_data[idx] as u32;
                        let g = image_data[idx + 1] as u32;
                        let b = image_data[idx + 2] as u32;

                        let brightness = (r + g + b) / 3;
                        let block_idx = (brightness / 32).min(7) as usize;
                        self.line_buffer.push(BLOCKS[block_idx]);
                    } else {
                        self.line_buffer.push(' ');
                    }
                } else {
                    self.line_buffer.push(' ');
                }
            }

            write!(
                writer,
                "\x1b[{};{}H{}",
                row + cy as u16 + 1,
                col + 1,
                self.line_buffer
            )?;
        }

        Ok(())
    }
}
