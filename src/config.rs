use glam::{Vec2, Vec4};

pub struct Config {
    pub num_iterations: u32,
    pub roots: Vec<RootConfig>,
    pub camera: CameraConfig,
}

impl Config {
    pub fn apply(&mut self, event: &ConfigChangeEvent) {
        match event {
            &ConfigChangeEvent::NumIterations(v) => {
                self.num_iterations = v;
            }
            &ConfigChangeEvent::AddRoot => {
                self.roots.push(Default::default());
            }
            &ConfigChangeEvent::RemoveRoot { index } => {
                self.roots.remove(index);
            }
            &ConfigChangeEvent::RootPosition { index, position } => {
                if let Some(root) = self.roots.get_mut(index) {
                    root.position = position;
                }
            }
            &ConfigChangeEvent::RootColor { index, color } => {
                if let Some(root) = self.roots.get_mut(index) {
                    root.color = color;
                }
            }
            &ConfigChangeEvent::CameraPosition(v) => {
                self.camera.position = v;
            }
            &ConfigChangeEvent::CameraZoom(v) => {
                self.camera.zoom = v;
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            num_iterations: 30,
            roots: vec![
                RootConfig {
                    position: Vec2::new(0.5, 0.0),
                    color: Vec4::new(0.0, 0.75, 0.0, 1.0),
                },
                RootConfig {
                    position: Vec2::new(-0.5, 0.0),
                    color: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
            ],
            camera: Default::default(),
        }
    }
}

pub struct RootConfig {
    pub position: Vec2,
    pub color: Vec4,
}

impl Default for RootConfig {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            color: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }
}

pub struct CameraConfig {
    pub position: Vec2,
    pub zoom: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

pub enum ConfigChangeEvent {
    NumIterations(u32),
    AddRoot,
    RemoveRoot { index: usize },
    RootPosition { index: usize, position: Vec2 },
    RootColor { index: usize, color: Vec4 },
    CameraPosition(Vec2),
    CameraZoom(f32),
}
