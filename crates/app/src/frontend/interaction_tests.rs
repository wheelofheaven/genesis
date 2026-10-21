//! Drive the real egui widgets so navigation isn't tested only by assigning states.
use super::*;
use egui::{Event, Pos2};

fn draw(
    front: &mut Frontend,
    ctx: &egui::Context,
    session: &mut Session,
    camera: &mut Camera,
    events: Vec<Event>,
) -> egui::FullOutput {
    draw_at(front, ctx, session, camera, events, egui::vec2(1280., 800.))
}
fn draw_at(
    front: &mut Frontend,
    ctx: &egui::Context,
    session: &mut Session,
    camera: &mut Camera,
    events: Vec<Event>,
    size: egui::Vec2,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ctx| {
            front.show(ctx, session, camera);
        },
    )
}
fn text_position(shape: &egui::Shape, label: &str) -> Option<Pos2> {
    match shape {
        egui::Shape::Text(text) if text.galley.job.text == label => {
            Some(text.pos + text.galley.size() * 0.5)
        }
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| text_position(s, label)),
        _ => None,
    }
}
fn click(
    front: &mut Frontend,
    ctx: &egui::Context,
    session: &mut Session,
    camera: &mut Camera,
    label: &str,
) {
    // A new screen gets its own layout before pointer events arrive.
    draw(front, ctx, session, camera, vec![]);
    let output = draw(front, ctx, session, camera, vec![]);
    let position = output
        .shapes
        .iter()
        .find_map(|s| text_position(&s.shape, label))
        .unwrap_or_else(|| panic!("missing visible control: {label}"));
    for pressed in [true, false] {
        draw(
            front,
            ctx,
            session,
            camera,
            vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}
#[test]
fn expedition_widgets_preserve_region_and_allow_back_navigation() {
    let ctx = egui::Context::default();
    crate::ui::theme::install(&ctx);
    let mut front = Frontend::default();
    let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
    let mut camera = Camera::default();
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Begin expedition",
    );
    assert_eq!(front.screen, Screen::Destination);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Select a region",
    );
    assert_eq!(front.screen, Screen::Setup);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        Region::LevantHighlands.name(),
    );
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Review expedition",
    );
    assert_eq!(front.screen, Screen::Briefing);
    assert_eq!(front.region, Region::LevantHighlands);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Back to regions",
    );
    assert_eq!(front.screen, Screen::Setup);
    assert_eq!(front.region, Region::LevantHighlands);
    assert_eq!(session.sim.time, 0.);
}

