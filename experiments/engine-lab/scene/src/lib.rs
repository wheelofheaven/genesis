//! Shared static fixture, not gameplay or a social simulation.
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 640;
pub const BACKGROUND: [f32; 4] = [0.72, 0.80, 0.83, 1.0];
pub const EYE: [f32; 3] = [24.0, 22.0, 28.0];
pub const TARGET: [f32; 3] = [0.0, 0.0, 0.0];
pub const VIEW_HEIGHT: f32 = 30.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

pub fn view_projection(aspect: f32) -> Mat4 {
    let half = VIEW_HEIGHT / 2.0;
    Mat4::orthographic_rh(-half * aspect, half * aspect, -half, half, 0.1, 100.0)
        * Mat4::look_at_rh(Vec3::from(EYE), Vec3::from(TARGET), Vec3::Y)
}

/// A single vertex-colored mesh shared by both renderers. Face shading is baked
/// to avoid comparing different lighting algorithms. Colors are linear RGB.
pub fn settlement() -> Vec<Vertex> {
    let mut vertices = Vec::new();
    cuboid(
        &mut vertices,
        [0.0, -0.4, 0.0],
        [24.0, 0.8, 20.0],
        [0.68, 0.49, 0.28],
    );
    cuboid(
        &mut vertices,
        [0.0, 0.015, 0.0],
        [2.3, 0.03, 19.8],
        [0.83, 0.66, 0.43],
    );
    for z in [-6.0, 0.0, 6.0] {
        for x in [-7.0, -3.8, 4.0, 7.8] {
            let h = if x > 0.0 { 2.0 } else { 2.8 };
            cuboid(
                &mut vertices,
                [x, h / 2.0, z],
                [2.5, h, 3.0],
                [0.82, 0.70, 0.50],
            );
            cuboid(
                &mut vertices,
                [x, h + 0.12, z],
                [2.7, 0.24, 3.2],
                [0.46, 0.27, 0.19],
            );
        }
    }
    // Inhabitants are geometry placeholders; no animation or agent AI.
    for i in 0..48 {
        let x = ((i * 17 % 41) as f32 / 41.0 - 0.5) * 2.0;
        let z = (i as f32 / 48.0 - 0.5) * 17.0;
        let cloth = if i % 3 == 0 {
            [0.12, 0.29, 0.35]
        } else {
            [0.65, 0.28, 0.17]
        };
        cuboid(&mut vertices, [x, 0.38, z], [0.25, 0.65, 0.25], cloth);
        cuboid(
            &mut vertices,
            [x, 0.82, z],
            [0.23, 0.23, 0.23],
            [0.56, 0.34, 0.20],
        );
    }
    vertices
}

fn cuboid(out: &mut Vec<Vertex>, center: [f32; 3], size: [f32; 3], color: [f32; 3]) {
    let corners = [
        [-1., -1., -1.],
        [1., -1., -1.],
        [1., 1., -1.],
        [-1., 1., -1.],
        [-1., -1., 1.],
        [1., -1., 1.],
        [1., 1., 1.],
        [-1., 1., 1.],
    ];
    let faces = [
        ([0, 3, 2, 1], 0.72),
        ([4, 5, 6, 7], 0.86),
        ([0, 4, 7, 3], 0.65),
        ([1, 2, 6, 5], 0.8),
        ([3, 7, 6, 2], 1.0),
        ([0, 1, 5, 4], 0.5),
    ];
    for (face, shade) in faces {
        for n in [0, 1, 2, 0, 2, 3] {
            let corner = corners[face[n]];
            out.push(Vertex {
                position: std::array::from_fn(|axis| {
                    center[axis] + corner[axis] * size[axis] * 0.5
                }),
                color: [color[0] * shade, color[1] * shade, color[2] * shade, 1.0],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_fits_shared_camera_and_has_valid_colors() {
        let vertices = settlement();
        assert_eq!(vertices.len(), 4392);
        let camera = view_projection(WIDTH as f32 / HEIGHT as f32);
        for vertex in vertices {
            let p = camera.project_point3(Vec3::from(vertex.position));
            assert!(p.is_finite());
            assert!(p.x.abs() <= 1.0 && p.y.abs() <= 1.0);
            assert!((0.0..=1.0).contains(&p.z));
            assert!(
                vertex
                    .color
                    .iter()
                    .all(|c| c.is_finite() && (0.0..=1.0).contains(c))
            );
        }
    }

    #[test]
    fn cuboid_faces_point_outward_for_backface_culling() {
        let mut vertices = Vec::new();
        cuboid(&mut vertices, [0.; 3], [2.; 3], [1.; 3]);
        for triangle in vertices.chunks_exact(3) {
            let a = Vec3::from(triangle[0].position);
            let b = Vec3::from(triangle[1].position);
            let c = Vec3::from(triangle[2].position);
            assert!((b - a).cross(c - a).dot((a + b + c) / 3.0) > 0.0);
        }
    }
}
