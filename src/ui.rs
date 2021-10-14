use wgpu::CommandEncoder;

use crate::GraphicsContext;

pub struct UiRenderer {}

impl UiRenderer {
    pub fn new(gfx: &GraphicsContext) -> Self {
        Self {}
    }

    pub fn draw(&self, command_encoder: &mut CommandEncoder, frame: &wgpu::TextureView) {}
}
