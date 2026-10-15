//! Seeded regional generation. This is a plausible landscape recipe, not a
//! calibrated Earth or plate-tectonics model. All distances use game units.
mod cover;
mod drainage;
mod geology;
pub mod noise;
mod region;
pub mod relief;
pub use region::{GeoPoint, Region};
mod settlement;
#[cfg(test)]
mod tests;

use crate::{SimState, config::SimConfig, grid::Grid, spatial::Position};
pub use drainage::River;

use serde::{Deserialize, Serialize};

pub const DEFAULT_SEED: u64 = 7319;
pub const WIDTH: u32 = 384;
pub const HEIGHT: u32 = 320;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Geography {
    /// Version of the generation recipe. Zero marks the old authored fixture.
    pub version: u32,
    #[serde(default)]
    pub region: Region,
    #[serde(default)]
    pub climate: crate::ecology::Climate,
    pub rivers: Vec<River>,
    #[serde(skip)]
    pub channels: crate::surface::channels::Index,
    pub cover: Vec<Cover>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cover {
    pub position: Position,
    pub size: f32,
    pub kind: crate::ecology::CoverKind,
    pub seed: u64,
}

/// A new Mediterranean-inspired coastal region with reproducible terrain,
/// rainfall, drainage, formations, and a settlement on suitable ground.
pub fn generate(seed: u64) -> SimState {
    generate_region(seed, Region::LevantCoast)
}
pub fn generate_region(seed: u64, region: Region) -> SimState {
    generate_scenario(seed, region, true)
}

/// Generate a regional sandbox with an optional starting settlement.
pub fn generate_scenario(seed: u64, region: Region, inhabited: bool) -> SimState {
    let mut sim = SimState::new(SimConfig {
        width: WIDTH,
        height: HEIGHT,
        seed,
        tick_dt: 0.1,
        ..Default::default()
    });
    if region != Region::Island {
        sim.grid.boundary = crate::grid::Boundary::Clamp;
    }
    relief::build(&mut sim.grid, seed, region);
    weather_slopes(&mut sim.grid);
    // Iterated stream-power-inspired incision. A generation heuristic, not a
    // running sediment transport solver. Re-route after each terrain pass.
    for _ in 0..16 {
        let flow = drainage::route(&mut sim.grid, sim.sea_level);
        let old: Vec<_> = sim.grid.cells().iter().map(|c| c.elevation).collect();
        for &i in &flow.order {
            if let Some(j) = flow.receivers[i] {
                let slope = (old[i] - old[j]).max(0.);
                let incision = (flow.accumulation[i].sqrt() * slope.sqrt() * 0.035).min(0.30);
                sim.grid.cells_mut()[i].elevation = (old[i] - incision).max(sim.sea_level + 0.015);
            }
        }
    }
    let flow = drainage::route(&mut sim.grid, sim.sea_level);
    sim.geography = Geography {
        version: 9,
        climate: crate::ecology::Climate::for_region(region),
        region,
        rivers: drainage::rivers(
            &sim.grid,
            &flow,
            sim.sea_level,
            if region == Region::LevantCoast {
                240.
            } else {
                110.
            },
        ),
        cover: Vec::new(),
        channels: Default::default(),
    };
    sim.geography.channels =
        crate::surface::channels::Index::new(WIDTH, HEIGHT, &sim.geography.rivers);
    sim.step(0., &[]);
    sim.subsurface = geology::generate(&sim.grid, seed, sim.sea_level, region);
    for c in sim.grid.cells_mut() {
        if c.water_depth > 0. {
            continue;
        }
        c.fertility = (0.22 + c.moisture * 0.65 - c.elevation * 0.004).clamp(0., 1.);
        c.fungus = c.moisture * 0.55;
        c.plants = (c.moisture * c.fertility * 1.5).clamp(0., 0.9);
    }
    if inhabited {
        settlement::seed(&mut sim);
    }
    sim.geography.cover = cover::generate(&sim);
    if inhabited {
        crate::people::populate(&mut sim);
    }
    sim
}

fn weather_slopes(grid: &mut Grid) {
    let width = grid.width as usize;
    let height = grid.height as usize;
    for _ in 0..12 {
        let mut delta = vec![0.; grid.cells().len()];
        for (i, c) in grid.cells().iter().enumerate() {
            if c.elevation <= 0. {
                continue;
            }
            let mut lowest = i;
            for j in drainage::neighbours(i, width, height) {
                if grid.cells()[j].elevation < grid.cells()[lowest].elevation {
                    lowest = j;
                }
            }
            let transfer = ((c.elevation - grid.cells()[lowest].elevation - 0.65) * 0.16).max(0.);
            delta[i] -= transfer;
            delta[lowest] += transfer;
        }
        for (c, d) in grid.cells_mut().iter_mut().zip(delta) {
            c.elevation += d;
        }
    }
}

/// Proximity to generated watercourses for placement and landscape ecology.
pub fn river_distance(sim: &SimState, p: Position) -> f32 {
    let mut nearest = f32::INFINITY;
    for river in &sim.geography.rivers {
        if let Some([x0, z0, x1, z1]) = river.bounds {
            let dx = (x0 - p.x).max(0.).max(p.x - x1);
            let dz = (z0 - p.z).max(0.).max(p.z - z1);
            if dx * dx + dz * dz >= nearest {
                continue;
            }
        }
        for pair in river.points.windows(2) {
            let dx = pair[1].x - pair[0].x;
            let dz = pair[1].z - pair[0].z;
            let t = (((p.x - pair[0].x) * dx + (p.z - pair[0].z) * dz)
                / (dx * dx + dz * dz).max(0.0001))
            .clamp(0., 1.);
            nearest = nearest
                .min((p.x - pair[0].x - t * dx).powi(2) + (p.z - pair[0].z - t * dz).powi(2));
        }
    }
    nearest.sqrt()
}
