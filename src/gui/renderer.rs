//! wgpu-based GUI renderer implementing the Renderer trait

use crate::render::{DirtyRegion, ImageParams, Renderer};
use crate::style::Style;
use anyhow::Result;
use glyphon::{
    Attrs, Buffer as GlyphonBuffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer as GlyphonText, Viewport, Weight,
};
use std::sync::Arc;
use wgpu;
use wgpu::util::DeviceExt;
use winit::window::Window;

/// Cell dimensions in pixels for the monospace grid
#[derive(Debug, Clone, Copy)]
struct CellSize {
    width: f32,
    height: f32,
}

/// A pending image draw operation
struct ImageEntry {
    data: Vec<u8>,
    width: u32,
    height: u32,
    dst_x: f32,
    dst_y: f32,
    dst_w: f32,
    dst_h: f32,
    is_rgba: bool,
}

/// Vertex for textured quad rendering
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlitVertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
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

    // Image rendering
    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    image_buffers: Vec<ImageEntry>,

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

const BLIT_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_texture, s_sampler, in.tex_coord);
}
"#;

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

        // Blit pipeline for image rendering
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit_shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("blit_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let blit_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("blit_pipeline_layout"),
                bind_group_layouts: &[&blit_bind_group_layout],
                immediate_size: 0,
            });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit_pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<BlitVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

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
            blit_pipeline,
            blit_bind_group_layout,
            blit_sampler,
            image_buffers: Vec::new(),
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

    /// Convert pixel position to NDC (normalized device coordinates)
    fn pixel_to_ndc(&self, x: f32, y: f32) -> (f32, f32) {
        let ndc_x = (x / self.surface_config.width as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (y / self.surface_config.height as f32) * 2.0;
        (ndc_x, ndc_y)
    }

    /// Flush accumulated images to the GPU
    fn flush_images(&mut self, view: &wgpu::TextureView) -> Result<()> {
        if self.image_buffers.is_empty() {
            return Ok(());
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("image_encoder"),
            });

        for entry in &self.image_buffers {
            // Convert RGB to RGBA if needed
            let rgba_data = if entry.is_rgba {
                entry.data.clone()
            } else {
                entry
                    .data
                    .chunks(3)
                    .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                    .collect()
            };

            let texture = self.device.create_texture_with_data(
                &self.queue,
                &wgpu::TextureDescriptor {
                    label: Some("image_texture"),
                    size: wgpu::Extent3d {
                        width: entry.width,
                        height: entry.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &rgba_data,
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("blit_bind_group"),
                layout: &self.blit_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                    },
                ],
            });

            // Build quad vertices in NDC
            let (x0, y0) = self.pixel_to_ndc(entry.dst_x, entry.dst_y);
            let (x1, y1) = self.pixel_to_ndc(entry.dst_x + entry.dst_w, entry.dst_y + entry.dst_h);

            let vertices = [
                BlitVertex { position: [x0, y0], tex_coord: [0.0, 0.0] },
                BlitVertex { position: [x1, y0], tex_coord: [1.0, 0.0] },
                BlitVertex { position: [x0, y1], tex_coord: [0.0, 1.0] },
                BlitVertex { position: [x1, y0], tex_coord: [1.0, 0.0] },
                BlitVertex { position: [x1, y1], tex_coord: [1.0, 1.0] },
                BlitVertex { position: [x0, y1], tex_coord: [0.0, 1.0] },
            ];

            let vertex_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("blit_vertices"),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("blit_pass"),
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

                pass.set_pipeline(&self.blit_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.draw(0..6, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.image_buffers.clear();

        Ok(())
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

    fn queue_image(&mut self, params: &ImageParams, is_rgba: bool) {
        let (px, py) = self.cell_to_pixel(params.col, params.row);
        let dst_w = params
            .width_cells
            .map(|c| c as f32 * self.cell_size.width)
            .unwrap_or(params.width as f32);
        let dst_h = params
            .height_cells
            .map(|r| r as f32 * self.cell_size.height)
            .unwrap_or(params.height as f32);

        self.image_buffers.push(ImageEntry {
            data: params.data.to_vec(),
            width: params.width,
            height: params.height,
            dst_x: px,
            dst_y: py,
            dst_w,
            dst_h,
            is_rgba,
        });
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
        self.image_buffers.clear();
        self.dirty.mark_all(self.cols, self.rows);
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn fill_rect(&mut self, bounds: crate::layout::Rect, color: crate::theme::Color) -> Result<()> {
        let (r, g, b) = color.to_rgb();
        // 1x1 RGBA pixel, scaled to fill the rect via the blit pipeline
        let pixel = vec![r, g, b, 255];
        let (px, py) = self.cell_to_pixel(bounds.x, bounds.y);
        let dst_w = bounds.width as f32 * self.cell_size.width;
        let dst_h = bounds.height as f32 * self.cell_size.height;

        self.image_buffers.push(ImageEntry {
            data: pixel,
            width: 1,
            height: 1,
            dst_x: px,
            dst_y: py,
            dst_w,
            dst_h,
            is_rgba: true,
        });
        Ok(())
    }

    fn render_image(&mut self, params: &ImageParams) -> Result<()> {
        self.queue_image(params, false);
        Ok(())
    }

    fn render_image_rgba(&mut self, params: &ImageParams) -> Result<()> {
        self.queue_image(params, true);
        Ok(())
    }

    fn clear_images(&mut self) -> Result<()> {
        self.image_buffers.clear();
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
        self.image_buffers.clear();
        self.cursor_col = 0;
        self.cursor_row = 0;

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| anyhow::anyhow!("Failed to get surface texture: {}", e))?;

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
            self.flush_images(&view)?;
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
