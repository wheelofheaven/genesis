use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Face definition for cube-sphere generation.
struct Face {
    right: Vec3,
    up: Vec3,
    normal: Vec3,
}

const FACES: [Face; 6] = [
    // +X: right=-Z, up=+Y
    Face {
        right: Vec3::NEG_Z,
        up: Vec3::Y,
        normal: Vec3::X,
    },
    // -X: right=+Z, up=+Y
    Face {
        right: Vec3::Z,
        up: Vec3::Y,
        normal: Vec3::NEG_X,
    },
    // +Y: right=+X, up=-Z
    Face {
        right: Vec3::X,
        up: Vec3::NEG_Z,
        normal: Vec3::Y,
    },
    // -Y: right=+X, up=+Z
    Face {
        right: Vec3::X,
        up: Vec3::Z,
        normal: Vec3::NEG_Y,
    },
    // +Z: right=+X, up=+Y
    Face {
        right: Vec3::X,
        up: Vec3::Y,
        normal: Vec3::Z,
    },
    // -Z: right=-X, up=+Y
    Face {
        right: Vec3::NEG_X,
        up: Vec3::Y,
        normal: Vec3::NEG_Z,
    },
];

/// Cube-sphere mesh generator.
///
/// Subdivides each face of a cube into a `resolution x resolution` grid,
/// then normalizes vertices onto a sphere.
pub struct CubeSphere {
    resolution: u32,
}

impl CubeSphere {
    pub fn new(resolution: u32) -> Self {
        Self { resolution }
    }

    /// Build a Bevy `Mesh` with the given radius and heightmap function.
    ///
    /// `heightmap_fn` takes a unit sphere point and returns a height offset.
    pub fn build_mesh(
        &self,
        radius: f32,
        heightmap_fn: impl Fn(Vec3) -> f32,
    ) -> Mesh {
        let res = self.resolution;
        let verts_per_face = (res + 1) * (res + 1);
        let total_verts = 6 * verts_per_face as usize;
        let total_indices = 6 * (res * res * 6) as usize;

        let mut positions = Vec::with_capacity(total_verts);
        let mut normals = Vec::with_capacity(total_verts);
        let mut uvs = Vec::with_capacity(total_verts);
        let mut colors = Vec::with_capacity(total_verts);
        let mut indices = Vec::with_capacity(total_indices);

        for (face_idx, face) in FACES.iter().enumerate() {
            let base = face_idx as u32 * verts_per_face;

            // Generate vertices for this face.
            for y in 0..=res {
                for x in 0..=res {
                    let fx = (x as f32 / res as f32) * 2.0 - 1.0;
                    let fy = (y as f32 / res as f32) * 2.0 - 1.0;

                    let cube_point = face.normal + face.right * fx + face.up * fy;
                    let sphere_point = cube_point.normalize();

                    let height = heightmap_fn(sphere_point);
                    let position = sphere_point * (radius + height);

                    positions.push(position.to_array());
                    normals.push(sphere_point.to_array());

                    // Face-local UVs in [0,1].
                    uvs.push([x as f32 / res as f32, y as f32 / res as f32]);

                    // Vertex color: subtle blue shift based on height.
                    // Lower = deeper blue, higher = slightly lighter.
                    let t = (height / radius).clamp(-0.02, 0.02) * 25.0 + 0.5;
                    colors.push([
                        0.05 + 0.1 * t,
                        0.15 + 0.15 * t,
                        0.5 + 0.2 * t,
                        1.0,
                    ]);
                }
            }

            // Generate indices (two triangles per quad, counter-clockwise).
            for y in 0..res {
                for x in 0..res {
                    let a = base + y * (res + 1) + x;
                    let b = a + 1;
                    let c = a + (res + 1);
                    let d = c + 1;

                    // Counter-clockwise winding, outward-facing.
                    indices.push(a);
                    indices.push(c);
                    indices.push(b);

                    indices.push(b);
                    indices.push(c);
                    indices.push(d);
                }
            }
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_count() {
        let cs = CubeSphere::new(4);
        let mesh = cs.build_mesh(100.0, |_| 0.0);
        // 6 faces * (4+1)^2 = 6 * 25 = 150
        assert_eq!(mesh.count_vertices(), 150);
    }

    #[test]
    fn test_all_vertices_on_sphere() {
        let radius = 100.0;
        let cs = CubeSphere::new(8);
        let mesh = cs.build_mesh(radius, |_| 0.0);

        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap();
        let bevy::mesh::VertexAttributeValues::Float32x3(positions) = positions else {
            panic!("expected Float32x3");
        };

        for pos in positions {
            let dist = Vec3::from_array(*pos).length();
            assert!(
                (dist - radius).abs() < 0.01,
                "vertex at distance {dist}, expected {radius}"
            );
        }
    }

    #[test]
    fn test_normals_point_outward() {
        let cs = CubeSphere::new(4);
        let mesh = cs.build_mesh(50.0, |_| 0.0);

        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let normals_attr = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap();

        let bevy::mesh::VertexAttributeValues::Float32x3(positions) = positions else {
            panic!("expected Float32x3");
        };
        let bevy::mesh::VertexAttributeValues::Float32x3(normals_vals) = normals_attr
        else {
            panic!("expected Float32x3");
        };

        for (pos, normal) in positions.iter().zip(normals_vals.iter()) {
            let p = Vec3::from_array(*pos);
            let n = Vec3::from_array(*normal);
            assert!(
                p.dot(n) > 0.0,
                "normal should point outward from origin"
            );
        }
    }

    #[test]
    fn test_heightmap_applies() {
        let radius = 50.0;
        let height_offset = 5.0;
        let cs = CubeSphere::new(4);
        let mesh = cs.build_mesh(radius, |_| height_offset);

        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let bevy::mesh::VertexAttributeValues::Float32x3(positions) = positions else {
            panic!("expected Float32x3");
        };

        for pos in positions {
            let dist = Vec3::from_array(*pos).length();
            assert!(
                (dist - (radius + height_offset)).abs() < 0.01,
                "vertex at distance {dist}, expected {}",
                radius + height_offset
            );
        }
    }
}
