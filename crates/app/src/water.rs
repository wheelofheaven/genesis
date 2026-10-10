//! Small surface-conforming river patches. Cross-channel subdivisions prevent
//! the broad water strips cutting through triangle creases in the valley floor.
use crate::terrain;
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{SimState, spatial::Position, units::metres};
use glam::Vec2;

pub fn rivers(sim: &SimState, out: &mut Vec<Vertex>) {
    for river in &sim.geography.rivers {
        let mut points = Vec::new();
        for pair in river.points.windows(2) {
            let a = Vec2::new(pair[0].x, pair[0].z);
            let b = Vec2::new(pair[1].x, pair[1].z);
            let steps = (a.distance(b) / metres(1.5)).ceil().max(1.) as usize;
            for i in 0..steps {
                points.push(a.lerp(b, i as f32 / steps as f32));
            }
        }
        if let Some(p) = river.points.last() {
            points.push(Vec2::new(p.x, p.z));
        }
        let mut previous: Option<[Vertex; 7]> = None;
        for (i, &p) in points.iter().enumerate() {
            let a = points[i.saturating_sub(1)];
            let b = points[(i + 1).min(points.len() - 1)];
            let tangent = (b - a).normalize_or_zero();
            let lateral = Vec2::new(-tangent.y, tangent.x);
            let width = river.width
                * (0.3 + 0.7 * i as f32 / (points.len() - 1).max(1) as f32)
                * (i as f32 / 16.).min(1.);
            let row = std::array::from_fn(|j| {
                let side = j as f32 / 6. - 0.5;
                let q = p + lateral * width * side;
                let mut v = terrain::point(sim, Position::new(q.x, q.y));
                v.y += metres(0.18);
                Vertex {
                    position: v.to_array(),
                    color: [0.16, 0.25, 0.20, -(1.9 + (1. - side.abs() * 2.) * 0.8)],
                }
            });
            if let Some(last) = previous {
                for j in 0..6 {
                    let ps = [last[j], last[j + 1], row[j], row[j + 1]];
                    for id in [0, 1, 2, 2, 1, 3] {
                        out.push(ps[id]);
                    }
                }
            }
            previous = Some(row);
        }
    }
}
