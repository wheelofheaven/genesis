//! Overview at the top, world actions below, contextual details at the right.
use super::{
    icons::{self, Icon},
    theme::*,
};
use crate::{
    frontend::{Frontend, Screen},
    session::Session,
};
use egui::{Align2, RichText};

fn area(id: &str, anchor: Align2, offset: [f32; 2]) -> egui::Area {
    egui::Area::new(egui::Id::new(id))
        .anchor(anchor, offset)
        .order(egui::Order::Foreground)
        .movable(false)
        .fade_in(false)
}

pub fn identity(ctx: &egui::Context, session: &mut Session) -> egui::Rect {
    area("identity", Align2::LEFT_TOP, [16., 16.])
        .show(ctx, |ui| {
            card().inner_margin(12).show(ui, |ui| {
                ui.set_width(218.);
                ui.spacing_mut().item_spacing.y = 6.;
                ui.label(
                    RichText::new(session.sim.geography.region.name())
                        .size(16.)
                        .color(TEXT),
                );
                ui.horizontal(|ui| {
                    ui.spacing_mut().interact_size.y = 20.;
                    for (icon, count, label) in [
                        (Icon::House, session.sim.buildings.len(), "Dwellings"),
                        (Icon::Person, session.sim.people.all().len(), "Residents"),
                    ] {
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(18., 18.), egui::Sense::hover());
                        icons::paint(ui.painter(), rect, icon, MUTED);
                        response.on_hover_text(label);
                        ui.label(RichText::new(count.to_string()).size(13.).color(TEXT))
                            .on_hover_text(label);
                        ui.add_space(8.);
                    }
                    if ui
                        .small_button("Supplies")
                        .on_hover_text("Household water, food and collection tasks")
                        .clicked()
                    {
                        session.set_water_overview(!session.water_overview);
                    }
                });
            });
        })
        .response
        .rect
}

pub fn utilities(ctx: &egui::Context, session: &mut Session, front: &mut Frontend) -> egui::Rect {
    area("utilities", Align2::RIGHT_TOP, [-16., 16.])
        .show(ctx, |ui| {
            card().inner_margin(6).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.;
                    if icons::button(
                        ui,
                        Icon::Globe,
                        "Region",
                        session.world_tools,
                        false,
                        "Region information and generation",
                    )
                    .clicked()
                    {
                        let open = !session.world_tools;
                        session.set_placing(false);
                        session.survey.open = false;
                        session.landscape_tools = false;
                        session.world_tools = open;
                    }
                    if icons::button(ui, Icon::Help, "Help", session.help, false, "Help · H")
                        .clicked()
                    {
                        session.help = !session.help;
                    }
                    if icons::button(
                        ui,
                        Icon::Menu,
                        "Menu",
                        false,
                        false,
                        "Menu and settings · Esc",
                    )
                    .clicked()
                    {
                        front.screen = Screen::Pause;
                    }
                });
            });
        })
        .response
        .rect
}

pub fn time(ctx: &egui::Context, session: &mut Session, compact: bool) -> egui::Rect {
    let (anchor, offset) = if compact {
        (Align2::RIGHT_TOP, [-16., 82.])
    } else {
        (Align2::RIGHT_BOTTOM, [-16., -16.])
    };
    area("time", anchor, offset)
        .show(ctx, |ui| {
            card().inner_margin(8).show(ui, |ui| {
                ui.spacing_mut().button_padding.x = 9.;
                ui.horizontal(|ui| {
                    if icons::button(
                        ui,
                        if session.clock.paused {
                            Icon::Play
                        } else {
                            Icon::Pause
                        },
                        "Pause / resume",
                        false,
                        false,
                        "Pause / resume · Space",
                    )
                    .clicked()
                    {
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
                    ui.add_enabled_ui(session.clock.paused, |ui| {
                        if icons::button(
                            ui,
                            Icon::Step,
                            "Step",
                            false,
                            false,
                            "Advance one tick while paused",
                        )
                        .clicked()
                        {
                            session.step_once();
                        }
                    });
                });
            });
        })
        .response
        .rect
}

pub fn dock(ctx: &egui::Context, session: &mut Session) -> egui::Rect {
    area("tool_dock", Align2::CENTER_BOTTOM, [0., -16.])
        .show(ctx, |ui| {
            card().inner_margin(6).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.;
                    let exploring = !session.placing
                        && !session.path.active
                        && !session.landscape_tools
                        && !session.survey.open
                        && !session.world_tools
                        && !session.water_overview;
                    if icons::button(
                        ui,
                        Icon::Explore,
                        "Explore",
                        exploring,
                        true,
                        "Explore and select · Esc",
                    )
                    .clicked()
                    {
                        session.set_placing(false);
                        session.landscape_tools = false;
                        session.survey.open = false;
                        session.world_tools = false;
                    }
                    if icons::button(
                        ui,
                        Icon::House,
                        "Build",
                        session.placing,
                        true,
                        "Place a dwelling or barley plot · B\nQ / E: rotate dwelling · Shift-click: repeat",
                    )
                    .clicked()
                    {
                        session.set_placing(!session.placing);
                    }
                    if icons::button(
                        ui,
                        Icon::Path,
                        "Paths",
                        session.path.active,
                        true,
                        "Lay a footpath · P\nClick waypoints · Enter builds · Backspace undoes",
                    )
                    .clicked()
                    {
                        session.set_path_tool(!session.path.active);
                    }
                    if icons::button(
                        ui,
                        Icon::Terrain,
                        "Landscape",
                        session.landscape_tools,
                        true,
                        "Raise, lower, or plant at the selected location",
                    )
                    .clicked()
                    {
                        let open = !session.landscape_tools;
                        session.set_placing(false);
                        session.landscape_tools = open;
                        session.survey.open = false;
                        session.world_tools = false;
                    }
                    if icons::button(
                        ui,
                        Icon::Survey,
                        "Survey",
                        session.survey.open,
                        true,
                        &format!(
                            "{} · V\nHydrology, geology, and seafloor",
                            session.survey.label()
                        ),
                    )
                    .clicked()
                    {
                        session.set_survey(!session.survey.open);
                    }
                });
            });
        })
        .response
        .rect
}

pub fn feedback(ctx: &egui::Context, session: &Session) -> Option<egui::Rect> {
    if session.pending_count() == 0 && session.message.is_empty() {
        return None;
    }
    Some(
        area("feedback", Align2::LEFT_TOP, [16., 102.])
            .show(ctx, |ui| {
                card().inner_margin(12).show(ui, |ui| {
                    ui.set_width((ctx.content_rect().width() - 350.).clamp(140., 218.));
                    if session.pending_count() > 0 {
                        ui.label(
                            RichText::new(format!("{} actions queued", session.pending_count()))
                                .color(HIGHLIGHT),
                        );
                        if session.clock.paused {
                            muted(ui, "Resume or Step to apply changes.");
                        }
                    } else {
                        ui.label(RichText::new(&session.message).size(12.).color(
                            if session.message_is_error {
                                ERROR
                            } else {
                                ACCENT
                            },
                        ));
                    }
                });
            })
            .response
            .rect,
    )
}
