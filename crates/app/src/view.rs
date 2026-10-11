//! Window/GPU lifecycle and UI composition. No simulation rules.
use crate::{scene, session::Session, terrain, ui};
use genesis_renderer::{
    camera::Camera,
    gpu::{Gpu, WorldTarget},
};
use std::{
    error::Error,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{event::WindowEvent, window::Window};
mod preparation;
use preparation::{Pending, Tag};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct View {
    pub window: Arc<Window>,
    pub atmosphere: genesis_renderer::atmosphere::Atmosphere,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: Gpu,
    depth: wgpu::Texture,
    tone_map: genesis_renderer::tonemap::ToneMap,
    glass: genesis_renderer::glass::Glass,
    gui_state: egui_winit::State,
    gui_renderer: egui_wgpu::Renderer,
    pub drawable: bool,
    picker: Option<(u64, terrain::Picker)>,
    last_world_upload: Instant,
    terrain_cache: Option<(u64, Arc<terrain::Cache>)>,
    scene_pending: Option<Pending>,
    scene_epoch: u64,
    refresh_needed: bool,
    terrain_changed: bool,
    terrain_prepared: bool,
    started: Instant,
    scenery_buildings: Option<usize>,
    foliage_batches: Vec<usize>,
    scenery: Option<crate::landscape::Scenery>,
    scenery_camera: Option<glam::Mat4>,
    ground_batches: [usize; terrain::ground_cover::BATCHES],
    occupied: Vec<egui::Rect>,
    path_tool_active: bool,
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
        // egui supplies gamma-space premultiplied colors. Blend UI through an
        // UNORM view; keep the world's lighting and frosting in linear light.
        config.view_formats = vec![config.format.remove_srgb_suffix()];
        let mut gpu = Gpu::new(&adapter).await?;
        surface.configure(&gpu.device, &config);
        let tone_map = genesis_renderer::tonemap::ToneMap::new(
            &gpu,
            config.width,
            config.height,
            config.format,
        );
        let depth = gpu.multisample_texture(
            config.width,
            config.height,
            wgpu::TextureFormat::Depth32Float,
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
            config.format.remove_srgb_suffix(),
            egui_wgpu::RendererOptions::default(),
        );
        let glass =
            genesis_renderer::glass::Glass::new(&gpu, config.width, config.height, config.format);
        let foliage_batches = crate::landscape::meshes()
            .iter()
            .map(|mesh| gpu.instances.add_scenery(&gpu.device, mesh))
            .collect();
        let ground_batches = terrain::ground_cover::meshes()
            .map(|mesh| gpu.instances.add_scenery(&gpu.device, &mesh));
        Ok(Self {
            ground_batches,
            foliage_batches,
            scenery: None,
            scenery_camera: None,
            glass,
            occupied: Vec::new(),
            path_tool_active: false,
            picker: None,
            terrain_cache: None,
            scene_pending: None,
            scene_epoch: 0,
            refresh_needed: false,
            terrain_changed: false,
            terrain_prepared: false,
            started: Instant::now(),
            atmosphere: Default::default(),
            scenery_buildings: None,
            last_world_upload: Instant::now(),
            window,
            surface,
            config,
            gpu,
            depth,
            tone_map,
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
        self.glass
            .resize(&self.gpu, width, height, self.config.format);
        self.tone_map.resize(&self.gpu, width, height);
        self.depth = self
            .gpu
            .multisample_texture(width, height, wgpu::TextureFormat::Depth32Float);
    }

    pub fn input(&mut self, event: &WindowEvent, playing: bool) -> bool {
        // A focused toolbar button must not swallow camera keys or turn Space
        // into an accidental second build. There are no text editors in this HUD.
        if let WindowEvent::KeyboardInput { event, .. } = event
            && let winit::keyboard::PhysicalKey::Code(code) = event.physical_key
            && playing
            && (crate::input::Input::is_shortcut(code)
                || (code == winit::keyboard::KeyCode::Enter && self.path_tool_active))
        {
            return false;
        }
        self.gui_state.on_window_event(&self.window, event).consumed
    }

    fn ensure_terrain(&mut self, session: &Session, camera: &Camera) {
        let revision = session.terrain_revision;
        let key = terrain::lod::Key::at(camera.eye());
        let foot = genesis_sim_core::spatial::Position::new(
            camera.focus.x + session.sim.grid.width as f32 / 2.,
            camera.focus.z + session.sim.grid.height as f32 / 2.,
        );
        let needs_ground = camera.is_ground()
            && self.terrain_cache.as_ref().is_some_and(|(_, c)| {
                c.key != key && c.detail_at(foot) < genesis_sim_core::surface::SUBDIVISIONS
            });
        if needs_ground
            || self
                .terrain_cache
                .as_ref()
                .is_none_or(|(r, _)| *r != revision)
        {
            self.scene_epoch += 1;
            let cache = Arc::new(terrain::Cache::at(&session.sim, key));
            self.picker = Some((revision, terrain::Picker::from_cache(&session.sim, &cache)));
            self.terrain_cache = Some((revision, cache));
            self.terrain_changed = true;
            self.terrain_prepared = false;
            self.scenery_buildings = None;
        }
        let tag = Tag {
            revision,
            key,
            survey: session.survey,
            epoch: self.scene_epoch,
        };
        if let Some(pending) = &self.scene_pending {
            match pending.receiver.try_recv() {
                Ok(ready) => {
                    if pending.matches(tag, session.urgent_redraw) {
                        if let Some(picker) = ready.picker {
                            self.picker = Some((revision, picker));
                            self.terrain_changed = true;
                            self.terrain_prepared = true;
                        }
                        self.gpu.world.upload_prepared(
                            &self.gpu.device,
                            &self.gpu.queue,
                            &ready.world,
                        );
                        self.terrain_cache = Some((revision, ready.cache));
                        self.last_world_upload = Instant::now();
                    } else {
                        self.refresh_needed = true;
                    }
                    self.scene_pending = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.scene_pending = None;
                    self.refresh_needed = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.scene_pending.is_none()
            && self
                .terrain_cache
                .as_ref()
                .is_some_and(|(_, c)| c.key != key)
        {
            self.scene_pending = Some(Pending::spawn(session.sim.clone(), tag, None));
        }
    }

    pub fn pick(
        &mut self,
        session: &Session,
        camera: &Camera,
        cursor: glam::Vec2,
    ) -> Option<genesis_sim_core::spatial::Position> {
        self.ensure_terrain(session, camera);
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
    pub fn pick_object(
        &mut self,
        session: &Session,
        camera: &Camera,
        cursor: glam::Vec2,
        ground: Option<genesis_sim_core::spatial::Position>,
    ) -> Option<crate::interaction::Object> {
        let size = self.window.inner_size();
        let (origin, dir) = camera.ray(
            cursor,
            glam::Vec2::new(size.width as f32, size.height as f32),
        )?;
        let ground = ground.map(|p| {
            let mut v = terrain::point(&session.sim, p);
            if let Some((_, cache)) = &self.terrain_cache {
                v.y = cache.height_at(&session.sim, p);
            }
            if !session.survey.seafloor {
                v.y = v.y.max(session.sim.sea_level);
            }
            v
        });
        crate::interaction::pick(&session.sim, origin, dir, ground)
    }
    /// Native smoke tests also feed egui: macOS cursor warps need not emit movement.
    pub fn test_pointer(&mut self, cursor: glam::Vec2) {
        let scale = self.gui_state.egui_ctx().pixels_per_point();
        self.gui_state
            .egui_input_mut()
            .events
            .push(egui::Event::PointerMoved(egui::pos2(
                cursor.x / scale,
                cursor.y / scale,
            )));
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
        let mut custom_cursor = false;
        let output = context.run(self.gui_state.take_egui_input(&self.window), |ctx| {
            self.occupied = if front.playing() {
                ui::show(ctx, session, camera, front, &mut self.atmosphere)
            } else {
                front.show(ctx, session, camera)
            };
            if self.window.has_focus() || capture.is_some() {
                let world = front.playing()
                    && ctx
                        .pointer_hover_pos()
                        .is_some_and(|p| !self.occupied.iter().any(|r| r.contains(p)));
                custom_cursor = crate::cursor::paint(ctx, session, world);
            }
        });
        self.path_tool_active = session.path.active;
        self.gui_state
            .handle_platform_output(&self.window, output.platform_output);
        self.window.set_cursor_visible(!custom_cursor);
        if front.surface_backdrop() {
            self.ensure_terrain(session, camera);
            session.dirty |= std::mem::take(&mut self.refresh_needed);
            if self.terrain_changed {
                if let Some((_, cache)) = &self.terrain_cache {
                    self.gpu.contacts.upload(&self.gpu.queue, &cache.contacts);
                    for (batch, placements) in self.ground_batches.iter().zip(&cache.ground_cover) {
                        self.gpu.instances.upload(
                            &self.gpu.device,
                            &self.gpu.queue,
                            *batch,
                            placements,
                        );
                    }
                }
                if !self.terrain_prepared {
                    session.dirty = true;
                    session.urgent_redraw = true;
                }
                session.overlay_dirty = true;
                self.terrain_changed = false;
                self.terrain_prepared = false;
            }
        }
        if front.surface_backdrop()
            && session.dirty
            && (session.urgent_redraw
                || self.last_world_upload.elapsed()
                    >= Duration::from_millis(if session.survey.active() { 250 } else { 1000 }))
        {
            if self.scenery_buildings != Some(session.sim.buildings.len()) {
                self.scenery = Some(crate::landscape::Scenery::new(&session.sim));
                self.scenery_camera = None;
                self.scenery_buildings = Some(session.sim.buildings.len());
            }
            if session.urgent_redraw {
                // Explicit edits and initial/teleported views publish immediately.
                // An older worker result must never overwrite this newer state.
                self.scene_epoch += 1;
                if let Some((_, cache)) = &self.terrain_cache {
                    self.gpu.upload_scene(&scene::mesh(
                        &session.sim,
                        session.survey,
                        cache,
                        &cache.rivers,
                    ));
                }
                self.last_world_upload = Instant::now();
                session.dirty = false;
                session.urgent_redraw = false;
            } else if self.scene_pending.is_none()
                && let Some((revision, cache)) = &self.terrain_cache
            {
                self.scene_pending = Some(Pending::spawn(
                    session.sim.clone(),
                    Tag {
                        revision: *revision,
                        key: cache.key,
                        survey: session.survey,
                        epoch: self.scene_epoch,
                    },
                    Some(Arc::clone(cache)),
                ));
                self.last_world_upload = Instant::now();
                session.dirty = false;
            }
        }
        let aspect = self.config.width as f32 / self.config.height as f32;
        let matrix = camera.view_projection(aspect);
        if front.surface_backdrop() && self.scenery_camera != Some(matrix) {
            if let Some(scenery) = &self.scenery {
                let visible = scenery.visible(camera, aspect);
                for (i, (&batch, placements)) in
                    self.foliage_batches.iter().zip(&visible.main).enumerate()
                {
                    self.gpu
                        .instances
                        .upload(&self.gpu.device, &self.gpu.queue, batch, placements);
                    for (cascade, shadows) in visible.shadows.iter().enumerate() {
                        self.gpu.instances.upload_shadows(
                            &self.gpu.device,
                            &self.gpu.queue,
                            batch,
                            cascade,
                            &shadows[i],
                        );
                    }
                }
            }
            self.scenery_camera = Some(matrix);
        }
        if session.overlay_dirty {
            self.gpu.upload_overlay(&scene::overlay(
                session,
                self.terrain_cache.as_ref().map(|(_, cache)| cache.as_ref()),
            ));
            session.overlay_dirty = false;
        }
        let panels = if front.surface_backdrop() {
            ui::glass::panels(&output.shapes, output.pixels_per_point)
        } else {
            Vec::new()
        };
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
        let gui_target = capture_texture
            .as_ref()
            .unwrap_or(&frame.texture)
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.config.format.remove_srgb_suffix()),
                ..Default::default()
            });
        let mut encoder = self.gpu.encode(
            WorldTarget {
                size: [self.config.width, self.config.height],
                color: &self.tone_map.multisample,
                resolve: &self.tone_map.resolve,
                depth: &self.depth.create_view(&Default::default()),
            },
            camera,
            self.config.width as f32 / self.config.height as f32,
            if capture.is_some() {
                1.
            } else {
                self.started.elapsed().as_secs_f64()
            },
            genesis_renderer::atmosphere::Atmosphere {
                sea_level: session.sim.sea_level,
                survey: session.survey.active(),
                // Survey colours carry data; keep them free of aerial tint.
                density: if session.survey.active() {
                    0.
                } else {
                    self.atmosphere.density
                },
                ..self.atmosphere
            },
            capture.is_some(),
        );
        self.tone_map.composite(
            &self.gpu,
            &mut encoder,
            if panels.is_empty() {
                &target
            } else {
                &self.glass.scene
            },
            session.survey.active(),
        );
        if !panels.is_empty() {
            self.glass
                .composite(&self.gpu, &mut encoder, &target, &panels);
        }
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
                    view: &gui_target,
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
            if let Some(timings) = self.gpu.capture_timings()? {
                let [near, far, world, sky] = timings.map(|t| {
                    t.map_or_else(|| "unavailable".to_string(), |ms| format!("{ms:.3} ms"))
                });
                eprintln!(
                    "GPU capture passes: near shadows {near}; wide shadows {far}; world {world}; sky {sky} (excludes display/glass/UI)"
                );
            }
            let camera_volume =
                genesis_renderer::visibility::Frustum::new(camera.view_projection(aspect));
            let sun = std::array::from_fn::<_, { genesis_renderer::shadows::CASCADES }, _>(|i| {
                self.gpu.world.stats(
                    &genesis_renderer::visibility::Frustum::new(
                        genesis_renderer::shadows::Shadows::cascade_matrix(camera.focus, i),
                    ),
                    true,
                )
            });
            eprintln!(
                "world chunks: upload {:?}; camera {:?}; sun {:?}",
                self.gpu.world.last_upload,
                self.gpu.world.stats(&camera_volume, false),
                sun
            );
            eprintln!(
                "shadow scenery: {:?}",
                std::array::from_fn::<_, { genesis_renderer::shadows::CASCADES }, _>(|i| {
                    self.gpu.instances.shadow_stats(i)
                })
            );
        }
        frame.present();
        Ok(true)
    }
}
