// Module temporarily unused: egui disabled due to bevy_egui Metal crash on macOS 26.
#![allow(dead_code)]

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::input::tool::ToolKind;
use crate::plugins::simulation::{SimEventLog, Simulation};

/// Which data layer is shown on the grid.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayMode {
    #[default]
    Elevation,
    Temperature,
    Moisture,
    Fertility,
    Biome,
    Life,
}

/// Current player tool selection.
#[derive(Resource, Default)]
pub struct ActiveTool(pub ToolKind);

/// Simulation speed setting.
#[derive(Resource)]
pub struct SimSpeed(pub u32);

impl Default for SimSpeed {
    fn default() -> Self {
        Self(1)
    }
}

impl SimSpeed {
    pub fn multiplier(&self) -> f32 {
        self.0 as f32
    }
}

pub struct OverlayUiPlugin;

impl Plugin for OverlayUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverlayMode>()
            .insert_resource(ActiveTool::default())
            .add_systems(Update, draw_ui);
    }
}

fn draw_ui(
    mut contexts: EguiContexts,
    mut overlay: ResMut<OverlayMode>,
    mut tool: ResMut<ActiveTool>,
    mut speed: ResMut<SimSpeed>,
    sim: Res<Simulation>,
    log: Res<SimEventLog>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // Left panel: tools
    egui::SidePanel::left("tools_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.heading("Tools");
            ui.separator();

            let tools = [
                (ToolKind::TerraformRaise, "Raise Land"),
                (ToolKind::TerraformLower, "Lower Land"),
                (ToolKind::SeedFungus, "Seed Fungus"),
                (ToolKind::SeedPlants, "Seed Plants"),
                (ToolKind::BuildOutpost, "Build Outpost"),
            ];

            for (kind, label) in &tools {
                if ui.selectable_label(tool.0 == *kind, *label).clicked() {
                    tool.0 = *kind;
                }
            }

            ui.separator();
            ui.heading("Speed");
            let speeds = [(0, "Paused"), (1, "1x"), (4, "4x"), (16, "16x")];
            ui.horizontal_wrapped(|ui| {
                for (val, label) in &speeds {
                    if ui.selectable_label(speed.0 == *val, *label).clicked() {
                        speed.0 = *val;
                    }
                }
            });

            ui.separator();
            ui.heading("Status");
            ui.label(format!("Time: {:.0}", sim.0.time));
            ui.label(format!("Stability: {:.1}%", sim.0.stability_score * 100.0));
            ui.label(format!("Research: {:.1}", sim.0.research_points));
            ui.label(format!("Outposts: {}", sim.0.outposts.len()));
        });

    // Top panel: overlay toggles
    egui::TopBottomPanel::top("overlay_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Overlay:");
            let overlays = [
                (OverlayMode::Elevation, "Elevation"),
                (OverlayMode::Temperature, "Temperature"),
                (OverlayMode::Moisture, "Moisture"),
                (OverlayMode::Fertility, "Fertility"),
                (OverlayMode::Biome, "Biome"),
                (OverlayMode::Life, "Life"),
            ];
            for (mode, label) in &overlays {
                if ui.selectable_label(*overlay == *mode, *label).clicked() {
                    *overlay = *mode;
                }
            }
        });
    });

    // Bottom panel: expedition log
    egui::TopBottomPanel::bottom("log_panel")
        .resizable(true)
        .default_height(120.0)
        .show(ctx, |ui| {
            ui.heading("Expedition Log");
            ui.separator();
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for entry in log.0.iter().rev().take(50) {
                        ui.label(entry);
                    }
                });
        });
}
