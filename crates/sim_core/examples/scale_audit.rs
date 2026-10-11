//! Report the metric relationships behind a generated region.
use genesis_sim_core::{
    units::METRES_PER_UNIT as M,
    worldgen::{self, Region},
};
fn main() {
    for region in Region::ALL {
        let sim = worldgen::generate_region(worldgen::DEFAULT_SEED, region);
        let land = sim
            .grid
            .cells()
            .iter()
            .filter(|c| c.elevation > sim.sea_level)
            .count();
        let trees = sim.geography.cover.iter().filter(|c| !c.rock).count();
        let elevation = sim
            .grid
            .cells()
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), c| {
                (lo.min(c.elevation * M), hi.max(c.elevation * M))
            });
        let width = sim
            .geography
            .rivers
            .iter()
            .map(|r| r.width * M)
            .fold(0_f32, f32::max);
        println!(
            "{}: {:.2} × {:.2} km, relief {:.0}..{:.0} m, {trees} trees / {:.1} km² land, {} watercourses (maximum width {width:.1} m), {} dwellings",
            region.name(),
            sim.grid.width as f32 * M / 1000.,
            sim.grid.height as f32 * M / 1000.,
            elevation.0,
            elevation.1,
            land as f32 * M * M / 1e6,
            sim.geography.rivers.len(),
            sim.buildings.len()
        );
    }
}
