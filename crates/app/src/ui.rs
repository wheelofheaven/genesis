mod navigation;
mod survey;
pub mod theme;

use crate::session::Session;
use egui::{Align2, Color32, RichText, Stroke};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{actions::Action, spatial};
use theme::*;

/// Floating cards return their actual bounds to keep world input out of the HUD.
pub fn show(ctx: &egui::Context, session: &mut Session, camera: &mut Camera) -> Vec<egui::Rect> {
    let mut occupied = Vec::new();
    let identity = egui::Area::new("identity".into())
        .anchor(Align2::LEFT_TOP, [20., 20.])
        .order(egui::Order::Foreground)
        .movable(false)
        .fade_in(false)
        .show(ctx, |ui| {
            card().show(ui, |ui| {
                ui.set_width(236.);
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(32., 38.), egui::Sense::hover());
                    let painter = ui.painter();
                    let center = rect.center();
                    painter.circle_stroke(center, 12., Stroke::new(1.5, ACCENT));
                    painter.line_segment(
                        [center + egui::vec2(-16., 4.), center + egui::vec2(16., 4.)],
                        Stroke::new(1.5, GOLD),
                    );
                    painter.circle_filled(center - egui::vec2(0., 5.), 3., GOLD);
                    ui.vertical(|ui| {
                        eyebrow(ui, "G E N E S I S");
                        ui.label(RichText::new("Coastal region").size(19.).color(INK));
                    });
                });
                muted(
                    ui,
                    format!(
                        "{} dwellings  ·  {}",
                        session.sim.buildings.len(),
                        if session.survey.active() {
                            session.survey.label()
                        } else if session.placing {
                            "Shaping a settlement"
                        } else {
                            "A world taking shape"
                        }
                    ),
                );
            });
        });
    occupied.push(identity.response.rect);
    let time = egui::Area::new("time".into())
        .anchor(Align2::RIGHT_TOP, [-20., 20.])
        .order(egui::Order::Foreground)
        .movable(false)
        .fade_in(false)
        .show(ctx, |ui| {
            card().inner_margin(10).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(8., 8.), egui::Sense::hover());
                    ui.painter().circle_filled(
                        r.center(),
                        3.,
                        if session.clock.paused { GOLD } else { ACCENT },
                    );
                    if button(
                        ui,
                        if session.clock.paused {
                            "Resume"
                        } else {
                            "Pause"
                        },
                        false,
                        "Space · pause or resume the world",
                    ) {
                        session.clock.paused = !session.clock.paused;
                    }
                    ui.separator();
                    for speed in [1, 4, 16] {
                        if button(
                            ui,
                            &format!("{speed}×"),
                            session.clock.speed() == speed,
                            "1 / 2 / 3 · simulation speed",
                        ) {
                            session.clock.set_speed(speed);
                        }
                    }
                    if ui
                        .add_enabled(session.clock.paused, egui::Button::new("Step"))
                        .on_hover_text("Advance one tick while paused")
                        .clicked()
                    {
                        session.step_once();
                    }
                });
            });
        });
    occupied.push(time.response.rect);
    occupied.push(navigation::show(ctx, session, camera));
    occupied.push(navigation::scale(ctx, camera));

    if session.placing || session.landscape_tools || session.survey.open || session.world_tools {
        let available =
            (ctx.content_rect().height() - identity.response.rect.bottom() - 186.).max(100.);
        let tool = egui::Area::new("tool_card".into())
            .fixed_pos([20., identity.response.rect.bottom() + 16.])
            .default_height(available + 38.)
            .order(egui::Order::Foreground)
            .movable(false)
            .fade_in(false)
            .show(ctx, |ui| {
                card().inner_margin(18).show(ui, |ui| {
                    ui.set_width(228.);
                    ui.set_max_height(available);
                    egui::ScrollArea::vertical()
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                        )
                        .max_height(available)
                        .show(ui, |ui| {
                            if session.world_tools {
                                world(ui, session, camera);
                            } else if session.survey.open {
                                survey::panel(ui, session);
                            } else if session.placing {
                                building(ui, session, ctx.content_rect().height() < 650.);
                            } else {
                                landscape(ui, session);
                            }
                        });
                });
            });
        occupied.push(tool.response.rect);
    }

    let dock = egui::Area::new("tool_dock".into())
        .anchor(Align2::CENTER_BOTTOM, [0., -56.])
        .order(egui::Order::Foreground)
        .movable(false)
        .fade_in(false)
        .show(ctx, |ui| {
            card().inner_margin(10).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if button(
                        ui,
                        "Explore",
                        !session.placing
                            && !session.landscape_tools
                            && !session.survey.open
                            && !session.world_tools,
                        "Select a location and explore · Esc",
                    ) {
                        session.set_placing(false);
                        session.landscape_tools = false;
                        session.survey.open = false;
                        session.world_tools = false;
                    }
                    if button(
                        ui,
                        "Survey   V",
                        session.survey.open,
                        "Hydrology and geological layers · V",
                    ) {
                        session.set_survey(!session.survey.open);
                    }
                    if button(ui, "Build   B", session.placing, "Place a dwelling · B") {
                        session.set_placing(!session.placing);
                    }
                    if button(
                        ui,
                        "Landscape",
                        session.landscape_tools,
                        "Raise, lower, or plant at the selected location",
                    ) {
                        session.set_placing(false);
                        session.landscape_tools = !session.landscape_tools;
                        session.survey.open = false;
                        session.world_tools = false;
                    }
                    if button(
                        ui,
                        "World",
                        session.world_tools,
                        "Seed and region generation",
                    ) {
                        let open = !session.world_tools;
                        session.set_placing(false);
                        session.survey.open = false;
                        session.landscape_tools = false;
                        session.world_tools = open;
                    }
                    ui.separator();
                    if button(
                        ui,
                        "Help   H",
                        session.help,
                        "Navigation and keyboard controls · H",
                    ) {
                        session.help = !session.help;
                    }
                });
            });
        });
    occupied.push(dock.response.rect);
    let hint = egui::Area::new("hints".into())
        .anchor(Align2::CENTER_BOTTOM, [0., -12.])
        .order(egui::Order::Foreground)
        .movable(false)
        .fade_in(false)
        .show(ctx, |ui| {
            ui.spacing_mut().interact_size.y = 16.;
            ui.spacing_mut().item_spacing.y = 0.;
            card()
                .inner_margin(egui::Margin::symmetric(12, 5))
                .corner_radius(8)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if session.placing {
                            key_hint(ui, "Click", "Build");
                            key_hint(ui, "Q / E", "Rotate");
                            key_hint(ui, "Shift", "Repeat");
                            key_hint(ui, "Esc", "Cancel");
                        } else {
                            key_hint(ui, "WASD", "Move");
                            key_hint(ui, "Drag right", "Orbit");
                            key_hint(ui, "Scroll", "Zoom");
                        }
                    });
                });
        });
    occupied.push(hint.response.rect);
    if session.pending_count() > 0 || !session.message.is_empty() {
        // Feedback belongs near the action dock, without obscuring the placement site.
        let response = egui::Area::new("feedback".into())
            .anchor(Align2::CENTER_BOTTOM, [0., -128.])
            .order(egui::Order::Foreground)
            .movable(false)
            .fade_in(false)
            .show(ctx, |ui| {
                card()
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.set_max_width(370.);
                        if session.pending_count() > 0 {
                            ui.label(
                                RichText::new(format!(
                                    "{} action{} queued",
                                    session.pending_count(),
                                    if session.pending_count() == 1 {
                                        ""
                                    } else {
                                        "s"
                                    }
                                ))
                                .strong()
                                .color(GOLD),
                            );
                            if session.clock.paused {
                                muted(ui, "Resume or Step to apply your changes.");
                            }
                        } else {
                            ui.label(RichText::new(&session.message).color(
                                if session.message_is_error {
                                    ERROR
                                } else {
                                    ACCENT
                                },
                            ));
                        }
                    });
            });
        occupied.push(response.response.rect);
    }
    if let Some(rect) = survey::legend(ctx, session) {
        occupied.push(rect);
    }
    survey::labels(ctx, session, camera, &occupied);
    if session.help
        && let Some(rect) = navigation::help(ctx, &mut session.help)
    {
        occupied.push(rect);
    }
    occupied
}

