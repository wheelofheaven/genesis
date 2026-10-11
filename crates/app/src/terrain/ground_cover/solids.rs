//! Local solid footprints keep understorey out of visible boulders and trunks.
use genesis_sim_core::{SimState, spatial::Position, units::metres};
use std::collections::BTreeMap;

const CELL: f32 = metres(8.);
#[derive(Clone, Copy)]
struct Solid {
    position: Position,
    radius: f32,
}
pub(super) struct Solids(BTreeMap<(i32, i32), Vec<Solid>>);
fn key(p: Position) -> (i32, i32) {
    ((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32)
}

impl Solids {
    pub fn new(
        sim: &SimState,
        center: Position,
        radius: f32,
        clearance: &crate::scene::Clearance,
    ) -> Self {
        let mut bins = BTreeMap::<_, Vec<_>>::new();
        for plant in &sim.geography.cover {
            if !plant.kind.tree() && !plant.kind.rock() {
                continue;
            }
            // Include the tree's buried root flare and the rock's conservative
            // silhouette. Shrubs remain permeable to smaller ground vegetation.
            let reach = if plant.kind.tree() {
                metres(0.45)
            } else {
                plant.kind.collision_radius()
            } * plant.size;
            if (plant.position.x - center.x).hypot(plant.position.z - center.z)
                > radius + reach + metres(1.)
                || clearance.hides_cover(plant)
                || crate::terrain::point(sim, plant.position).y < sim.sea_level + metres(0.3)
            {
                continue;
            }
            let lo = key(Position::new(
                plant.position.x - reach,
                plant.position.z - reach,
            ));
            let hi = key(Position::new(
                plant.position.x + reach,
                plant.position.z + reach,
            ));
            for z in lo.1..=hi.1 {
                for x in lo.0..=hi.0 {
                    bins.entry((x, z)).or_default().push(Solid {
                        position: plant.position,
                        radius: reach,
                    });
                }
            }
        }
        Self(bins)
    }

    pub fn blocks(&self, p: Position, reach: f32) -> bool {
        let lo = key(Position::new(p.x - reach, p.z - reach));
        let hi = key(Position::new(p.x + reach, p.z + reach));
        for z in lo.1..=hi.1 {
            for x in lo.0..=hi.0 {
                if self.0.get(&(x, z)).is_some_and(|items| {
                    items.iter().any(|solid| {
                        (p.x - solid.position.x).powi(2) + (p.z - solid.position.z).powi(2)
                            <= (solid.radius + reach).powi(2)
                    })
                }) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_sim_core::{
        config::SimConfig, ecology::CoverKind, settlement::Building, worldgen::Cover,
    };

    #[test]
    fn solid_bins_match_footprints_and_ignore_cleared_and_submerged_objects() {
        let mut sim = SimState::new(SimConfig {
            width: 8,
            height: 8,
            ..Default::default()
        });
        for cell in sim.grid.cells_mut() {
            cell.elevation = 2.;
        }
        let center = Position::new(4., 4.);
        sim.geography.cover = vec![
            Cover {
                position: center,
                size: 1.2,
                seed: 1,
                kind: CoverKind::Limestone,
            },
            Cover {
                position: Position::new(4.21, 4.13),
                size: 0.9,
                seed: 2,
                kind: CoverKind::Pine,
            },
        ];
        let index = |sim: &SimState| {
            Solids::new(sim, center, metres(54.), &crate::scene::Clearance::new(sim))
        };
        let solids = index(&sim);
        for x in -30..30 {
            for z in -30..30 {
                let p = Position::new(4. + x as f32 * 0.013, 4. + z as f32 * 0.011);
                for reach in [0., metres(0.3), metres(0.6)] {
                    let expected = sim.geography.cover.iter().any(|plant| {
                        let r =
                            metres(if plant.kind.tree() { 0.45 } else { 1.7 }) * plant.size + reach;
                        (p.x - plant.position.x).hypot(p.z - plant.position.z) <= r
                    });
                    assert_eq!(solids.blocks(p, reach), expected);
                }
            }
        }
        sim.buildings.push(Building {
            id: 1,
            position: center,
            rotation: 0.,
        });
        assert!(!index(&sim).blocks(center, 0.));
        sim.buildings.clear();
        sim.sea_level = 3.;
        assert!(!index(&sim).blocks(center, 0.));
    }
}
