//! One-time, metre-space ambient visibility bake for the procedural box kit.
//! Short rays resolve joints that are smaller than a directional shadow texel.
use genesis_renderer::geometry::Vertex;
use glam::{Vec3, Vec4};

const REACH: f32 = 0.65;
const SPACING: f32 = 0.18;
const SAMPLES: usize = 32;
const BIAS: f32 = 0.0005;

pub(super) struct Baked {
    pub vertices: Vec<Vertex>,
    pub visibility: Vec<f32>,
}

#[derive(Clone, Copy)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl Bounds {
    fn of(vertices: &[Vertex]) -> Self {
        let mut bounds = Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        };
        for vertex in vertices {
            let p = Vec3::from_array(vertex.position);
            bounds.min = bounds.min.min(p);
            bounds.max = bounds.max.max(p);
        }
        bounds
    }

    fn near(self, other: Self) -> bool {
        (self.min - other.max)
            .max(other.min - self.max)
            .max(Vec3::ZERO)
            .length_squared()
            < REACH * REACH
    }

    fn hit(self, origin: Vec3, direction: Vec3) -> Option<f32> {
        let mut enter: f32 = 0.;
        let mut exit = REACH;
        for axis in 0..3 {
            if direction[axis].abs() < 1e-7 {
                if origin[axis] < self.min[axis] || origin[axis] > self.max[axis] {
                    return None;
                }
            } else {
                let a = (self.min[axis] - origin[axis]) / direction[axis];
                let b = (self.max[axis] - origin[axis]) / direction[axis];
                enter = enter.max(a.min(b));
                exit = exit.min(a.max(b));
                if exit < enter {
                    return None;
                }
            }
        }
        Some(enter)
    }
}

fn visibility(p: Vec3, normal: Vec3, rays: &[Vec3], bounds: &[Bounds]) -> f32 {
    let origin = p + normal * BIAS;
    let point = Bounds {
        min: origin,
        max: origin,
    };
    let mut distances = [REACH; SAMPLES];
    for bound in bounds.iter().filter(|b| b.near(point)) {
        for (distance, &ray) in distances.iter_mut().zip(rays) {
            if let Some(hit) = bound.hit(origin, ray) {
                *distance = distance.min(hit);
            }
        }
    }
    // Cosine-distributed rays already integrate diffuse hemisphere weighting.
    // A quadratic distance falloff prevents a hard halo at the reach limit.
    1. - distances
        .iter()
        .map(|d| (1. - d / REACH).powi(2))
        .sum::<f32>()
        / SAMPLES as f32
}

// Retain detail only where interpolating a larger planar rectangle would lose
// contact shading. The error bound applies to visibility and weathered albedo.
fn emit(grid: &[(Vertex, f32)], stride: usize, rect: [usize; 4], out: &mut Baked) {
    let [x0, y0, x1, y1] = rect;
    let corners = [
        y0 * stride + x0,
        y0 * stride + x1,
        y1 * stride + x1,
        y1 * stride + x0,
    ];
    let mut flat = true;
    'samples: for y in y0..=y1 {
        for x in x0..=x1 {
            let u = (x - x0) as f32 / (x1 - x0) as f32;
            let v = (y - y0) as f32 / (y1 - y0) as f32;
            let weights = if u >= v {
                [1. - u, u - v, v, 0.]
            } else {
                [1. - v, 0., u, v - u]
            };
            let mut visibility = 0.;
            let mut color = Vec4::ZERO;
            for (corner, weight) in corners.into_iter().zip(weights) {
                visibility += grid[corner].1 * weight;
                color += Vec4::from_array(grid[corner].0.color) * weight;
            }
            let sample = grid[y * stride + x];
            if (visibility - sample.1).abs() > 0.025
                || (color - Vec4::from_array(sample.0.color))
                    .abs()
                    .max_element()
                    > 0.004
            {
                flat = false;
                break 'samples;
            }
        }
    }
    if flat || (x1 - x0 == 1 && y1 - y0 == 1) {
        for corner in [0, 1, 2, 0, 2, 3] {
            out.vertices.push(grid[corners[corner]].0);
            out.visibility.push(grid[corners[corner]].1);
        }
    } else if x1 - x0 >= y1 - y0 && x1 - x0 > 1 {
        let mid = (x0 + x1) / 2;
        emit(grid, stride, [x0, y0, mid, y1], out);
        emit(grid, stride, [mid, y0, x1, y1], out);
    } else {
        let mid = (y0 + y1) / 2;
        emit(grid, stride, [x0, y0, x1, mid], out);
        emit(grid, stride, [x0, mid, x1, y1], out);
    }
}

