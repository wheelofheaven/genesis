use bevy::prelude::*;

use crate::heightmap::sample_heightmap;
use crate::mesh::CubeSphere;

/// Configuration for planet generation.
#[derive(Resource)]
pub struct PlanetConfig {
    pub radius: f32,
    pub resolution: u32,
    pub heightmap_amplitude: f32,
    pub heightmap_frequency: f32,
}

impl Default for PlanetConfig {
    fn default() -> Self {
        Self {
            radius: 300.0,
            resolution: 64,
            heightmap_amplitude: 2.0,
            heightmap_frequency: 3.0,
        }
    }
}

/// Marker component for the planet entity.
#[derive(Component)]
pub struct Planet;

/// Plugin that spawns a procedural cube-sphere planet.
pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlanetConfig>()
            .add_systems(Startup, spawn_planet);
    }
}

fn spawn_planet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<PlanetConfig>,
) {
    // DEBUG: Use Bevy's built-in sphere to test if the crash is mesh-related.
    let _ = &config;

    commands.spawn((
        Planet,
        Mesh3d(meshes.add(Sphere::new(300.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.15, 0.4),
            ..default()
        })),
        Transform::from_translation(Vec3::ZERO),
    ));
}
