//! Destination → regional scenario → briefing → local settlement.
use super::{
    Frontend, Mode, Screen,
    screens::{action, heading},
};
use crate::{session::Session, ui::theme::*};
use egui::RichText;
use genesis_renderer::camera::Camera;
use genesis_sim_core::{catalog, worldgen::Region};

fn route(ui: &mut egui::Ui, step: usize) {
    ui.horizontal_wrapped(|ui| {
        for (i, text) in ["01 DESTINATION", "02 REGION", "03 BRIEFING"]
            .iter()
            .enumerate()
        {
            ui.label(
                RichText::new(*text)
                    .monospace()
                    .size(10.)
                    .color(if step == i { ACCENT } else { MUTED }),
            );
        }
    });
    ui.add_space(16.);
}
pub fn destination(front: &mut Frontend, ui: &mut egui::Ui) {
    route(ui, 0);
    heading(
        ui,
        "EXPEDITION / DESTINATION",
        "Choose a world",
        "Every community belongs to a larger world.",
    );
    for planet in catalog::PLANETS {
        let selected = front.planet_id == planet.id;
        if action(
            ui,
            &format!("{}   /   {}", planet.name, planet.system),
            selected,
        ) {
            front.planet_id = planet.id.into();
        }
        muted(ui, planet.environment);
    }
    ui.add_space(16.);
    eyebrow(ui, "EXPEDITION ROLE");
    ui.label(RichText::new(front.mode.name()).size(20.));
    ui.label(front.mode.description());
    ui.add_space(14.);
    muted(
        ui,
        "Earth is the first destination. Additional planets will follow as their environments and scenarios become available.",
    );
    ui.add_space(18.);
    if action(ui, "Select a region", true) {
        front.screen = Screen::Setup;
    }
    if action(ui, "Back", false) {
        front.escape();
    }
}
pub fn region(front: &mut Frontend, ui: &mut egui::Ui) {
    route(ui, 1);
    heading(
        ui,
        "EARTH / REGIONAL SCENARIOS",
        "Find your place",
        "Choose the environment your expedition will work in.",
    );
    for region in Region::ALL {
        if catalog::scenario(&front.planet_id, region).is_some()
            && action(ui, region.name(), front.region == region)
        {
            front.region = region;
        }
    }
    ui.add_space(10.);
    ui.label(front.region.description());
    if let Some(anchor) = front.region.anchor() {
        muted(
            ui,
            format!(
                "Regional anchor  {:.2}° N / {:.2}° E",
                anchor.latitude, anchor.longitude
            ),
        );
    } else {
        muted(ui, "Synthetic island · no geographic anchor");
    }
    ui.add_space(14.);
    eyebrow(ui, "SCENARIO / PROCEDURAL PREVIEW");
    ui.label("Undated regional sandbox");
    muted(
        ui,
        "Historical Earth scenarios, from the past through the present, will require their own researched content.",
    );
    ui.add_space(18.);
    if action(ui, "Review expedition", true) {
        front.screen = Screen::Briefing;
        front.message.clear();
    }
    if action(ui, "Back to destinations", false) {
        front.escape();
    }
}
pub fn briefing(front: &mut Frontend, ui: &mut egui::Ui) {
    route(ui, 2);
    heading(
        ui,
        "EXPEDITION / BRIEFING",
        "Ready for the surface",
        front.region.name(),
    );
    if let Some(scenario) = catalog::scenario(&front.planet_id, front.region) {
        eyebrow(ui, "LOCAL CONDITIONS");
        ui.label(scenario.conditions);
        ui.add_space(12.);
        eyebrow(ui, "STARTING SITE");
        ui.label(if front.mode == Mode::Settlement {
            "A small settlement with dwellings and connecting paths."
        } else {
            "Uninhabited terrain, ready for your first intervention."
        });
        ui.add_space(12.);
        eyebrow(ui, "AVAILABLE INTERVENTIONS");
        ui.label("Survey water and minerals. Place dwellings. Shape terrain and seed plants.");
        muted(
            ui,
            if scenario.start_year.is_none() {
                "Undated procedural preview · individual inhabitants are not simulated yet."
            } else {
                "Historical scenario"
            },
        );
    }
    ui.add_space(12.);
    ui.collapsing("Generation settings", |ui| {
        ui.label("World seed");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut front.seed)
                    .desired_width(170.)
                    .char_limit(20),
            );
            if ui.button("Shuffle").clicked() {
                front.seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
                    .wrapping_rem(1_000_000_000)
                    .to_string();
            }
        });
        muted(
            ui,
            "The same scenario and seed reproduce the starting landscape.",
        );
    });
    ui.add_space(14.);
    if action(ui, "Deploy expedition", true) {
        if front.seed.parse::<u64>().is_err() {
            front.message = "Enter a whole-number seed in Generation settings.".into();
        } else if front.has_session {
            front.screen = Screen::ConfirmNew;
            front.message.clear();
        } else {
            front.start();
        }
    }
    if action(ui, "Back to regions", false) {
        front.escape();
    }
}
pub fn arrival(
    front: &mut Frontend,
    ctx: &egui::Context,
    session: &mut Session,
    camera: &mut Camera,
) -> Vec<egui::Rect> {
    egui::Area::new("surface_arrival".into()).anchor(egui::Align2::CENTER_BOTTOM, [0., -24.])
        .movable(false).fade_in(false).order(egui::Order::Foreground).show(ctx, |ui| {
            card().inner_margin(20).show(ui, |ui| {
                ui.set_width((ctx.content_rect().width() - 88.).clamp(240., 510.));
                eyebrow(ui, "EXPEDITION / ARRIVAL");
                ui.heading("Surface link established");
                ui.label(session.sim.geography.region.name());
                muted(ui, format!("{} dwellings · {} watercourses · {} mineral deposits", session.sim.buildings.len(), session.sim.geography.rivers.len(), session.sim.subsurface.deposits.len()));
                ui.add_space(6.);
                ui.label(if front.mode == Mode::Settlement {
                    "Your first site is in view. Inspect its water supply, explore the land, and choose where to build."
                } else { "Survey this region and choose a site for its first settlement." });
                ui.add_space(6.);
                if action(ui, if front.mode == Mode::Settlement { "Enter settlement" } else { "Enter World Builder" }, true) { front.screen = Screen::Playing; }
                if ui.link("Open regional overview instead").clicked() {
                    crate::terrain::home_camera(&session.sim, camera);
                    front.screen = Screen::Playing;
                }
                muted(ui, "WASD to move · scroll to zoom · Esc to enter now");
            });
        });
    vec![ctx.content_rect()]
}
