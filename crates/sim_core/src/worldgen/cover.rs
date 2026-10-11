//! Metric vegetation sampling independent of the environmental cell spacing.
use super::{
    Cover,
    noise::{noise, unit},
    river_distance,
};
use crate::{
    SimState,
    spatial::Position,
    units::{METRES_PER_UNIT, metres},
};

pub fn generate(sim: &SimState) -> Vec<Cover> {
    let seed = sim.config.seed;
    let spacing = metres(10.);
    let width = ((sim.grid.width - 1) as f32 / spacing) as u32;
    let height = ((sim.grid.height - 1) as f32 / spacing) as u32;
    let mut cover = Vec::new();
    for z in 0..height {
        for x in 0..width {
            let i = u64::from(z) * u64::from(width) + u64::from(x);
            let p = Position::new(
                (x as f32 + 0.15 + unit(seed ^ 811, i) * 0.7) * spacing,
                (z as f32 + 0.15 + unit(seed ^ 821, i) * 0.7) * spacing,
            );
            let elevation = crate::spatial::elevation(&sim.grid, p);
            let plants = crate::spatial::sample(&sim.grid, p, |c| c.plants);
            let rock = elevation > metres(360.) || unit(seed ^ 835, i) < 0.08;
            let patch = (noise(seed ^ 111, p.x / metres(140.), p.z / metres(140.)) + 0.45).max(0.);
            // Boulders retain their sparse distribution as vegetation sampling
            // becomes finer. Rainfall/fertility still decide where groves grow.
            let probability = if rock {
                0.085 * (spacing * METRES_PER_UNIT / 20.).powi(2)
            } else {
                plants * patch * 0.9
            };
            if elevation < sim.sea_level + metres(9.) || unit(seed ^ 831, i) > probability {
                continue;
            }
            if sim.buildings.iter().any(|b| {
                (b.position.x - p.x).powi(2) + (b.position.z - p.z).powi(2)
                    < crate::settlement::Building::TREE_CLEARANCE.powi(2)
            }) || sim
                .paths
                .iter()
                .flat_map(|p| &p.points)
                .any(|q| (q.x - p.x).powi(2) + (q.z - p.z).powi(2) < metres(3.).powi(2))
                || river_distance(sim, p) < metres(8.)
            {
                continue;
            }
            cover.push(Cover {
                position: p,
                size: 0.45 + unit(seed ^ 841, i) * 0.8,
                rock,
            });
        }
    }
    cover
}
