//! Small procedural dwellings. Dimensions are metres until the final transform;
//! collision, selection and placement continue to use the simulation footprint.
use crate::terrain;
use genesis_renderer::geometry::{Vertex, cuboid};
use genesis_sim_core::{SimState, settlement::Building};
use glam::Vec3;
use std::sync::OnceLock;
mod occlusion;

const PLASTER: f32 = 3.0;
const STONE: f32 = 3.1;
const WOOD: f32 = 3.2;
const EARTH: f32 = 3.3;
const WALL: [f32; 3] = [0.66, 0.56, 0.40];

// Unlike the generic preview cuboid, building albedo has no fixed face shading.
// Apply sunlight after rotation so identical walls share the world's sun.
fn part(out: &mut Vec<Vertex>, center: [f32; 3], size: [f32; 3], color: [f32; 3], material: f32) {
    let start = out.len();
    cuboid(out, center, size, color);
    for v in &mut out[start..] {
        v.color = [color[0], color[1], color[2], material];
    }
}

fn opening(out: &mut Vec<Vertex>, x: f32, bottom: f32, width: f32, height: f32, front: f32) {
    // A recessed backing and separate boards leave real narrow joints and wall
    // reveals. The door is closed: this pass does not add traversable interiors.
    part(
        out,
        [x, bottom + height * 0.5, front - 0.23],
        [width, height, 0.04],
        [0.075, 0.054, 0.032],
        WOOD,
    );
    let boards = if bottom == 0. { 6 } else { 4 };
    let pitch = width / boards as f32;
    for i in 0..boards {
        let variation = 0.90 + (i * 7 % 5) as f32 * 0.035;
        part(
            out,
            [
                x - width * 0.5 + (i as f32 + 0.5) * pitch,
                bottom + height * 0.5,
                front - 0.185,
            ],
            [pitch - 0.006, height - 0.025, 0.05],
            [0.24 * variation, 0.145 * variation, 0.070 * variation],
            WOOD,
        );
    }
    for y in [0.24, 0.76] {
        part(
            out,
            [x, bottom + height * y, front - 0.147],
            [width * 0.90, 0.065, 0.025],
            [0.17, 0.10, 0.052],
            WOOD,
        );
    }
    // Small timber lintel and stone sill, with their own shadows and silhouettes.
    part(
        out,
        [x, bottom + height + 0.095, front + 0.035],
        [width + 0.30, 0.18, 0.18],
        [0.30, 0.21, 0.115],
        WOOD,
    );
    if bottom > 0. {
        part(
            out,
            [x, bottom - 0.035, front + 0.025],
            [width + 0.16, 0.07, 0.17],
            [0.51, 0.47, 0.37],
            STONE,
        );
    } else {
        part(
            out,
            [x + width * 0.30, 1.05, front - 0.11],
            [0.10, 0.055, 0.065],
            [0.13, 0.09, 0.045],
            WOOD,
        );
    }
}

fn foundation(out: &mut Vec<Vertex>, foundation_depth: f32) {
    let w = Building::HALF_WIDTH * 40.;
    let d = Building::HALF_DEPTH * 40.;
    let front = d * 0.5;
    // A continuous foundation closes gaps behind the irregular stone courses.
    part(
        out,
        [0., (0.10 - foundation_depth) * 0.5, 0.],
        [w, foundation_depth + 0.10, d],
        [0.34, 0.30, 0.23],
        STONE,
    );
    // Terraforming can expose a deep foundation after placement. Detail only
    // the upper 1.4 m; keep the solid support below it at constant geometry cost.
    let dressed_height = (foundation_depth + 0.10).min(1.40);
    let rows = (dressed_height / 0.28).ceil().max(1.) as usize;
    let course = dressed_height / rows as f32;
    for row in 0..rows {
        let y = 0.10 - (row as f32 + 0.5) * course;
        for side in 0..4 {
            let length = if side < 2 { w } else { d - 0.20 };
            let mut a = -length * 0.5;
            let mut i = 0;
            while a < length * 0.5 - 0.01 {
                let span = (0.34 + ((i * 7 + row * 3 + side * 11) % 9) as f32 * 0.035)
                    .min(length * 0.5 - a);
                let p = a + span * 0.5;
                let shade = 0.90 + ((i + row * 3 + side) % 5) as f32 * 0.04;
                let (center, size) = if side < 2 {
                    (
                        [
                            p,
                            y,
                            if side == 0 {
                                front - 0.045
                            } else {
                                -front + 0.045
                            },
                        ],
                        [span - 0.009, course - 0.01, 0.10],
                    )
                } else {
                    (
                        [
                            if side == 2 {
                                w * 0.5 - 0.045
                            } else {
                                -w * 0.5 + 0.045
                            },
                            y,
                            p,
                        ],
                        [0.10, course - 0.01, span - 0.009],
                    )
                };
                part(
                    out,
                    center,
                    size,
                    [0.49 * shade, 0.44 * shade, 0.34 * shade],
                    STONE,
                );
                a += span;
                i += 1;
            }
        }
    }
}

