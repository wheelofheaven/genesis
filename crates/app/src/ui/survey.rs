//! Survey controls, legends, and a schematic selected-column depth profile.
use super::theme::*;
use crate::{session::Session, terrain};
use egui::{Align2, Color32, FontId, RichText, Sense, Stroke};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{
    actions::Action,
    spatial,
    subsurface::{Mineral, PumpMode},
};

pub fn panel(ui: &mut egui::Ui, session: &mut Session) {
    ui.spacing_mut().item_spacing.y = 6.;
    ui.spacing_mut().interact_size.y = 26.;
    ui.spacing_mut().button_padding.y = 6.;
    ui.heading("Beneath the surface");
    ui.label(RichText::new("Synthetic coastal survey").size(11.).color(MUTED))
        .on_hover_text("Generated scenario data. Depth, storage and yield use game units; this is not a geological reconstruction of the Levant.");
    let before = session.survey;
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut session.survey.water, "Hydrology");
        ui.checkbox(&mut session.survey.minerals, "Geology");
        ui.checkbox(&mut session.survey.seafloor, "Seafloor");
    });
    ui.collapsing("Signal settings", |ui| {
        ui.add(egui::Slider::new(&mut session.survey.opacity, 0.2..=1.).text("Signal"));
        if ui.small_button("Surface only").clicked() {
            session.survey.water = false;
            session.survey.minerals = false;
            session.survey.seafloor = false;
        }
    });
    if before != session.survey {
        session.refresh_survey();
    }
    ui.collapsing("Reading the survey", |ui| legend_contents(ui, session));
    ui.separator();
    let p = session.position;
    let reading = session.sim.subsurface.reading(p);
    let ground = spatial::elevation(&session.sim.grid, p);
    eyebrow(ui, "SELECTED GROUND");
    if let Some(geo) = session.sim.geography.region.geographic_position(
        p,
        session.sim.grid.width,
        session.sim.grid.height,
    ) {
        muted(
            ui,
            format!("{:.4}° N · {:.4}° E", geo.latitude, geo.longitude),
        );
    }
    let elevation = ground * genesis_sim_core::units::METRES_PER_UNIT;
    ui.label(format!("Elevation {elevation:+.0} m"));
    if ground < session.sim.sea_level {
        muted(
            ui,
            format!(
                "Water depth {:.0} m",
                (session.sim.sea_level - ground) * genesis_sim_core::units::METRES_PER_UNIT
            ),
        );
    }

    muted(
        ui,
        format!("{:.1}, {:.1} · {}", p.x, p.z, reading.rock.name()),
    );
    if session.survey.water {
        if let Some(well) = session.sim.subsurface.wells.iter().find(|w| {
            (w.position.x - p.x).powi(2) + (w.position.z - p.z).powi(2)
                < genesis_sim_core::units::metres(2.5).powi(2)
        }) {
            let id = well.id;
            let active = well.active;
            let mut fill_buffer = well.pump_mode == PumpMode::FillBuffer;
            ui.label(RichText::new(format!("Borehole {id}")).strong());
            ui.label(well.pumping_status(&session.sim.grid, session.sim.sea_level));
            muted(
                ui,
                format!(
                    "Recovered {:.1} u · yield {:.2} u/s",
                    well.recovered, well.last_yield
                ),
            );
            if let Some(supply) = &well.supply {
                ui.label(format!(
                    "Collectable {:.1} / {:.0} u",
                    supply.stored,
                    genesis_sim_core::subsurface::supply::CAPACITY
                ));
                muted(
                    ui,
                    format!(
                        "Reserved {:.1} u · collected {:.1} u",
                        session.sim.people.reserved_water(id),
                        supply.collected
                    ),
                );
            }
            if ui.checkbox(&mut fill_buffer, "Fill buffer only")
                .on_hover_text("Pause extraction at capacity and refill as water is collected. Uncheck for continuous pumping, which releases surplus water. Stop pumping overrides either mode.")
                .changed() {
                session.queue(Action::SetWellPumpMode { id, mode: if fill_buffer { PumpMode::FillBuffer } else { PumpMode::Continuous } });
            }
            if button(
                ui,
                if active {
                    "Stop pumping"
                } else {
                    "Resume pumping"
                },
                false,
                "Applied on the next simulation tick",
            ) {
                session.queue(Action::SetWellActive {
                    id,
                    active: !active,
                });
            }
        } else {
            let valid = session.sim.validate_well(p);
            if ui
                .add_enabled(valid.is_ok(), egui::Button::new("Drill borehole here"))
                .on_hover_text("Extract groundwater from this location. Pumping reduces local storage; recharge restores it gradually.")
                .clicked()
            {
                session.queue(Action::DrillWell { position: p });
            }
            if let Err(reason) = valid {
                muted(ui, reason.to_string());
            }
        }
    }
    ui.separator();
    if session.survey.water {
        ui.label(
            RichText::new(format!(
                "Groundwater  {:.0} / {:.0} u",
                reading.stored, reading.capacity
            ))
            .strong(),
        );
        ui.add(
            egui::ProgressBar::new(reading.saturation())
                .fill(Color32::from_rgb(44, 153, 184))
                .text(format!(
                    "{:.0}% of local capacity",
                    reading.saturation() * 100.
                )),
        );
        if reading.capacity < 5. {
            muted(ui, "No mapped aquifer here.");
        } else if reading.stored < 0.001 {
            muted(ui, "Aquifer depleted at this point.");
        } else {
            muted(
                ui,
                format!(
                    "Water table {:.1} u below ground",
                    (ground - reading.water_level).max(0.)
                ),
            );
        }
        muted(
            ui,
            format!(
                "Permeability {:.0}% · recharge {:.2} u/s",
                reading.permeability * 100.,
                reading.recharge
            ),
        );
    }
    if session.survey.minerals {
        let mut found = false;
        for deposit in &session.sim.subsurface.deposits {
            let grade = deposit.concentration(p);
            if grade > 0.001 {
                found = true;
                ui.colored_label(
                    color(deposit.mineral),
                    format!(
                        "{} · {:.0}% concentration",
                        deposit.mineral.name(),
                        grade * 100.
                    ),
                );
                muted(
                    ui,
                    format!(
                        "Depth {:.1}–{:.1} u",
                        (ground - deposit.roof).max(0.),
                        (ground - deposit.roof + deposit.thickness).max(0.)
                    ),
                );
            }
        }
        if !found {
            muted(ui, "No mapped deposit at this point.");
        }
    }
    if session.survey.active() {
        ui.collapsing("Underground depth profile", |ui| profile(ui, session));
    }
    ui.collapsing("Mapped deposits", |ui| {
        let deposits = session.sim.subsurface.deposits.clone();
        for d in deposits {
            if ui
                .button(format!("Select {} deposit", d.mineral.name()))
                .clicked()
            {
                session.select(d.center);
            }
        }
    });
}

