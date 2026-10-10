use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    /// RGB is linear colour. W packs material: 1 solid, 2 terrain,
    /// 3 weathered building, 4 seafloor with depth contours;
    /// negative values encode water depth in metres as -(1 + depth).
    /// These are opaque materials; W is not transparency.
    pub color: [f32; 4],
}

pub fn cuboid(out: &mut Vec<Vertex>, center: [f32; 3], size: [f32; 3], color: [f32; 3]) {
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
