//! Separate river surfaces, intersected with the shared incised channel bed.
use crate::terrain;
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{SimState, spatial::Position, surface::channels, units::metres};
use glam::Vec2;
mod depth_cache;
mod sheet;
mod slopes;
mod surface;

pub fn rivers(sim: &SimState, cache: &terrain::Cache, out: &mut Vec<Vertex>) {
    rivers_profiled(sim, cache, out, |_| {});
}

pub(crate) fn rivers_profiled(
    sim: &SimState,
    cache: &terrain::Cache,
    out: &mut Vec<Vertex>,
    completed: impl FnMut(&'static str),
) {
    let mut depths = depth_cache::DepthCache::new();
    rivers_with_depth(sim, cache, out, completed, |p| depths.at(sim, p));
}

fn rivers_with_depth(
    sim: &SimState,
    cache: &terrain::Cache,
    out: &mut Vec<Vertex>,
    mut completed: impl FnMut(&'static str),
    depth: impl FnMut(Position) -> f32,
) {
    let incised = sim.geography.version >= 8;
    let slopes = slopes::Slopes::new(sim);
    completed("river_slopes");
    let mut sheet = sheet::Sheet::default();
    // Water, bed geometry and bank shading use the same distant approximation.
    let simplification = |p| terrain::lod::channel_simplification(sim, cache.key.eye(), p);
    for river in &sim.geography.rivers {
        let mut points = Vec::new();
        let mut widths = Vec::new();
        for (segment, pair) in river.points.windows(2).enumerate() {
            let a = Vec2::new(pair[0].x, pair[0].z);
            let b = Vec2::new(pair[1].x, pair[1].z);
            let spacing = metres(1.5 + 6.5 * simplification(pair[0]).min(simplification(pair[1])));
            let steps = (a.distance(b) / spacing).ceil().max(1.) as usize;
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
            let span = width
                + if incised {
                    channels::BANK * 2. * (1. - simplification(Position::new(p.x, p.y)))
                } else {
                    0.
                };
            let level = channels::level(sim, Position::new(p.x, p.y));
            let bed_material = match sim.subsurface.reading(Position::new(p.x, p.y)).rock {
                genesis_sim_core::subsurface::Rock::Limestone => -1.,
                genesis_sim_core::subsurface::Rock::Sandstone => -2.,
                genesis_sim_core::subsurface::Rock::Basalt => -3.,
            };
            let row = std::array::from_fn(|j| {
                let side = j as f32 / 2. - 0.5;
                let q = p + lateral * span * side;
                let slope = if incised { Vec2::ZERO } else { slopes.at(q) };
                let height = if incised {
                    level
                } else {
                    cache.height_at(sim, Position::new(q.x, q.y)) + metres(0.18)
                };
                Vertex {
                    position: [
                        q.x - sim.grid.width as f32 / 2.,
                        height,
                        q.y - sim.grid.height as f32 / 2.,
                    ],
                    color: [
                        bed_material,
                        slope.x,
                        slope.y,
                        -(1.9 + (1. - side.abs() * 2.) * 0.8),
                    ],
                }
            });
            if let Some(last) = previous {
                for j in 0..2 {
                    let ps = [last[j], last[j + 1], row[j], row[j + 1]];
                    for ids in [[0, 1, 2], [2, 1, 3]] {
                        let tri = ids.map(|id| ps[id]);
                        if incised {
                            sheet.add(sim, cache, tri);
                        } else {
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
            }
            previous = Some(row);
        }
    }
    completed("river_projection");
    if incised {
        sheet.finish(sim, &slopes, out, simplification, depth);
    }
    completed("river_surface");
}

/// Independent uncached depth queries protect the memoization boundary while
/// retaining exactly the same receiver/polygon ordering and shading arithmetic.
#[cfg(test)]
pub(crate) fn reference_rivers(sim: &SimState, cache: &terrain::Cache) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    rivers_with_depth(
        sim,
        cache,
        &mut vertices,
        |_| {},
        |p| surface::channel_depth(sim, p),
    );
    vertices
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sloping_confluence_has_one_surface_and_is_independent_of_branch_order() {
        use genesis_sim_core::{config::SimConfig, worldgen::River};
        use glam::Vec3;
        let mut sim = SimState::new(SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        sim.geography.version = 9;
        for z in 0..16 {
            for x in 0..16 {
                sim.grid.get_mut(x, z).elevation = 2. + x as f32 * 0.03 + z as f32 * 0.07;
            }
        }
        let river = |points| River {
            points,
            width: metres(6.),
            widths: vec![],
            bounds: None,
        };
        sim.geography.rivers = vec![
            river(vec![
                Position::new(4., 8.),
                Position::new(8., 8.),
                Position::new(12., 8.),
            ]),
            river(vec![Position::new(8., 4.), Position::new(8., 8.)]),
        ];
        let hits = |mesh: &[Vertex], p: Vec2| {
            mesh.chunks_exact(3)
                .filter_map(|tri| {
                    crate::terrain::intersect(
                        Vec3::new(p.x, 20., p.y),
                        -Vec3::Y,
                        [tri[0], tri[1], tri[2]].map(|v| Vec3::from_array(v.position)),
                    )
                })
                .map(|distance| 20. - distance)
                .collect::<Vec<_>>()
        };
        for eye in [Vec3::new(0., 2.9, 0.), Vec3::new(0., 100., 0.)] {
            sim.geography.channels = channels::Index::new(16, 16, &sim.geography.rivers);
            let a = terrain::Cache::at(&sim, terrain::lod::Key::at(eye));
            sim.geography.rivers.reverse();
            sim.geography.channels = channels::Index::new(16, 16, &sim.geography.rivers);
            let b = terrain::Cache::at(&sim, terrain::lod::Key::at(eye));
            for i in 0..200 {
                // Off-grid samples across the shared wet core, away from banks.
                let p = Vec2::new(
                    (i as f32 * 0.618034).fract() * 0.5 - 0.2497,
                    (i as f32 * 0.414214).fract() * 0.08 - 0.0393,
                );
                let first = hits(&a.rivers, p);
                let reversed = hits(&b.rivers, p);
                assert!(
                    !first.is_empty() && !reversed.is_empty(),
                    "wet core hole at {p:?}"
                );
                // A shared triangle edge may be hit twice; it must have one height.
                for &height in first.iter().chain(&reversed) {
                    assert!(
                        (height - first[0]).abs() < metres(0.002),
                        "two water levels at {p:?}: {first:?} / {reversed:?}"
                    );
                }
            }
            for tri in a.rivers.chunks_exact(3) {
                let [a, b, c] = [tri[0], tri[1], tri[2]].map(|v| Vec3::from_array(v.position));
                assert!(a.is_finite() && b.is_finite() && c.is_finite());
                assert!((b - a).cross(c - a).y > 1e-12);
            }
        }
    }

    #[test]
    fn close_channel_water_stays_above_the_rendered_bed_with_real_depth() {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        sim.geography.version = 8;
        for c in sim.grid.cells_mut() {
            c.elevation = 2.;
        }
        sim.geography
            .rivers
            .push(genesis_sim_core::worldgen::River {
                points: vec![
                    Position::new(7., 7.),
                    Position::new(8., 8.),
                    Position::new(9., 8.5),
                ],
                width: metres(4.),
                widths: vec![],
                bounds: None,
            });
        sim.geography.channels = channels::Index::new(16, 16, &sim.geography.rivers);
        let cache = terrain::Cache::at(&sim, terrain::lod::Key::at(glam::Vec3::new(0., 2.2, 0.)));
        assert!(!cache.rivers.is_empty());
        for tri in cache.rivers.chunks_exact(3) {
            for weights in [[1. / 3.; 3], [0.1, 0.3, 0.6]] {
                let p: [f32; 3] =
                    std::array::from_fn(|i| (0..3).map(|j| tri[j].position[i] * weights[j]).sum());
                let ground = cache.height_at(&sim, Position::new(p[0] + 8., p[2] + 8.));
                let depth: f32 = (0..3).map(|j| (-tri[j].color[3] - 1.) * weights[j]).sum();
                assert!(p[1] >= ground - metres(0.001));
                assert!(((p[1] - ground) * 20. - depth).abs() < 0.002);
                assert!((p[1] - channels::level(&sim, Position::new(8., 8.))).abs() < 1e-5);
            }
        }
        // A regional mesh must not magnify a metre-wide bed into 20 m spikes.
        // The physical bed remains present when the rendered view is simplified.
        let far = terrain::Cache::at(&sim, terrain::lod::Key::at(glam::Vec3::new(0., 100., 0.)));
        let center = Position::new(8., 8.);
        assert!((far.height_at(&sim, center) - 2.).abs() < 1e-5);
        assert!(genesis_sim_core::surface::elevation(&sim, center) < 2. - metres(0.25));
        assert!(!far.rivers.is_empty());
        // Distant water uses the channel's optical depth, not the 4 cm display
        // offset over terrain whose narrow incision has been simplified away.
        let center_depth = far
            .rivers
            .iter()
            .filter(|v| {
                (v.position[0] - (center.x - 8.)).hypot(v.position[2] - (center.z - 8.))
                    < metres(0.05)
            })
            .map(|v| -v.color[3] - 1.)
            .fold(0., f32::max);
        assert!((center_depth - 0.64).abs() < 0.02, "{center_depth}");
        assert!(far.rivers.iter().any(|v| -v.color[3] - 1. < 0.02));
        for mesh in [&cache.rivers, &far.rivers] {
            // A flat channel carries a vertical shading normal, independent of
            // ribbon direction and camera detail. GB stores slopes, not UVs.
            assert!(
                mesh.iter()
                    .all(|v| v.color[1].abs() < 1e-5 && v.color[2].abs() < 1e-5)
            );
        }
    }
}
