//! Kitty graphics protocol rendering backend

use super::{
    GraphicsBackend, GraphicsRenderer, ImageParams, ESCAPE_BUFFER_CAPACITY, LINE_BUFFER_CAPACITY,
};
use anyhow::Result;
use image::{ImageBuffer, Rgb, Rgba};
use std::fmt::Write as FmtWrite;
use std::io::{Cursor, Write};

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

/// Encode RGB8 buffer to PNG
fn rgb_to_png(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>> {
    let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, data)
        .ok_or_else(|| anyhow::anyhow!("Invalid RGB buffer dimensions"))?;

    let mut png_data = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_data), image::ImageFormat::Png)?;
    Ok(png_data)
}

/// Encode RGBA8 buffer to PNG
fn rgba_to_png(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>> {
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, data)
        .ok_or_else(|| anyhow::anyhow!("Invalid RGBA buffer dimensions"))?;

    let mut png_data = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_data), image::ImageFormat::Png)?;
    Ok(png_data)
}

/// Kitty graphics protocol renderer
pub(super) struct KittyRenderer {
    in_tmux: bool,
    /// Pre-allocated buffer for building escape sequences
    line_buffer: String,
    /// Pre-allocated buffer for command parameters
    escape_buffer: String,
    /// Current animation frame number (for Kitty animation protocol)
    animation_image_id: Option<u32>,
    /// Whether the animation has been initialized (first frame sent)
    animation_initialized: bool,
    /// Cached tmux pane offset (refreshed on demand)
    tmux_pane_offset: Option<TmuxPaneOffset>,
}

impl KittyRenderer {
    pub(super) fn new(in_tmux: bool) -> Self {
        let tmux_pane_offset = if in_tmux {
            TmuxPaneOffset::query()
        } else {
            None
        };

        KittyRenderer {
            in_tmux,
            line_buffer: String::with_capacity(LINE_BUFFER_CAPACITY),
            escape_buffer: String::with_capacity(ESCAPE_BUFFER_CAPACITY),
            animation_image_id: None,
            animation_initialized: false,
            tmux_pane_offset,
        }
    }

    // ---- All methods below are identical to the original ImageRenderer impl ----
    // Only the struct name has changed. No behavioral changes.

    /// Render using Kitty graphics protocol
    #[allow(clippy::too_many_arguments)]
    fn render_kitty<W: Write + ?Sized>(
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
        let png_data = rgb_to_png(width, height, image_data)?;
        self.render_kitty_encoded(writer, &png_data, col, row, width_cells, height_cells)
    }

    /// Render using Kitty graphics protocol with RGBA (alpha transparency support)
    #[allow(clippy::too_many_arguments)]
    fn render_kitty_rgba<W: Write + ?Sized>(
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
        let png_data = rgba_to_png(width, height, image_data)?;
        self.render_kitty_encoded(writer, &png_data, col, row, width_cells, height_cells)
    }

    /// Shared Kitty rendering: encode PNG to base64, transmit with a=T, fixed image ID
    #[allow(clippy::too_many_arguments)]
    fn render_kitty_encoded<W: Write + ?Sized>(
        &mut self,
        writer: &mut W,
        png_data: &[u8],
        col: u16,
        row: u16,
        width_cells: Option<u16>,
        height_cells: Option<u16>,
    ) -> Result<()> {
        let encoded = self.encode_base64(png_data);
        let cols = width_cells.unwrap_or(40);
        let rows = height_cells.unwrap_or(10);
        let image_id: u32 = 1;

        self.escape_buffer.clear();
        write!(
            self.escape_buffer,
            "a=T,f=100,t=d,i={},c={},r={},C=1,q=2",
            image_id, cols, rows
        )
        .ok();

        const CHUNK_SIZE: usize = 4096;
        let total_chunks = encoded.len().div_ceil(CHUNK_SIZE);
        let cmd_str = self.escape_buffer.clone();

        if self.in_tmux {
            self.render_kitty_placeholder(writer, &encoded, image_id, cols, rows, col, row)?;
        } else {
            write!(writer, "\x1b[{};{}H", row + 1, col + 1)?;
            self.render_kitty_direct(writer, &encoded, &cmd_str, total_chunks)?;
        }

        self.animation_initialized = true;
        Ok(())
    }

