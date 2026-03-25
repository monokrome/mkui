//! wgpu-based GUI renderer implementing the Renderer trait

use crate::graphics::ImageParams;
use crate::render::{DirtyRegion, Renderer};
use crate::style::Style;
use anyhow::Result;
use glyphon::{
    Attrs, Buffer as GlyphonBuffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer as GlyphonText, Viewport, Weight,
};
use std::sync::Arc;
use wgpu;
use winit::window::Window;

/// Cell dimensions in pixels for the monospace grid
#[derive(Debug, Clone, Copy)]
struct CellSize {
    width: f32,
    height: f32,
}

/// GPU-accelerated renderer for native windowed applications
///
/// Implements the `Renderer` trait using wgpu for rendering and glyphon for
/// text. The underlying wgpu device, queue, and surface are accessible for
/// custom rendering (e.g., attached surfaces).
pub struct WgpuRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    // Text rendering
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: GlyphonText,
    viewport: Viewport,
    text_buffers: Vec<TextEntry>,

    // Grid state
    cell_size: CellSize,
    cols: u16,
    rows: u16,
    font_size: f32,
    cursor_col: u16,
    cursor_row: u16,
    cursor_visible: bool,

    // Frame state
    dirty: DirtyRegion,
    current_texture: Option<wgpu::SurfaceTexture>,
}

/// A pending text draw operation accumulated during a frame
struct TextEntry {
    text: String,
    col: u16,
    row: u16,
    style: Style,
}

impl WgpuRenderer {
    /// Create a new wgpu renderer for the given window
    ///
    /// The font size determines the cell grid dimensions.
    pub fn new(window: Arc<Window>, font_size: f32) -> Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("mkui"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            },
        ))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Text rendering setup
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
        let text_renderer = GlyphonText::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = Viewport::new(&device, &cache);

        // Measure monospace cell size
        let cell_size = measure_cell_size(&mut font_system, font_size);
        let cols = (surface_config.width as f32 / cell_size.width) as u16;
        let rows = (surface_config.height as f32 / cell_size.height) as u16;

        Ok(WgpuRenderer {
            device,
            queue,
            surface,
            surface_config,
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            viewport,
            text_buffers: Vec::new(),
            cell_size,
            cols,
            rows,
            font_size,
            cursor_col: 0,
            cursor_row: 0,
            cursor_visible: true,
            dirty: DirtyRegion::new(),
            current_texture: None,
        })
    }

    /// Get the wgpu device for custom rendering
    pub fn device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    /// Get the wgpu queue for custom rendering
    pub fn queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    /// Get the surface format
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    /// Handle window resize
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.cols = (width as f32 / self.cell_size.width) as u16;
        self.rows = (height as f32 / self.cell_size.height) as u16;
    }

    /// Convert cell position to pixel position
    fn cell_to_pixel(&self, col: u16, row: u16) -> (f32, f32) {
        (
            col as f32 * self.cell_size.width,
            row as f32 * self.cell_size.height,
        )
    }

    /// Flush accumulated text entries to the GPU
    fn flush_text(&mut self, view: &wgpu::TextureView) -> Result<()> {
        if self.text_buffers.is_empty() {
            return Ok(());
        }

        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        let mut text_areas = Vec::new();
        let mut glyph_buffers = Vec::new();

        for entry in &self.text_buffers {
            let (px, py) = self.cell_to_pixel(entry.col, entry.row);

            let metrics = Metrics::new(self.font_size, self.font_size * 1.2);
            let mut buffer = GlyphonBuffer::new(&mut self.font_system, metrics);

            let mut attrs = Attrs::new().family(Family::Monospace);
            if entry.style.bold == Some(true) {
                attrs = attrs.weight(Weight::BOLD);
            }

            buffer.set_text(
                &mut self.font_system,
                &entry.text,
                &attrs,
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);

            let color = style_to_glyphon_color(&entry.style);

            glyph_buffers.push((buffer, px, py, color));
        }

        for (buffer, px, py, color) in &glyph_buffers {
            text_areas.push(TextArea {
                buffer,
                left: *px,
                top: *py,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: self.surface_config.width as i32,
                    bottom: self.surface_config.height as i32,
                },
                default_color: *color,
                custom_glyphs: &[],
            });
        }

        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.text_atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .map_err(|e| anyhow::anyhow!("Text prepare failed: {:?}", e))?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("text_encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.text_renderer
                .render(&self.text_atlas, &self.viewport, &mut pass)
                .map_err(|e| anyhow::anyhow!("Text render failed: {:?}", e))?;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.text_buffers.clear();

        Ok(())
    }
}