struct SaveSlot(std::path::PathBuf);
impl SaveSlot {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("genesis-menu-{}", genesis_storage::new_world_id()));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn path(&self) -> std::path::PathBuf {
        self.0.join("world.json")
    }
}
impl Drop for SaveSlot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn menu_save_load_and_fresh_launch_preserve_role_identity_and_pending_irrigation() {
    let slot = SaveSlot::new();
    let ctx = egui::Context::default();
    crate::ui::theme::install(&ctx);
    let mut session = Session::from_sim(genesis_sim_core::scenarios::dry_cultivated_settlement());
    session.mode = Mode::WorldBuilder; // Role is not inferred from existing population.
    session.clock.paused = true;
    for _ in 0..200 {
        session.step_once();
    }
    session.queue(genesis_sim_core::actions::Action::IrrigateCrop {
        source: genesis_sim_core::food::SourceId(1),
        well: 1,
    });
    let before = serde_json::to_vec(&*session.sim).unwrap();
    let identity = session.world_id.clone();
    let mut camera = Camera::default();
    let mut front = Frontend {
        screen: Screen::Pause,
        has_session: true,
        ..Default::default()
    };
    front.set_save_path(slot.path());
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Save expedition",
    );
    assert!(front.message.is_empty());
    assert!(!front.notice.is_empty());
    assert_eq!(serde_json::to_vec(&*session.sim).unwrap(), before);
    assert_eq!(session.pending_count(), 1);
    session.step_once();
    let unsaved = serde_json::to_vec(&*session.sim).unwrap();
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Load saved expedition",
    );
    assert_eq!(front.screen, Screen::ConfirmLoad);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Keep current expedition",
    );
    assert_eq!(front.screen, Screen::Pause);
    assert_eq!(serde_json::to_vec(&*session.sim).unwrap(), unsaved);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Load saved expedition",
    );
    let revision = session.terrain_revision;
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Load saved expedition",
    );
    assert!(front.playing());
    assert_eq!(serde_json::to_vec(&*session.sim).unwrap(), before);
    assert_eq!(session.terrain_revision, revision + 1);
    assert_eq!(camera.focus, session.selected_point());
    assert_eq!(
        camera.visible_height(),
        genesis_sim_core::units::metres(100.)
    );
    assert_eq!(session.pending_count(), 1);
    assert_eq!(session.world_id, identity);
    assert_eq!(front.mode, Mode::WorldBuilder);
    assert!(session.clock.paused);

    let mut fresh = Frontend::default();
    fresh.set_save_path(slot.path());
    let mut placeholder = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
    click(
        &mut fresh,
        &ctx,
        &mut placeholder,
        &mut camera,
        "Load saved expedition",
    );
    assert!(fresh.playing() && fresh.has_session);
    assert_eq!(placeholder.mode, Mode::WorldBuilder);
    assert_eq!(placeholder.world_id, identity);
    assert_eq!(serde_json::to_vec(&*placeholder.sim).unwrap(), before);
    assert_eq!(placeholder.pending_count(), 1);
    placeholder.step_once();
    assert_eq!(serde_json::to_vec(&*placeholder.sim).unwrap(), unsaved);
    assert_eq!(placeholder.pending_count(), 0);
}

#[test]
fn failed_save_cannot_quit_or_deploy_and_successful_save_and_quit_keeps_a_backup() {
    let slot = SaveSlot::new();
    let ctx = egui::Context::default();
    crate::ui::theme::install(&ctx);
    let mut session = Session::from_sim(genesis_sim_core::scenarios::living_settlement());
    let mut camera = Camera::default();
    let mut front = Frontend {
        screen: Screen::Pause,
        has_session: true,
        ..Default::default()
    };
    // A file where the parent directory should be produces a deterministic I/O failure.
    let blocked = slot.0.join("blocked");
    std::fs::write(&blocked, "keep").unwrap();
    front.set_save_path(blocked.join("world.json"));
    let before = serde_json::to_vec(&*session.sim).unwrap();
    click(&mut front, &ctx, &mut session, &mut camera, "Quit");
    click(&mut front, &ctx, &mut session, &mut camera, "Save and quit");
    assert!(!front.quit);
    assert_eq!(front.screen, Screen::ConfirmQuit);
    assert!(!front.message.is_empty());
    assert_eq!(serde_json::to_vec(&*session.sim).unwrap(), before);
    click(&mut front, &ctx, &mut session, &mut camera, "Stay here");
    assert_eq!(front.screen, Screen::Pause);
    front.screen = Screen::ConfirmNew;
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Save and deploy",
    );
    assert_eq!(front.screen, Screen::ConfirmNew);
    assert!(front.loading.is_none());

    front.set_save_path(slot.path());
    assert!(front.save_expedition(&session));
    let original = std::fs::read(slot.path()).unwrap();
    session.clock.paused = true;
    session.step_once();
    front.screen = Screen::Pause;
    front.request_quit();
    click(&mut front, &ctx, &mut session, &mut camera, "Save and quit");
    assert!(front.quit);
    assert!(front.message.is_empty());
    let restored = genesis_storage::load(&slot.path()).unwrap();
    assert_eq!(restored.simulation.completed_ticks(), 1);
    assert_eq!(
        std::fs::read(genesis_storage::backup_path(&slot.path()).unwrap()).unwrap(),
        original
    );
}