fn structure(out: &mut Vec<Vertex>) {
    let w = Building::HALF_WIDTH * 40.;
    let d = Building::HALF_DEPTH * 40.;
    let h = Building::WALL_HEIGHT * 20.;
    let t = 0.30;
    let front = d * 0.5;
    for side in [-1., 1.] {
        part(
            out,
            [side * (w - t) * 0.5, h * 0.5, 0.],
            [t, h, d],
            WALL,
            PLASTER,
        );
    }
    part(
        out,
        [0., h * 0.5, -front + t * 0.5],
        [w - 2. * t, h, t],
        WALL,
        PLASTER,
    );
    // Front wall is split around openings instead of hiding them behind a slab.
    let mut left = -w * 0.5 + t;
    for (x, width, bottom, height) in [
        (-1.8, 0.50, 1.45, 0.65),
        (0., 0.85, 0., 1.95),
        (1.8, 0.50, 1.45, 0.65),
    ] {
        let edge = x - width * 0.5;
        part(
            out,
            [(left + edge) * 0.5, h * 0.5, front - t * 0.5],
            [edge - left, h, t],
            WALL,
            PLASTER,
        );
        if bottom > 0. {
            part(
                out,
                [x, bottom * 0.5, front - t * 0.5],
                [width, bottom, t],
                WALL,
                PLASTER,
            );
        }
        let top = bottom + height;
        part(
            out,
            [x, (top + h) * 0.5, front - t * 0.5],
            [width, h - top, t],
            WALL,
            PLASTER,
        );
        opening(out, x, bottom, width, height, front);
        left = x + width * 0.5;
    }
    let right = w * 0.5 - t;
    part(
        out,
        [(left + right) * 0.5, h * 0.5, front - t * 0.5],
        [right - left, h, t],
        WALL,
        PLASTER,
    );
    part(
        out,
        [0., h + 0.075, 0.],
        [w, 0.15, d],
        [0.48, 0.36, 0.24],
        EARTH,
    );
    for x in [-1., 1.] {
        part(
            out,
            [x * (w * 0.5 - 0.1), h + 0.325, 0.],
            [0.2, 0.35, d],
            WALL,
            PLASTER,
        );
    }
    for z in [-1., 1.] {
        part(
            out,
            [0., h + 0.325, z * (front - 0.1)],
            [w, 0.35, 0.2],
            WALL,
            PLASTER,
        );
    }
    for i in 0..6 {
        part(
            out,
            [-w * 0.4 + i as f32 * w * 0.16, h - 0.15, 0.],
            [0.16, 0.18, d + 0.35],
            [0.28, 0.18, 0.085],
            WOOD,
        );
    }
    part(
        out,
        [0., 0.10, front + 0.3],
        [1.15, 0.20, 0.65],
        [0.52, 0.47, 0.37],
        STONE,
    );
}

fn prototype() -> &'static occlusion::Baked {
    static PROTOTYPE: OnceLock<occlusion::Baked> = OnceLock::new();
    PROTOTYPE.get_or_init(|| {
        let mut mesh = Vec::new();
        structure(&mut mesh);
        let mut occluders = mesh.clone();
        foundation(&mut occluders, 0.10);
        occlusion::bake(&mesh, &occluders, |v| {
            if v.color[3] == PLASTER {
                // Resolved near the wall foot, rather than interpolated across
                // the entire height of a large quad.
                let stain = (1.0 - v.position[1] / 0.65).clamp(0., 1.) * 0.16;
                v.color[0] *= 1. - stain * 0.35;
                v.color[1] *= 1. - stain * 0.7;
                v.color[2] *= 1. - stain;
            }
        })
    })
}

