//! Small local crown index for cosmetic understorey placement.
use genesis_sim_core::{SimState, ecology::CoverKind, spatial::Position, units::metres};
use std::collections::BTreeMap;
const CELL: f32 = metres(8.);
const MAX_RADIUS: f32 = metres(7.5);
struct Crown {
    position: Position,
    radius: f32,
    pine: bool,
}
pub(super) struct Canopy(BTreeMap<(i32, i32), Vec<Crown>>);
fn key(p: Position) -> (i32, i32) {
    ((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32)
}
impl Canopy {
    pub fn new(
        sim: &SimState,
        center: Position,
        radius: f32,
        clearance: &crate::scene::Clearance,
    ) -> Self {
        let mut bins = BTreeMap::<_, Vec<_>>::new();
        for plant in sim.geography.cover.iter().filter(|p| p.kind.tree()) {
            if (plant.position.x - center.x).hypot(plant.position.z - center.z)
                > radius + MAX_RADIUS
            {
                continue;
            }
            if clearance.hides_cover(plant)
                || crate::terrain::point(sim, plant.position).y < sim.sea_level + metres(0.3)
            {
                continue;
            }
            let extent = match plant.kind {
                CoverKind::Broadleaf => 4.8,
                CoverKind::Pine => 4.0,
                _ => 3.2,
            };
            bins.entry(key(plant.position)).or_default().push(Crown {
                position: plant.position,
                radius: metres(extent * plant.size).min(MAX_RADIUS),
                pine: plant.kind == CoverKind::Pine,
            });
        }
        Self(bins)
    }
    /// Strongest local crown, avoiding unbounded accumulation in dense stands.
    pub fn at(&self, p: Position) -> (f32, bool) {
        let (x, z) = key(p);
        let mut result = (0., false);
        for dz in -1..=1 {
            for dx in -1..=1 {
                if let Some(crowns) = self.0.get(&(x + dx, z + dz)) {
                    for crown in crowns {
                        let d = (p.x - crown.position.x).hypot(p.z - crown.position.z);
                        let weight = (1. - d / crown.radius.max(metres(0.1))).clamp(0., 1.);
                        let weight = weight * weight * (3. - 2. * weight);
                        if weight > result.0 {
                            result = (weight, crown.pine);
                        }
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_trees_leave_no_cosmetic_canopy() {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 8,
            height: 8,
            ..Default::default()
        });
        for c in sim.grid.cells_mut() {
            c.elevation = 2.;
        }
        let center = Position::new(4., 4.);
        sim.geography.cover = vec![genesis_sim_core::worldgen::Cover {
            position: center,
            size: 1.,
            seed: 7,
            kind: CoverKind::Pine,
        }];
        let canopy = |sim: &SimState| {
            Canopy::new(sim, center, metres(50.), &crate::scene::Clearance::new(sim))
        };
        assert_eq!(canopy(&sim).at(center), (1., true));
        sim.buildings.push(genesis_sim_core::settlement::Building {
            id: 1,
            position: center,
            rotation: 1.,
        });
        assert_eq!(canopy(&sim).at(center), (0., false));
        sim.buildings.clear();
        sim.paths.push(genesis_sim_core::settlement::Path {
            id: 0,
            points: vec![Position::new(3., 4.), Position::new(5., 4.)],
            width: metres(1.2),
        });
        assert_eq!(canopy(&sim).at(center), (0., false));
        sim.paths.clear();
        sim.sea_level = 3.;
        assert_eq!(canopy(&sim).at(center), (0., false));
    }
}
