use std::{cell::Cell, sync::Arc};

use anyhow::Context;
use fractal::FractalRenderer;
use pollster::block_on;
use winit::{
    dpi::PhysicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub(crate) mod fractal;

pub(crate) type GraphicsContext = Arc<GraphicsContextInner>;

pub(crate) struct GraphicsContextInner {
    pub(crate) window: Window,
    pub(crate) instance: wgpu::Instance,
    pub(crate) surface: wgpu::Surface,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) render_format: wgpu::TextureFormat,
    pub(crate) surface_size: Cell<PhysicalSize<u32>>,
}

impl GraphicsContextInner {
    async fn new(window: Window) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: Default::default(),
                compatible_surface: Some(&surface),
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

pub(crate) struct App {
    gfx: GraphicsContext,
    fractal_renderer: FractalRenderer,
}

impl App {
    pub(crate) async fn new(window: Window) -> anyhow::Result<Self> {
        let gfx = Arc::new(GraphicsContextInner::new(window).await?);
        let fractal = FractalRenderer::new(&gfx);
        Ok(Self {
            gfx,
            fractal_renderer: fractal,
        })
    }

    pub(crate) fn redraw(&mut self) -> anyhow::Result<()> {
        let frame = loop {
            match self.gfx.surface.get_current_frame() {
                Ok(frame) => break frame.output,
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
        self.gfx.queue.submit([encoder.finish()]);

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
