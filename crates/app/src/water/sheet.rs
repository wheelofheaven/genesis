//! One projected water sheet per terrain receiver. Unioning ribbon footprints
//! removes duplicate surfaces; shared receiver corners give all fragments the
//! same height, slope and bed material at a junction (including LOD edges).
use super::{slopes::Slopes, surface};
use crate::terrain;
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{SimState, spatial::Position, subsurface::Rock, units::metres};
use glam::{DVec2, Vec2, Vec3, Vec3Swizzles};
use std::collections::{BTreeMap, HashMap};

type Polygon = Vec<DVec2>;
const AREA_EPSILON: f64 = 1e-12;

struct Patch {
    ground: [Vec3; 3],
    pieces: Vec<Polygon>,
}

#[derive(Default)]
pub(super) struct Sheet {
    patches: BTreeMap<usize, Patch>,
}

fn area(poly: &[DVec2]) -> f64 {
    let Some(&a) = poly.first() else {
        return 0.;
    };
    (1..poly.len().saturating_sub(1))
        .map(|i| (poly[i] - a).perp_dot(poly[i + 1] - a).abs() * 0.5)
        .sum()
}

fn clip(poly: &[DVec2], distance: impl Fn(DVec2) -> f64) -> Polygon {
    let Some(&last) = poly.last() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut a = last;
    let mut da = distance(a);
    for &b in poly {
        let db = distance(b);
        if (da >= 0.) != (db >= 0.) {
            out.push(a.lerp(b, da / (da - db)));
        }
        if db >= 0. {
            out.push(b);
        }
        a = b;
        da = db;
    }
    out
}

fn bounds(poly: &[DVec2]) -> (DVec2, DVec2) {
    poly.iter().fold(
        (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY)),
        |(min, max), &p| (min.min(p), max.max(p)),
    )
}

/// Subtract one convex clockwise footprint, emitting disjoint convex pieces.
fn subtract(mut poly: Polygon, occupied: &[DVec2]) -> Vec<Polygon> {
    let (a0, a1) = bounds(&poly);
    let (b0, b1) = bounds(occupied);
    if a1.x <= b0.x || b1.x <= a0.x || a1.y <= b0.y || b1.y <= a0.y {
        return vec![poly];
    }
    // Touching neighbours need no subdivision. Without this intersection test,
    // extending every occupied edge's line fragments unrelated pieces repeatedly.
    let mut overlap = poly.clone();
    for i in 0..occupied.len() {
        let a = occupied[i];
        let d = occupied[(i + 1) % occupied.len()] - a;
        overlap = clip(&overlap, |p| (p - a).perp_dot(d));
        if area(&overlap) <= AREA_EPSILON {
            return vec![poly];
        }
    }
    let mut result = Vec::new();
    for i in 0..occupied.len() {
        let a = occupied[i];
        let d = occupied[(i + 1) % occupied.len()] - a;
        let distance = |p: DVec2| (p - a).perp_dot(d);
        let outside = clip(&poly, |p| -distance(p));
        if area(&outside) > AREA_EPSILON {
            result.push(outside);
        }
        poly = clip(&poly, distance);
        if area(&poly) <= AREA_EPSILON {
            break;
        }
    }
    result
}

fn insert(pieces: &mut Vec<Polygon>, polygon: Polygon) {
    let mut remainder = vec![polygon];
    for existing in pieces.iter() {
        remainder = remainder
            .into_iter()
            .flat_map(|p| subtract(p, existing))
            .collect();
        if remainder.is_empty() {
            return;
        }
    }
    pieces.extend(remainder);
}

impl Sheet {
    pub fn add(&mut self, sim: &SimState, cache: &terrain::Cache, triangle: [Vertex; 3]) {
        let mut footprint =
            triangle.map(|v| DVec2::new(v.position[0] as f64, v.position[2] as f64));
        if (footprint[1] - footprint[0]).perp_dot(footprint[2] - footprint[0]) > 0. {
            footprint.swap(1, 2);
        }
        let (min, max) = bounds(&footprint);
        let position = |p: DVec2| {
            Position::new(
                p.x as f32 + sim.grid.width as f32 * 0.5,
                p.y as f32 + sim.grid.height as f32 * 0.5,
            )
        };
        cache.indexed_triangles_in(position(min), position(max), |id, ground| {
            let mut polygon = footprint.to_vec();
            for i in 0..3 {
                let a = ground[i].xz().as_dvec2();
                let d = ground[(i + 1) % 3].xz().as_dvec2() - a;
                polygon = clip(&polygon, |p| (p - a).perp_dot(d));
            }
            if area(&polygon) > AREA_EPSILON {
                insert(
                    &mut self
                        .patches
                        .entry(id)
                        .or_insert_with(|| Patch {
                            ground,
                            pieces: Vec::new(),
                        })
                        .pieces,
                    polygon,
                );
            }
        });
    }