    /// Render Kitty graphics directly (not in tmux)
    fn render_kitty_direct<W: Write + ?Sized>(
        &mut self,
        writer: &mut W,
        encoded: &str,
        cmd_str: &str,
        total_chunks: usize,
    ) -> Result<()> {
        const CHUNK_SIZE: usize = 4096;

        for (i, chunk) in encoded.as_bytes().chunks(CHUNK_SIZE).enumerate() {
            let is_first_chunk = i == 0;
            let is_last_chunk = i == total_chunks - 1;
            let m = if is_last_chunk { 0 } else { 1 };

            self.line_buffer.clear();

            if is_first_chunk {
                write!(self.line_buffer, "\x1b_G{},m={};", cmd_str, m).ok();
            } else {
                write!(self.line_buffer, "\x1b_Gm={};", m).ok();
            }

            // SAFETY: Base64 output is always valid ASCII/UTF-8
            self.line_buffer
                .push_str(unsafe { std::str::from_utf8_unchecked(chunk) });
            self.line_buffer.push_str("\x1b\\");

            write!(writer, "{}", self.line_buffer)?;
        }

        Ok(())
    }

    /// Render Kitty graphics through tmux passthrough (legacy fallback)
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    fn render_kitty_tmux<W: Write + ?Sized>(
        &mut self,
        writer: &mut W,
        encoded: &str,
        cmd_str: &str,
        total_chunks: usize,
        col: u16,
        row: u16,
    ) -> Result<()> {
        const CHUNK_SIZE: usize = 4096;

        for (i, chunk) in encoded.as_bytes().chunks(CHUNK_SIZE).enumerate() {
            let is_first_chunk = i == 0;
            let is_last_chunk = i == total_chunks - 1;
            let m = if is_last_chunk { 0 } else { 1 };

            self.line_buffer.clear();

            if is_first_chunk {
                write!(
                    self.line_buffer,
                    "\x1b[{};{}H\x1b_G{},m={};",
                    row + 1,
                    col + 1,
                    cmd_str,
                    m
                )
                .ok();
            } else {
                write!(self.line_buffer, "\x1b_Gm={};", m).ok();
            }

            // SAFETY: Base64 output is always valid ASCII/UTF-8
            self.line_buffer
                .push_str(unsafe { std::str::from_utf8_unchecked(chunk) });
            self.line_buffer.push_str("\x1b\\");

            let escaped = self.line_buffer.replace('\x1b', "\x1b\x1b");
            write!(writer, "\x1bPtmux;{}\x1b\\", escaped)?;
        }

        Ok(())
    }

