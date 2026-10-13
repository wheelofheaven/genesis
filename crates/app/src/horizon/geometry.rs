//! Immutable visual continuation geometry with independently refreshed palettes.
//! Only the central region is simulated and selectable.
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{
    SimState,
    spatial::{self, Position},
    worldgen::{Region, relief},
};
use glam::Vec3;

#[derive(Clone, Copy)]
pub(super) enum Paint {
    Empty,
    Sample(Position, Vec3),
    Average([usize; 4]),
    Lerp(usize, usize, f32),
}
pub(super) struct Node {
    pub position: [f32; 3],
    pub paint: Paint,
}
#[derive(Clone, Copy)]
struct Point {
    position: [f32; 3],
    id: usize,
}
impl Node {
    fn point(&self, id: usize) -> Point {
        Point {
            position: self.position,
            id,
        }
    }
}
#[derive(Default)]
pub(super) struct Geometry {
    pub nodes: Vec<Node>,
    pub triangles: Vec<[usize; 3]>,
    pub water: Vec<Vertex>,
}
impl Geometry {
    fn node(&mut self, position: [f32; 3], paint: Paint) -> Point {
        let id = self.nodes.len();
        self.nodes.push(Node { position, paint });
        self.nodes[id].point(id)
    }
    fn triangle(&mut self, points: [Point; 3], sea: f32) {
        self.triangles.push(points.map(|p| p.id));
        if points.iter().any(|p| p.position[1] < sea) {
            self.water.extend(points.map(|p| Vertex {
                position: [p.position[0], sea, p.position[2]],
                color: [
                    0.24,
                    0.32,
                    0.24,
                    -(1. + (sea - p.position[1]).max(0.) * 20.),
                ],
            }));
        }
    }
    pub(super) fn new(sim: &SimState) -> Self {
        let mut result = Self::default();
        if sim.geography.region == Region::Island {
            return result;
        }
        result.node([0.; 3], Paint::Empty);
        let w = sim.grid.width as i32;
        let h = sim.grid.height as i32;
        let height = |x: f32, z: f32| {
            let raw = relief::regional_sample(sim.config.seed, sim.geography.region, x, z).0;
            let distance = (-x).max(x - w as f32).max(-z).max(z - h as f32).max(0.);
            let blend = (distance / 8.).clamp(0., 1.);
            let edge = spatial::elevation(&sim.grid, Position::new(x, z));
            edge * (1. - blend) + raw * blend
        };
        for (inner, outer, step, fine_step) in [(0, 8, 1, 1), (8, 192, 4, 1), (192, 768, 16, 4)] {
            let width = ((w + outer * 2) / step + 1) as usize;
            let rows = ((h + outer * 2) / step + 1) as usize;
            let mut vertices = Vec::with_capacity(width * rows);
            for row in 0..rows {
                for col in 0..width {
                    let x = (col as i32 * step - outer) as f32;
                    let z = (row as i32 * step - outer) as f32;
                    // The active field (or finer ring) already owns this hole.
                    // Reserve indexing space without resampling its interior.
                    if x > -inner as f32
                        && x < (w + inner) as f32
                        && z > -inner as f32
                        && z < (h + inner) as f32
                    {
                        vertices.push(result.nodes[0].point(0));
                        continue;
                    }
                    let y = height(x, z);
                    let normal = Vec3::new(
                        height(x - 0.5, z) - height(x + 0.5, z),
                        1.,
                        height(x, z - 0.5) - height(x, z + 0.5),
                    )
                    .normalize();
                    // Continue the edge's environmental palette into the visual
                    // context. Bounded fields clamp at the region edge; this is
                    // extrapolated scenery, not simulated off-map ecology.
                    vertices.push(result.node(
                        [x - w as f32 * 0.5, y, z - h as f32 * 0.5],
                        Paint::Sample(Position::new(x, z), normal),
                    ));
                }
            }
            for row in 0..rows - 1 {
                for col in 0..width - 1 {
                    let x = col as i32 * step - outer;
                    let z = row as i32 * step - outer;
                    if x >= -inner && x < w + inner && z >= -inner && z < h + inner {
                        continue;
                    }
                    let i = row * width + col;
                    let touches_inner = step > fine_step
                        && (((x + step == -inner || x == w + inner)
                            && z >= -inner
                            && z < h + inner)
                            || ((z + step == -inner || z == h + inner)
                                && x >= -inner
                                && x < w + inner));
                    if touches_inner {
                        // Stitch every fine boundary sample into a coarse fan;
                        // this closes T-junction cracks without vertical skirts.
                        let corners = [
                            vertices[i],
                            vertices[i + width],
                            vertices[i + width + 1],
                            vertices[i + 1],
                        ];
                        let center = result.node(
                            std::array::from_fn(|axis| {
                                corners.iter().map(|v| v.position[axis]).sum::<f32>() * 0.25
                            }),
                            Paint::Average(corners.map(|p| p.id)),
                        );
                        for edge in 0..4 {
                            let a = corners[edge];
                            let b = corners[(edge + 1) % 4];
                            let ax = a.position[0] + w as f32 * 0.5;
                            let az = a.position[2] + h as f32 * 0.5;
                            let bx = b.position[0] + w as f32 * 0.5;
                            let bz = b.position[2] + h as f32 * 0.5;
                            let boundary = (ax == bx
                                && (ax == -inner as f32 || ax == (w + inner) as f32))
                                || (az == bz && (az == -inner as f32 || az == (h + inner) as f32));
                            let steps = if boundary { step / fine_step } else { 1 };
                            let mut at = |n: i32| {
                                let t = n as f32 / steps as f32;
                                let mut v = a;
                                v.position = std::array::from_fn(|axis| {
                                    a.position[axis] * (1. - t) + b.position[axis] * t
                                });

                                if boundary {
                                    v.position[1] =
                                        height(ax * (1. - t) + bx * t, az * (1. - t) + bz * t);
                                }
                                result.node(v.position, Paint::Lerp(a.id, b.id, t))
                            };
                            let edges: Vec<_> =
                                (0..steps).map(|n| [center, at(n), at(n + 1)]).collect();
                            for tri in edges {
                                result.triangle(tri, sim.sea_level);
                            }
                        }
                    } else {
                        for ids in [[i, i + width, i + 1], [i + 1, i + width, i + width + 1]] {
                            result.triangle(ids.map(|j| vertices[j]), sim.sea_level);
                        }
                    }
                }
            }
        }
        result
    }
}
