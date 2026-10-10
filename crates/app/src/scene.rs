//! Read-only adapter: world positions are simulation data, meshes are disposable.
use crate::{session::Session, terrain};
use genesis_renderer::geometry::{Vertex, cuboid};
use genesis_sim_core::{SimState, settlement::Building, spatial::Position, units::metres};
use glam::Vec3;

fn triangle(out: &mut Vec<Vertex>, points: [Vec3; 3], color: [f32; 3]) {
    for p in points {
        out.push(Vertex {
            position: p.to_array(),
            color: [color[0], color[1], color[2], 1.],
        });
    }
}
fn ribbon(sim: &SimState, out: &mut Vec<Vertex>, points: &[Position], width: f32, color: [f32; 3]) {
    if points.len() < 2 {
        return;
    }
    let edges: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let a = points[i.saturating_sub(1)];
            let b = points[(i + 1).min(points.len() - 1)];
            let d = glam::Vec2::new(b.x - a.x, b.z - a.z).normalize_or_zero();
            [-1., 1.].map(|side| {
                let p = Position::new(
                    p.x - d.y * width * 0.5 * side,
                    p.z + d.x * width * 0.5 * side,
                );
                terrain::surface_point(sim, p) + Vec3::Y * metres(0.06)
            })
        })
        .collect();
    for e in edges.windows(2) {
        // Both faces permit arbitrary path direction and steep camera views.
        for ids in [[0, 1, 2], [2, 1, 3], [2, 1, 0], [3, 1, 2]] {
            let ps = [e[0][0], e[0][1], e[1][0], e[1][1]];
            triangle(out, ids.map(|i| ps[i]), color);
        }
    }
}
fn house(sim: &SimState, out: &mut Vec<Vertex>, b: &Building) {
    let start = out.len();
    let corners = b.footprint();
    let high = corners
        .iter()
        .map(|p| terrain::point(sim, *p).y)
        .fold(f32::NEG_INFINITY, f32::max);
    let low = corners
        .iter()
        .map(|p| terrain::point(sim, *p).y)
        .fold(f32::INFINITY, f32::min);
    // The same dimensions drive the solid model, footprint, and build preview.
    let width = Building::HALF_WIDTH * 2.;
    let depth = Building::HALF_DEPTH * 2.;
    let height = Building::WALL_HEIGHT;
    cuboid(
        out,
        [0., (high + low) * 0.5, 0.],
        [width, high - low + metres(0.15), depth],
        [0.44, 0.37, 0.27],
    );
    cuboid(
        out,
        [0., high + height * 0.5, 0.],
        [width, height, depth],
        [0.77, 0.64, 0.44],
    );
    cuboid(
        out,
        [0., high + height + metres(0.075), 0.],
        [width, metres(0.15), depth],
        [0.51, 0.34, 0.23],
    );
    // Human-scale door, lintel, and low roof parapet.
    cuboid(
        out,
        [0., high + metres(0.975), depth * 0.5 + metres(0.01)],
        [metres(0.85), metres(1.95), metres(0.02)],
        [0.15, 0.12, 0.085],
    );
    cuboid(
        out,
        [0., high + metres(2.05), depth * 0.5 + metres(0.06)],
        [metres(1.15), metres(0.18), metres(0.16)],
        [0.61, 0.48, 0.32],
    );
    for x in [-1., 1.] {
        cuboid(
            out,
            [
                x * (width * 0.5 - metres(0.1)),
                high + height + metres(0.325),
                0.,
            ],
            [metres(0.2), metres(0.35), depth],
            [0.73, 0.58, 0.39],
        );
    }
    for z in [-1., 1.] {
        cuboid(
            out,
            [
                0.,
                high + height + metres(0.325),
                z * (depth * 0.5 - metres(0.1)),
            ],
            [width, metres(0.35), metres(0.2)],
            [0.73, 0.58, 0.39],
        );
    }
    // Small shuttered openings, exposed roof joists, and a stone doorstep.
    for x in [-1.8, 1.8] {
        cuboid(
            out,
            [metres(x), high + metres(1.8), depth * 0.5 + metres(0.02)],
            [metres(0.5), metres(0.65), metres(0.05)],
            [0.24, 0.20, 0.13],
        );
        cuboid(
            out,
            [metres(x), high + metres(2.16), depth * 0.5 + metres(0.06)],
            [metres(0.72), metres(0.12), metres(0.15)],
            [0.54, 0.44, 0.30],
        );
    }
    for i in 0..6 {
        cuboid(
            out,
            [
                -width * 0.4 + i as f32 * width * 0.16,
                high + height - metres(0.15),
                0.,
            ],
            [metres(0.16), metres(0.18), depth + metres(0.35)],
            [0.28, 0.21, 0.13],
        );
    }
    cuboid(
        out,
        [0., high + metres(0.10), depth * 0.5 + metres(0.3)],
        [metres(1.15), metres(0.20), metres(0.65)],
        [0.52, 0.47, 0.37],
    );
    let (s, c) = b.rotation.sin_cos();
    let center = terrain::point(sim, b.position);
    for v in &mut out[start..] {
        v.color[3] = 3.;
        let [x, y, z] = v.position;
        v.position = [center.x + c * x - s * z, y, center.z + s * x + c * z];
    }
}