fn world(ui: &mut egui::Ui, session: &mut Session, camera: &mut Camera) {
    eyebrow(ui, "WORLD / GENERATION");
    ui.heading("A place to begin");
    muted(ui, "Mediterranean-inspired coast");
    ui.label(format!("Seed {}", session.sim.config.seed));
    muted(
        ui,
        format!(
            "{:.2} × {:.2} km region",
            session.sim.grid.width as f32 * genesis_sim_core::units::METRES_PER_UNIT / 1000.,
            session.sim.grid.height as f32 * genesis_sim_core::units::METRES_PER_UNIT / 1000.
        ),
    );
    muted(
        ui,
        format!(
            "{} watercourses · {} deposits",
            session.sim.geography.rivers.len(),
            session.sim.subsurface.deposits.len()
        ),
    );
    ui.separator();
    muted(
        ui,
        "Ridges, valleys and drainage shape the region. The settlement seeks gentle ground near water.",
    );
    ui.add_space(6.);
    if button(
        ui,
        "Generate next seed",
        false,
        "Replace this region and its edits with the next seed",
    ) {
        session.regenerate(session.sim.config.seed.wrapping_add(1));
        crate::terrain::home_camera(&session.sim, camera);
    }
    if button(
        ui,
        "Recreate this seed",
        false,
        "Restart this region from its original generated state",
    ) {
        session.regenerate(session.sim.config.seed);
        crate::terrain::home_camera(&session.sim, camera);
    }
    muted(
        ui,
        "Generating a region replaces the current settlement and all edits.",
    );
    ui.separator();
    muted(
        ui,
        "Generated landscape · rivers follow the original terrain and do not yet reroute after edits.",
    );
}

