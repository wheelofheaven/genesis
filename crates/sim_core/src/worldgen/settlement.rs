use super::{noise::unit, river_distance};
use crate::{
    SimState,
    settlement::{Building, Path},
    spatial::{self, Position},
    units::metres,
};

pub fn seed(sim: &mut SimState) {
    // Prefer a gentle lowland terrace near fresh water, without flattening it.
    let mut best = None;
    let mut score = f32::NEG_INFINITY;
    for z in (6..sim.grid.height - 6).step_by(2) {
        for x in (6..sim.grid.width - 6).step_by(2) {
            let p = Position::new(x as f32 + 0.31, z as f32 + 0.27);
            let b = Building {
                id: 0,
                position: p,
                rotation: 0.,
            };
            if sim.validate_building(&b).is_err() {
                continue;
            }
            let h = spatial::elevation(&sim.grid, p);
            let slope = b
                .footprint()
                .iter()
                .map(|p| (spatial::elevation(&sim.grid, *p) - h).abs())
                .sum::<f32>();
            let river = river_distance(sim, p);
            // Site a community, not just one valid house. A flat spot on a
            // steep hillside may have no suitable neighbouring plots.
            let neighbours = (0..8)
                .filter(|i| {
                    let angle = *i as f32 * std::f32::consts::TAU / 8.;
                    let q = Position::new(
                        p.x + angle.cos() * metres(14.),
                        p.z + angle.sin() * metres(14.),
                    );
                    sim.validate_building(&Building {
                        id: 0,
                        position: q,
                        rotation: angle,
                    })
                    .is_ok()
                })
                .count();
            let rank = neighbours as f32 * 2.
                - slope * 5.
                - (h - 1.6).abs() * 0.4
                - (river - 3.).abs() * 0.18;
            if rank > score {
                score = rank;
                best = Some(p);
            }
        }
    }
    let Some(center) = best else {
        return;
    };
    for i in 0..150 {
        let angle = i as f32 * 2.399963;
        let radius = if i == 0 {
            0.
        } else {
            metres(10.) + (i as f32).sqrt() * metres(2.5)
        };
        let p = Position::new(
            center.x + angle.cos() * radius,
            center.z + angle.sin() * radius,
        );
        let b = Building {
            id: sim.buildings.len() as u64 + 1,
            position: p,
            rotation: angle + unit(sim.config.seed, i) * 0.5,
        };
        if sim.validate_building(&b).is_err() {
            continue;
        }
        // Only join a short, dry, gentle walk; do not bridge rivers by accident.
        if !sim.buildings.is_empty() {
            let nearest = sim
                .buildings
                .iter()
                .min_by(|a, b| {
                    let distance = |q: Position| (q.x - p.x).powi(2) + (q.z - p.z).powi(2);
                    distance(a.position).total_cmp(&distance(b.position))
                })
                .map(|b| b.position)
                .unwrap_or(center);
            let mut points = Vec::new();
            let dx = nearest.x - p.x;
            let dz = nearest.z - p.z;
            let length = (dx * dx + dz * dz).sqrt();
            for step in 0..=24 {
                let t = step as f32 / 24.;
                let bend = (t * std::f32::consts::PI).sin() * metres(0.8);
                points.push(Position::new(
                    p.x + dx * t - dz / length * bend,
                    p.z + dz * t + dx / length * bend,
                ));
            }
            if points
                .iter()
                .any(|p| spatial::elevation(&sim.grid, *p) < 0.2 || river_distance(sim, *p) < 1.)
            {
                continue;
            }
            sim.paths.push(Path {
                points,
                width: Building::PATH_WIDTH,
            });
        }
        sim.buildings.push(b);
        if sim.buildings.len() == 6 {
            break;
        }
    }
}