fn local_mesh(out: &mut Vec<Vertex>, foundation_depth: f32) {
    foundation(out, foundation_depth);
    out.extend_from_slice(&prototype().vertices);
}

fn rotate_and_light(
    mesh: &mut [Vertex],
    b: &Building,
    center: Vec3,
    high: f32,
    visibility: Option<&[f32]>,
) {
    debug_assert!(visibility.is_none_or(|v| v.len() == mesh.len()));
    let (s, c) = b.rotation.sin_cos();
    let sun = Vec3::new(-0.5, 0.75, 0.3).normalize();
    for (index, tri) in mesh.chunks_exact_mut(3).enumerate() {
        // Derive normals before adding kilometre-scale world offsets. Small
        // subdivisions must not introduce quantized lighting seams on a plane.
        let [a, b, d] = [tri[0], tri[1], tri[2]].map(|v| Vec3::from_array(v.position));
        let n = (b - a).cross(d - a).normalize_or_zero();
        let normal = Vec3::new(c * n.x - s * n.z, n.y, s * n.x + c * n.z);
        let direct = 0.58 * normal.dot(sun).max(0.);
        for (corner, v) in tri.iter_mut().enumerate() {
            let [x, y, z] = v.position.map(|v| v / 20.);
            v.position = [center.x + c * x - s * z, high + y, center.z + s * x + c * z];
            let ambient = visibility.map_or(1., |v| v[index * 3 + corner]);
            // Short-range visibility affects sky fill. Direct sunlight still
            // reaches exposed faces; directional shadows remain a GPU concern.
            let shade = 0.42 * (0.35 + 0.65 * ambient) + direct;
            for channel in &mut v.color[..3] {
                *channel *= shade;
            }
        }
    }
}

pub(crate) fn support(sim: &SimState, out: &mut Vec<Vertex>, b: &Building) {
    let heights = b.footprint().map(|p| terrain::point(sim, p).y);
    let high = heights.into_iter().fold(f32::NEG_INFINITY, f32::max);
    let low = heights.into_iter().fold(f32::INFINITY, f32::min);
    let start = out.len();
    foundation(out, (high - low) * 20. + 0.10);
    rotate_and_light(
        &mut out[start..],
        b,
        terrain::point(sim, b.position),
        high,
        None,
    );
}

pub(crate) fn house(sim: &SimState, out: &mut Vec<Vertex>, b: &Building) {
    let heights = b.footprint().map(|p| terrain::point(sim, p).y);
    let high = heights.into_iter().fold(f32::NEG_INFINITY, f32::max);
    let low = heights.into_iter().fold(f32::INFINITY, f32::min);
    let start = out.len();
    local_mesh(out, (high - low) * 20. + 0.10);
    let split = out.len() - prototype().vertices.len();
    let center = terrain::point(sim, b.position);
    rotate_and_light(&mut out[start..split], b, center, high, None);
    rotate_and_light(
        &mut out[split..],
        b,
        center,
        high,
        Some(&prototype().visibility),
    );
}

