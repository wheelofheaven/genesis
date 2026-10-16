use super::*;

fn frame(
    ctx: &egui::Context,
    size: egui::Vec2,
    session: &mut Session,
    front: &mut crate::frontend::Frontend,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Vec<egui::Rect>) {
    frame_with_camera(ctx, size, session, front, events, &mut Camera::default())
}

fn frame_with_camera(
    ctx: &egui::Context,
    size: egui::Vec2,
    session: &mut Session,
    front: &mut crate::frontend::Frontend,
    events: Vec<egui::Event>,
    camera: &mut Camera,
) -> (egui::FullOutput, Vec<egui::Rect>) {
    let mut occupied = Vec::new();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ctx| {
            occupied = show(ctx, session, camera, front, &mut Default::default());
        },
    );
    (output, occupied)
}

#[test]
fn hud_stays_on_screen_without_overlapping_at_supported_scales() {
    for size in [
        egui::vec2(563., 400.),
        egui::vec2(760., 540.),
        egui::vec2(900., 540.),
        egui::vec2(1100., 800.),
        egui::vec2(1120., 540.),
        egui::vec2(1280., 800.),
    ] {
        for tool in 0..18 {
            let ctx = egui::Context::default();
            theme::install(&ctx);
            let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
            if tool == 7 {
                session = Session::from_sim(genesis_sim_core::scenarios::living_settlement());
                session.selected_object = Some(crate::interaction::Object::Person(
                    session.sim.people.all()[0].id,
                ));
            }
            if tool >= 8 {
                let mut state = if tool == 8 {
                    genesis_sim_core::scenarios::water_settlement()
                } else {
                    genesis_sim_core::scenarios::living_settlement()
                };
                for _ in 0..20 {
                    state.step(0.1, &[]);
                }
                session = Session::from_sim(state);
                session.selected_object = Some(crate::interaction::Object::Person(
                    session.sim.people.all()[0].id,
                ));
            }
            session.water_overview = tool >= 10;
            if tool == 11 {
                session.replace_sim(genesis_sim_core::scenarios::coastal_outpost());
                session.water_overview = true;
            }
            if tool >= 12 {
                let mut state = genesis_sim_core::scenarios::supplied_settlement();
                state.subsurface.wells[0].active = false;
                for _ in 0..20 {
                    state.step(0.1, &[]);
                }
                session.replace_sim(state);
                session.water_overview = tool == 12;
                session.supply_food = true;
                session.selected_object = Some(if tool == 13 {
                    crate::interaction::Object::FoodSource(session.sim.food.sources()[0].id)
                } else {
                    crate::interaction::Object::Person(session.sim.people.all()[0].id)
                });
            }
            session.path.active = tool == 5;
            if tool == 6 {
                session.selected_object = Some(crate::interaction::Object::Building(1));
            }
            if tool >= 15 {
                session.replace_sim(genesis_sim_core::scenarios::cultivated_settlement());
                session.selected_object = Some(crate::interaction::Object::FoodSource(
                    session.sim.food.sources()[0].id,
                ));
                session.water_overview = tool == 17;
                session.supply_food = true;
                session.hover = Some(genesis_sim_core::spatial::Position::new(8.5, 8.));
            }
            session.placing = tool == 1 || tool == 15;
            session.crop_placing = tool == 15;
            session.landscape_tools = tool == 2;
            session.survey.open = tool == 3;
            session.survey.water = true;
            session.survey.minerals = true;
            session.world_tools = tool == 4;
            session.message = "Borehole 1 online".into();
            let mut front = crate::frontend::Frontend::default();
            // egui Areas settle measured dimensions on the following frame.
            for _ in 0..3 {
                frame(&ctx, size, &mut session, &mut front, vec![]);
            }
            let (_, rects) = frame(&ctx, size, &mut session, &mut front, vec![]);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size).expand(1.);
            for (i, rect) in rects.iter().enumerate() {
                assert!(
                    screen.contains_rect(*rect),
                    "{size:?}, tool {tool}, panel {i}: {rect:?}"
                );
                for (j, other) in rects.iter().enumerate().skip(i + 1) {
                    assert!(
                        !rect.shrink(1.).intersects(other.shrink(1.)),
                        "{size:?}, tool {tool}: panels {i} {rect:?} and {j} {other:?} overlap"
                    );
                }
            }
        }
    }
}

fn text_position(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
    match shape {
        egui::Shape::Text(text) if text.galley.job.text == label => {
            Some(text.pos + text.galley.size() * 0.5)
        }
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| text_position(s, label)),
        _ => None,
    }
}