#[test]
fn missing_and_corrupt_menu_loads_preserve_world_camera_and_navigation() {
    let slot = SaveSlot::new();
    let ctx = egui::Context::default();
    crate::ui::theme::install(&ctx);
    let mut session = Session::from_sim(genesis_sim_core::scenarios::living_settlement());
    session.queue(genesis_sim_core::actions::Action::SeedPlants {
        x: 4,
        y: 4,
        radius: 1,
    });
    let mut camera = Camera::default();
    camera.orbit(glam::Vec2::new(40., 10.));
    let projection = camera.view_projection(1.6);
    let before = serde_json::to_vec(&*session.sim).unwrap();
    let identity = session.world_id.clone();
    let revision = session.terrain_revision;
    let mut front = Frontend::default();
    front.set_save_path(slot.path());
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Load saved expedition",
    );
    assert_eq!(front.screen, Screen::Home);
    assert!(!front.has_session);
    assert!(front.message.contains("No saved expedition"));
    front.has_session = true;
    front.screen = Screen::Pause;
    std::fs::write(slot.path(), "damaged save").unwrap();
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Load saved expedition",
    );
    assert_eq!(front.screen, Screen::ConfirmLoad);
    assert!(front.surface_backdrop());
    front.request_quit();
    assert!(front.surface_backdrop());
    front.escape();
    assert_eq!(front.screen, Screen::ConfirmLoad);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Load saved expedition",
    );
    assert!(front.has_session);
    assert_eq!(front.screen, Screen::ConfirmLoad);
    assert!(!front.message.is_empty());
    assert_eq!(serde_json::to_vec(&*session.sim).unwrap(), before);
    assert_eq!(session.world_id, identity);
    assert_eq!(session.terrain_revision, revision);
    assert_eq!(session.pending_count(), 1);
    assert_eq!(camera.view_projection(1.6), projection);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Keep current expedition",
    );
    assert_eq!(front.screen, Screen::Pause);
}

#[test]
fn save_and_load_controls_are_visible_in_compact_menus_without_writing() {
    let slot = SaveSlot::new();
    for size in [
        egui::vec2(563., 400.),
        egui::vec2(760., 540.),
        egui::vec2(1280., 800.),
    ] {
        for (screen, labels) in [
            (
                Screen::Home,
                ["Continue expedition", "Load saved expedition"],
            ),
            (Screen::Pause, ["Save expedition", "Load saved expedition"]),
            (
                Screen::ConfirmLoad,
                ["Load saved expedition", "Keep current expedition"],
            ),
            (Screen::ConfirmQuit, ["Save and quit", "Stay here"]),
            (Screen::ConfirmNew, ["Save and deploy", "Back to briefing"]),
        ] {
            let ctx = egui::Context::default();
            crate::ui::theme::install(&ctx);
            let mut front = Frontend {
                screen,
                has_session: true,
                ..Default::default()
            };
            front.set_save_path(slot.path());
            let mut session = Session::from_sim(genesis_sim_core::scenarios::living_settlement());
            let before = serde_json::to_vec(&*session.sim).unwrap();
            let mut camera = Camera::default();
            for _ in 0..3 {
                draw_at(&mut front, &ctx, &mut session, &mut camera, vec![], size);
            }
            let output = draw_at(&mut front, &ctx, &mut session, &mut camera, vec![], size);
            for label in labels {
                let position = output
                    .shapes
                    .iter()
                    .find_map(|s| {
                        text_position(&s.shape, label).filter(|p| s.clip_rect.contains(*p))
                    })
                    .unwrap_or_else(|| {
                        panic!("{screen:?} {size:?}: {label} is clipped or missing")
                    });
                assert!(egui::Rect::from_min_size(Pos2::ZERO, size).contains(position));
            }
            assert_eq!(serde_json::to_vec(&*session.sim).unwrap(), before);
            assert!(!slot.path().exists());
        }
    }
}