    pub fn finish(
        self,
        sim: &SimState,
        field: &Slopes,
        out: &mut Vec<Vertex>,
        simplification: impl Fn(Position) -> f32,
        mut channel_depth: impl FnMut(Position) -> f32,
    ) {
        let offset = Vec2::new(sim.grid.width as f32 * 0.5, sim.grid.height as f32 * 0.5);
        let position = |p: Vec2| Position::new(p.x + offset.x, p.y + offset.y);
        let mut corners = HashMap::new();
        for patch in self.patches.into_values() {
            let water = patch.ground.map(|v| {
                *corners
                    .entry([v.x.to_bits(), v.z.to_bits()])
                    .or_insert_with(|| {
                        let p = position(v.xz());
                        let (level, slope) = field.sample(v.xz() + offset);
                        let level = level.unwrap_or(v.y - metres(0.25));
                        let simplify = simplification(p);
                        let height = level + (v.y + metres(0.04) - level) * simplify;
                        let material = match sim.subsurface.reading(p).rock {
                            Rock::Limestone => -1.,
                            Rock::Sandstone => -2.,
                            Rock::Basalt => -3.,
                        };
                        Vertex {
                            position: [v.x, height, v.z],
                            color: [material, slope.x, slope.y, -(1. + (height - v.y) * 20.)],
                        }
                    })
            });
            let a = patch.ground[0].xz().as_dvec2();
            let u = patch.ground[1].xz().as_dvec2() - a;
            let v = patch.ground[2].xz().as_dvec2() - a;
            let determinant = u.perp_dot(v);
            for polygon in patch.pieces {
                let face: Vec<_> = polygon
                    .into_iter()
                    .map(|p| {
                        let delta = p - a;
                        let b = delta.perp_dot(v) / determinant;
                        let c = u.perp_dot(delta) / determinant;
                        let weights = [(1. - b - c) as f32, b as f32, c as f32];
                        Vertex {
                            position: [
                                p.x as f32,
                                (0..3).map(|i| water[i].position[1] * weights[i]).sum(),
                                p.y as f32,
                            ],
                            color: std::array::from_fn(|j| {
                                (0..3).map(|i| water[i].color[j] * weights[i]).sum()
                            }),
                        }
                    })
                    .collect();
                let mut face = surface::clip(&face, |v| -v.color[3] - 1.);
                for vertex in &mut face {
                    let p = position(Vec2::new(vertex.position[0], vertex.position[2]));
                    let simplify = simplification(p);
                    if simplify > 0. {
                        let depth = (-vertex.color[3] - 1.) / 20.;
                        vertex.color[3] =
                            -(1. + (depth + (channel_depth(p) - depth) * simplify).max(0.) * 20.);
                    }
                }
                for i in 1..face.len().saturating_sub(1) {
                    let tri = [face[0], face[i], face[i + 1]];
                    let [a, b, c] = tri.map(|v| Vec3::from_array(v.position));
                    if (b - a).cross(c - a).y > 1e-12 {
                        out.extend(tri);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn touching_diagonal_pieces_are_not_fragmented() {
        let ring: Vec<_> = (0..32)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::TAU / 32.;
                DVec2::new(angle.cos(), angle.sin())
            })
            .collect();
        let mut pieces = Vec::new();
        for i in 0..32 {
            insert(&mut pieces, vec![DVec2::ZERO, ring[(i + 1) % 32], ring[i]]);
        }
        assert_eq!(
            pieces.len(),
            32,
            "touching fans should retain their original triangles"
        );
        assert!(pieces.iter().all(|p| p.len() == 3));
    }

    fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Polygon {
        vec![
            DVec2::new(x, y),
            DVec2::new(x, y + h),
            DVec2::new(x + w, y + h),
            DVec2::new(x + w, y),
        ]
    }
    #[test]
    fn union_preserves_area_without_overlap_for_crossings_duplicates_and_shared_edges() {
        for offset in [DVec2::ZERO, DVec2::new(190., -159.)] {
            let mut pieces = Vec::new();
            for polygon in [
                rectangle(0., 0., 3., 1.),
                rectangle(1., -1., 1., 3.),
                rectangle(0., 0., 3., 1.),
                rectangle(3., 0., 1., 1.),
            ] {
                insert(
                    &mut pieces,
                    polygon.into_iter().map(|p| p + offset).collect(),
                );
            }
            assert!((pieces.iter().map(|p| area(p)).sum::<f64>() - 6.).abs() < 1e-9);
            for (i, a) in pieces.iter().enumerate() {
                for b in &pieces[i + 1..] {
                    let mut overlap = a.clone();
                    for j in 0..b.len() {
                        let origin = b[j];
                        let d = b[(j + 1) % b.len()] - origin;
                        overlap = clip(&overlap, |p| (p - origin).perp_dot(d));
                    }
                    assert!(area(&overlap) < 1e-10);
                }
            }
        }
    }
}
