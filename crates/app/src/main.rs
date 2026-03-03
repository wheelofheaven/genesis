use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod input;
mod plugins;
mod render;
mod ui;

use plugins::simulation::SimulationPlugin;
use render::grid_render::GridRenderPlugin;
use ui::overlay::OverlayUiPlugin;

fn main() {
    // Single-pass mode: multipass crashes on Metal/macOS.
    // See https://github.com/bevyengine/bevy/issues/18149
    #[allow(deprecated)]
    let egui_plugin = EguiPlugin {
        enable_multipass_for_primary_context: false,
        ..default()
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Genesis — Ocean Terraformer".into(),
                resolution: (1280, 800).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(egui_plugin)
        .add_plugins(SimulationPlugin)
        .add_plugins(GridRenderPlugin)
        .add_plugins(OverlayUiPlugin)
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
