//! Kitty graphics protocol rendering backend

use super::{get_diacritic, ImageRenderer, PLACEHOLDER_CHAR};
use anyhow::Result;
use std::fmt::Write as FmtWrite;
use std::io::Write;

impl ImageRenderer {
    /// Render using Kitty graphics protocol
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    pub(super) fn render_kitty<W: Write>(
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
        let png_data = crate::render::image_helpers::rgb_to_png(width, height, image_data)?;
        self.render_kitty_encoded(writer, &png_data, col, row, width_cells, height_cells)
    }

    /// Render using Kitty graphics protocol with RGBA (alpha transparency support)
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    pub(super) fn render_kitty_rgba<W: Write>(
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
        let png_data = crate::render::image_helpers::rgba_to_png(width, height, image_data)?;
        self.render_kitty_encoded(writer, &png_data, col, row, width_cells, height_cells)
    }

    /// Shared Kitty rendering: encode PNG to base64, transmit with a=T, fixed image ID
    #[allow(clippy::too_many_arguments)]
    fn render_kitty_encoded<W: Write>(
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
    fn render_kitty_direct<W: Write>(
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
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    fn render_kitty_tmux<W: Write>(
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
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    fn render_kitty_placeholder<W: Write>(
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
