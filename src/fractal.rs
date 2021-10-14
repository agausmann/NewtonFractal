use crate::GraphicsContext;
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec4};
use wgpu::util::DeviceExt;

const MAX_ROOTS: usize = 10;
const MAX_COEFFICIENTS: usize = 1 + MAX_ROOTS;

pub(crate) struct FractalRenderer {
    gfx: GraphicsContext,
    params_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl FractalRenderer {
    pub(crate) fn new(gfx: &GraphicsContext) -> Self {
        let params_buffer = gfx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FractalRenderer.params_buffer"),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                contents: Params::default()
                    .roots(&[
                        Root {
                            position: from_polar(0.5, 0.0_f32.to_radians()),
                            color: Vec4::new(0.0, 1.0, 1.0, 1.0) * 0.8,
                        },
                        Root {
                            position: from_polar(0.5, 120.0_f32.to_radians()),
                            color: Vec4::new(1.0, 0.0, 1.0, 1.0) * 0.8,
                        },
                        Root {
                            position: from_polar(0.5, 240.0_f32.to_radians()),
                            color: Vec4::new(1.0, 1.0, 0.0, 1.0) * 0.8,
                        },
                    ])
                    .as_bytes(),
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

    pub(crate) fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
    ) {
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

#[derive(Debug, Clone)]
pub struct Params {
    abi: ParamsAbi,
}

impl Params {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn num_iterations(mut self, num_iterations: u32) -> Self {
        self.abi.num_iterations = num_iterations;
        self
    }

    pub fn viewport(mut self, top_left: Vec2, bottom_right: Vec2) -> Self {
        self.abi.viewport_min = top_left.into();
        self.abi.viewport_max = bottom_right.into();
        self
    }

    pub fn roots(mut self, roots: &[Root]) -> Self {
        assert!(
            roots.len() < MAX_ROOTS,
            "too many roots, must be at most {}",
            MAX_ROOTS
        );
        self.abi.num_roots = roots.len() as _;

        for (slot, root) in self.abi.roots.iter_mut().zip(roots) {
            slot.position = root.position.into();
            slot.color = root.color.into();
        }

        // Compute coefficients:
        let mut p = [Vec2::ZERO; MAX_COEFFICIENTS];
        p[0] = Vec2::new(1.0, 0.0);
        for root in roots {
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
        for (slot, coef) in self.abi.coefficients.iter_mut().zip(p) {
            *slot = coef.into();
        }

        self
    }

    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(&self.abi)
    }
}

fn complex_mul(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x)
}

pub struct Root {
    pub position: Vec2,
    pub color: Vec4,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            abi: ParamsAbi {
                num_iterations: 10,
                _padding: [0u8; 4],
                viewport_min: [-1.0, 1.0],
                viewport_max: [1.0, -1.0],
                num_roots: 0,
                _padding_2: [0u8; 4],
                roots: [RootAbi {
                    color: [0.0; 4],
                    position: [0.0; 2],
                    _padding: [0; 8],
                }; MAX_ROOTS],
                coefficients: [[0.0; 2]; MAX_COEFFICIENTS],
            },
        }
    }
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

fn from_polar(r: f32, theta: f32) -> Vec2 {
    return r * Vec2::new(theta.cos(), theta.sin());
}
