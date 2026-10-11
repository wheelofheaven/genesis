//! Clustered, multi-layer cover with stable site identity and geological stones.
use super::{
    Cover,
    noise::{noise, unit},
};
use crate::{
    SimState,
    ecology::{self, CoverKind},
    spatial::{self, Position},
    units::metres,
};
pub fn generate(sim: &SimState) -> Vec<Cover> {
    let seed = sim.config.seed;
    let spacing = metres(10.);
    let width = ((sim.grid.width - 1) as f32 / spacing) as u32;
    let height = ((sim.grid.height - 1) as f32 / spacing) as u32;
    let mut out = Vec::new();
    for z in 0..height {
        for x in 0..width {
            let id = (u64::from(z) << 32) | u64::from(x);
            let p = Position::new(
                (x as f32 + 0.08 + unit(seed ^ 811, id) * 0.84) * spacing,
                (z as f32 + 0.08 + unit(seed ^ 821, id) * 0.84) * spacing,
            );
            let elevation = spatial::elevation(&sim.grid, p);
            if elevation < sim.sea_level + metres(1.5) {
                continue;
            }
            // Evaluate expensive habitat queries only at inhabited patches.
            let patch =
                (noise(seed ^ 111, p.x / metres(170.), p.z / metres(170.)) + 0.5).clamp(0., 1.);
            if unit(seed ^ 831, id) > 0.16 + patch * 0.34 {
                continue;
            }
            let h = ecology::habitat(sim, p);
            if h.river_distance < metres(8.) {
                continue;
            }
            let weights = ecology::weights(h);
            let rock_chance = (0.045 + h.slope * 0.32 + (1. - h.moisture) * 0.05).clamp(0., 0.55);
            let kind = if unit(seed ^ 835, id) < rock_chance {
                match h.rock {
                    crate::subsurface::Rock::Limestone => CoverKind::Limestone,
                    crate::subsurface::Rock::Sandstone => CoverKind::Sandstone,
                    crate::subsurface::Rock::Basalt => CoverKind::Basalt,
                }
            } else {
                let total: f32 = weights.iter().sum();
                if total <= 0. {
                    continue;
                }
                let mut choice = unit(seed ^ 847, id) * total;
                let mut index = 4;
                for (i, w) in weights.iter().enumerate() {
                    choice -= w;
                    if choice <= 0. {
                        index = i;
                        break;
                    }
                }
                CoverKind::ALL[index]
            };
            let size = if kind.rock() {
                0.2 + unit(seed ^ 841, id).powi(2) * 1.15
            } else {
                0.55 + unit(seed ^ 841, id) * 0.7
            };
            if blocked(sim, p, kind, size) {
                continue;
            }
            out.push(Cover {
                position: p,
                size,
                kind,
                seed: id ^ seed.rotate_left(17),
            });
            if kind.rock() {
                for n in 0..3 {
                    let child = id.wrapping_mul(5).wrapping_add(n);
                    let a = unit(seed ^ 887, child) * std::f32::consts::TAU;
                    let r = metres(1.5 + unit(seed ^ 889, child) * 2.5);
                    let q = Position::new(p.x + a.cos() * r, p.z + a.sin() * r);
                    let size = 0.05 + unit(seed ^ 891, child) * 0.18;
                    if q.x >= 0.
                        && q.z >= 0.
                        && q.x < sim.grid.width as f32 - 1.
                        && q.z < sim.grid.height as f32 - 1.
                        && spatial::elevation(&sim.grid, q) > sim.sea_level + metres(1.5)
                        && super::river_distance(sim, q) >= metres(8.)
                        && !blocked(sim, q, kind, size)
                    {
                        out.push(Cover {
                            position: q,
                            size,
                            kind,
                            seed: child ^ seed.rotate_left(31),
                        });
                    }
                }
            }
            // Understorey grows in loose clusters, not one object at every lattice point.
            if kind.tree() || kind == CoverKind::EvergreenShrub {
                for n in 0..2 {
                    let child = id.wrapping_mul(3).wrapping_add(n);
                    let a = unit(seed ^ 863, child) * std::f32::consts::TAU;
                    let r = metres(2.5 + unit(seed ^ 867, child) * 3.);
                    let q = Position::new(p.x + a.cos() * r, p.z + a.sin() * r);
                    let shrub = if h.moisture > 0.48 {
                        CoverKind::EvergreenShrub
                    } else {
                        CoverKind::DryShrub
                    };
                    let size = 0.45 + unit(seed ^ 871, child) * 0.55;
                    if q.x < 0.
                        || q.z < 0.
                        || q.x >= sim.grid.width as f32 - 1.
                        || q.z >= sim.grid.height as f32 - 1.
                        || spatial::elevation(&sim.grid, q) < sim.sea_level + metres(1.5)
                        || super::river_distance(sim, q) < metres(8.)
                        || blocked(sim, q, shrub, size)
                    {
                        continue;
                    }
                    out.push(Cover {
                        position: q,
                        size,
                        kind: shrub,
                        seed: child ^ seed.rotate_left(29),
                    });
                }
            }
        }
    }
    out
}
fn blocked(sim: &SimState, p: Position, kind: CoverKind, size: f32) -> bool {
    let clearance = if kind.tree() { metres(6.) } else { metres(4.) };
    sim.buildings
        .iter()
        .any(|b| crate::paths::distance(b.position, p) < clearance)
        || sim.paths.iter().any(|path| {
            crate::paths::on_path(path, p, kind.collision_radius() * size + metres(0.6))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cover_is_repeatable_multilayer_and_uses_local_geology() {
        let sim = super::super::generate(super::super::DEFAULT_SEED);
        assert_eq!(sim.geography.cover, generate(&sim));
        let cover = &sim.geography.cover;
        assert!(cover.iter().any(|c| c.kind.tree()));
        assert!(cover.iter().any(|c| c.kind == CoverKind::EvergreenShrub));
        assert!(cover.iter().any(|c| c.kind == CoverKind::DryShrub));
        assert!(cover.iter().any(|c| c.kind.rock() && c.size < 0.25));
        for c in cover {
            assert!(c.size.is_finite() && c.size > 0. && c.size < 1.4);
            assert!(!blocked(&sim, c.position, c.kind, c.size));
            if c.kind.rock() {
                // Small debris inherits its parent formation; allow it to cross a cell boundary.
                if c.size >= 0.25 {
                    let expected = match sim.subsurface.reading(c.position).rock {
                        crate::subsurface::Rock::Limestone => CoverKind::Limestone,
                        crate::subsurface::Rock::Sandstone => CoverKind::Sandstone,
                        crate::subsurface::Rock::Basalt => CoverKind::Basalt,
                    };
                    assert_eq!(c.kind, expected);
                }
            }
        }
    }
}
