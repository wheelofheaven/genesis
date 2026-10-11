//! Continuous surface marking geometry in world units.
use crate::terrain;
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{SimState, spatial::Position, units::metres};
use glam::{Vec2, Vec3};

fn triangle(out: &mut Vec<Vertex>, points: [Vec3; 3], color: [f32; 3]) {
    for p in points {
        out.push(Vertex {
            position: p.to_array(),
            color: [color[0], color[1], color[2], 1.],
        });
    }
}
pub(crate) fn ribbon(
    sim: &SimState,
    out: &mut Vec<Vertex>,
    points: &[Position],
    width: f32,
    color: [f32; 3],
) {
    if points.len() < 2 {
        return;
    }
    let edges: Vec<_> = edges(points, width)
        .into_iter()
        .map(|pair| pair.map(|p| terrain::surface_point(sim, p) + Vec3::Y * metres(0.06)))
        .collect();
    for e in edges.windows(2) {
        // Both faces permit arbitrary path direction and steep camera views.
        for ids in [[0, 1, 2], [2, 1, 3], [2, 1, 0], [3, 1, 2]] {
            let ps = [e[0][0], e[0][1], e[1][0], e[1][1]];
            triangle(out, ids.map(|i| ps[i]), color);
        }
    }
}

/// A bounded miter keeps width through bends and joins a closed loop seamlessly.
pub(super) fn edges(points: &[Position], width: f32) -> Vec<[Position; 2]> {
    if points.len() < 2 {
        return Vec::new();
    }
    let closed = points.len() > 2
        && genesis_sim_core::paths::distance(points[0], points[points.len() - 1]) < metres(0.001);
    points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let p = if closed && i + 1 == points.len() {
                &points[0]
            } else {
                p
            };
            let previous = points[if i == 0 && closed {
                points.len() - 2
            } else {
                i.saturating_sub(1)
            }];
            let next = points[if i + 1 == points.len() && closed {
                1
            } else {
                (i + 1).min(points.len() - 1)
            }];
            let mut incoming = Vec2::new(p.x - previous.x, p.z - previous.z).normalize_or_zero();
            let mut outgoing = Vec2::new(next.x - p.x, next.z - p.z).normalize_or_zero();
            if incoming == Vec2::ZERO {
                incoming = outgoing;
            }
            if outgoing == Vec2::ZERO {
                outgoing = incoming;
            }
            let tangent = (incoming + outgoing).normalize_or_zero();
            let tangent = if tangent == Vec2::ZERO {
                outgoing
            } else {
                tangent
            };
            let normal = Vec2::new(-tangent.y, tangent.x);
            let side = normal * (width * 0.5 / tangent.dot(outgoing).max(0.5));
            [-1., 1.].map(|sign| Position::new(p.x + side.x * sign, p.z + side.y * sign))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_markings_keep_corner_width_and_seam_at_any_rotation() {
        for angle in [0.0_f32, 0.7, 2.1] {
            let (s, c) = angle.sin_cos();
            let points: Vec<_> = [(0., 0.), (6.5, 0.), (6.5, 4.8), (0., 4.8), (0., 0.)]
                .into_iter()
                .map(|(x, z)| Position::new(metres(x * c - z * s), metres(x * s + z * c)))
                .collect();
            let width = metres(0.15);
            let rows = edges(&points, width);
            assert_eq!(rows.first(), rows.last());
            let mut area = 0.;
            for pair in rows.windows(2) {
                let [a, b, c, d] = [pair[0][0], pair[0][1], pair[1][0], pair[1][1]]
                    .map(|p| Vec3::new(p.x, 0., p.z));
                area += ((b - a).cross(c - a).y + (d - b).cross(c - b).y) * 0.5;
            }
            let expected = metres(2. * (6.5 + 4.8)) * width;
            assert!((area - expected).abs() < 1e-6);
        }
        for points in [
            vec![Position::new(0., 0.); 3],
            vec![
                Position::new(0., 0.),
                Position::new(1., 0.),
                Position::new(0., 0.),
            ],
        ] {
            for (p, pair) in points.iter().zip(edges(&points, metres(0.15))) {
                for v in pair {
                    assert!(v.x.is_finite() && v.z.is_finite());
                    assert!(genesis_sim_core::paths::distance(*p, v) <= metres(0.151));
                }
            }
        }
    }
}
