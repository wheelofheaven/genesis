//! Curved, tapered blades with continuous lighting normals and height coordinates.
use genesis_renderer::instances::SceneryVertex;
use genesis_sim_core::{units::metres, worldgen::noise::unit};
use glam::Vec3;

pub(super) fn build(rush: bool) -> Vec<SceneryVertex> {
    let mut out = Vec::new();
    // Ten two-segment grass blades use the same 90 vertices as the former six
    // three-segment blades. Rushes retain their 189-vertex budget.
    let count = if rush { 9 } else { 10 };
    let steps = if rush { 4 } else { 2 };
    for i in 0..count {
        let angle = i as f32 * 2.399_963 + (unit(1117, i) - 0.5) * 0.9;
        let direction = Vec3::new(angle.cos(), 0., angle.sin());
        let side = Vec3::new(-angle.sin(), 0., angle.cos());
        let root = direction * metres(0.02 + unit(1123, i) * if rush { 0.04 } else { 0.075 });
        let height = metres(if rush {
            0.55 + unit(1129, i) * 0.39
        } else {
            0.13 + unit(1129, i) * 0.19
        });
        let bend = metres(
            if rush { 0.10 } else { 0.035 } + unit(1139, i) * if rush { 0.10 } else { 0.065 },
        );
        let width = metres(
            if rush { 0.006 } else { 0.005 } + unit(1151, i) * if rush { 0.006 } else { 0.005 },
        );
        let tint = 0.82 + unit(1153, i) * 0.32;
        let vertex = |t: f32, across: f32| {
            let point = root
                + Vec3::Y * height * t
                + direction * bend * t * t
                + side * width * (1. - t) * across;
            let tangent = Vec3::Y * height + direction * (2. * bend * t);
            // A gently curved shading cross-section avoids flat paper ribbons.
            let normal = (side.cross(tangent).normalize() + side * across * 0.22).normalize();
            let tip = t * t;
            SceneryVertex {
                position: point.to_array(),
                normal: normal.to_array(),
                color: [
                    (0.17 + tip * 0.04) * tint,
                    (0.255 + tip * 0.025) * tint,
                    (0.065 + tip * 0.025) * tint,
                    10.,
                ],
                uv: [(across + 1.) * 0.5, t],
            }
        };
        for j in 0..steps {
            let t0 = j as f32 / steps as f32;
            let t1 = (j + 1) as f32 / steps as f32;
            let a = [vertex(t0, -1.), vertex(t0, 1.)];
            if j + 1 == steps {
                out.extend_from_slice(&[a[0], a[1], vertex(1., 0.)]);
            } else {
                let b = [vertex(t1, -1.), vertex(t1, 1.)];
                out.extend_from_slice(&[a[0], a[1], b[0], a[1], b[1], b[0]]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blades_keep_the_budget_and_have_continuous_normals_and_root_coordinates() {
        for (rush, count) in [(false, 90), (true, 189)] {
            let mesh = build(rush);
            assert_eq!(mesh.len(), count);
            for (a, b) in mesh.iter().zip(build(rush)) {
                assert_eq!(a.position, b.position);
                assert_eq!(a.normal, b.normal);
                assert_eq!(a.color, b.color);
                assert_eq!(a.uv, b.uv);
            }
            for triangle in mesh.chunks_exact(3) {
                let p: [_; 3] = std::array::from_fn(|i| Vec3::from_array(triangle[i].position));
                let face = (p[1] - p[0]).cross(p[2] - p[0]);
                assert!(face.length() > 1e-7);
                for v in triangle {
                    assert!(face.normalize().dot(Vec3::from_array(v.normal)) > 0.8);
                    assert!((0. ..=1.).contains(&v.uv[0]) && (0. ..=1.).contains(&v.uv[1]));
                    if v.uv[1] == 0. {
                        assert_eq!(v.position[1], 0.);
                    }
                    for same in mesh.iter().filter(|other| other.position == v.position) {
                        assert_eq!(same.normal, v.normal);
                        assert_eq!(same.uv, v.uv);
                    }
                }
            }
        }
    }
}