fn color(mineral: Mineral) -> Color32 {
    match mineral {
        Mineral::Copper => Color32::from_rgb(128, 57, 18),
        Mineral::Iron => Color32::from_rgb(83, 49, 129),
        Mineral::Clay => Color32::from_rgb(108, 79, 15),
    }
}

fn profile(ui: &mut egui::Ui, session: &Session) {
    let p = session.position;
    let r = session.sim.subsurface.reading(p);
    let ground = spatial::elevation(&session.sim.grid, p);
    let deposits: Vec<_> = session
        .sim
        .subsurface
        .deposits
        .iter()
        .filter(|d| d.concentration(p) > 0.001)
        .collect();
    let depth = deposits
        .iter()
        .map(|d| ground - d.roof + d.thickness)
        .fold((ground - r.base).max(1.), f32::max)
        * 1.1;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 112.), Sense::hover());
    let painter = ui.painter_at(rect);
    let top = rect.top() + 16.;
    let bottom = rect.bottom() - 16.;
    let y = |e: f32| top + ((ground - e) / depth).clamp(0., 1.) * (bottom - top);
    let strip = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 4., top),
        egui::pos2(rect.left() + 58., bottom),
    );
    painter.rect_filled(strip, 3, Color32::from_rgb(192, 178, 145));
    if session.survey.water && r.capacity > 0. {
        let reservoir = egui::Rect::from_min_max(
            egui::pos2(strip.left(), y(r.top)),
            egui::pos2(strip.right(), y(r.base)),
        );
        painter.rect_filled(reservoir, 0, Color32::from_rgb(156, 188, 190));
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(strip.left(), y(r.water_level)),
                egui::pos2(strip.right(), y(r.base)),
            ),
            0,
            Color32::from_rgb(47, 139, 173),
        );
        painter.line_segment(
            [
                egui::pos2(strip.left(), y(r.water_level)),
                egui::pos2(strip.right() + 8., y(r.water_level)),
            ],
            Stroke::new(1.5, Color32::from_rgb(34, 100, 146)),
        );
    }
    if session.survey.minerals {
        for d in deposits {
            let min = egui::pos2(strip.left() + 14., y(d.roof));
            let max = egui::pos2(strip.right() - 4., y(d.roof - d.thickness));
            painter.rect_filled(egui::Rect::from_min_max(min, max), 2, color(d.mineral));
        }
    }
    painter.text(
        egui::pos2(rect.left() + 66., top),
        Align2::LEFT_TOP,
        "Surface",
        FontId::proportional(11.),
        TEXT,
    );
    painter.text(
        egui::pos2(rect.left() + 66., top + 23.),
        Align2::LEFT_TOP,
        r.rock.name(),
        FontId::proportional(11.),
        MUTED,
    );
    painter.text(
        egui::pos2(rect.left() + 66., bottom),
        Align2::LEFT_BOTTOM,
        format!("{depth:.1} depth units"),
        FontId::proportional(11.),
        MUTED,
    );
    painter.text(
        egui::pos2(rect.left() + 4., rect.bottom()),
        Align2::LEFT_BOTTOM,
        "Vertical profile · selected location",
        FontId::proportional(10.),
        MUTED,
    );
}

