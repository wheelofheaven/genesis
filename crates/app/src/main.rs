use bevy::asset::AssetPlugin;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{RenderCreation, WgpuFeatures, WgpuLimits, WgpuSettings};
use genesis_planet::PlanetPlugin;
use genesis_renderer::OrbitCamera;

mod input;
mod plugins;
mod render;
mod ui;

use plugins::simulation::SimulationPlugin;

fn main() {
    // Workaround for two macOS 26 Tahoe + Metal issues:
    // 1. GPU preprocessing (indirect draw) crashes — disable the features that
    //    enable it so Bevy falls back to CPU-side batch preparation.
    // 2. wgpu-types 27 validates min/max_subgroup_size unconditionally, but
    //    Bevy's constrained_limits logic can create an invalid state where
    //    min_subgroup_size (clamped up) > max_subgroup_size (clamped down to 0).
    //    Setting max_subgroup_size to u32::MAX prevents the downward clamp.
    let wgpu_settings = WgpuSettings {
        disabled_features: Some(
            WgpuFeatures::INDIRECT_FIRST_INSTANCE | WgpuFeatures::PUSH_CONSTANTS,
        ),
        constrained_limits: Some(WgpuLimits {
            max_subgroup_size: u32::MAX,
            ..WgpuLimits::default()
        }),
        ..default()
    };

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Genesis — Ocean Terraformer".into(),
                        resolution: (1280, 800).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "../../assets".to_string(),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(wgpu_settings),
                    ..default()
                }),
        )
        .add_plugins(genesis_renderer::CameraPlugin)
        .add_plugins(SimulationPlugin)
        .add_plugins(PlanetPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(mut commands: Commands) {
    // 3D camera with orbit controls and tonemapping.
    // NoIndirectDrawing: workaround for macOS 26 Tahoe Metal driver crash
    // with indirect draw calls during GPU preprocessing.
    commands.spawn((
        Camera3d::default(),
        Tonemapping::TonyMcMapface,
        OrbitCamera::default(),
        bevy::render::view::NoIndirectDrawing,
    ));

    // Directional light (sun).
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.5, 0.0)),
    ));
}
