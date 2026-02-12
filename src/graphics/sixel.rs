//! Sixel graphics rendering backend

use super::ImageRenderer;
use anyhow::Result;
use std::io::Write;

impl ImageRenderer {
    /// Render using Sixel graphics
    #[allow(clippy::too_many_arguments)] // Image rendering requires position + dimensions
    pub(super) fn render_sixel<W: Write>(
        &mut self,
        writer: &mut W,
        image_data: &[u8],
        width: u32,
        height: u32,
        col: u16,
        row: u16,
    ) -> Result<()> {
        use image::{ImageBuffer, Rgb};

        let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, image_data.to_vec())
            .ok_or_else(|| anyhow::anyhow!("Invalid image data"))?;

        write!(writer, "\x1b[{};{}H", row + 1, col + 1)?;

        let sixel_data = encode_sixel(&img)?;

        if self.in_tmux {
            let escaped = sixel_data.replace('\x1b', "\x1b\x1b");
            write!(writer, "\x1bPtmux;{}\x1b\\", escaped)?;
        } else {
            write!(writer, "{}", sixel_data)?;
        }

        Ok(())
    }
}

/// Encode image to sixel format (simplified implementation)
fn encode_sixel(img: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>) -> Result<String> {
    let mut output = String::new();

    output.push_str("\x1bPq");

    let (width, height) = img.dimensions();

    for y in (0..height).step_by(6) {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];

            output.push_str(&format!(
                "#{};2;{};{};{}",
                1,
                r * 100 / 255,
                g * 100 / 255,
                b * 100 / 255
            ));
            output.push('#');
            output.push('1');
            output.push('?');
        }
        output.push('$');
        output.push('-');
    }

    output.push_str("\x1b\\");

    Ok(output)
}
