//! Window/GPU lifecycle and UI composition. No simulation rules.
use crate::{scene, session::Session, terrain, ui};
use genesis_renderer::{camera::Camera, gpu::Gpu};
use std::{
    error::Error,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{event::WindowEvent, window::Window};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct View {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: Gpu,
    depth: wgpu::Texture,
    gui_state: egui_winit::State,
    gui_renderer: egui_wgpu::Renderer,
    pub drawable: bool,
    picker: Option<(u64, terrain::Picker)>,
    last_world_upload: Instant,
    terrain_cache: Option<(u64, terrain::Cache)>,
    started: Instant,
    scenery_cache: Vec<genesis_renderer::geometry::Vertex>,
    scenery_buildings: usize,
    occupied: Vec<egui::Rect>,
}

impl View {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
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
            .ok_or("surface configuration unavailable")?;
        config.desired_maximum_frame_latency = 1;
        config.format = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(|f| f.is_srgb())
            .ok_or("sRGB surface unavailable")?;
        let gpu = Gpu::new(&adapter, config.format).await?;
        surface.configure(&gpu.device, &config);
        let depth = gpu.texture(
            config.width,
            config.height,
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let context = egui::Context::default();
        ui::theme::install(&context);
        let gui_state = egui_winit::State::new(
            context,
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let gui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            config.format,
            egui_wgpu::RendererOptions::default(),
        );
        Ok(Self {
            occupied: Vec::new(),
            picker: None,
            terrain_cache: None,
            started: Instant::now(),
            scenery_cache: Vec::new(),
            scenery_buildings: 0,
            last_world_upload: Instant::now(),
            window,
            surface,
            config,
            gpu,
            depth,
            gui_state,
            gui_renderer,
            drawable: size.width > 0 && size.height > 0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.drawable = width > 0 && height > 0;
        if !self.drawable {
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

    pub fn input(&mut self, event: &WindowEvent, playing: bool) -> bool {
        // A focused toolbar button must not swallow camera keys or turn Space
        // into an accidental second build. There are no text editors in this HUD.
        if let WindowEvent::KeyboardInput { event, .. } = event
            && let winit::keyboard::PhysicalKey::Code(code) = event.physical_key
            && playing
            && crate::input::Input::is_shortcut(code)
        {
            return false;
        }
        self.gui_state.on_window_event(&self.window, event).consumed
    }

    pub fn pick(
        &mut self,
        session: &Session,
        camera: &Camera,
        cursor: glam::Vec2,
    ) -> Option<genesis_sim_core::spatial::Position> {
        if self
            .picker
            .as_ref()
            .is_none_or(|(revision, _)| *revision != session.terrain_revision)
        {
            self.picker = Some((session.terrain_revision, terrain::Picker::new(&session.sim)));
        }
        let size = self.window.inner_size();
        let (origin, dir) = camera.ray(
            cursor,
            glam::Vec2::new(size.width as f32, size.height as f32),
        )?;
        self.picker
            .as_ref()?
            .1
            .pick(origin, dir, !session.survey.seafloor)
    }
    pub fn pointer_over_ui(&self, cursor: glam::Vec2) -> bool {
        let scale = self.gui_state.egui_ctx().pixels_per_point();
        let p = egui::pos2(cursor.x / scale, cursor.y / scale);
        self.occupied.iter().any(|rect| rect.contains(p))
    }

    pub fn draw(
        &mut self,
        session: &mut Session,
        camera: &mut Camera,
        capture: Option<&Path>,
        front: &mut crate::frontend::Frontend,
    ) -> Result<bool> {
        if !self.drawable {
            return Ok(false);
        }
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.resize(self.config.width, self.config.height);
                return Ok(false);
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        let context = self.gui_state.egui_ctx().clone();
        context.set_zoom_factor(front.settings.scale);
        let output = context.run(self.gui_state.take_egui_input(&self.window), |ctx| {
            self.occupied = if front.playing() {
                ui::show(ctx, session, camera, front)
            } else {
                front.show(ctx, session, camera)
            };
        });
        self.gui_state
            .handle_platform_output(&self.window, output.platform_output);
        if front.surface_backdrop()
            && session.dirty
            && (session.urgent_redraw
                || self.last_world_upload.elapsed()
                    >= Duration::from_millis(if session.survey.active() { 250 } else { 1000 }))
        {
            if self
                .terrain_cache
                .as_ref()
                .is_none_or(|(revision, _)| *revision != session.terrain_revision)
            {
                self.scenery_cache.clear();
                self.terrain_cache =
                    Some((session.terrain_revision, terrain::Cache::new(&session.sim)));
            }
            if self.scenery_cache.is_empty()
                || self.scenery_buildings != session.sim.buildings.len()
            {
                self.scenery_cache.clear();
                crate::water::rivers(&session.sim, &mut self.scenery_cache);
                crate::landscape::cover(&session.sim, &mut self.scenery_cache);
                self.scenery_buildings = session.sim.buildings.len();
            }
            if let Some((_, cache)) = &self.terrain_cache {
                self.gpu.upload_scene(&scene::mesh(
                    &session.sim,
                    session.survey,
                    cache,
                    &self.scenery_cache,
                ));
            }
            self.last_world_upload = Instant::now();
            session.dirty = false;
            session.urgent_redraw = false;
        }
        if session.overlay_dirty {
            self.gpu.upload_overlay(&scene::overlay(session));
            session.overlay_dirty = false;
        }
        let jobs = context.tessellate(output.shapes, output.pixels_per_point);
        for (id, delta) in &output.textures_delta.set {
            self.gui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, delta);
        }
        let capture_texture = capture.map(|_| {
            self.gpu.texture(
                self.config.width,
                self.config.height,
                self.config.format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            )
        });
        let target = capture_texture
            .as_ref()
            .unwrap_or(&frame.texture)
            .create_view(&Default::default());
        let mut encoder = self.gpu.encode(
            &target,
            &self.depth.create_view(&Default::default()),
            camera.view_projection(self.config.width as f32 / self.config.height as f32),
            camera.direction(),
            if capture.is_some() {
                1.
            } else {
                self.started.elapsed().as_secs_f32()
            },
        );
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: output.pixels_per_point,
        };
        let callbacks = self.gui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &jobs,
            &screen,
        );
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("inspector"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.gui_renderer
                .render(&mut pass.forget_lifetime(), &jobs, &screen);
        }
        self.gpu
            .queue
            .submit(callbacks.into_iter().chain([encoder.finish()]));
        for id in &output.textures_delta.free {
            self.gui_renderer.free_texture(id);
        }
        if let (Some(texture), Some(path)) = (capture_texture.as_ref(), capture) {
            self.gpu.capture(texture, path)?;
        }
        frame.present();
        Ok(true)
    }
}