#[test]
fn dock_switches_tools_and_menu_is_reachable() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
    let mut front = crate::frontend::Frontend::default();
    for (label, expected) in [
        ("Supplies", 7),
        ("Paths", 6),
        ("Build", 1),
        ("Survey", 2),
        ("Supplies", 7),
        ("Landscape", 3),
        ("Explore", 0),
        ("Region", 4),
        ("Menu", 5),
    ] {
        let size = egui::vec2(760., 540.);
        frame(&ctx, size, &mut session, &mut front, vec![]);
        let (output, _) = frame(&ctx, size, &mut session, &mut front, vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|s| text_position(&s.shape, label))
            .or_else(|| {
                ctx.data(|d| {
                    d.get_temp::<egui::Rect>(egui::Id::new(("icon_button", label)))
                        .map(|r| r.center())
                })
            })
            .expect(label);
        for pressed in [true, false] {
            frame(
                &ctx,
                size,
                &mut session,
                &mut front,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(session.water_overview, expected == 7);
        assert_eq!(session.path.active, expected == 6);
        assert_eq!(session.placing, expected == 1);
        assert_eq!(session.survey.open, expected == 2);
        assert_eq!(session.landscape_tools, expected == 3);
        assert_eq!(session.world_tools, expected == 4 || expected == 5);
        if expected == 5 {
            assert_eq!(front.screen, crate::frontend::Screen::Pause);
        }
    }
}

#[test]
fn collection_cancel_button_queues_a_validated_action() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut state = genesis_sim_core::scenarios::water_settlement();
    for _ in 0..20 {
        state.step(0.1, &[]);
    }
    let mut session = Session::from_sim(state);
    let id = session.sim.people.all()[0].id;
    session.select_object(
        Some(crate::interaction::Object::Person(id)),
        session.position,
    );
    let before = session.sim.people.get(id).unwrap().water_task.clone();
    assert!(before.is_some());
    let mut front = crate::frontend::Frontend::default();
    let size = egui::vec2(1100., 800.);
    for _ in 0..3 {
        frame(&ctx, size, &mut session, &mut front, vec![]);
    }
    let (out, _) = frame(&ctx, size, &mut session, &mut front, vec![]);
    let position = out
        .shapes
        .iter()
        .find_map(|s| text_position(&s.shape, "Cancel collection"))
        .unwrap();
    for pressed in [true, false] {
        frame(
            &ctx,
            size,
            &mut session,
            &mut front,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(session.pending_count(), 1);
    assert_eq!(session.sim.people.get(id).unwrap().water_task, before);
    session.clock.paused = true;
    session.step_once();
    assert!(session.sim.people.get(id).unwrap().water_task.is_none());
    assert_eq!(
        session.sim.people.get(id).unwrap().water_issue,
        Some(genesis_sim_core::people::WaterIssue::Cancelled)
    );
}

#[test]
fn water_overview_focuses_stable_home_without_changing_the_simulation() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut state = genesis_sim_core::scenarios::water_settlement();
    state.buildings.reverse();
    let mut session = Session::from_sim(state);
    session.set_water_overview(true);
    let before = serde_json::to_vec(&*session.sim).unwrap();
    let mut camera = Camera::default();
    let mut front = crate::frontend::Frontend::default();
    let size = egui::vec2(1280., 800.);
    for _ in 0..3 {
        frame_with_camera(&ctx, size, &mut session, &mut front, vec![], &mut camera);
    }
    let rect = ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("water_home", 1_u64))))
        .unwrap();
    for pressed in [true, false] {
        frame_with_camera(
            &ctx,
            size,
            &mut session,
            &mut front,
            vec![
                egui::Event::PointerMoved(rect.center()),
                egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut camera,
        );
    }
    assert_eq!(
        session.selected_object,
        Some(crate::interaction::Object::Building(1))
    );
    assert!(session.water_overview);
    assert_eq!(
        session.position,
        session
            .sim
            .buildings
            .iter()
            .find(|b| b.id == 1)
            .unwrap()
            .position
    );
    let projected = camera
        .view_projection(size.x / size.y)
        .project_point3(session.selected_point());
    assert!(projected.x.abs() < 0.001 && projected.y.abs() < 0.001);
    // Reach the final row through the real scroll area, including compact layouts.
    let compact = egui::vec2(760., 540.);
    for _ in 0..3 {
        frame_with_camera(&ctx, compact, &mut session, &mut front, vec![], &mut camera);
    }
    let (_, bounds) =
        frame_with_camera(&ctx, compact, &mut session, &mut front, vec![], &mut camera);
    let pointer = bounds[6].center();
    frame_with_camera(
        &ctx,
        compact,
        &mut session,
        &mut front,
        vec![
            egui::Event::PointerMoved(pointer),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., -2000.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
        &mut camera,
    );
    for _ in 0..30 {
        frame_with_camera(&ctx, compact, &mut session, &mut front, vec![], &mut camera);
    }
    let rect = ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("water_home", 6_u64))))
        .unwrap();
    assert!(
        bounds[6].contains(rect.center()),
        "last row {rect:?}, panel {:?}",
        bounds[6]
    );
    for pressed in [true, false] {
        frame_with_camera(
            &ctx,
            compact,
            &mut session,
            &mut front,
            vec![
                egui::Event::PointerMoved(rect.center()),
                egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut camera,
        );
    }
    assert_eq!(
        session.selected_object,
        Some(crate::interaction::Object::Building(6))
    );
    assert_eq!(serde_json::to_vec(&*session.sim).unwrap(), before);
    assert_eq!(session.pending_count(), 0);
    crate::input::Input::default().key(
        winit::keyboard::KeyCode::Escape,
        true,
        &mut session,
        &mut camera,
    );
    assert!(!session.water_overview);
    session.set_water_overview(true);
    session.replace_sim(genesis_sim_core::scenarios::coastal_outpost());
    assert!(!session.water_overview);
    assert_eq!(session.selected_object, None);
}