    /// Render Kitty graphics using Unicode placeholders (for tmux compatibility)
    ///
    /// This is the recommended approach from Kitty documentation for tmux:
    /// 1. Transmit image data via passthrough with U=1 to enable virtual placement
    /// 2. Output placeholder characters (U+10EEEE) with diacritics as normal text
    /// 3. The image renders where the placeholder characters appear in the terminal
    #[allow(clippy::too_many_arguments)]
    fn render_kitty_placeholder<W: Write + ?Sized>(
        &mut self,
        writer: &mut W,
        encoded: &str,
        image_id: u32,
        cols: u16,
        rows: u16,
        col: u16,
        row: u16,
    ) -> Result<()> {
        const CHUNK_SIZE: usize = 4096;

        let total_chunks = encoded.len().div_ceil(CHUNK_SIZE);

        for (i, chunk) in encoded.as_bytes().chunks(CHUNK_SIZE).enumerate() {
            let is_first_chunk = i == 0;
            let is_last_chunk = i == total_chunks - 1;
            let m = if is_last_chunk { 0 } else { 1 };

            self.line_buffer.clear();

            if is_first_chunk {
                write!(
                    self.line_buffer,
                    "\x1b_Ga=T,f=100,t=d,i={},c={},r={},U=1,q=2,m={};",
                    image_id, cols, rows, m
                )
                .ok();
            } else {
                write!(self.line_buffer, "\x1b_Gm={};", m).ok();
            }

            // SAFETY: Base64 output is always valid ASCII/UTF-8
            self.line_buffer
                .push_str(unsafe { std::str::from_utf8_unchecked(chunk) });
            self.line_buffer.push_str("\x1b\\");

            let escaped = self.line_buffer.replace('\x1b', "\x1b\x1b");
            write!(writer, "\x1bPtmux;{}\x1b\\", escaped)?;
        }

        // Output Unicode placeholders as normal text
        write!(writer, "\x1b[{};{}H", row + 1, col + 1)?;

        if image_id < 256 {
            write!(writer, "\x1b[38;5;{}m", image_id)?;
        } else {
            let id_r = ((image_id >> 16) & 0xFF) as u8;
            let id_g = ((image_id >> 8) & 0xFF) as u8;
            let id_b = (image_id & 0xFF) as u8;
            write!(writer, "\x1b[38;2;{};{};{}m", id_r, id_g, id_b)?;
        }

        for r in 0..rows {
            if r > 0 {
                write!(writer, "\x1b[{};{}H", row + 1 + r, col + 1)?;
            }

            for c in 0..cols {
                let row_diacritic = get_diacritic(r as u8);
                let col_diacritic = get_diacritic(c as u8);
                write!(
                    writer,
                    "{}{}{}",
                    PLACEHOLDER_CHAR, row_diacritic, col_diacritic
                )?;
            }
        }

        write!(writer, "\x1b[39m")?;

        Ok(())
    }

    /// Encode raw bytes to base64 with pre-sized buffer
    fn encode_base64(&self, data: &[u8]) -> String {
        let encoded_size = (data.len() * 4 / 3) + 4;
        let mut encoded = String::with_capacity(encoded_size);
        base64::Engine::encode_string(
            &base64::engine::general_purpose::STANDARD,
            data,
            &mut encoded,
        );
        encoded
    }
}

impl GraphicsRenderer for KittyRenderer {
    fn render_rgb(&mut self, writer: &mut dyn Write, params: &ImageParams) -> Result<()> {
        self.render_kitty(
            writer,
            params.data,
            params.width,
            params.height,
            params.col,
            params.row,
            params.width_cells,
            params.height_cells,
        )
    }

    fn render_rgba(&mut self, writer: &mut dyn Write, params: &ImageParams) -> Result<()> {
        self.render_kitty_rgba(
            writer,
            params.data,
            params.width,
            params.height,
            params.col,
            params.row,
            params.width_cells,
            params.height_cells,
        )
    }

    fn delete_all_images(&mut self, writer: &mut dyn Write) -> Result<()> {
        self.reset_animation();

        let delete_cmd = "\x1b_Ga=d,d=I,i=1,q=2\x1b\\";

        if self.in_tmux {
            let escaped = delete_cmd.replace('\x1b', "\x1b\x1b");
            write!(writer, "\x1bPtmux;{}\x1b\\", escaped)?;
        } else {
            write!(writer, "{}", delete_cmd)?;
        }

        Ok(())
    }

    fn reset_animation(&mut self) {
        self.animation_image_id = None;
        self.animation_initialized = false;
    }

    fn refresh_pane_info(&mut self) {
        if self.in_tmux {
            self.tmux_pane_offset = TmuxPaneOffset::query();
        }
    }

    fn backend_type(&self) -> GraphicsBackend {
        GraphicsBackend::Kitty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_png() {
        let data = vec![255, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0];

        let result = rgb_to_png(2, 2, &data);
        assert!(result.is_ok());

        let png = result.unwrap();
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