pub(super) fn bake(mesh: &[Vertex], occluders: &[Vertex], finish: impl Fn(&mut Vertex)) -> Baked {
    debug_assert_eq!(
        mesh.len() % 36,
        0,
        "baker accepts the axis-aligned cuboid kit"
    );
    debug_assert_eq!(occluders.len() % 36, 0);
    let boxes: Vec<_> = occluders.chunks_exact(36).map(Bounds::of).collect();
    let mut result = Baked {
        vertices: Vec::new(),
        visibility: Vec::new(),
    };
    for face in mesh.chunks_exact(6) {
        let origin = Vec3::from_array(face[0].position);
        let u = Vec3::from_array(face[1].position) - origin;
        let v = Vec3::from_array(face[5].position) - origin;
        let normal = u.cross(v).normalize();
        let tangent = u.normalize();
        let bitangent = normal.cross(tangent);
        let rays: Vec<_> = (0..SAMPLES)
            .map(|i| {
                let radius = ((i as f32 + 0.5) / SAMPLES as f32).sqrt();
                let angle = i as f32 * 2.399_963_1;
                tangent * (radius * angle.cos())
                    + bitangent * (radius * angle.sin())
                    + normal * (1. - radius * radius).sqrt()
            })
            .collect();
        let bounds = Bounds::of(face);
        let nearby: Vec<_> = boxes.iter().copied().filter(|b| b.near(bounds)).collect();
        let nx = (u.length() / SPACING).ceil().max(1.) as usize;
        let ny = (v.length() / SPACING).ceil().max(1.) as usize;
        let mut grid = Vec::with_capacity((nx + 1) * (ny + 1));
        for y in 0..=ny {
            for x in 0..=nx {
                let p = origin + u * (x as f32 / nx as f32) + v * (y as f32 / ny as f32);
                let mut vertex = Vertex {
                    position: p.to_array(),
                    color: face[0].color,
                };
                finish(&mut vertex);
                grid.push((vertex, visibility(p, normal, &rays, &nearby)));
            }
        }
        emit(&grid, nx + 1, [0, 0, nx, ny], &mut result);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_renderer::geometry::cuboid;

    #[test]
    fn short_rays_handle_parallel_axes_and_reject_distant_geometry() {
        let b = Bounds {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        };
        let hit = b
            .hit(Vec3::new(0.5, 0.5, 1.2), -Vec3::Z)
            .expect("near face");
        assert!((hit - 0.2).abs() < 1e-6);
        assert_eq!(b.hit(Vec3::new(0.5, 0.5, 2.), -Vec3::Z), None);
        assert_eq!(b.hit(Vec3::new(2., 0.5, 1.2), -Vec3::Z), None);
        assert_eq!(b.hit(Vec3::new(0.5, 0.5, 1.2), Vec3::Z), None);
    }

    #[test]
    fn isolated_box_stays_open_but_a_nearby_wall_occludes_its_base() {
        let mut floor = Vec::new();
        cuboid(&mut floor, [0., -0.1, 0.], [2., 0.2, 2.], [1.; 3]);
        let open = bake(&floor, &floor, |_| {});
        assert_eq!(
            open.vertices.len(),
            floor.len(),
            "unoccluded planes stay coarse"
        );
        assert!(open.visibility.iter().all(|v| (*v - 1.).abs() < 1e-6));
        let mut scene = floor.clone();
        cuboid(&mut scene, [-0.9, 0.5, 0.], [0.2, 1., 2.], [1.; 3]);
        let shaded = bake(&floor, &scene, |_| {});
        let top = |x: f32| {
            shaded
                .vertices
                .iter()
                .zip(&shaded.visibility)
                .filter(|(v, _)| v.position[1].abs() < 1e-6 && (v.position[0] - x).abs() < 0.15)
                .map(|(_, &visibility)| visibility)
                .fold(1., f32::min)
        };
        assert!(
            top(-0.75) < 0.85,
            "adjacent wall must shade the receiver: {}",
            top(-0.75)
        );
        assert_eq!(top(0.8), 1., "distant surfaces must remain open");
        assert!(
            shaded
                .visibility
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
        );
        for tri in shaded.vertices.chunks_exact(3) {
            let p = [tri[0], tri[1], tri[2]].map(|v| Vec3::from_array(v.position));
            assert!((p[1] - p[0]).cross(p[2] - p[0]).length() > 1e-8);
        }
    }
}
