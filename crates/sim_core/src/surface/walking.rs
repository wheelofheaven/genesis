//! Read-only ground traversal queries, reusable by future individual agents.
use super::elevation;
use crate::{SimState, spatial::Position, units::metres};

pub fn standable(sim: &SimState, p: Position) -> bool {
    Query::local(sim, [p.x, p.z, p.x, p.z]).standable(p)
}
pub fn advance(sim: &SimState, from: Position, to: Position) -> Position {
    Query::local(
        sim,
        [
            from.x.min(to.x),
            from.z.min(to.z),
            from.x.max(to.x),
            from.z.max(to.z),
        ],
    )
    .advance(from, to)
}

/// Scoped read view for repeated queries. It filters only obstacle candidates;
/// terrain, river and building rules are identical to the ground camera rules.
pub(crate) struct Query<'a> {
    sim: &'a SimState,
    cover: Vec<&'a crate::worldgen::Cover>,
}
impl<'a> Query<'a> {
    pub(crate) fn local(sim: &'a SimState, bounds: [f32; 4]) -> Self {
        let cover = sim
            .geography
            .cover
            .iter()
            .filter(|p| {
                let r = metres(0.3) + p.kind.collision_radius() * p.size;
                p.position.x + r >= bounds[0]
                    && p.position.z + r >= bounds[1]
                    && p.position.x - r <= bounds[2]
                    && p.position.z - r <= bounds[3]
            })
            .collect();
        Self { sim, cover }
    }
    pub(crate) fn standable(&self, p: Position) -> bool {
        let sim = self.sim;
        let radius = metres(0.3);
        if !p.x.is_finite()
            || !p.z.is_finite()
            || p.x < radius
            || p.z < radius
            || p.x > sim.grid.width as f32 - radius
            || p.z > sim.grid.height as f32 - radius
            || elevation(sim, p) < sim.sea_level + metres(0.1)
        {
            return false;
        }
        for b in &sim.buildings {
            let (s, c) = b.rotation.sin_cos();
            let dx = p.x - b.position.x;
            let dz = p.z - b.position.z;
            if (dx * c + dz * s).abs() < crate::settlement::Building::HALF_WIDTH + radius
                && (dz * c - dx * s).abs() < crate::settlement::Building::HALF_DEPTH + radius
            {
                return false;
            }
        }
        for plant in &self.cover {
            if (!plant.kind.tree() && !plant.kind.rock())
                || (plant.kind.rock() && plant.size < 0.15)
            {
                continue;
            }
            let r = radius + plant.kind.collision_radius() * plant.size;
            if (plant.position.x - p.x).powi(2) + (plant.position.z - p.z).powi(2) < r * r {
                return false;
            }
        }
        crate::worldgen::river_distance(sim, p) >= metres(7.)
    }

    /// Sweep short steps rather than teleporting through a narrow obstacle.
    pub(crate) fn advance(&self, from: Position, to: Position) -> Position {
        let sim = self.sim;
        if !to.x.is_finite() || !to.z.is_finite() {
            return from;
        }
        let dx = to.x - from.x;
        let dz = to.z - from.z;
        let distance = (dx * dx + dz * dz).sqrt();
        // Application frame limits movement to 0.4 m. Keep public queries bounded.
        if distance > metres(10.) {
            return from;
        }
        let steps = (distance / metres(0.15)).ceil().max(1.) as u32;
        let mut current = from;
        let mut height = elevation(sim, from);
        for i in 1..=steps {
            let p = Position::new(
                from.x + dx * i as f32 / steps as f32,
                from.z + dz * i as f32 / steps as f32,
            );
            if !self.standable(p) {
                break;
            }
            let next = elevation(sim, p);
            if (next - height).abs() > distance / steps as f32 + metres(0.02) {
                break;
            }
            current = p;
            height = next;
        }
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sweeps_stop_at_walls_steep_ground_and_water() {
        let mut sim = SimState::new(crate::config::SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        for c in sim.grid.cells_mut() {
            c.elevation = 2.;
        }
        sim.buildings.push(crate::settlement::Building {
            id: 1,
            position: Position::new(5., 5.),
            rotation: 0.,
        });
        let from = Position::new(5. - metres(7.), 5.);
        let hit = advance(&sim, from, Position::new(5. + metres(2.), 5.));
        assert!(
            hit.x > from.x && hit.x < 5. - crate::settlement::Building::HALF_WIDTH - metres(0.3)
        );
        assert!(standable(&sim, hit));
        assert!(!standable(&sim, Position::new(5., 5.)));
        sim.buildings.clear();
        for z in 0..16 {
            for x in 0..16 {
                sim.grid.get_mut(x, z).elevation = x as f32 * 2.;
            }
        }
        assert_eq!(
            advance(&sim, from, Position::new(from.x + metres(1.), from.z)),
            from
        );
        sim.sea_level = 100.;
        assert_eq!(
            advance(&sim, from, Position::new(from.x + metres(1.), from.z)),
            from
        );
    }
}
