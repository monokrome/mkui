//! Linux framebuffer rendering backend

use super::ImageRenderer;
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

impl ImageRenderer {
    /// Render using Linux framebuffer
    pub(super) fn render_framebuffer(
        &self,
        image_data: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<()> {
        let mut fb = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open("/dev/fb0")?;

        fb.write_all(image_data)?;
        fb.flush()?;

        Ok(())
    }
}
