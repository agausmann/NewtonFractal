use std::{cell::Cell, sync::Arc};

use anyhow::Context;
use fractal::FractalRenderer;
use pollster::block_on;
use ui::UiRenderer;
use winit::{
    dpi::PhysicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub mod fractal;
pub mod ui;

pub type GraphicsContext = Arc<GraphicsContextInner>;

pub struct GraphicsContextInner {
    pub window: Window,
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub render_format: wgpu::TextureFormat,
    pub surface_size: Cell<PhysicalSize<u32>>,
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

        let surface_size = Cell::new(window.inner_size());

        let out = Self {
            window,
            instance,
            surface,
            adapter,
            device,
            queue,
            render_format,
            surface_size,
        };
        out.reconfigure();
        Ok(out)
    }

    fn reconfigure(&self) {
        let size = self.window.inner_size();
        self.surface_size.set(size);
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
}

impl App {
    pub async fn new(window: Window) -> anyhow::Result<Self> {
        let gfx = Arc::new(GraphicsContextInner::new(window).await?);
        let fractal_renderer = FractalRenderer::new(&gfx);
        let ui_renderer = UiRenderer::new(&gfx);
        Ok(Self {
            gfx,
            fractal_renderer,
            ui_renderer,
        })
    }

    pub fn redraw(&mut self) -> anyhow::Result<()> {
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
        self.fractal_renderer.draw(&mut encoder, &frame_view);
        self.ui_renderer.draw(&mut encoder, &frame_view);
        self.gfx.queue.submit([encoder.finish()]);
        frame.present();

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Newton Fractal")
        .build(&event_loop)
        .context("failed to create window")?;

    let mut app = block_on(App::new(window))?;

    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            *control_flow = ControlFlow::Wait;
            app.gfx.window.request_redraw();
        }
        Event::RedrawRequested(..) => {
            app.redraw().unwrap();
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::Resized(..) | WindowEvent::ScaleFactorChanged { .. } => {
                app.gfx.reconfigure();
            }
            _ => {}
        },
        _ => {}
    });
}
