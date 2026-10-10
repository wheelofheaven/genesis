mod gpu;

use genesis_lab_scene as scene;
use std::{error::Error, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct View {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: gpu::Gpu,
    depth: wgpu::Texture,
}

impl View {
    async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("surface has no supported configuration")?;
        config.format = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(|f| f.is_srgb())
            .ok_or("sRGB surface unavailable")?;
        let gpu = gpu::Gpu::new(&adapter, config.format).await?;
        surface.configure(&gpu.device, &config);
        let depth = gpu.texture(
            config.width,
            config.height,
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        Ok(Self {
            window,
            surface,
            config,
            gpu,
            depth,
        })
    }
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.gpu.device, &self.config);
        self.depth = self.gpu.texture(
            width,
            height,
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
    }
    fn draw(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        self.gpu.draw(
            &frame.texture.create_view(&Default::default()),
            &self.depth.create_view(&Default::default()),
            self.config.width,
            self.config.height,
        );
        frame.present();
        Ok(())
    }
}

#[derive(Default)]
struct App {
    view: Option<View>,
    error: Option<String>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        let result = (|| -> Result<View, Box<dyn Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Genesis engine lab — direct wgpu")
                        .with_inner_size(winit::dpi::PhysicalSize::new(
                            scene::WIDTH,
                            scene::HEIGHT,
                        )),
                )?,
            );
            pollster::block_on(View::new(window))
        })();
        match result {
            Ok(view) => {
                view.window.request_redraw();
                self.view = Some(view);
            }
            Err(error) => {
                self.error = Some(error.to_string());
                event_loop.exit();
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(view) = &mut self.view else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                view.resize(size.width, size.height);
                view.window.request_redraw();
            }
            WindowEvent::RedrawRequested => match view.draw() {
                Ok(()) | Err(wgpu::SurfaceError::Timeout) => {}
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    view.resize(view.config.width, view.config.height);
                    view.window.request_redraw();
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                    event_loop.exit();
                }
            },
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {}
        [flag, path] if flag == "--capture" => {
            return pollster::block_on(gpu::capture(std::path::Path::new(path)));
        }
        _ => return Err("usage: genesis-lab-custom [--capture output.png]".into()),
    }
    let event_loop = EventLoop::new()?;
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}
