//! Small surface-conforming river patches. Cross-channel subdivisions prevent
//! the broad water strips cutting through triangle creases in the valley floor.
use crate::terrain;
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{SimState, spatial::Position, units::metres};
use glam::Vec2;
mod surface;

pub fn rivers(sim: &SimState, cache: &terrain::Cache, out: &mut Vec<Vertex>) {
    for river in &sim.geography.rivers {
        let mut points = Vec::new();
        let mut widths = Vec::new();
        for (segment, pair) in river.points.windows(2).enumerate() {
            let a = Vec2::new(pair[0].x, pair[0].z);
            let b = Vec2::new(pair[1].x, pair[1].z);
            let steps = (a.distance(b) / metres(1.5)).ceil().max(1.) as usize;
            for i in 0..steps {
                let t = i as f32 / steps as f32;
                points.push(a.lerp(b, t));
                let start = river.widths.get(segment).copied().unwrap_or(river.width);
                let end = river
                    .widths
                    .get(segment + 1)
                    .copied()
                    .unwrap_or(river.width);
                widths.push(start + (end - start) * t);
            }
        }
        if let Some(p) = river.points.last() {
            points.push(Vec2::new(p.x, p.z));
            widths.push(river.widths.last().copied().unwrap_or(river.width));
        }
        let mut previous: Option<[Vertex; 3]> = None;
        for (i, &p) in points.iter().enumerate() {
            let a = points[i.saturating_sub(1)];
            let b = points[(i + 1).min(points.len() - 1)];
            let tangent = (b - a).normalize_or_zero();
            let lateral = Vec2::new(-tangent.y, tangent.x);
            let width = widths[i];
            let row = std::array::from_fn(|j| {
                let side = j as f32 / 2. - 0.5;
                let q = p + lateral * width * side;
                let mut v = terrain::point(sim, Position::new(q.x, q.y));
                v.y = cache.height_at(sim, Position::new(q.x, q.y)) + metres(0.18);
                Vertex {
                    position: v.to_array(),
                    color: [0.16, 0.25, 0.20, -(1.9 + (1. - side.abs() * 2.) * 0.8)],
                }
            });
            if let Some(last) = previous {
                for j in 0..2 {
                    let ps = [last[j], last[j + 1], row[j], row[j + 1]];
                    for ids in [[0, 1, 2], [2, 1, 3]] {
                        let tri = ids.map(|id| ps[id]);
                        let detail = tri
                            .iter()
                            .map(|v| {
                                cache.detail_at(Position::new(
                                    v.position[0] + sim.grid.width as f32 / 2.,
                                    v.position[2] + sim.grid.height as f32 / 2.,
                                ))
                            })
                            .max()
                            .unwrap_or(1);
                        surface::drape_on(sim, tri, out, detail, |p| cache.height_at(sim, p));
                    }
                }
            }
            previous = Some(row);
        }
    }
}
