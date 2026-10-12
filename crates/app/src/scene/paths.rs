//! Lit, feathered footpath decals clipped to the current adaptive terrain mesh.
//! The simulated route/width is unchanged; this is cosmetic surface treatment.
use crate::terrain;
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{
    SimState, paths,
    settlement::Building,
    spatial::{self, Position},
    subsurface::Rock,
    units::metres,
};
use glam::{Vec2, Vec3};

pub(super) fn entrance(sim: &SimState, b: &Building) -> Option<Vec<Position>> {
    let (s, c) = b.rotation.sin_cos();
    let door = Position::new(
        b.position.x - s * (Building::HALF_DEPTH + metres(0.1)),
        b.position.z + c * (Building::HALF_DEPTH + metres(0.1)),
    );
    let nearest = sim
        .paths
        .iter()
        .flat_map(|p| p.points.windows(2))
        .map(|p| paths::nearest(door, p[0], p[1]))
        .min_by(|a, b| paths::distance(door, *a).total_cmp(&paths::distance(door, *b)))?;
    let control = Position::new(door.x - s * metres(1.5), door.z + c * metres(1.5));
    // Bounded by the existing route sampling recipe, with enough samples at bends.
    let points = (0..=16)
        .map(|i| {
            let t = i as f32 / 16.;
            let u = 1. - t;
            Position::new(
                u * u * door.x + 2. * u * t * control.x + t * t * nearest.x,
                u * u * door.z + 2. * u * t * control.z + t * t * nearest.z,
            )
        })
        .collect();
    Some(paths::samples(&genesis_sim_core::settlement::Path {
        id: 0,
        points,
        width: Building::ENTRANCE_WIDTH,
    }))
}

fn clip(poly: &[Vertex], a: Vec3, b: Vec3) -> Vec<Vertex> {
    let mut out = Vec::new();
    let Some(mut previous) = poly.last().copied() else {
        return out;
    };
    let distance =
        |v: Vertex| (b.z - a.z) * (v.position[0] - a.x) - (b.x - a.x) * (v.position[2] - a.z);
    let mut dp = distance(previous);
    for &current in poly {
        let dc = distance(current);
        if (dp >= 0.) != (dc >= 0.) {
            let t = dp / (dp - dc);
            out.push(Vertex {
                position: std::array::from_fn(|i| {
                    previous.position[i] + (current.position[i] - previous.position[i]) * t
                }),
                color: std::array::from_fn(|i| {
                    previous.color[i] + (current.color[i] - previous.color[i]) * t
                }),
            });
        }
        if dc >= 0. {
            out.push(current);
        }
        previous = current;
        dp = dc;
    }
    out
}

fn conform(
    triangle: [Vertex; 3],
    ground: [Vec3; 3],
    out: &mut Vec<Vertex>,
    offset: f32,
    lit: bool,
) {
    let mut polygon = triangle.to_vec();
    for i in 0..3 {
        polygon = clip(&polygon, ground[i], ground[(i + 1) % 3]);
    }
    let normal = (ground[1] - ground[0]).cross(ground[2] - ground[0]);
    if normal.y <= 1e-12 {
        return;
    }
    let light = 0.48
        + 0.52
            * normal
                .normalize()
                .dot(Vec3::new(-0.5, 0.75, 0.3).normalize())
                .max(0.);
    for v in &mut polygon {
        v.position[1] = ground[0].y
            - (normal.x * (v.position[0] - ground[0].x) + normal.z * (v.position[2] - ground[0].z))
                / normal.y
            + offset;
        if lit {
            for c in &mut v.color[..3] {
                *c *= light;
            }
        }
    }
    for i in 1..polygon.len().saturating_sub(1) {
        let tri = [polygon[0], polygon[i], polygon[i + 1]];
        let [a, b, c] = tri.map(|v| Vec3::from_array(v.position));
        if (b - a).cross(c - a).y > 1e-12 {
            out.extend(tri);
        }
    }
}