#[test]
fn food_inspection_is_read_only_and_cancellation_uses_the_command_boundary() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut state = genesis_sim_core::scenarios::supplied_settlement();
    state.subsurface.wells[0].active = false;
    for _ in 0..20 {
        state.step(0.1, &[]);
    }
    let mut session = Session::from_sim(state);
    let id = session.sim.people.all()[0].id;
    assert!(session.sim.people.get(id).unwrap().food_task.is_some());
    session.select_object(
        Some(crate::interaction::Object::Person(id)),
        session.position,
    );
    let mut front = crate::frontend::Frontend::default();
    let size = egui::vec2(1100., 800.);
    for label in ["Cancel gathering", "Food"] {
        if label == "Food" {
            session.set_water_overview(true);
        }
        let before = serde_json::to_vec(&*session.sim).unwrap();
        for _ in 0..3 {
            frame(&ctx, size, &mut session, &mut front, vec![]);
        }
        let (output, _) = frame(&ctx, size, &mut session, &mut front, vec![]);
        let point = output
            .shapes
            .iter()
            .find_map(|s| text_position(&s.shape, label))
            .unwrap();
        for pressed in [true, false] {
            frame(
                &ctx,
                size,
                &mut session,
                &mut front,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(serde_json::to_vec(&*session.sim).unwrap(), before);
    }
    assert!(session.supply_food);
    assert_eq!(session.pending_count(), 1);
    assert!(session.sim.people.get(id).unwrap().food_task.is_some());
    session.clock.paused = true;
    session.step_once();
    assert!(session.sim.people.get(id).unwrap().food_task.is_none());
    assert_eq!(
        session.sim.people.get(id).unwrap().food_issue,
        Some(genesis_sim_core::people::FoodIssue::Cancelled)
    );
}

#[test]
fn crop_tool_queues_placement_and_read_only_inspection_uses_current_growth() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut session = Session::from_sim(genesis_sim_core::scenarios::water_settlement());
    let mut front = crate::frontend::Frontend::default();
    let size = egui::vec2(1100., 800.);
    session.set_placing(true);
    for _ in 0..3 {
        frame(&ctx, size, &mut session, &mut front, vec![]);
    }
    let (output, _) = frame(&ctx, size, &mut session, &mut front, vec![]);
    let point = output
        .shapes
        .iter()
        .find_map(|s| text_position(&s.shape, "Barley plot"))
        .unwrap();
    for pressed in [true, false] {
        frame(
            &ctx,
            size,
            &mut session,
            &mut front,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(session.crop_placing);
    session.set_hover(Some(genesis_sim_core::spatial::Position::new(7.5, 8.)));
    session.build_hovered(false);
    assert!(session.sim.food.sources().is_empty());
    assert_eq!(session.pending_count(), 1);
    session.clock.paused = true;
    session.step_once();
    assert_eq!(session.sim.food.sources().len(), 1);
    let before = serde_json::to_vec(&*session.sim).unwrap();
    for _ in 0..3 {
        frame(&ctx, size, &mut session, &mut front, vec![]);
    }
    assert_eq!(serde_json::to_vec(&*session.sim).unwrap(), before);
    assert!(session.selected_object.is_some());
}
