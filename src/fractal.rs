use crate::{config::Config, GraphicsContext};
use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use wgpu::util::DeviceExt;

const MAX_ROOTS: usize = 10;
const MAX_COEFFICIENTS: usize = 1 + MAX_ROOTS;

pub struct FractalRenderer {
    gfx: GraphicsContext,
    params_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl FractalRenderer {
    pub fn new(gfx: &GraphicsContext) -> Self {
        let params_buffer = gfx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FractalRenderer.params_buffer"),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                contents: bytemuck::bytes_of(&ParamsAbi::from(&Config::default())),
            });
        let bind_group_layout =
            gfx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FractalRenderer.bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let pipeline_layout = gfx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("FractalRenderer.pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let shader_module = gfx
            .device
            .create_shader_module(&wgpu::include_wgsl!("fractal.wgsl"));
        let render_pipeline = gfx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("FractalRenderer.render_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "main",
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "main",
                    targets: &[wgpu::ColorTargetState {
                        format: gfx.render_format,
                        blend: None,
                        write_mask: Default::default(),
                    }],
                }),
            });
        let bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FractalRenderer.bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            }],
        });
        Self {
            gfx: gfx.clone(),
            params_buffer,
            render_pipeline,
            bind_group,
        }
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        config: &Config,
    ) {
        self.gfx.queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::bytes_of(&ParamsAbi::from(config)),
        );
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("FractalRenderer.render_pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: frame_view,
                resolve_target: None,
                ops: Default::default(),
            }],
            depth_stencil_attachment: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

fn complex_mul(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x)
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct ParamsAbi {
    num_iterations: u32,
    _padding: [u8; 4],
    viewport_min: [f32; 2],
    viewport_max: [f32; 2],
    num_roots: u32,
    _padding_2: [u8; 4],
    roots: [RootAbi; MAX_ROOTS],
    coefficients: [[f32; 2]; MAX_COEFFICIENTS],
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct RootAbi {
    color: [f32; 4],
    position: [f32; 2],
    _padding: [u8; 8],
}

impl From<&Config> for ParamsAbi {
    fn from(config: &Config) -> Self {
        assert!(
            config.roots.len() < MAX_ROOTS,
            "too many roots, must be at most {}",
            MAX_ROOTS
        );

        let mut roots = [RootAbi::zeroed(); MAX_ROOTS];
        let mut coefficients = [<[f32; 2]>::zeroed(); MAX_COEFFICIENTS];

        for (slot, root) in roots.iter_mut().zip(&config.roots) {
            slot.position = root.position.into();
            slot.color = root.color.into();
        }

        // Compute coefficients:
        let mut p = [Vec2::ZERO; MAX_COEFFICIENTS];
        p[0] = Vec2::new(1.0, 0.0);
        for root in &config.roots {
            let mut q = p.clone();
            // Multiply p by x (shift forward)
            for i in (1..MAX_COEFFICIENTS).rev() {
                p[i] = p[i - 1];
            }
            p[0] = Vec2::ZERO;
            // Multiply q by root
            for term in &mut q {
                *term = complex_mul(*term, root.position)
            }
            // Element-wise subtract q from p
            for (a, b) in p.iter_mut().zip(&q) {
                *a -= *b;
            }
        }
        for (slot, coef) in coefficients.iter_mut().zip(p) {
            *slot = coef.into();
        }

        Self {
            num_iterations: config.num_iterations,
            _padding: [0; 4],
            viewport_min: [-1.0, 1.0],
            viewport_max: [1.0, -1.0],
            num_roots: config.roots.len() as u32,
            _padding_2: [0; 4],
            roots,
            coefficients,
        }
    }
}
