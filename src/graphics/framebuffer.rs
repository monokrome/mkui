//! Linux framebuffer rendering backend

use super::{GraphicsBackend, GraphicsRenderer};
use crate::render::ImageParams;
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

/// Linux framebuffer renderer
pub(super) struct FramebufferRenderer;

impl FramebufferRenderer {
    /// Render using Linux framebuffer
    fn render_framebuffer(
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

impl GraphicsRenderer for FramebufferRenderer {
    fn render_rgb(&mut self, _writer: &mut dyn Write, params: &ImageParams) -> Result<()> {
        self.render_framebuffer(params.data, params.width, params.height)
    }

    fn backend_type(&self) -> GraphicsBackend {
        GraphicsBackend::Framebuffer
    }
}
