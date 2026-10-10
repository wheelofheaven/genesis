// Module temporarily unused: GridRenderPlugin disabled while planet crate handles rendering.
#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

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

/// Handle to the grid texture.
#[derive(Resource)]
struct GridTexture(Handle<Image>);

/// Marker for the grid mesh entity.
#[derive(Component)]
struct GridSprite;

pub struct GridRenderPlugin;

impl Plugin for GridRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GridDisplay::default())
            .insert_resource(OverlayMode::default())
            .add_systems(Startup, spawn_grid_texture)
            .add_systems(Update, update_grid_texture);
    }
}

fn spawn_grid_texture(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim: Res<Simulation>,
    display: Res<GridDisplay>,
) {
    let w = sim.0.grid.width;
    let h = sim.0.grid.height;

    // Create an RGBA image for the grid.
    let image = Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[25, 50, 100, 255], // Dark ocean blue default
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    let mut image = image;
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    let image_handle = images.add(image);
    commands.insert_resource(GridTexture(image_handle.clone()));

    // Grid dimensions in world units.
    let grid_width = w as f32 * display.cell_size;
    let grid_height = h as f32 * display.cell_size;

    // Create a plane mesh lying on the XZ plane.
    let mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::new(grid_width / 2.0, grid_height / 2.0)));

    let material = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        GridSprite,
    ));
}

fn update_grid_texture(
    sim: Res<Simulation>,
    overlay: Res<OverlayMode>,
    grid_tex: Res<GridTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(image) = images.get_mut(&grid_tex.0) else {
        return;
    };

    let w = sim.0.grid.width;
    let h = sim.0.grid.height;

    let Some(data) = image.data.as_mut() else {
        return;
    };

    for y in 0..h {
        for x in 0..w {
            let cell = sim.0.grid.get(x as i32, y as i32);
            let [r, g, b] = match *overlay {
                OverlayMode::Elevation => elevation_color(cell.elevation, cell.water_depth),
                OverlayMode::Temperature => {
                    gradient_color(cell.temp, [77, 128, 230], [230, 77, 51])
                }
                OverlayMode::Moisture => {
                    gradient_color(cell.moisture, [217, 191, 128], [51, 102, 204])
                }
                OverlayMode::Fertility => {
                    gradient_color(cell.fertility, [128, 115, 102], [51, 179, 77])
                }
                OverlayMode::Biome => biome_color(cell.biome),
                OverlayMode::Life => life_color(cell.fungus, cell.plants, cell.water_depth),
            };

            let idx = ((y * w + x) * 4) as usize;
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }
}

// --- Color helpers (return [r, g, b] as u8) ---

fn elevation_color(elevation: f32, water_depth: f32) -> [u8; 3] {
    if water_depth > 0.0 {
        let d = (water_depth / 3.0).min(1.0);
        [
            ((0.1 - d * 0.05) * 255.0) as u8,
            ((0.25 - d * 0.1) * 255.0) as u8,
            ((0.5 + d * 0.3) * 255.0) as u8,
        ]
    } else {
        let h = (elevation / 5.0).clamp(0.0, 1.0);
        [
            ((0.4 + h * 0.4) * 255.0) as u8,
            ((0.55 - h * 0.2) * 255.0) as u8,
            ((0.3 - h * 0.15) * 255.0) as u8,
        ]
    }
}

fn gradient_color(t: f32, low: [u8; 3], high: [u8; 3]) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (low[0] as f32 + (high[0] as f32 - low[0] as f32) * t) as u8,
        (low[1] as f32 + (high[1] as f32 - low[1] as f32) * t) as u8,
        (low[2] as f32 + (high[2] as f32 - low[2] as f32) * t) as u8,
    ]
}

fn biome_color(biome: genesis_sim_core::grid::Biome) -> [u8; 3] {
    use genesis_sim_core::grid::Biome;
    match biome {
        Biome::Ocean => [25, 51, 128],
        Biome::Beach => [230, 217, 153],
        Biome::Grassland => [115, 179, 89],
        Biome::Forest => [38, 115, 51],
        Biome::Tundra => [191, 204, 217],
        Biome::Desert => [217, 191, 115],
        Biome::Mountain => [140, 128, 128],
    }
}

fn life_color(fungus: f32, plants: f32, water_depth: f32) -> [u8; 3] {
    if water_depth > 0.0 {
        return [25, 51, 102];
    }
    let r = (128.0 - plants * 77.0 + fungus * 25.0).clamp(0.0, 255.0) as u8;
    let g = (115.0 + plants * 102.0 + fungus * 38.0).clamp(0.0, 255.0) as u8;
    let b = (102.0 - plants * 51.0 - fungus * 25.0).clamp(0.0, 255.0) as u8;
    [r, g, b]
}