pub fn mesh(
    sim: &SimState,
    layers: crate::survey::Survey,
    terrain_cache: &terrain::Cache,
    cover: &[Vertex],
) -> Vec<Vertex> {
    let mut out = Vec::new();
    terrain_cache.mesh(sim, &mut out, layers);
    out.extend_from_slice(cover);
    for path in &sim.paths {
        ribbon(sim, &mut out, &path.points, path.width, [0.59, 0.44, 0.27]);
    }
    if layers.minerals {
        for d in &sim.subsurface.deposits {
            for radius in [1., 0.7] {
                let points: Vec<_> = (0..=80)
                    .map(|i| {
                        let a = i as f32 / 80. * std::f32::consts::TAU;
                        d.outline(a, radius)
                    })
                    .collect();
                ribbon(
                    sim,
                    &mut out,
                    &points,
                    if radius == 1. { 0.1 } else { 0.045 },
                    crate::survey::mineral_color(d.mineral),
                );
            }
        }
    }
    for well in &sim.subsurface.wells {
        let p = terrain::surface_point(sim, well.position);
        cuboid(
            &mut out,
            [p.x, p.y + metres(0.1), p.z],
            [metres(0.7), metres(0.2), metres(0.7)],
            [0.3, 0.4, 0.43],
        );
        cuboid(
            &mut out,
            [p.x, p.y + metres(0.65), p.z],
            [metres(0.12), metres(1.1), metres(0.12)],
            if well.active {
                [0.1, 0.72, 0.9]
            } else {
                [0.4, 0.4, 0.4]
            },
        );
        cuboid(
            &mut out,
            [p.x, p.y + metres(1.2), p.z],
            [metres(0.6), metres(0.12), metres(0.12)],
            [0.65, 0.78, 0.79],
        );
    }
    for b in &sim.buildings {
        house(sim, &mut out, b);
        // A short curved entrance walk joins the authored centerline.
        let (s, c) = b.rotation.sin_cos();
        let door = Position::new(
            b.position.x - s * (Building::HALF_DEPTH + metres(0.1)),
            b.position.z + c * (Building::HALF_DEPTH + metres(0.1)),
        );
        if let Some(nearest) = sim.paths.iter().flat_map(|p| &p.points).min_by(|a, b| {
            let dist = |p: &Position| (p.x - door.x).powi(2) + (p.z - door.z).powi(2);
            dist(a).total_cmp(&dist(b))
        }) {
            let control = Position::new(door.x - s * metres(1.5), door.z + c * metres(1.5));
            let points: Vec<_> = (0..=16)
                .map(|i| {
                    let t = i as f32 / 16.;
                    let u = 1. - t;
                    Position::new(
                        u * u * door.x + 2. * u * t * control.x + t * t * nearest.x,
                        u * u * door.z + 2. * u * t * control.z + t * t * nearest.z,
                    )
                })
                .collect();
            ribbon(
                sim,
                &mut out,
                &points,
                Building::ENTRANCE_WIDTH,
                [0.59, 0.44, 0.27],
            );
        }
    }
    out
}

pub fn overlay(session: &Session) -> Vec<Vertex> {
    let sim = &session.sim;
    let mut out = Vec::new();
    if session.placing && session.hover.is_none() {
        return out;
    }
    if session.placing {
        let b = session.candidate();
        let valid = sim.validate_building(&b).is_ok();
        if valid {
            house(sim, &mut out, &b);
            for v in &mut out {
                v.color = [
                    v.color[0] * 0.5 + 0.1,
                    v.color[1] * 0.5 + 0.45,
                    v.color[2] * 0.5 + 0.2,
                    1.,
                ];
            }
        }
        let corners = b.footprint().map(|p| {
            Position::new(
                b.position.x + (p.x - b.position.x) * 1.2,
                b.position.z + (p.z - b.position.z) * 1.2,
            )
        });
        let mut points = Vec::new();
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            for j in 0..8 {
                let t = j as f32 / 8.;
                points.push(Position::new(a.x + (b.x - a.x) * t, a.z + (b.z - a.z) * t));
            }
        }
        points.push(corners[0]);
        ribbon(
            sim,
            &mut out,
            &points,
            metres(0.15),
            if valid {
                [0.35, 0.85, 0.44]
            } else {
                [0.95, 0.23, 0.13]
            },
        );
    } else {
        let points: Vec<_> = (0..=64)
            .map(|i| {
                let a = i as f32 / 64. * std::f32::consts::TAU;
                Position::new(
                    session.position.x + a.cos() * 0.48,
                    session.position.z + a.sin() * 0.48,
                )
            })
            .collect();
        ribbon(sim, &mut out, &points, 0.035, [0.95, 0.76, 0.35]);
    }
    if session.survey.seafloor {
        for v in &mut out {
            let p = Position::new(
                v.position[0] + sim.grid.width as f32 * 0.5,
                v.position[2] + sim.grid.height as f32 * 0.5,
            );
            if terrain::point(sim, p).y < sim.sea_level {
                v.position[1] = terrain::point(sim, p).y + metres(0.1);
            }
        }
    }
    out
}