pub fn legend(ctx: &egui::Context, session: &Session, dock_top: f32) -> Option<egui::Rect> {
    if !session.survey.active()
        || session.survey.open
        || session.placing
        || session.path.active
        || session.selected_object.is_some()
        || session.landscape_tools
        || session.world_tools
        || session.water_overview
    {
        return None;
    }
    Some(
        egui::Area::new("survey_legend".into())
            .anchor(
                Align2::RIGHT_TOP,
                [
                    -16.,
                    if ctx.content_rect().width() < 1120. {
                        148.
                    } else {
                        88.
                    },
                ],
            )
            .order(egui::Order::Foreground)
            .movable(false)
            .fade_in(false)
            .show(ctx, |ui| {
                card().inner_margin(12).show(ui, |ui| {
                    ui.set_width(158.);
                    let top = if ctx.content_rect().width() < 1120. {
                        148.
                    } else {
                        88.
                    };
                    egui::ScrollArea::vertical()
                        .max_height((dock_top - top - 42.).max(70.))
                        .show(ui, |ui| legend_contents(ui, session));
                });
            })
            .response
            .rect,
    )
}

pub fn legend_contents(ui: &mut egui::Ui, session: &Session) {
    eyebrow(ui, "SUBSURFACE SIGNALS");
    if session.survey.seafloor {
        muted(ui, "Seafloor exposed");
        muted(ui, "Depth contours · 25 m");
    }
    if session.survey.water {
        muted(ui, "Groundwater stored");
        let (rect, _) = ui.allocate_exact_size(egui::vec2(158., 8.), Sense::hover());
        for i in 0..32 {
            let t = i as f32 / 31.;
            let rgb = crate::survey::water_color(t * 200.);
            let c: Color32 = egui::Rgba::from_rgb(rgb.x, rgb.y, rgb.z).into();
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(rect.left() + i as f32 * 158. / 32., rect.top()),
                    egui::pos2(rect.left() + (i + 1) as f32 * 158. / 32., rect.bottom()),
                ),
                0,
                c,
            );
        }
        muted(ui, "0                     200+ u");
    }
    if session.survey.minerals {
        for m in [Mineral::Copper, Mineral::Iron, Mineral::Clay] {
            ui.colored_label(color(m), format!("— {}", m.name()));
        }
        muted(ui, "Outlines = body extent");
    }
}

pub fn labels(ctx: &egui::Context, session: &Session, camera: &Camera, occupied: &[egui::Rect]) {
    if !session.survey.minerals {
        return;
    }
    let screen = ctx.content_rect();
    let matrix = camera.view_projection(screen.width() / screen.height());
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        "deposit_labels".into(),
    ));
    for d in &session.sim.subsurface.deposits {
        let world = terrain::surface_point(&session.sim, d.center) + glam::Vec3::Y * 0.08;
        let ndc = matrix.project_point3(world);
        let p = egui::pos2(
            screen.left() + (ndc.x + 1.) * 0.5 * screen.width(),
            screen.top() + (1. - ndc.y) * 0.5 * screen.height(),
        );
        let rect = egui::Rect::from_center_size(p, egui::vec2(74., 24.));
        if !screen.contains_rect(rect) || occupied.iter().any(|r| r.intersects(rect)) {
            continue;
        }
        painter.rect_filled(rect, 5, glass_tint());
        painter.text(
            p,
            Align2::CENTER_CENTER,
            d.mineral.name(),
            FontId::proportional(12.),
            color(d.mineral),
        );
    }
}
