use bevy::prelude::*;

use crate::plugins::simulation::Simulation;
use crate::ui::overlay::OverlayMode;

/// Display settings for the grid renderer.
#[derive(Resource)]
pub struct GridDisplay {
    pub cell_size: f32,
}

impl Default for GridDisplay {
    fn default() -> Self {
        Self { cell_size: 6.0 }
    }
}

/// Marker component for grid tile sprites.
#[derive(Component)]
pub struct GridTile {
    pub gx: u32,
    pub gy: u32,
}

pub struct GridRenderPlugin;

impl Plugin for GridRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GridDisplay::default())
            .insert_resource(OverlayMode::default())
            .add_systems(Startup, spawn_grid_tiles)
            .add_systems(
                Update,
                (update_tile_colors, crate::input::tool::handle_mouse_input),
            );
    }
}

fn spawn_grid_tiles(mut commands: Commands, sim: Res<Simulation>, display: Res<GridDisplay>) {
    let w = sim.0.grid.width;
    let h = sim.0.grid.height;
    let half_w = w as f32 / 2.0;
    let half_h = h as f32 / 2.0;

    for y in 0..h {
        for x in 0..w {
            let px = (x as f32 - half_w) * display.cell_size;
            let py = -(y as f32 - half_h) * display.cell_size; // Flip Y for screen coords.

            commands.spawn((
                Sprite {
                    color: Color::srgb(0.1, 0.2, 0.4),
                    custom_size: Some(Vec2::splat(display.cell_size)),
                    ..default()
                },
                Transform::from_xyz(px, py, 0.0),
                GridTile { gx: x, gy: y },
            ));
        }
    }
}

fn update_tile_colors(
    sim: Res<Simulation>,
    overlay: Res<OverlayMode>,
    mut tiles: Query<(&GridTile, &mut Sprite)>,
) {
    for (tile, mut sprite) in &mut tiles {
        let cell = sim.0.grid.get(tile.gx as i32, tile.gy as i32);
        sprite.color = match *overlay {
            OverlayMode::Elevation => elevation_color(cell.elevation, cell.water_depth),
            OverlayMode::Temperature => gradient_color(cell.temp, COLD_COLOR, HOT_COLOR),
            OverlayMode::Moisture => gradient_color(cell.moisture, DRY_COLOR, WET_COLOR),
            OverlayMode::Fertility => gradient_color(cell.fertility, BARREN_COLOR, FERTILE_COLOR),
            OverlayMode::Biome => biome_color(cell.biome),
            OverlayMode::Life => life_color(cell.fungus, cell.plants, cell.water_depth),
        };
    }
}

// --- Color helpers ---

const COLD_COLOR: Color = Color::srgb(0.3, 0.5, 0.9);
const HOT_COLOR: Color = Color::srgb(0.9, 0.3, 0.2);
const DRY_COLOR: Color = Color::srgb(0.85, 0.75, 0.5);
const WET_COLOR: Color = Color::srgb(0.2, 0.4, 0.8);
const BARREN_COLOR: Color = Color::srgb(0.5, 0.45, 0.4);
const FERTILE_COLOR: Color = Color::srgb(0.2, 0.7, 0.3);

fn elevation_color(elevation: f32, water_depth: f32) -> Color {
    if water_depth > 0.0 {
        let depth_factor = (water_depth / 3.0).min(1.0);
        Color::srgb(
            0.1 - depth_factor * 0.05,
            0.25 - depth_factor * 0.1,
            0.5 + depth_factor * 0.3,
        )
    } else {
        let height_factor = (elevation / 5.0).clamp(0.0, 1.0);
        Color::srgb(
            0.4 + height_factor * 0.4,
            0.55 - height_factor * 0.2,
            0.3 - height_factor * 0.15,
        )
    }
}

fn gradient_color(t: f32, low: Color, high: Color) -> Color {
    let t = t.clamp(0.0, 1.0);
    let low = low.to_srgba();
    let high = high.to_srgba();
    Color::srgb(
        low.red + (high.red - low.red) * t,
        low.green + (high.green - low.green) * t,
        low.blue + (high.blue - low.blue) * t,
    )
}

fn biome_color(biome: genesis_sim_core::grid::Biome) -> Color {
    use genesis_sim_core::grid::Biome;
    match biome {
        Biome::Ocean => Color::srgb(0.1, 0.2, 0.5),
        Biome::Beach => Color::srgb(0.9, 0.85, 0.6),
        Biome::Grassland => Color::srgb(0.45, 0.7, 0.35),
        Biome::Forest => Color::srgb(0.15, 0.45, 0.2),
        Biome::Tundra => Color::srgb(0.75, 0.8, 0.85),
        Biome::Desert => Color::srgb(0.85, 0.75, 0.45),
        Biome::Mountain => Color::srgb(0.55, 0.5, 0.5),
    }
}

fn life_color(fungus: f32, plants: f32, water_depth: f32) -> Color {
    if water_depth > 0.0 {
        return Color::srgb(0.1, 0.2, 0.4);
    }
    // Blend fungus (brownish) and plants (green) over a neutral base.
    let base = Color::srgb(0.5, 0.45, 0.4);
    let base = base.to_srgba();
    Color::srgb(
        (base.red - plants * 0.3 + fungus * 0.1).clamp(0.0, 1.0),
        (base.green + plants * 0.4 + fungus * 0.15).clamp(0.0, 1.0),
        (base.blue - plants * 0.2 - fungus * 0.1).clamp(0.0, 1.0),
    )
}
