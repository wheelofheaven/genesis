//! Disposable construction footprints for cosmetic vegetation, not simulation collision.
use genesis_sim_core::{
    SimState, paths, settlement::Building, spatial::Position, units::metres, worldgen::Cover,
};
use std::collections::BTreeMap;

const CELL: f32 = metres(8.);

enum Shape {
    House {
        center: Position,
        sin: f32,
        cos: f32,
    },
    Walk {
        a: Position,
        b: Position,
        radius: f32,
    },
}
impl Shape {
    fn blocks(&self, p: Position, building_margin: f32, path_margin: f32) -> bool {
        match *self {
            Self::House { center, sin, cos } => {
                let dx = p.x - center.x;
                let dz = p.z - center.z;
                // Include the stone plinth and front step; rounded corner clearance.
                let x = (dx * cos + dz * sin).abs() - Building::HALF_WIDTH - metres(0.2);
                let z = (dz * cos - dx * sin - metres(0.25)).abs()
                    - Building::HALF_DEPTH
                    - metres(0.45);
                x.max(0.).hypot(z.max(0.)) <= building_margin
            }
            Self::Walk { a, b, radius } => {
                paths::distance(p, paths::nearest(p, a, b)) <= radius + path_margin
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct Clearance {
    shapes: Vec<Shape>,
    bins: BTreeMap<(i32, i32), Vec<usize>>,
}
fn key(p: Position) -> (i32, i32) {
    ((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32)
}
impl Clearance {
    pub fn new(sim: &SimState) -> Self {
        let mut out = Self::default();
        for b in &sim.buildings {
            let (sin, cos) = b.rotation.sin_cos();
            let r = (Building::HALF_WIDTH + metres(0.2)).hypot(Building::HALF_DEPTH + metres(0.7));
            out.insert(
                Shape::House {
                    center: b.position,
                    sin,
                    cos,
                },
                Position::new(b.position.x - r, b.position.z - r),
                Position::new(b.position.x + r, b.position.z + r),
            );
            if let Some(points) = super::paths::entrance(sim, b) {
                out.walk(&points, Building::ENTRANCE_WIDTH);
            }
        }
        for path in &sim.paths {
            out.walk(&paths::samples(path), path.width);
        }
        out
    }
    fn insert(&mut self, shape: Shape, min: Position, max: Position) {
        let lo = key(min);
        let hi = key(max);
        for z in lo.1..=hi.1 {
            for x in lo.0..=hi.0 {
                self.bins.entry((x, z)).or_default().push(self.shapes.len());
            }
        }
        self.shapes.push(shape);
    }
    fn walk(&mut self, points: &[Position], width: f32) {
        // Include the rendered feathered shoulder, not only the stored path width.
        let radius = width * 0.5 + metres(0.08);
        for pair in points.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            self.insert(
                Shape::Walk { a, b, radius },
                Position::new(a.x.min(b.x) - radius, a.z.min(b.z) - radius),
                Position::new(a.x.max(b.x) + radius, a.z.max(b.z) + radius),
            );
        }
    }
    pub fn blocks(&self, p: Position, building_margin: f32, path_margin: f32) -> bool {
        let r = building_margin.max(path_margin);
        let lo = key(Position::new(p.x - r, p.z - r));
        let hi = key(Position::new(p.x + r, p.z + r));
        for z in lo.1..=hi.1 {
            for x in lo.0..=hi.0 {
                if self.bins.get(&(x, z)).is_some_and(|ids| {
                    ids.iter()
                        .any(|&i| self.shapes[i].blocks(p, building_margin, path_margin))
                }) {
                    return true;
                }
            }
        }
        false
    }
    pub fn hides_cover(&self, plant: &Cover) -> bool {
        let building_radius = if plant.kind.tree() { 4.8 } else { 1.7 };
        let path_radius = if plant.kind.tree() {
            metres(0.35)
        } else if plant.kind.rock() {
            metres(1.7)
        } else {
            metres(1.1)
        };
        self.blocks(
            plant.position,
            metres(building_radius) * plant.size,
            path_radius * plant.size + metres(0.2),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_sim_core::{config::SimConfig, settlement::Path};
    fn fixture() -> SimState {
        let mut sim = SimState::new(SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        sim.buildings = vec![Building {
            id: 1,
            position: Position::new(8., 8.),
            rotation: 0.7,
        }];
        sim.paths = vec![Path {
            points: vec![Position::new(5., 9.), Position::new(11., 9.)],
            width: metres(1.2),
        }];
        sim
    }
    #[test]
    fn clearance_follows_rotated_walls_steps_and_entrances() {
        let sim = fixture();
        let index = Clearance::new(&sim);
        let b = &sim.buildings[0];
        let (s, c) = b.rotation.sin_cos();
        let local = |x, z| {
            Position::new(
                b.position.x + c * metres(x) - s * metres(z),
                b.position.z + s * metres(x) + c * metres(z),
            )
        };
        assert!(index.blocks(local(0., 0.), 0., 0.));
        assert!(index.blocks(local(3.4, 0.), 0., 0.));
        assert!(index.blocks(local(0., 3.), 0., 0.)); // front step
        assert!(!index.blocks(local(3.8, 0.), metres(0.2), metres(0.2)));
        assert!(!index.blocks(local(0., -3.), metres(0.2), metres(0.2)));
        for p in super::super::paths::entrance(&sim, b).expect("entrance") {
            assert!(index.blocks(p, 0., 0.));
        }
    }
    #[test]
    fn bins_match_exhaustive_geometry_across_boundaries_and_margins() {
        let sim = fixture();
        let index = Clearance::new(&sim);
        for i in 0..240 {
            for j in 0..120 {
                let p = Position::new(4.8 + i as f32 * 0.027, 7.5 + j as f32 * 0.019);
                for (house, path) in [
                    (0., 0.),
                    (metres(0.4), metres(0.3)),
                    (metres(9.), metres(0.8)),
                ] {
                    let expected = index.shapes.iter().any(|s| s.blocks(p, house, path));
                    assert_eq!(index.blocks(p, house, path), expected, "{p:?}");
                }
            }
        }
        assert!(index.bins.values().map(Vec::len).max().unwrap() < index.shapes.len());
        assert!(!Clearance::default().blocks(Position::new(-1., -1.), 1., 1.));
    }
}
