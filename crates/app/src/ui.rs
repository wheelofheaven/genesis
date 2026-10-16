mod cultivation;
pub mod glass;
mod hud;
pub(crate) mod icons;
#[cfg(test)]
mod interaction_tests;
mod navigation;
mod objects;
mod survey;
pub mod theme;
mod water;
mod weather;

use crate::session::Session;
use egui::{Align2, Color32, RichText};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{actions::Action, spatial};
use theme::*;

/// Floating cards return their actual bounds to keep world input out of the HUD.
pub fn show(
    ctx: &egui::Context,
    session: &mut Session,
    camera: &mut Camera,
    front: &mut crate::frontend::Frontend,
    atmosphere: &mut genesis_renderer::atmosphere::Atmosphere,
) -> Vec<egui::Rect> {
    let compact = ctx.content_rect().width() < 1120.;
    let mut occupied = vec![
        hud::identity(ctx, session),
        hud::utilities(ctx, session, front),
        hud::time(ctx, session, compact),
        hud::dock(ctx, session),
        navigation::show(ctx, session, camera),
        navigation::scale(ctx, camera),
    ];
    if session.placing
        || session.path.active
        || session.selected_object.is_some()
        || session.landscape_tools
        || session.survey.open
        || session.world_tools
        || session.water_overview
    {
        let top = if compact { 148. } else { 88. };
        let available = (occupied[3].top() - top - 50.).max(80.);
        let tool = egui::Area::new("tool_card".into())
            .anchor(Align2::RIGHT_TOP, [-16., top])
            .order(egui::Order::Foreground)
            .movable(false)
            .fade_in(false)
            .show(ctx, |ui| {
                card().inner_margin(16).show(ui, |ui| {
                    ui.set_width(248.);
                    ui.set_max_height(available);
                    egui::ScrollArea::vertical()
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                        )
                        .max_height(available)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                eyebrow(ui, "INSPECTOR");
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button("×")
                                            .on_hover_text("Close tool · Esc")
                                            .clicked()
                                        {
                                            session.set_placing(false);
                                            session.selected_object = None;
                                            session.survey.open = false;
                                            session.landscape_tools = false;
                                            session.world_tools = false;
                                        }
                                    },
                                );
                            });
                            ui.separator();
                            if session.survey.active() && !session.survey.open {
                                ui.collapsing("Active survey legend", |ui| {
                                    survey::legend_contents(ui, session)
                                });
                            }
                            if session.water_overview {
                                water::panel(ui, session, camera);
                            } else if session.world_tools {
                                weather::show(ui, atmosphere);
                                ui.separator();
                                world(ui, session, camera);
                            } else if session.survey.open {
                                survey::panel(ui, session);
                            } else if session.placing {
                                building(ui, session, ctx.content_rect().height() < 650.);
                            } else if session.path.active {
                                objects::path_tool(ui, session);
                            } else if session.landscape_tools {
                                landscape(ui, session);
                            } else {
                                objects::selection(ui, session);
                            }
                        });
                });
            });
        occupied.push(tool.response.rect);
    }
    if let Some(rect) = hud::feedback(ctx, session) {
        occupied.push(rect);
    }
    if let Some(rect) = survey::legend(ctx, session, occupied[3].top()) {
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
    ui.heading("Regional survey");
    muted(ui, session.sim.geography.region.description());
    muted(
        ui,
        "Procedural preview · Earth reconstruction in development",
    );
    egui::ComboBox::from_id_salt("region_choice")
        .selected_text(session.region_choice.name())
        .show_ui(ui, |ui| {
            for region in genesis_sim_core::worldgen::Region::ALL {
                ui.selectable_value(&mut session.region_choice, region, region.name());
            }
        });
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
        "Generate selected region",
        false,
        "Replace the current region using the selected place and current seed",
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
    ui.horizontal(|ui| {
        if button(ui, "Dwelling", !session.crop_placing, "Place a home") {
            session.crop_placing = false;
            session.overlay_dirty = true;
        }
        if button(
            ui,
            "Barley plot",
            session.crop_placing,
            "Plant a renewable food source",
        ) {
            session.crop_placing = true;
            session.overlay_dirty = true;
        }
    });
    if session.crop_placing {
        cultivation::placement(ui, session);
        return;
    }
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
        .fill(Color32::from_white_alpha(16))
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
    ui.heading("Terrain intervention");
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
