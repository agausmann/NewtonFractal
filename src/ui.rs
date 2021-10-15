use std::time::Duration;

use glam::{Vec3, Vec4Swizzles};
use imgui::Condition;
use imgui_winit_support::HiDpiMode;
use wgpu::CommandEncoder;
use winit::event_loop::{EventLoop, EventLoopProxy};

use crate::{
    config::{Config, ConfigChangeEvent},
    AppEvent, Event, GraphicsContext,
};

pub struct UiRenderer {
    gfx: GraphicsContext,
    imgui: imgui::Context,
    platform: imgui_winit_support::WinitPlatform,
    renderer: imgui_wgpu::Renderer,
    event_proxy: EventLoopProxy<AppEvent>,
}

impl UiRenderer {
    pub fn new(gfx: &GraphicsContext, event_loop: &EventLoop<AppEvent>) -> Self {
        let mut imgui = imgui::Context::create();
        imgui.style_mut().use_classic_colors();

        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), &gfx.window, HiDpiMode::Default);

        let mut renderer_config = imgui_wgpu::RendererConfig::default();
        renderer_config.texture_format = gfx.render_format;
        let renderer =
            imgui_wgpu::Renderer::new(&mut imgui, &gfx.device, &gfx.queue, renderer_config);

        Self {
            gfx: gfx.clone(),
            imgui,
            platform,
            renderer,
            event_proxy: event_loop.create_proxy(),
        }
    }

    pub fn update(&mut self, dt: Duration) {
        self.imgui.io_mut().update_delta_time(dt);
    }

    pub fn handle_event(&mut self, event: &Event) {
        self.platform
            .handle_event(self.imgui.io_mut(), &self.gfx.window, &event)
    }

    pub fn has_keyboard_focus(&self) -> bool {
        self.imgui.io().want_capture_keyboard
    }

    pub fn has_mouse_focus(&self) -> bool {
        self.imgui.io().want_capture_mouse
    }

    pub fn draw(
        &mut self,
        command_encoder: &mut CommandEncoder,
        frame: &wgpu::TextureView,
        config: &Config,
    ) -> anyhow::Result<()> {
        self.platform
            .prepare_frame(self.imgui.io_mut(), &self.gfx.window)?;

        let ui = self.imgui.frame();
        // Manually split borrow outside of closure:
        let event_proxy = &self.event_proxy;
        let config_change = |event| {
            event_proxy.send_event(AppEvent::ConfigChange(event)).ok();
        };

        imgui::Window::new("Config")
            .size([320.0, 400.0], Condition::FirstUseEver)
            .build(&ui, || {
                if ui.collapsing_header("Simulation", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                    let mut num_iterations = config.num_iterations as i32;
                    if ui
                        .input_int("Iterations", bytemuck::cast_mut(&mut num_iterations))
                        .step(1)
                        .build()
                    {
                        config_change(ConfigChangeEvent::NumIterations(num_iterations.max(0) as _));
                    }
                }
                if ui.collapsing_header("Camera", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                    let mut position = config.camera.position.to_array();
                    if ui.input_float2("Position", &mut position).build() {
                        config_change(ConfigChangeEvent::CameraPosition(position.into()));
                    };
                    let mut zoom = config.camera.zoom;
                    if ui
                        .input_float("Zoom", &mut zoom)
                        .step(config.camera.zoom * 0.01)
                        .build()
                    {
                        config_change(ConfigChangeEvent::CameraZoom(zoom));
                    };
                }

                if ui.collapsing_header("Roots", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                    for (i, root) in config.roots.iter().enumerate() {
                        imgui::TreeNode::new(&format!("{}", i + 1)).build(&ui, || {
                            let mut position = root.position.to_array();
                            if ui.input_float2("Position", &mut position).build() {
                                config_change(ConfigChangeEvent::RootPosition {
                                    index: i,
                                    position: position.into(),
                                });
                            }
                            let mut color = root.color.xyz().to_array();
                            if imgui::ColorEdit::new("Color", &mut color)
                                .alpha(false)
                                .build(&ui)
                            {
                                config_change(ConfigChangeEvent::RootColor {
                                    index: i,
                                    color: Vec3::from(color).extend(1.0),
                                });
                            }
                        });
                    }

                    if ui.button("Add Root") {
                        config_change(ConfigChangeEvent::AddRoot);
                    }
                }
            });

        self.platform.prepare_render(&ui, &self.gfx.window);

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UiRenderer.render_pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: frame,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        let draw_data = ui.render();
        self.renderer.render(
            draw_data,
            &self.gfx.queue,
            &self.gfx.device,
            &mut render_pass,
        )?;

        Ok(())
    }
}
