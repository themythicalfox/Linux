//! wgpu renderer for the [`DrawList`] (compiled only with the `gpu` feature).
//!
//! The design is deliberately simple and fast: every primitive becomes one
//! instance of a single full-screen-aligned quad, and a signed-distance-field
//! fragment shader draws rounded rectangles, soft glows and ring/arc segments
//! from per-instance parameters. One pipeline, one draw call per frame — which
//! is what keeps the dock/wheel/lock overlays cheap enough to animate at 120+
//! FPS alongside a game.
//!
//! The CPU-side packing ([`pack_instances`]) is separated out and unit-tested
//! without a GPU; only [`Renderer`] itself needs a device.

use crate::scene::{DrawList, Primitive};
use bytemuck::{Pod, Zeroable};

/// Shape discriminator handed to the shader per instance.
const SHAPE_RECT: u32 = 0;
const SHAPE_GLOW: u32 = 1;
const SHAPE_ARC: u32 = 2;

/// Per-instance data uploaded to the GPU. One quad is expanded to cover `rect`
/// (or the glow/arc bounding box) and the shader does the SDF work.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct InstanceRaw {
    /// Bounding box in logical pixels: x, y, w, h.
    pub bounds: [f32; 4],
    /// Fill/primary color, straight sRGB.
    pub color: [f32; 4],
    /// Border color, straight sRGB.
    pub border: [f32; 4],
    /// Packed params: corner radius, border width, blur, intensity.
    pub params: [f32; 4],
    /// Arc params: start angle, sweep (radians), thickness, unused.
    pub arc: [f32; 4],
    /// shape tag + padding to keep 16-byte alignment.
    pub shape: [u32; 4],
}

impl InstanceRaw {
    /// wgpu vertex-buffer layout for the instance stream.
    #[cfg(feature = "gpu")]
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;
        // 6 vec4 slots at locations 1..=6 (location 0 is the quad corner).
        const ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
            1 => Float32x4, 2 => Float32x4, 3 => Float32x4,
            4 => Float32x4, 5 => Float32x4, 6 => Uint32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

/// Convert a [`DrawList`] into a flat instance buffer. Text and particle
/// primitives are skipped here (text goes through a separate glyph pass, and
/// particles are uploaded as point sprites); everything else becomes one
/// [`InstanceRaw`]. Pure and unit-tested without a GPU.
pub fn pack_instances(list: &DrawList) -> Vec<InstanceRaw> {
    let mut out = Vec::with_capacity(list.primitives.len());
    for p in &list.primitives {
        match p {
            Primitive::RoundedRect { rect, radius, fill, border, border_width, blur } => {
                out.push(InstanceRaw {
                    bounds: [rect.x, rect.y, rect.w, rect.h],
                    color: fill.to_array(),
                    border: border.to_array(),
                    params: [*radius, *border_width, *blur, 0.0],
                    arc: [0.0; 4],
                    shape: [SHAPE_RECT, 0, 0, 0],
                });
            }
            Primitive::Glow { center, radius, color, intensity } => {
                let r = *radius;
                out.push(InstanceRaw {
                    bounds: [center.x - r, center.y - r, r * 2.0, r * 2.0],
                    color: color.to_array(),
                    border: [0.0; 4],
                    params: [r, 0.0, 0.0, *intensity],
                    arc: [0.0; 4],
                    shape: [SHAPE_GLOW, 0, 0, 0],
                });
            }
            Primitive::Arc { center, radius, thickness, start_deg, sweep_deg, color } => {
                let r = *radius + *thickness;
                out.push(InstanceRaw {
                    bounds: [center.x - r, center.y - r, r * 2.0, r * 2.0],
                    color: color.to_array(),
                    border: [0.0; 4],
                    params: [*radius, 0.0, 0.0, 1.0],
                    arc: [start_deg.to_radians(), sweep_deg.to_radians(), *thickness, 0.0],
                    shape: [SHAPE_ARC, 0, 0, 0],
                });
            }
            // Text and particles are handled by dedicated passes.
            Primitive::Text { .. } | Primitive::Particles(_) => {}
        }
    }
    out
}

/// Screen uniform: logical resolution, used by the vertex shader to map pixels
/// to clip space.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ScreenUniform {
    pub resolution: [f32; 2],
    pub _pad: [f32; 2],
}

/// The WGSL for the whole UI pass. A unit quad is expanded per instance and an
/// SDF picks the shape. Kept inline so the crate carries its own shaders.
pub const SHADER: &str = include_str!("../../../assets/shaders/ui.wgsl");

// ---------------------------------------------------------------------------
// GPU-bound part. Everything below needs a wgpu device/surface.
// ---------------------------------------------------------------------------