fn building(ui: &mut egui::Ui, session: &mut Session, compact: bool) {
    eyebrow(ui, "SETTLEMENT / BUILD");
    ui.heading("A new dwelling");
    muted(ui, "6.5 × 4.8 m · single storey");
    if !compact {
        muted(ui, "Choose a place for the settlement to grow.");
    }
    ui.add_space(4.);
    let result = session
        .hover
        .map(|_| session.sim.validate_building(&session.candidate()));
    let (text, color) = match result {
        Some(Ok(())) => ("Ready to build".to_owned(), ACCENT),
        Some(Err(reason)) => (reason.to_string(), ERROR),
        None => ("Move onto the landscape".to_owned(), MUTED),
    };
    egui::Frame::new()
        .fill(Color32::from_rgb(231, 232, 212))
        .inner_margin(10)
        .corner_radius(7)
        .show(ui, |ui| {
            ui.set_min_width(208.);
            ui.label(RichText::new(text).strong().color(color));
        });
    ui.add_space(4.);
    eyebrow(ui, "ORIENTATION");
    ui.horizontal(|ui| {
        if button(ui, "−15°", false, "Rotate left · Q") {
            session.rotate_building(-15_f32.to_radians());
        }
        ui.add_sized(
            [54., 32.],
            egui::Label::new(
                RichText::new(format!("{:.0}°", session.building_rotation.to_degrees())).strong(),
            ),
        );
        if button(ui, "+15°", false, "Rotate right · E") {
            session.rotate_building(15_f32.to_radians());
        }
    });
    if !compact {
        muted(ui, "Click to place. Hold Shift to build another.");
    }
    if button(ui, "Cancel placement", false, "Return to exploration · Esc") {
        session.set_placing(false);
    }
}

fn landscape(ui: &mut egui::Ui, session: &mut Session) {
    eyebrow(ui, "LANDSCAPE / SHAPE");
    ui.heading("Tend the land");
    muted(ui, "Click the landscape to choose where to work.");
    let p = session.position;
    let h = spatial::elevation(&session.sim.grid, p);
    ui.add_space(4.);
    ui.label(
        RichText::new(if h <= session.sim.sea_level {
            "Waterfront"
        } else {
            "Selected ground"
        })
        .strong(),
    );
    muted(ui, format!("Position {:.1}, {:.1}", p.x, p.z));
    let (x, z) = session.selected;
    ui.horizontal(|ui| {
        if button(ui, "Raise", false, "Lift the ground around this point") {
            session.queue(Action::Terraform {
                x,
                y: z,
                radius: 3,
                strength: 0.4,
            });
        }
        if button(ui, "Lower", false, "Lower the ground around this point") {
            session.queue(Action::Terraform {
                x,
                y: z,
                radius: 3,
                strength: -0.4,
            });
        }
        if button(ui, "Plant", false, "Seed plants on dry ground") {
            session.queue(Action::SeedPlants { x, y: z, radius: 2 });
        }
    });
    ui.add_space(4.);
    muted(ui, "Changes follow the land’s contours.");
    if button(ui, "Done", false, "Return to exploration · Esc") {
        session.landscape_tools = false;
    }
}
