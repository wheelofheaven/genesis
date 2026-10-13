//! Frozen pre-cache extractor used as an independent bitwise regression oracle.
//! Coarse visual continuation of a continental region. Only the central window
//! is simulated and selectable; the context uses the same procedural source.
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{
    SimState,
    spatial::{self, Position},
    worldgen::{Region, relief},
};
use glam::Vec3;

#[derive(Default)]
pub struct Cache {
    ground: Vec<Vertex>,
    water: Vec<Vertex>,
}
impl Cache {
    pub fn new(sim: &SimState) -> Self {
        let mut result = Self::default();
        if sim.geography.region == Region::Island {
            return result;
        }
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
                        vertices.push(Vertex {
                            position: [0.; 3],
                            color: [0.; 4],
                        });
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
                    vertices.push(crate::terrain::colored_vertex(
                        sim,
                        Position::new(x, z),
                        Vec3::new(x - w as f32 * 0.5, y, z - h as f32 * 0.5),
                        normal,
                        0.,
                        Default::default(),
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
                        let mut center = corners[0];
                        center.position = std::array::from_fn(|axis| {
                            corners.iter().map(|v| v.position[axis]).sum::<f32>() * 0.25
                        });
                        center.color = std::array::from_fn(|axis| {
                            corners.iter().map(|v| v.color[axis]).sum::<f32>() * 0.25
                        });
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
                            let at = |n: i32| {
                                let t = n as f32 / steps as f32;
                                let mut v = a;
                                v.position = std::array::from_fn(|axis| {
                                    a.position[axis] * (1. - t) + b.position[axis] * t
                                });
                                v.color = std::array::from_fn(|axis| {
                                    a.color[axis] * (1. - t) + b.color[axis] * t
                                });
                                if boundary {
                                    v.position[1] =
                                        height(ax * (1. - t) + bx * t, az * (1. - t) + bz * t);
                                }
                                v
                            };
                            for n in 0..steps {
                                result.triangle([center, at(n), at(n + 1)], sim.sea_level);
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
    fn triangle(&mut self, vertices: [Vertex; 3], sea: f32) {
        self.ground.extend_from_slice(&vertices);
        if vertices.iter().any(|v| v.position[1] < sea) {
            for mut v in vertices {
                v.color = [
                    0.24,
                    0.32,
                    0.24,
                    -(1. + (sea - v.position[1]).max(0.) * 20.),
                ];
                v.position[1] = sea;
                self.water.push(v);
            }
        }
    }
    pub fn append(&self, sim: &SimState, out: &mut Vec<Vertex>, layers: crate::survey::Survey) {
        for &v in &self.ground {
            let mut v = v;
            if layers.seafloor && v.position[1] < sim.sea_level {
                let depth = (sim.sea_level - v.position[1]) * 20.;
                let shade = (v.color[0] / 0.55).clamp(0.35, 1.);
                let c = Vec3::new(0.31, 0.45, 0.42)
                    .lerp(Vec3::new(0.075, 0.19, 0.27), (depth / 500.).clamp(0., 1.))
                    * shade;
                v.color = [c.x, c.y, c.z, 4.];
            }
            out.push(v);
        }
        if !layers.seafloor {
            out.extend_from_slice(&self.water);
        }
    }
}
