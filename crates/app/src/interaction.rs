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

#[cfg(test)]
mod tests {
    use super::*;
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
