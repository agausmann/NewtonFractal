use std::{sync::Arc, time::Instant};

use anyhow::Context;
use config::{Config, ConfigChangeEvent};
use fractal::FractalRenderer;
use pollster::block_on;
use ui::UiRenderer;
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub mod config;
pub mod fractal;
pub mod ui;

pub type Event<'a> = winit::event::Event<'a, AppEvent>;

pub enum AppEvent {
    ConfigChange(ConfigChangeEvent),
}

pub type GraphicsContext = Arc<GraphicsContextInner>;

pub struct GraphicsContextInner {
    pub window: Window,
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub render_format: wgpu::TextureFormat,
}

impl GraphicsContextInner {
    async fn new(window: Window) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .context("failed to create adapter")?;
        let (device, queue) = adapter
            .request_device(&Default::default(), None)
            .await
            .context("failed to create device")?;
        // let render_format = surface
        //     .get_preferred_format(&adapter)
        //     .context("failed to select a render format")?;
        let render_format = wgpu::TextureFormat::Rgba8Unorm;

        let out = Self {
            window,
            instance,
            surface,
            adapter,
            device,
            queue,
            render_format,
        };
        out.reconfigure();
        Ok(out)
    }

    fn reconfigure(&self) {
        let size = self.window.inner_size();
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.render_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
            },
        );
    }
}

pub struct App {
    gfx: GraphicsContext,
    fractal_renderer: FractalRenderer,
    ui_renderer: UiRenderer,
    last_frame: Instant,
    config: Config,
}

impl App {
    pub async fn new(window: Window, event_loop: &EventLoop<AppEvent>) -> anyhow::Result<Self> {
        let gfx = Arc::new(GraphicsContextInner::new(window).await?);
        let fractal_renderer = FractalRenderer::new(&gfx);
        let ui_renderer = UiRenderer::new(&gfx, event_loop);
        Ok(Self {
            gfx,
            fractal_renderer,
            ui_renderer,
            last_frame: Instant::now(),
            config: Default::default(),
        })
    }

    pub fn handle_event(&mut self, event: &Event, control_flow: &mut ControlFlow) {
        self.ui_renderer.handle_event(event);
        match event {
            Event::MainEventsCleared => {
                self.gfx.window.request_redraw();
            }
            Event::RedrawRequested(..) => {
                let now = Instant::now();
                let dt = now - self.last_frame;
                self.last_frame = now;
                self.ui_renderer.update(dt);

                self.redraw().unwrap();
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(..) | WindowEvent::ScaleFactorChanged { .. } => {
                    self.gfx.reconfigure();
                }
                _ => {}
            },
            Event::UserEvent(AppEvent::ConfigChange(config_change)) => {
                self.config.apply(config_change);
            }
            _ => {}
        }
    }

    fn redraw(&mut self) -> anyhow::Result<()> {
        let frame = loop {
            match self.gfx.surface.get_current_texture() {
                Ok(frame) => break frame,
                Err(wgpu::SurfaceError::Lost) => {
                    self.gfx.reconfigure();
                }
                Err(wgpu::SurfaceError::Timeout) | Err(wgpu::SurfaceError::Outdated) => {
                    return Ok(());
                }
                Err(err) => {
                    return Err(err.into());
                }
            }
        };

        let frame_view = frame.texture.create_view(&Default::default());
        let mut encoder = self.gfx.device.create_command_encoder(&Default::default());
        self.fractal_renderer
            .draw(&mut encoder, &frame_view, &self.config);
        self.ui_renderer
            .draw(&mut encoder, &frame_view, &self.config)?;
        self.gfx.queue.submit([encoder.finish()]);
        frame.present();

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::with_user_event();
    let window = WindowBuilder::new()
        .with_title("Newton Fractal")
        .build(&event_loop)
        .context("failed to create window")?;

    let mut app = block_on(App::new(window, &event_loop))?;

    event_loop.run(move |event, _, control_flow| {
        app.handle_event(&event, control_flow);
    });
}