pub(super) fn surface(
    sim: &SimState,
    cache: &terrain::Cache,
    out: &mut Vec<Vertex>,
    points: &[Position],
    width: f32,
) {
    if points.len() < 2 {
        return;
    }
    let mut distance = vec![0.];
    for pair in points.windows(2) {
        distance.push(distance[distance.len() - 1] + paths::distance(pair[0], pair[1]));
    }
    let total = distance[distance.len() - 1];
    let rows: Vec<[Vertex; 4]> = points
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let a = points[i.saturating_sub(1)];
            let b = points[(i + 1).min(points.len() - 1)];
            let d = Vec2::new(b.x - a.x, b.z - a.z).normalize_or_zero();
            let edge = width * 0.5 + metres(0.08);
            let core = (width * 0.5 - metres(0.14)).max(width * 0.2);
            let cap = (distance[i].min(total - distance[i]) / metres(0.25)).clamp(0., 1.);
            [(-edge, 0.), (-core, 1.), (core, 1.), (edge, 0.)].map(|(offset, opacity)| {
                let p = Position::new(p.x - d.y * offset, p.z + d.x * offset);
                let mut color = match sim.subsurface.reading(p).rock {
                    Rock::Limestone => [0.42, 0.34, 0.235],
                    Rock::Sandstone => [0.40, 0.28, 0.18],
                    Rock::Basalt => [0.29, 0.27, 0.22],
                };
                let damp = 1. - spatial::sample(&sim.grid, p, |c| c.moisture).clamp(0., 1.) * 0.18;
                for c in &mut color {
                    *c *= damp;
                }
                Vertex {
                    position: [
                        p.x - sim.grid.width as f32 * 0.5,
                        0.,
                        p.z - sim.grid.height as f32 * 0.5,
                    ],
                    color: [color[0], color[1], color[2], 14. + opacity * cap],
                }
            })
        })
        .collect();
    for pair in rows.windows(2) {
        triangles_under(sim, cache, pair.iter().flatten(), |ground| {
            for band in 0..3 {
                conform(
                    [pair[0][band], pair[0][band + 1], pair[1][band]],
                    ground,
                    out,
                    metres(0.012),
                    true,
                );
                conform(
                    [pair[0][band + 1], pair[1][band + 1], pair[1][band]],
                    ground,
                    out,
                    metres(0.012),
                    true,
                );
            }
        });
    }
}

fn triangles_under<'a>(
    sim: &SimState,
    cache: &terrain::Cache,
    vertices: impl Iterator<Item = &'a Vertex>,
    visit: impl FnMut([Vec3; 3]),
) {
    let (min, max) = vertices.fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(min, max), v| {
            let p = Vec2::new(v.position[0], v.position[2]);
            (min.min(p), max.max(p))
        },
    );
    let center = Vec2::new(sim.grid.width as f32, sim.grid.height as f32) * 0.5;
    cache.triangles_in(
        Position::new(min.x + center.x, min.y + center.y),
        Position::new(max.x + center.x, max.y + center.y),
        visit,
    );
}

