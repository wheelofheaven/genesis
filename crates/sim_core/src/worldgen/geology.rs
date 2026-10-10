use super::noise::{fbm, noise, unit};
use crate::{
    grid::Grid,
    spatial::{self, Position},
    subsurface::{Column, Deposit, Mineral, Rock, Subsurface},
};

pub fn generate(grid: &Grid, seed: u64, sea: f32, region: super::Region) -> Subsurface {
    let mut result = Subsurface::new(grid.width, grid.height);
    for (x, z, cell) in grid.iter_coords() {
        let p = Position::new(x as f32, z as f32);
        let ground = spatial::elevation(grid, p);
        let formation = fbm(seed ^ 1237, p.x / 18., p.z / 18.);
        let rock = if region != super::Region::Island {
            // Broad procedural material family only, not a mapped formation.
            if ground < 4. && formation < 0.15 {
                Rock::Sandstone
            } else {
                Rock::Limestone
            }
        } else if ground > 7. || formation > 0.24 {
            Rock::Basalt
        } else if formation > -0.12 {
            Rock::Limestone
        } else {
            Rock::Sandstone
        };
        let valley = (cell.drainage / 70.).sqrt().clamp(0., 1.);
        let sediment = ((5. - ground) / 5.).clamp(0., 1.) * 0.7 + valley * 0.3;
        let permeability = match rock {
            Rock::Sandstone => 0.65,
            Rock::Limestone => 0.42,
            Rock::Basalt => 0.12,
        };
        let thickness = 2. + sediment * 10. + noise(seed ^ 71, p.x / 9., p.z / 9.) * 0.8;
        let capacity = if ground > sea {
            thickness * (8. + permeability * 14.)
        } else {
            0.
        };
        let top = ground - 1.5 - (1. - sediment) * 3.;
        result.columns[(z * grid.width + x) as usize] = Column {
            rock,
            aquifer_top: top,
            aquifer_base: top - thickness,
            capacity,
            stored: capacity
                * (0.3 + cell.moisture * 0.3 + cell.rainfall * 0.25 + valley * 0.12)
                    .clamp(0., 0.95),
            recharge: capacity * cell.rainfall * 0.001,
            permeability,
        };
    }
    for (kind, mineral) in [Mineral::Copper, Mineral::Iron, Mineral::Clay]
        .into_iter()
        .enumerate()
    {
        for ordinal in 0..3 {
            let mut best = None;
            let mut score = f32::NEG_INFINITY;
            for (x, z, c) in grid.iter_coords() {
                if c.elevation < 0.8 || x < 5 || z < 5 || x > grid.width - 6 || z > grid.height - 6
                {
                    continue;
                }
                let p = Position::new(x as f32, z as f32);
                if result
                    .deposits
                    .iter()
                    .any(|d| (d.center.x - p.x).powi(2) + (d.center.z - p.z).powi(2) < 64.)
                {
                    continue;
                }
                let rock = result.columns[(z * grid.width + x) as usize].rock;
                let suitability = match mineral {
                    Mineral::Copper => {
                        if rock == Rock::Basalt {
                            2.
                        } else {
                            0.
                        }
                    }
                    Mineral::Iron => {
                        if rock != Rock::Sandstone {
                            2.
                        } else {
                            0.
                        }
                    }
                    Mineral::Clay => (4. - c.elevation).clamp(-2., 2.) + (c.drainage / 40.).min(1.),
                };
                let rank = suitability
                    + unit(
                        seed ^ (kind as u64 * 71 + ordinal),
                        (z * grid.width + x) as u64,
                    );
                if rank > score {
                    score = rank;
                    best = Some(p);
                }
            }
            if let Some(center) = best {
                let id = result.deposits.len() as u64 + 1;
                let depth = if mineral == Mineral::Clay {
                    0.8
                } else {
                    4. + unit(seed, id + 800) * 8.
                };
                result.deposits.push(Deposit {
                    id,
                    mineral,
                    center,
                    radii: [
                        2.5 + unit(seed, id + 200) * 3.5,
                        1.4 + unit(seed, id + 300) * 2.,
                    ],
                    rotation: unit(seed, id + 400) * std::f32::consts::TAU,
                    irregularity: 1.,
                    roof: spatial::elevation(grid, center) - depth,
                    thickness: if mineral == Mineral::Clay {
                        1.8
                    } else {
                        3. + unit(seed, id + 500) * 4.
                    },
                    grade: 0.3 + unit(seed, id + 600) * 0.55,
                });
            }
        }
    }
    result
}
