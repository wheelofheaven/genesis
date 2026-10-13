mod benchmark;
mod clock;
mod cursor;
mod dwelling;
mod extraction_benchmark;
mod frontend;
mod ground;
mod horizon;
mod input;
mod interaction;
mod interaction_smoke;
mod landscape;
mod menu_smoke;
mod path_tool;
mod persistence;
#[cfg(test)]
mod render_tests;
mod save_smoke;
mod scene;
mod scenery_capture;
mod selection;
mod session;
mod survey;
mod terrain;
mod ui;
mod view;
mod water;
mod world_smoke;

use genesis_renderer::camera::Camera;
use glam::Vec2;
use input::Input;
use session::Session;
use std::{
    error::Error,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use view::View;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Default)]
struct App {
    view: Option<View>,
    atmosphere: genesis_renderer::atmosphere::Atmosphere,
    front: frontend::Frontend,
    session: Session,
    camera: Camera,
    input: Input,
    last_frame: Option<Instant>,
    hidden: bool,
    error: Option<String>,
    capture: Option<PathBuf>,
    smoke: bool,
    save_smoke: bool,
    survey_smoke: bool,
    world_smoke: bool,
    interaction_smoke: bool,
    frames: u32,
    menu_smoke: Option<menu_smoke::Check>,
}

impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl ToString) {
        self.error = Some(error.to_string());
        event_loop.exit();
    }
    fn frame(&mut self) -> Result<bool> {
        let Some(view) = &mut self.view else {
            return Ok(false);
        };
        let now = Instant::now();
        let elapsed = self
            .last_frame
            .replace(now)
            .map_or(Duration::ZERO, |last| now - last);
        if self.hidden || !view.drawable {
            return Ok(false);
        }
        self.frames += 1;
        if self.menu_smoke.is_some() && [18, 24].contains(&self.frames) {
            let (width, height) = if self.frames == 18 {
                (760, 540)
            } else {
                (1280, 800)
            };
            let _ = view
                .window
                .request_inner_size(winit::dpi::LogicalSize::new(width, height));
        }
        let menu_capture = if let Some(check) = &mut self.menu_smoke {
            check
                .advance(
                    self.frames,
                    &mut self.front,
                    &mut self.session,
                    &mut self.camera,
                )?
                .and_then(|suffix| self.capture.as_ref().map(|p| p.with_extension(suffix)))
        } else {
            None
        };
        if self.world_smoke {
            world_smoke::advance(self.frames, &mut self.session, &mut self.camera)?;
        }
        self.front.poll(&mut self.session, &mut self.camera);
        if self.front.playing() {
            self.session.advance(elapsed);
            let speed = if self.camera.is_ground() {
                1.
            } else {
                self.front.settings.sensitivity
            };
            self.input.update_camera(
                &self.session.sim,
                &mut self.camera,
                elapsed.as_secs_f32() * speed,
                view.window.inner_size().height as f32,
            );
        } else {
            self.input.clear();
            self.session.set_hover(None);
        }
        if self.smoke && !self.survey_smoke && self.frames >= 8 {
            if self.frames == 8 {
                self.session.set_placing(true);
            }
            let p = terrain::surface_point(
                &self.session.sim,
                genesis_sim_core::spatial::Position::new(22.4, 20.4),
            );
            let size = view.window.inner_size();
            let ndc = self
                .camera
                .view_projection(size.width as f32 / size.height as f32)
                .project_point3(p);
            self.input.cursor = Some(Vec2::new(
                (ndc.x + 1.) * 0.5 * size.width as f32,
                (1. - ndc.y) * 0.5 * size.height as f32,
            ));
        }
        if self.survey_smoke && self.frames == 8 {
            self.session.set_survey(true);
            self.session.survey.minerals = true;
            self.session.refresh_survey();
            let site = genesis_sim_core::spatial::Position::new(18.25, 21.5);
            self.session.select(site);
            let before = self.session.sim.subsurface.reading(site).stored;
            self.session
                .queue(genesis_sim_core::actions::Action::DrillWell { position: site });
            for _ in 0..100 {
                self.session.step_once();
            }
            if self.session.sim.subsurface.wells.len() != 1
                || self.session.sim.subsurface.reading(site).stored >= before
            {
                return Err("survey intervention did not draw down groundwater".into());
            }
            let size = view.window.inner_size();
            let p = terrain::surface_point(&self.session.sim, site);
            let ndc = self
                .camera
                .view_projection(size.width as f32 / size.height as f32)
                .project_point3(p);
            let cursor = Vec2::new(
                (ndc.x + 1.) * 0.5 * size.width as f32,
                (1. - ndc.y) * 0.5 * size.height as f32,
            );
            let picked = view
                .pick(&self.session, &self.camera, cursor)
                .ok_or("survey picking failed")?;
            if (picked.x - site.x).abs() > 0.05 || (picked.z - site.z).abs() > 0.05 {
                return Err("survey changed world picking".into());
            }
        }
        if self.front.playing() && !self.input.orbit && !self.input.pan {
            let hover = self
                .input
                .cursor
                .filter(|p| !view.pointer_over_ui(*p))
                .and_then(|p| view.pick(&self.session, &self.camera, p));
            let object = if !self.session.placing && !self.session.path.active {
                self.input
                    .cursor
                    .filter(|p| !view.pointer_over_ui(*p))
                    .and_then(|p| view.pick_object(&self.session, &self.camera, p, hover))
            } else {
                None
            };
            self.session.set_hover(hover);
            self.session.set_hovered_object(object);
        }
        if self.interaction_smoke {
            interaction_smoke::advance(
                self.frames,
                &mut self.session,
                &mut self.camera,
                view,
                &mut self.input,
            )?;
        }
        if self.smoke && self.frames == 1 {
            // Exercise the same camera, command, clock, and resize paths as input.
            self.camera.orbit(Vec2::new(50., -10.));
            self.camera.pan(Vec2::new(-30., 10.), 800.);
            self.camera.zoom(1.);
            self.input
                .key(KeyCode::Space, true, &mut self.session, &mut self.camera);
            self.input
                .key(KeyCode::Space, false, &mut self.session, &mut self.camera);
            let before = self.session.sim.time;
            self.session.advance(Duration::from_secs(10));
            if self.session.sim.time != before {
                return Err("pause did not preserve time".into());
            }
            self.session
                .queue(genesis_sim_core::actions::Action::Terraform {
                    x: 15,
                    y: 12,
                    radius: 2,
                    strength: 0.4,
                });
            self.session.step_once();
            if self.session.pending_count() != 0 {
                return Err("step did not consume queued action".into());
            }
            let count = self.session.sim.buildings.len();
            self.session
                .select(genesis_sim_core::spatial::Position::new(25.31, 20.27));
            self.session.building_rotation = 0.47;
            self.session.set_placing(true);
            self.session.set_hover(Some(self.session.position));
            self.session.build_hovered(false);
            self.session.step_once();
            if self.session.sim.buildings.len() != count + 1 {
                return Err("free building placement failed".into());
            }
            if self.save_smoke {
                save_smoke::check(
                    &mut self.session,
                    self.capture
                        .as_ref()
                        .ok_or("save smoke needs a capture path")?,
                )?;
            }
            self.session.set_placing(false);
            view.resize(0, 0);
            if view.drawable {
                return Err("zero-sized surface was not suspended".into());
            }
            let size = view.window.inner_size();
            view.resize(size.width, size.height);
            let _ = view.window.request_inner_size(winit::dpi::LogicalSize::new(
                if self.survey_smoke { 1100 } else { 760 },
                if self.survey_smoke { 800 } else { 540 },
            ));
        }
        if self.smoke && !self.survey_smoke && self.frames == 30 && self.session.hover.is_none() {
            return Err("cursor-following preview has no terrain hit".into());
        }
        if self.survey_smoke && [12, 18, 24].contains(&self.frames) {
            self.session.survey.water = self.frames != 18;
            self.session.survey.minerals = self.frames != 12;
            self.session.refresh_survey();
        }
        let layer_capture = if self.interaction_smoke && [8, 18].contains(&self.frames) {
            self.capture.as_ref().map(|p| {
                p.with_extension(if self.frames == 8 {
                    "house.png"
                } else {
                    "draft.png"
                })
            })
        } else if self.survey_smoke && [16, 22].contains(&self.frames) {
            self.capture.as_ref().map(|path| {
                path.with_extension(if self.frames == 16 {
                    "water.png"
                } else {
                    "geology.png"
                })
            })
        } else if self.world_smoke && [10, 24].contains(&self.frames) {
            self.capture.as_ref().map(|path| {
                path.with_extension(if self.frames == 10 {
                    "initial.png"
                } else {
                    "survey.png"
                })
            })
        } else {
            None
        };
        let finished =
            self.frames >= 30 && self.menu_smoke.as_ref().is_none_or(|check| check.done());
        let capture = if finished {
            self.capture.as_deref()
        } else {
            menu_capture.as_deref().or(layer_capture.as_deref())
        };
        let rendered = view.draw(
            &mut self.session,
            &mut self.camera,
            capture,
            &mut self.front,
        )?;
        let fullscreen = self.front.settings.fullscreen;
        if view.window.fullscreen().is_some() != fullscreen {
            view.window
                .set_fullscreen(fullscreen.then_some(winit::window::Fullscreen::Borderless(None)));
        }
        if !self.front.playing() {
            self.input.clear();
        }
        if self.front.quit {
            return Ok(true);
        }
        if self.front.playing() && self.input.cursor.is_some_and(|p| !view.pointer_over_ui(p)) {
            use winit::window::CursorIcon;
            view.window
                .set_cursor(if self.input.pan || self.input.orbit {
                    CursorIcon::Grabbing
                } else if self.session.placing {
                    CursorIcon::Crosshair
                } else {
                    CursorIcon::Default
                });
        }
        Ok(rendered && capture.is_some() && finished)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        let result = (|| -> Result<View> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Genesis")
                        .with_active(self.capture.is_none())
                        .with_min_inner_size(winit::dpi::LogicalSize::new(
                            if self.survey_smoke { 1100 } else { 760 },
                            if self.survey_smoke { 800 } else { 540 },
                        ))
                        .with_inner_size(winit::dpi::LogicalSize::new(1280, 800)),
                )?,
            );
            pollster::block_on(View::new(window))
        })();
        match result {
            Ok(mut view) => {
                view.atmosphere = self.atmosphere;
                self.session.dirty = true;
                self.session.urgent_redraw = true;
                self.session.overlay_dirty = true;
                self.last_frame = Some(Instant::now());
                self.hidden = false;
                self.view = Some(view);
            }
            Err(error) => self.fail(event_loop, error),
        }
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.view = None;
        self.last_frame = None;
        self.input.clear();
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(view) = &mut self.view else {
            return;
        };
        if id != view.window.id() {
            return;
        }
        // Native captures must be repeatable even while the user works in another
        // window. Scripted smoke checks inject their own input below this boundary.
        if self.capture.is_some()
            && matches!(
                event,
                WindowEvent::KeyboardInput { .. }
                    | WindowEvent::MouseInput { .. }
                    | WindowEvent::MouseWheel { .. }
                    | WindowEvent::CursorMoved { .. }
                    | WindowEvent::ModifiersChanged(_)
            )
        {
            return;
        }
        let playing = self.front.playing();
        let consumed = view.input(&event, playing);
        let over_ui = self.input.cursor.is_some_and(|p| view.pointer_over_ui(p));
        if !matches!(event, WindowEvent::RedrawRequested) && view.drawable && !self.hidden {
            view.window.request_redraw();
        }
        match event {
            WindowEvent::CloseRequested => self.front.request_quit(),
            WindowEvent::Focused(false) => {
                self.input.clear();
                self.session.set_hover(None);
            }
            WindowEvent::CursorLeft { .. } => {
                self.input.cursor = None;
                self.session.set_hover(None);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.input.shift = modifiers.state().shift_key();
                self.input.alt = modifiers.state().alt_key();
            }
            WindowEvent::Occluded(hidden) => {
                // Automated captures must finish even when another window covers them.
                self.hidden = hidden && self.capture.is_none();
                self.last_frame = Some(Instant::now());
                self.input.clear();
            }
            WindowEvent::Resized(size) => {
                view.resize(size.width, size.height);
                self.last_frame = Some(Instant::now());
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    if pressed && !event.repeat && code == KeyCode::Escape {
                        if playing
                            && (self.camera.is_ground()
                                || self.session.placing
                                || self.session.path.active
                                || self.session.selected_object.is_some()
                                || self.session.help
                                || self.session.landscape_tools
                                || self.session.survey.open
                                || self.session.world_tools)
                        {
                            self.input
                                .key(code, true, &mut self.session, &mut self.camera);
                        } else {
                            self.front.escape();
                        }
                        self.input.clear();
                    } else if playing && (!pressed || (!consumed && !event.repeat)) {
                        self.input
                            .key(code, pressed, &mut self.session, &mut self.camera);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = Vec2::new(position.x as f32, position.y as f32);
                if let Some(previous) = self.input.cursor.replace(point) {
                    let delta = point - previous;
                    if playing && self.input.orbit {
                        self.camera.orbit(
                            delta * self.front.settings.sensitivity
                                / view.window.scale_factor() as f32,
                        );
                    }
                    if playing && self.input.pan {
                        self.camera.pan(
                            delta * self.front.settings.sensitivity,
                            view.window.inner_size().height as f32,
                        );
                    }
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let pressed = state == ElementState::Pressed;
                if playing && (!pressed || (!consumed && !over_ui)) {
                    match button {
                        MouseButton::Right => self.input.orbit = pressed,
                        MouseButton::Middle => self.input.pan = pressed,
                        MouseButton::Left if !pressed => self.input.pan = false,
                        MouseButton::Left if self.input.alt => self.input.pan = true,
                        MouseButton::Left if pressed => {
                            if let Some(cursor) = self.input.cursor {
                                let ground = view.pick(&self.session, &self.camera, cursor);
                                self.session.set_hover(ground);
                                if self.session.placing {
                                    self.session.build_hovered(self.input.shift);
                                } else if self.session.path.active {
                                    self.session.add_path_point();
                                } else {
                                    let object = view.pick_object(
                                        &self.session,
                                        &self.camera,
                                        cursor,
                                        ground,
                                    );
                                    if ground.is_some() || object.is_some() {
                                        self.session.select_object(
                                            object,
                                            ground.unwrap_or(self.session.position),
                                        );
                                    } else {
                                        self.session.select_object(None, self.session.position);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } if playing && !consumed && !over_ui => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => {
                        p.y as f32 / (40. * view.window.scale_factor() as f32)
                    }
                };
                let lines = lines
                    * self.front.settings.sensitivity
                    * if self.front.settings.invert_zoom {
                        -1.
                    } else {
                        1.
                    };
                if let Some(cursor) = self.input.cursor {
                    let height = view
                        .pick(&self.session, &self.camera, cursor)
                        .map_or(0., |p| self.session.visible_point(p).y);
                    let size = view.window.inner_size();
                    self.camera.zoom_at(
                        lines,
                        cursor,
                        Vec2::new(size.width as f32, size.height as f32),
                        height,
                    );
                } else {
                    self.camera.zoom(lines);
                }
            }
            WindowEvent::RedrawRequested => match self.frame() {
                Ok(true) => event_loop.exit(),
                Ok(false) => {}
                Err(error) => self.fail(event_loop, error),
            },
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(view) = &self.view
            && view.drawable
            && !self.hidden
        {
            let deadline = self.last_frame.unwrap_or_else(Instant::now)
                + Duration::from_secs_f64(1. / self.front.settings.fps as f64);
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            if Instant::now() >= deadline {
                view.window.request_redraw();
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--terrain-edit-benchmark")
    {
        let full_rebuild = args.get(2).is_some_and(|arg| arg == "--full-rebuild");
        if args.len() != 2 && !(args.len() == 3 && full_rebuild) {
            return Err(
                "usage: genesis-app --terrain-edit-benchmark report.json [--full-rebuild]".into(),
            );
        }
        return extraction_benchmark::edits::run(std::path::Path::new(&args[1]), full_rebuild);
    }
    if args
        .first()
        .is_some_and(|arg| arg == "--extraction-benchmark")
    {
        if args.len() != 2 {
            return Err("usage: genesis-app --extraction-benchmark report.json".into());
        }
        return extraction_benchmark::run(std::path::Path::new(&args[1]));
    }
    if args.first().is_some_and(|arg| arg == "--render-benchmark") {
        if args.len() != 2 {
            return Err("usage: genesis-app --render-benchmark report.json".into());
        }
        return pollster::block_on(benchmark::run(std::path::Path::new(&args[1])));
    }
    let mut app = App::default();
    if let Some(index) = args.iter().position(|arg| arg == "--weather") {
        app.atmosphere = genesis_renderer::atmosphere::Atmosphere::preview(
            args.get(index + 1)
                .ok_or("--weather needs clear, fair, or overcast")?,
        )
        .ok_or("--weather must be clear, fair, or overcast")?;
        args.drain(index..=index + 1);
    }
    let explicit_seed = args.iter().any(|arg| arg == "--seed");
    let explicit_region = args.iter().any(|arg| arg == "--region");
    let mut seed = genesis_sim_core::worldgen::DEFAULT_SEED;
    let mut region = genesis_sim_core::worldgen::Region::LevantCoast;
    let mut capture_tree = None;
    let mut capture_rock = None;
    if let Some(index) = args.iter().position(|arg| arg == "--rock") {
        use genesis_sim_core::ecology::CoverKind;
        capture_rock = Some(match args.get(index + 1).map(String::as_str) {
            Some("limestone") => CoverKind::Limestone,
            Some("sandstone") => CoverKind::Sandstone,
            Some("basalt") => CoverKind::Basalt,
            _ => return Err("--rock must be limestone, sandstone, or basalt".into()),
        });
        args.drain(index..=index + 1);
    }
    if let Some(index) = args.iter().position(|arg| arg == "--tree") {
        use genesis_sim_core::ecology::CoverKind;
        capture_tree = Some(match args.get(index + 1).map(String::as_str) {
            Some("broadleaf") => CoverKind::Broadleaf,
            Some("pine") => CoverKind::Pine,
            Some("riparian") => CoverKind::Riparian,
            _ => return Err("--tree must be broadleaf, pine, or riparian".into()),
        });
        args.drain(index..=index + 1);
    }
    if let Some(index) = args.iter().position(|arg| arg == "--seed") {
        seed = args
            .get(index + 1)
            .ok_or("--seed needs an unsigned integer")?
            .parse::<u64>()?;
        args.drain(index..=index + 1);
    }
    if let Some(index) = args.iter().position(|arg| arg == "--region") {
        region = match args.get(index + 1).map(String::as_str) {
            Some("coast") => genesis_sim_core::worldgen::Region::LevantCoast,
            Some("highlands") => genesis_sim_core::worldgen::Region::LevantHighlands,
            Some("island") => genesis_sim_core::worldgen::Region::Island,
            _ => return Err("--region must be coast, highlands, or island".into()),
        };
        args.drain(index..=index + 1);
    }
    if explicit_seed || explicit_region {
        app.session = Session::from_sim(genesis_sim_core::worldgen::generate_region(seed, region));
    }
    if capture_tree.is_some()
        && !args.first().is_some_and(|f| {
            f == "--ecology-capture"
                || f == "--ecology-ground-capture"
                || f == "--understorey-capture"
        })
    {
        return Err("--tree requires an ecology or understorey capture".into());
    }
    if capture_rock.is_some() && args.first().is_none_or(|f| f != "--rock-capture") {
        return Err("--rock requires --rock-capture".into());
    }
    match args.as_slice() {
        [] => {}
        [flag, path]
            if flag == "--capture"
                || flag == "--settlement-capture"
                || flag == "--ground-capture"
                || flag == "--sky-capture"
                || flag == "--house-capture"
                || flag == "--rock-capture"
                || flag == "--river-capture"
                || flag == "--river-ground-capture"
                || flag == "--ecology-capture"
                || flag == "--ecology-ground-capture"
                || flag == "--understorey-capture"
                || flag == "--landscape-capture"
                || flag == "--seafloor-capture"
                || flag == "--smoke-test"
                || flag == "--save-test"
                || flag == "--survey-test"
                || flag == "--world-test"
                || flag == "--interaction-test"
                || flag == "--menu-capture"
                || flag == "--menu-test" =>
        {
            app.capture = Some(PathBuf::from(path));
            app.smoke = flag == "--smoke-test" || flag == "--survey-test" || flag == "--save-test";
            app.save_smoke = flag == "--save-test";
            app.survey_smoke = flag == "--survey-test";
            app.world_smoke = flag == "--world-test";
            app.interaction_smoke = flag == "--interaction-test";
            if flag == "--menu-test" {
                app.menu_smoke = Some(menu_smoke::Check::default());
            }
        }
        _ => {
            return Err(
                "usage: genesis-app [--weather clear|fair|overcast] [--seed number] [--region coast|highlands|island] [--tree broadleaf|pine|riparian (ecology/understorey captures only)] [--rock limestone|sandstone|basalt (rock capture only)] [--menu-capture output.png | --menu-test output.png | --seafloor-capture output.png | --capture output.png | --landscape-capture output.png | --ground-capture output.png | --sky-capture output.png | --house-capture output.png | --rock-capture output.png | --river-capture output.png | --river-ground-capture output.png | --ecology-capture output.png | --ecology-ground-capture output.png | --understorey-capture output.png | --settlement-capture output.png | --world-test output.png | --interaction-test output.png | --smoke-test output.png | --survey-test output.png]".into(),
            );
        }
    }
    if app.capture.is_some()
        && !args
            .first()
            .is_some_and(|f| f == "--menu-capture" || f == "--menu-test")
    {
        app.front.screen = frontend::Screen::Playing;
        app.front.has_session = true;
    }
    if app.capture.is_some() {
        app.front.settings = frontend::Settings::default();
    }
    app.front.region = region;
    app.front.seed = seed.to_string();
    if app.smoke {
        if explicit_seed || explicit_region {
            return Err(
                "--seed applies to generated regions; use --world-test for seeded smoke checks"
                    .into(),
            );
        }
        // Keep the small authored regression fixture for camera/action tests.
        app.session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
    }
    crate::terrain::home_camera(&app.session.sim, &mut app.camera);
    if args
        .first()
        .is_some_and(|flag| flag == "--settlement-capture" || flag == "--landscape-capture")
    {
        app.camera.center(terrain::surface_point(
            &app.session.sim,
            app.session.position,
        ));
        app.camera
            .set_visible_height(if args[0] == "--landscape-capture" {
                55.
            } else {
                4.5
            });
    }
    if args.first().is_some_and(|flag| {
        flag == "--ecology-capture"
            || flag == "--ecology-ground-capture"
            || flag == "--understorey-capture"
    }) {
        let anchor = app.session.position;
        let plant = app
            .session
            .sim
            .geography
            .cover
            .iter()
            .filter(|c| c.kind.tree() && capture_tree.is_none_or(|kind| c.kind == kind))
            .min_by(|a, b| {
                genesis_sim_core::paths::distance(a.position, anchor)
                    .total_cmp(&genesis_sim_core::paths::distance(b.position, anchor))
            })
            .ok_or("region has no matching trees to inspect")?;
        eprintln!("vegetation inspection: {}", plant.kind.name());
        let p = plant.position;
        app.session.select(p);
        app.session.clock.paused = true;
        app.camera.center(
            terrain::point(&app.session.sim, p)
                + glam::Vec3::Y * genesis_sim_core::units::metres(3.),
        );
        app.camera.set_visible_height(2.2);
        if args[0] == "--understorey-capture" {
            use genesis_sim_core::{spatial::Position, surface::walking, units::metres};
            let sim = &app.session.sim;
            let stand = (0..64)
                .map(|i| {
                    let a = i as f32 * 2.399963;
                    Position::new(p.x + a.cos() * metres(2.5), p.z + a.sin() * metres(2.5))
                })
                .find(|q| walking::standable(sim, *q))
                .ok_or("no accessible ground beneath the selected tree")?;
            app.camera
                .enter_ground(terrain::point(sim, stand), metres(1.7));
            let target = Position::new(p.x * 0.55 + stand.x * 0.45, p.z * 0.55 + stand.z * 0.45);
            app.camera.look_at_ground(terrain::point(sim, target));
        }
        if args[0] == "--ecology-ground-capture" {
            ground::toggle(&app.session.sim, p, &mut app.camera);
            app.camera.look_at_ground(
                terrain::point(&app.session.sim, p)
                    + glam::Vec3::Y * genesis_sim_core::units::metres(4.),
            );
        }
    }
    if args
        .first()
        .is_some_and(|flag| flag == "--river-capture" || flag == "--river-ground-capture")
    {
        let sim = &app.session.sim;
        let p =
            sim.geography
                .rivers
                .iter()
                .flat_map(|river| {
                    river.points.iter().enumerate().map(move |(i, &p)| {
                        (p, river.widths.get(i).copied().unwrap_or(river.width))
                    })
                })
                .filter(|(p, _)| {
                    p.x > 5.
                        && p.z > 5.
                        && p.x < sim.grid.width as f32 - 5.
                        && p.z < sim.grid.height as f32 - 5.
                        && genesis_sim_core::surface::channels::level(sim, *p)
                            > sim.sea_level + genesis_sim_core::units::metres(4.)
                })
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(p, _)| p)
                .ok_or("region has no inland river to inspect")?;
        eprintln!(
            "river inspection: ({:.3}, {:.3}), bed {:.3}, water {:.3}",
            p.x,
            p.z,
            genesis_sim_core::surface::elevation(sim, p),
            genesis_sim_core::surface::channels::level(sim, p)
        );
        app.camera.center(terrain::point(sim, p));
        app.camera.set_visible_height(2.2);
        if args[0] == "--river-ground-capture" {
            ground::toggle(sim, p, &mut app.camera);
            if !app.camera.is_ground() {
                return Err("no accessible riverbank for inspection".into());
            }
            app.camera.look_at_ground(
                terrain::point(sim, p) + glam::Vec3::Y * genesis_sim_core::units::metres(0.8),
            );
        }
        app.session.select(p);
        app.session.clock.paused = true;
    }
    if args.first().is_some_and(|flag| flag == "--rock-capture") {
        scenery_capture::rock(&mut app.session, &mut app.camera, capture_rock)?;
    }
    if args.first().is_some_and(|flag| flag == "--ground-capture") {
        ground::toggle(&app.session.sim, app.session.position, &mut app.camera);
        app.session.clock.paused = true;
    }
    if args.first().is_some_and(|flag| flag == "--sky-capture") {
        let anchor = app.session.position;
        let stand = genesis_sim_core::spatial::Position::new(anchor.x + 5., anchor.z + 5.);
        ground::toggle(&app.session.sim, stand, &mut app.camera);
        if !app.camera.is_ground() {
            return Err("no accessible ground for sky inspection".into());
        }
        let eye = app.camera.eye();
        app.camera
            .look_at_ground(eye + glam::Vec3::new(-5., 2.4, -7.));
        app.session.clock.paused = true;
    }
    if args.first().is_some_and(|flag| flag == "--house-capture") {
        let sim = &app.session.sim;
        let b = sim.buildings.first().ok_or("no dwelling to inspect")?;
        let (s, c) = b.rotation.sin_cos();
        let distance = genesis_sim_core::settlement::Building::HALF_DEPTH
            + genesis_sim_core::units::metres(6.);
        let side = genesis_sim_core::units::metres(2.);
        let p = genesis_sim_core::spatial::Position::new(
            b.position.x - s * distance + c * side,
            b.position.z + c * distance + s * side,
        );
        app.camera
            .enter_ground(terrain::point(sim, p), genesis_sim_core::units::metres(1.7));
        let high = b
            .footprint()
            .map(|p| terrain::point(sim, p).y)
            .into_iter()
            .fold(f32::NEG_INFINITY, f32::max);
        let mut target = terrain::point(sim, b.position);
        target.y = high + genesis_sim_core::units::metres(1.45);
        app.camera.look_at_ground(target);
        app.session.clock.paused = true;
    }
    if args
        .first()
        .is_some_and(|flag| flag == "--seafloor-capture")
    {
        app.session.survey.open = true;
        app.session.survey.seafloor = true;
        app.session.clock.paused = true;
        app.session
            .select(genesis_sim_core::spatial::Position::new(65., 160.));
        app.session.refresh_survey();
    }
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}