/// Strong interaction colours on the same triangles as the visible ground.
/// A missing cache retains immediate feedback while the first terrain job finishes.
pub(crate) fn highlight(
    sim: &SimState,
    cache: Option<&terrain::Cache>,
    out: &mut Vec<Vertex>,
    points: &[Position],
    width: f32,
    color: [f32; 3],
    seafloor: bool,
) {
    let start = out.len();
    let Some(cache) = cache else {
        super::ribbon(sim, out, points, width, color);
        if seafloor {
            for v in &mut out[start..] {
                let p = Position::new(
                    v.position[0] + sim.grid.width as f32 * 0.5,
                    v.position[2] + sim.grid.height as f32 * 0.5,
                );
                v.position[1] = terrain::point(sim, p).y + metres(0.04);
            }
        }
        return;
    };
    // Build only planar top faces. Terrain queries and discarded back faces are
    // unnecessary because clipping below supplies the actual receiver heights.
    let rows: Vec<_> = super::markings::edges(points, width)
        .into_iter()
        .map(|pair| {
            pair.map(|p| Vertex {
                position: [
                    p.x - sim.grid.width as f32 * 0.5,
                    0.,
                    p.z - sim.grid.height as f32 * 0.5,
                ],
                color: [color[0], color[1], color[2], 1.],
            })
        })
        .collect();
    for pair in rows.windows(2) {
        triangles_under(sim, cache, pair.iter().flatten(), |ground| {
            conform(
                [pair[0][0], pair[0][1], pair[1][0]],
                ground,
                out,
                metres(0.04),
                false,
            );
            conform(
                [pair[0][1], pair[1][1], pair[1][0]],
                ground,
                out,
                metres(0.04),
                false,
            );
        });
    }
    if !seafloor {
        // Invalid ocean routes must still show red above the water.
        for v in &mut out[start..] {
            v.position[1] = v.position[1].max(sim.sea_level + metres(0.06));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_follow_visible_triangles_without_losing_area_or_width() {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 8,
            height: 8,
            ..Default::default()
        });
        sim.geography.version = 8;
        for z in 0..8 {
            for x in 0..8 {
                sim.grid.get_mut(x, z).elevation = 2. + ((x * 3 + z * 5) % 7) as f32 * 0.03;
            }
        }
        let path = genesis_sim_core::settlement::Path {
            id: 0,
            points: vec![Position::new(2.6, 3.1), Position::new(5.2, 4.4)],
            width: metres(1.2),
        };
        let points = paths::samples(&path);
        let expected_area = paths::length(&path) * (path.width + metres(0.16));
        for eye in [Vec3::new(0., 2.1, 0.), Vec3::new(0., 100., 0.)] {
            let cache = terrain::Cache::at(&sim, terrain::lod::Key::at(eye));
            let mut mesh = Vec::new();
            surface(&sim, &cache, &mut mesh, &points, path.width);
            assert!(!mesh.is_empty() && mesh.len() < 150_000);
            let mut area = 0.;
            for tri in mesh.chunks_exact(3) {
                let [a, b, c] = [tri[0], tri[1], tri[2]].map(|v| Vec3::from_array(v.position));
                area += (b - a).cross(c - a).y * 0.5;
                for weights in [[1. / 3.; 3], [0.2, 0.6, 0.2], [0.1, 0.1, 0.8]] {
                    let v = a * weights[0] + b * weights[1] + c * weights[2];
                    let ground = cache.height_at(&sim, Position::new(v.x + 4., v.z + 4.));
                    assert!((v.y - ground - metres(0.012)).abs() < 2e-5);
                }
            }
            assert!(
                (area - expected_area).abs() < 0.0001,
                "{area} != {expected_area}"
            );
            assert!(
                mesh.iter()
                    .all(|v| v.position.iter().chain(&v.color).all(|v| v.is_finite())
                        && (14.0..=15.00001).contains(&v.color[3]))
            );
            assert!(mesh.iter().any(|v| v.color[3] == 14.));
            assert!(mesh.iter().any(|v| v.color[3] == 15.));
        }
    }

    #[test]
    fn highlights_preserve_width_colour_and_contact_at_both_detail_levels() {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 8,
            height: 8,
            ..Default::default()
        });
        sim.geography.version = 8;
        for z in 0..8 {
            for x in 0..8 {
                sim.grid.get_mut(x, z).elevation = 2. + ((x + z * 3) % 5) as f32 * 0.02;
            }
        }
        let path = genesis_sim_core::settlement::Path {
            id: 0,
            points: vec![Position::new(2.6, 3.1), Position::new(5.2, 4.4)],
            width: metres(4.),
        };
        let points = paths::samples(&path);
        let color = [0.22, 0.80, 0.66];
        for eye in [Vec3::new(0., 2.1, 0.), Vec3::new(0., 100., 0.)] {
            let cache = terrain::Cache::at(&sim, terrain::lod::Key::at(eye));
            let mut out = Vec::new();
            highlight(
                &sim,
                Some(&cache),
                &mut out,
                &points,
                path.width,
                color,
                false,
            );
            assert!(!out.is_empty() && out.len() < 150_000);
            let mut area = 0.;
            for triangle in out.chunks_exact(3) {
                let [a, b, c] =
                    [triangle[0], triangle[1], triangle[2]].map(|v| Vec3::from_array(v.position));
                area += (b - a).cross(c - a).y * 0.5;
                let p = a * 0.2 + b * 0.3 + c * 0.5;
                let y = cache.height_at(&sim, Position::new(p.x + 4., p.z + 4.));
                assert!((p.y - y - metres(0.04)).abs() < 2e-5);
            }
            assert!((area - paths::length(&path) * path.width).abs() < 1e-4);
            assert!(
                out.iter()
                    .all(|v| v.color == [color[0], color[1], color[2], 1.])
            );
            sim.sea_level = 5.;
            out.clear();
            highlight(
                &sim,
                Some(&cache),
                &mut out,
                &points,
                path.width,
                [0.95, 0.22, 0.13],
                false,
            );
            assert!(
                out.iter()
                    .all(|v| v.position[1] == sim.sea_level + metres(0.06))
            );
            sim.sea_level = 0.;
        }
        let mut fallback = Vec::new();
        highlight(&sim, None, &mut fallback, &points, path.width, color, false);
        assert!(!fallback.is_empty());
    }

    #[test]
    fn entrances_join_segments_instead_of_distant_waypoints() {
        let mut sim = genesis_sim_core::scenarios::coastal_outpost();
        let b = Building {
            id: 1,
            position: Position::new(8., 8.),
            rotation: 0.,
        };
        sim.paths = vec![genesis_sim_core::settlement::Path {
            id: 0,
            points: vec![Position::new(4., 9.), Position::new(12., 9.)],
            width: metres(1.2),
        }];
        let points = entrance(&sim, &b).expect("path exists");
        let end = points.last().expect("entrance endpoint");
        assert!((end.x - 8.).abs() < 1e-6 && (end.z - 9.).abs() < 1e-6);
        sim.paths.clear();
        assert!(entrance(&sim, &b).is_none());
    }
}