impl Renderer for WgpuRenderer {
    fn write_text(&mut self, text: &str) -> Result<()> {
        self.text_buffers.push(TextEntry {
            text: text.to_string(),
            col: self.cursor_col,
            row: self.cursor_row,
            style: Style::new(),
        });
        self.cursor_col += text.chars().count() as u16;
        Ok(())
    }

    fn write_styled(&mut self, text: &str, style: &Style) -> Result<()> {
        self.text_buffers.push(TextEntry {
            text: text.to_string(),
            col: self.cursor_col,
            row: self.cursor_row,
            style: *style,
        });
        self.cursor_col += text.chars().count() as u16;
        Ok(())
    }

    fn write_repeated(&mut self, ch: char, count: usize) -> Result<()> {
        let text: String = std::iter::repeat_n(ch, count).collect();
        self.write_text(&text)
    }

    fn move_cursor(&mut self, col: u16, row: u16) -> Result<()> {
        self.cursor_col = col;
        self.cursor_row = row;
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<()> {
        self.cursor_visible = false;
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<()> {
        self.cursor_visible = true;
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        self.text_buffers.clear();
        self.dirty.mark_all(self.cols, self.rows);
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn render_image(&mut self, _params: &ImageParams) -> Result<()> {
        // Image rendering via wgpu textures — not yet implemented
        Ok(())
    }

    fn render_image_rgba(&mut self, _params: &ImageParams) -> Result<()> {
        // RGBA image rendering via wgpu textures — not yet implemented
        Ok(())
    }

    fn clear_images(&mut self) -> Result<()> {
        Ok(())
    }

    fn dimensions(&self) -> (u16, u16) {
        (self.cols, self.rows)
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
        self.text_buffers.clear();
        self.cursor_col = 0;
        self.cursor_row = 0;

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| anyhow::anyhow!("Failed to get surface texture: {}", e))?;

        // Clear to black
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clear_encoder"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.current_texture = Some(output);

        Ok(())
    }

    fn end_frame(&mut self) -> Result<()> {
        if let Some(output) = self.current_texture.take() {
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            self.flush_text(&view)?;
            output.present();
        }

        self.clear_dirty();
        Ok(())
    }
}

/// Measure the cell size for a monospace font at the given size
fn measure_cell_size(font_system: &mut FontSystem, font_size: f32) -> CellSize {
    let metrics = Metrics::new(font_size, font_size * 1.2);
    let mut buffer = GlyphonBuffer::new(font_system, metrics);

    let attrs = Attrs::new().family(Family::Monospace);
    buffer.set_text(font_system, "M", &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);

    let mut width = font_size * 0.6;
    for run in buffer.layout_runs() {
        if !run.glyphs.is_empty() {
            width = run.glyphs[0].w;
        }
    }

    CellSize {
        width,
        height: metrics.line_height,
    }
}

/// Convert mkui Style to glyphon color
fn style_to_glyphon_color(style: &Style) -> GlyphonColor {
    use crate::theme::Color;

    match style.fg {
        Some(Color::Rgb(r, g, b)) => GlyphonColor::rgb(r, g, b),
        Some(color) => {
            let (r, g, b) = color.to_rgb();
            GlyphonColor::rgb(r, g, b)
        }
        None => GlyphonColor::rgb(255, 255, 255),
    }
}