/// A drawable target plus the pipeline state needed to render [`DrawList`]s.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    quad: wgpu::Buffer,
    instances: wgpu::Buffer,
    instance_cap: u64,
    screen_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Renderer {
    /// Build a renderer for `target` at `width`x`height`. `target` must outlive
    /// the renderer (`'static` surface); the compositor owns the window/output.
    pub fn new<T>(target: T, width: u32, height: u32) -> Result<Self, RenderError>
    where
        T: wgpu::WindowHandle + 'static,
    {
        pollster::block_on(Self::new_async(target, width, height))
    }

    async fn new_async<T>(target: T, width: u32, height: u32) -> Result<Self, RenderError>
    where
        T: wgpu::WindowHandle + 'static,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let boxed: Box<dyn wgpu::WindowHandle> = Box::new(target);
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Window(boxed))
            .map_err(|e| RenderError::Surface(e.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| RenderError::Adapter(e.to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("archon-ui device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| RenderError::Device(e.to_string()))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 1, // low input latency
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("archon-ui shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        // A unit quad (two triangles) shared by every instance.
        let quad_verts: [[f32; 2]; 6] = [
            [0.0, 0.0], [1.0, 0.0], [1.0, 1.0],
            [0.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        ];
        let quad = create_buffer(&device, "quad", bytemuck::cast_slice(&quad_verts), wgpu::BufferUsages::VERTEX);

        let instance_cap = 1024;
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: instance_cap * std::mem::size_of::<InstanceRaw>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let screen = ScreenUniform { resolution: [width as f32, height as f32], _pad: [0.0; 2] };
        let screen_buf = create_buffer(&device, "screen", bytemuck::bytes_of(&screen), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let bind_layout: wgpu::BindGroupLayout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen bind layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen bind"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("archon-ui layout"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });

        let quad_layout = wgpu::VertexBufferLayout {
            array_stride: (std::mem::size_of::<[f32; 2]>()) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("archon-ui pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[quad_layout, InstanceRaw::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Renderer {
            device,
            queue,
            surface,
            config,
            pipeline,
            quad,
            instances,
            instance_cap,
            screen_buf,
            bind_group,
        })
    }

    /// React to an output resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        let screen = ScreenUniform { resolution: [width as f32, height as f32], _pad: [0.0; 2] };
        self.queue.write_buffer(&self.screen_buf, 0, bytemuck::bytes_of(&screen));
    }

    /// Draw one frame from `list`. Returns the number of instances drawn.
    pub fn render(&mut self, list: &DrawList) -> Result<usize, RenderError> {
        let raw = pack_instances(list);
        let count = raw.len().min(self.instance_cap as usize);
        if count > 0 {
            self.queue
                .write_buffer(&self.instances, 0, bytemuck::cast_slice(&raw[..count]));
        }

        // wgpu 29 returns a status enum rather than a Result; treat a suboptimal
        // frame as usable and anything else as "skip this frame".
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            other => return Err(RenderError::Surface(format!("{other:?}"))),
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("archon-ui encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("archon-ui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if count > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, self.quad.slice(..));
                pass.set_vertex_buffer(1, self.instances.slice(..));
                pass.draw(0..6, 0..count as u32);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(count)
    }
}

#[cfg(feature = "gpu")]
fn create_buffer(device: &wgpu::Device, label: &str, data: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: data,
        usage,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("failed to create surface: {0}")]
    Surface(String),
    #[error("no suitable GPU adapter: {0}")]
    Adapter(String),
    #[error("failed to create device: {0}")]
    Device(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{DrawList, Rect, Vec2};
    use archon_theme::Color;

    #[test]
    fn packs_rect_and_glow_skips_text() {
        let mut dl = DrawList::new();
        dl.glass_panel(Rect::new(0.0, 0.0, 64.0, 400.0), 14.0, Color::rgb(18, 18, 22), Color::rgb(255, 122, 26), 24.0);
        dl.glow_dot(Vec2::new(8.0, 200.0), 6.0, Color::rgb(255, 122, 26), 1.5);
        dl.push(Primitive::Text { pos: Vec2::new(0.0, 0.0), content: "hi".into(), size: 12.0, color: Color::rgb(255, 255, 255) });
        let packed = pack_instances(&dl);
        assert_eq!(packed.len(), 2); // text skipped
        assert_eq!(packed[0].shape[0], SHAPE_RECT);
        assert_eq!(packed[1].shape[0], SHAPE_GLOW);
    }

    #[test]
    fn glow_bounds_are_centered() {
        let mut dl = DrawList::new();
        dl.glow_dot(Vec2::new(100.0, 100.0), 10.0, Color::rgb(255, 122, 26), 1.0);
        let p = pack_instances(&dl)[0];
        assert_eq!(p.bounds, [90.0, 90.0, 20.0, 20.0]);
    }
}