/// Ambient-bake subdivisions do not change the surface. Keep picking on the
/// original cuboids so pointer motion never pays for the extra shading vertices.
pub(crate) fn picking_mesh(sim: &SimState, out: &mut Vec<Vertex>, b: &Building) {
    let heights = b.footprint().map(|p| terrain::point(sim, p).y);
    let high = heights.into_iter().fold(f32::NEG_INFINITY, f32::max);
    let low = heights.into_iter().fold(f32::INFINITY, f32::min);
    let start = out.len();
    foundation(out, (high - low) * 20. + 0.10);
    structure(out);
    rotate_and_light(
        &mut out[start..],
        b,
        terrain::point(sim, b.position),
        high,
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dwelling_mesh_is_bounded_and_openings_are_recessed() {
        for foundation in [0.10, 1.30, 25.] {
            let mut mesh = Vec::new();
            local_mesh(&mut mesh, foundation);
            eprintln!(
                "dwelling at {foundation:.2} m foundation: {} vertices",
                mesh.len()
            );
            assert!(mesh.len() < 32_000, "{} vertices", mesh.len());
            assert_eq!(mesh.len() % 3, 0);
            for tri in mesh.chunks_exact(3) {
                let p = [tri[0], tri[1], tri[2]].map(|v| Vec3::from_array(v.position));
                assert!((p[1] - p[0]).cross(p[2] - p[0]).length() > 0.000001);
            }
            assert!(
                mesh.iter()
                    .all(|v| v.position.iter().chain(&v.color).all(|v| v.is_finite()))
            );
            assert!(
                mesh.iter()
                    .all(|v| v.position[0].abs() < 3.26 && v.position[1] <= 3.301)
            );
            // A horizontal ray through the middle of the door first hits timber
            // behind the front wall plane; it must not hit a plaster slab.
            let hit = mesh
                .chunks_exact(3)
                .filter_map(|t| {
                    crate::terrain::intersect(
                        Vec3::new(0., 1., 4.),
                        -Vec3::Z,
                        [
                            Vec3::from_array(t[0].position),
                            Vec3::from_array(t[1].position),
                            Vec3::from_array(t[2].position),
                        ],
                    )
                    .map(|distance| (distance, t[0].color[3]))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .expect("closed door");
            assert_eq!(hit.1, WOOD);
            assert!(4. - hit.0 < 2.26);
        }
    }

    #[test]
    fn rotated_faces_receive_world_space_sunlight() {
        let b = Building {
            id: 0,
            position: genesis_sim_core::spatial::Position::new(0., 0.),
            rotation: std::f32::consts::FRAC_PI_2,
        };
        let mut mesh = Vec::new();
        part(&mut mesh, [0., 1., 0.], [1., 1., 1.], [1.; 3], STONE);
        let mut occluded = mesh.clone();
        let visibility = vec![0.; occluded.len()];
        rotate_and_light(&mut occluded, &b, Vec3::ZERO, 0., Some(&visibility));
        rotate_and_light(&mut mesh, &b, Vec3::ZERO, 0., None);
        // +Z face rotates to -X, which faces the sun; -Z rotates away from it.
        assert!(mesh[6].color[0] > mesh[0].color[0] + 0.20);
        assert!((mesh[0].color[0] - 0.42).abs() < 1e-5);
        for (open, closed) in mesh.iter().zip(occluded) {
            // Only the ambient contribution changes, regardless of face/sun angle.
            assert!((open.color[0] - closed.color[0] - 0.42 * 0.65).abs() < 1e-5);
        }
    }

    #[test]
    fn shading_subdivision_preserves_the_picking_surface() {
        let sim = genesis_sim_core::scenarios::coastal_outpost();
        let mut b = sim.buildings[0].clone();
        for angle in [0., 0.7] {
            b.rotation = angle;
            let mut visible = Vec::new();
            let mut pick = Vec::new();
            house(&sim, &mut visible, &b);
            picking_mesh(&sim, &mut pick, &b);
            assert!(pick.len() < 16_000);
            let center = terrain::point(&sim, b.position);
            for i in 0..24 {
                let a = i as f32 * std::f32::consts::TAU / 24.;
                let origin = center + Vec3::new(a.cos() * 0.5, 0.09, a.sin() * 0.5);
                for offset in [
                    Vec3::new(0., 0.075, 0.),
                    Vec3::new(0.08, 0.09, 0.10),
                    Vec3::new(0., 0.16, 0.),
                ] {
                    let direction = (center + offset - origin).normalize();
                    let hit = |mesh: &[Vertex]| {
                        mesh.chunks_exact(3)
                            .filter_map(|tri| {
                                terrain::intersect(
                                    origin,
                                    direction,
                                    [tri[0], tri[1], tri[2]].map(|v| Vec3::from_array(v.position)),
                                )
                            })
                            .min_by(f32::total_cmp)
                    };
                    match (hit(&visible), hit(&pick)) {
                        (Some(a), Some(b)) => assert!((a - b).abs() < 1e-5),
                        (None, None) => {}
                        _ => panic!("shading changed the dwelling silhouette"),
                    }
                }
            }
        }
    }
}
