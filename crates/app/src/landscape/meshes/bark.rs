//! Branch-local bark coordinates; shared geometry and vertex layout stay fixed.
use super::{Mesh, TAU, Vec3, vertex};

pub(super) fn branch(out: &mut Mesh, a: Vec3, b: Vec3, r0: f32, r1: f32, sides: usize) {
    let length = a.distance(b);
    let axis = (b - a) / length;
    let u = axis
        .cross(if axis.y.abs() < 0.95 {
            Vec3::Y
        } else {
            Vec3::X
        })
        .normalize();
    let v = axis.cross(u);
    // Integer circumference cells close the cylindrical texture seam. The
    // fractional material tag stores the period without another vertex field.
    let columns = (TAU * (r0 + r1) * 0.5 / 0.035).round().clamp(3., 128.);
    let material = 6. + columns / 256.;
    let start = a.dot(axis);
    for i in 0..sides {
        let t0 = i as f32 / sides as f32;
        let t1 = (i + 1) as f32 / sides as f32;
        let radial = |t: f32| u * (t * TAU).cos() + v * (t * TAU).sin();
        let n0 = radial(t0);
        // Close the seam exactly, without the floating-point sin(TAU) residue.
        let n1 = if i + 1 == sides { u } else { radial(t1) };
        let ps = [a + n0 * r0, a + n1 * r0, b + n0 * r1, b + n1 * r1];
        let uv = [
            [t0, start],
            [t1, start],
            [t0, start + length],
            [t1, start + length],
        ];
        let normal = |n: Vec3| (n + axis * ((r0 - r1) / length)).normalize();
        for j in [0, 1, 2, 2, 1, 3] {
            out.push(vertex(
                ps[j],
                normal(if j % 2 == 0 { n0 } else { n1 }),
                Vec3::new(0.23, 0.18, 0.115),
                material,
                uv[j],
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bark_mapping_closes_seams_and_follows_tapered_branches_at_every_lod() {
        for axis in [Vec3::Y, Vec3::X, Vec3::new(0.7, 0.3, -0.4).normalize()] {
            let a = Vec3::new(1., 2., -3.);
            let b = a + axis * 2.;
            for sides in [3, 4, 5, 6, 7, 9] {
                let mut mesh = Vec::new();
                branch(&mut mesh, a, b, 0.3, 0.1, sides);
                assert_eq!(mesh.len(), sides * 6);
                let first = mesh[0];
                let last = mesh[(sides - 1) * 6 + 1];
                assert_eq!(first.position, last.position);
                assert_eq!(first.normal, last.normal);
                assert_eq!(first.uv[0], 0.);
                assert_eq!(last.uv[0], 1.);
                assert_eq!(first.uv[1], last.uv[1]);
                assert_eq!((first.color[3] - 6.) * 256., 36.);
                for triangle in mesh.chunks_exact(3) {
                    let p: [_; 3] = std::array::from_fn(|i| Vec3::from_array(triangle[i].position));
                    let face = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
                    for vertex in triangle {
                        let n = Vec3::from_array(vertex.normal);
                        assert!((n.length() - 1.).abs() < 1e-6);
                        assert!(face.dot(n) > 0.4);
                        assert!((n.dot(axis) - 0.1 / 1.01_f32.sqrt()).abs() < 1e-6);
                        let axial = (Vec3::from_array(vertex.position) * 20.).dot(axis);
                        assert!((vertex.uv[1] - axial).abs() < 1e-5);
                    }
                }
            }
        }
    }
}
