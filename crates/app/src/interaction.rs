//! Object identity and picking; no selection is inferred from an ecological cell.
use genesis_sim_core::{SimState, paths, spatial::Position, units::metres};
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Object {
    Building(u64),
    Path(usize),
}
impl Object {
    pub fn label(self) -> String {
        match self {
            Self::Building(id) => format!("Dwelling {id}"),
            Self::Path(i) => format!("Footpath {}", i + 1),
        }
    }
}
/// Intersect visible building triangles before the terrain hit. This also works
/// when aiming at a wall or roof from human eye height with sky behind it.
pub fn pick(sim: &SimState, origin: Vec3, dir: Vec3, ground: Option<Vec3>) -> Option<Object> {
    let mut limit = ground.map_or(f32::INFINITY, |p| (p - origin).dot(dir) + metres(0.08));
    let mut hit = None;
    let mut mesh = Vec::new();
    for b in &sim.buildings {
        if !building_candidate(sim, b, origin, dir, limit) {
            continue;
        }
        mesh.clear();
        crate::scene::house(sim, &mut mesh, b);
        for tri in mesh.chunks_exact(3) {
            if let Some(t) = crate::terrain::intersect(
                origin,
                dir,
                [
                    Vec3::from_array(tri[0].position),
                    Vec3::from_array(tri[1].position),
                    Vec3::from_array(tri[2].position),
                ],
            ) && t < limit
            {
                limit = t;
                hit = Some(Object::Building(b.id));
            }
        }
    }
    hit.or_else(|| {
        let p = ground?;
        let p = Position::new(
            p.x + sim.grid.width as f32 * 0.5,
            p.z + sim.grid.height as f32 * 0.5,
        );
        sim.paths
            .iter()
            .enumerate()
            .filter_map(|(i, path)| {
                let d = path
                    .points
                    .windows(2)
                    .map(|s| paths::distance(p, paths::nearest(p, s[0], s[1])))
                    .fold(f32::INFINITY, f32::min);
                (d <= path.width * 0.5 + metres(0.2)).then_some((d, i))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, i)| Object::Path(i))
    })
}

// Test horizontal bounds before constructing detailed picking geometry. The
// padding includes joist ends, masonry and the projecting doorstep. Leaving Y
// unbounded avoids terrain queries and keeps roof/steep-angle rays conservative.
fn building_candidate(
    sim: &SimState,
    b: &genesis_sim_core::settlement::Building,
    origin: Vec3,
    dir: Vec3,
    limit: f32,
) -> bool {
    let corners = b.footprint();
    let mut near: f32 = 0.;
    let mut far = limit;
    for (axis, offset) in [
        (0, sim.grid.width as f32 * 0.5),
        (2, sim.grid.height as f32 * 0.5),
    ] {
        let values = corners.map(|p| if axis == 0 { p.x } else { p.z });
        let min = values.into_iter().fold(f32::INFINITY, f32::min) - offset - metres(0.70);
        let max = values.into_iter().fold(f32::NEG_INFINITY, f32::max) - offset + metres(0.70);
        if dir[axis].abs() < 1e-10 {
            if origin[axis] < min || origin[axis] > max {
                return false;
            }
        } else {
            let a = (min - origin[axis]) / dir[axis];
            let b = (max - origin[axis]) / dir[axis];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn building_bounds_retain_all_triangle_hits() {
        let sim = genesis_sim_core::scenarios::coastal_outpost();
        let mut b = sim.buildings[0].clone();
        for rotation in [0., 0.7, 1.57, 3.2] {
            b.rotation = rotation;
            let mut mesh = Vec::new();
            crate::scene::house(&sim, &mut mesh, &b);
            let center = crate::terrain::point(&sim, b.position);
            for i in 0..16 {
                let a = i as f32 * std::f32::consts::TAU / 16.;
                for height in [0.1, 1.7, 3.0, 8.0] {
                    let origin = center
                        + Vec3::new(a.cos() * metres(10.), metres(height), a.sin() * metres(10.));
                    // Include rays to the outer joists, doorway and stone step.
                    for target in mesh.iter().step_by(36) {
                        let dir = (Vec3::from_array(target.position) - origin).normalize();
                        let hit = mesh
                            .chunks_exact(3)
                            .filter_map(|t| {
                                crate::terrain::intersect(
                                    origin,
                                    dir,
                                    [
                                        Vec3::from_array(t[0].position),
                                        Vec3::from_array(t[1].position),
                                        Vec3::from_array(t[2].position),
                                    ],
                                )
                            })
                            .min_by(f32::total_cmp);
                        if let Some(distance) = hit {
                            assert!(building_candidate(
                                &sim,
                                &b,
                                origin,
                                dir,
                                distance + metres(0.01)
                            ));
                        }
                    }
                }
            }
            assert!(!building_candidate(
                &sim,
                &b,
                center + Vec3::X * metres(20.),
                Vec3::Y,
                f32::INFINITY
            ));
            assert!(building_candidate(
                &sim,
                &b,
                center + Vec3::Y * metres(20.),
                -Vec3::Y,
                f32::INFINITY
            ));
        }
    }
    #[test]
    fn rotated_walls_are_pickable_above_the_horizon_and_occluded_by_ground() {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        for c in sim.grid.cells_mut() {
            c.elevation = 1.;
        }
        let b = genesis_sim_core::settlement::Building {
            id: 42,
            position: Position::new(8., 8.),
            rotation: 0.7,
        };
        sim.buildings.push(b);
        let target = crate::terrain::point(&sim, Position::new(8., 8.)) + Vec3::Y * metres(1.7);
        let origin = target + Vec3::Z * metres(10.);
        assert_eq!(
            pick(&sim, origin, -Vec3::Z, None),
            Some(Object::Building(42))
        );
        assert_eq!(
            pick(&sim, origin, -Vec3::Z, Some(origin - Vec3::Z * metres(1.))),
            None
        );
        sim.buildings.clear();
        sim.paths.push(genesis_sim_core::settlement::Path {
            points: vec![Position::new(7., 8.), Position::new(9., 8.)],
            width: metres(1.2),
        });
        let ground = crate::terrain::point(&sim, Position::new(8., 8.));
        assert_eq!(
            pick(&sim, ground + Vec3::Y, -Vec3::Y, Some(ground)),
            Some(Object::Path(0))
        );
        assert_eq!(
            pick(
                &sim,
                ground + Vec3::Y,
                -Vec3::Y,
                Some(ground + Vec3::Z * metres(2.))
            ),
            None
        );
    }
}
