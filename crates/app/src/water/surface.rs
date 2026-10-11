//! Split water at terrain triangle edges so narrow channels cannot disappear
//! below creases. A finer strip alone cannot guarantee this at every crossing.
#[cfg(test)]
use crate::terrain;
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{SimState, spatial::Position, units::metres};

pub(super) fn clip(polygon: &[Vertex], distance: impl Fn(&Vertex) -> f32) -> Vec<Vertex> {
    let mut result = Vec::new();
    let Some(mut a) = polygon.last().copied() else {
        return result;
    };
    let mut da = distance(&a);
    for &b in polygon {
        let db = distance(&b);
        if (da >= 0.) != (db >= 0.) {
            let t = da / (da - db);
            result.push(Vertex {
                position: std::array::from_fn(|i| {
                    a.position[i] + (b.position[i] - a.position[i]) * t
                }),
                color: std::array::from_fn(|i| a.color[i] + (b.color[i] - a.color[i]) * t),
            });
        }
        if db >= 0. {
            result.push(b);
        }
        a = b;
        da = db;
    }
    result
}

#[cfg(test)]
pub(super) fn drape(sim: &SimState, triangle: [Vertex; 3], out: &mut Vec<Vertex>) {
    drape_on(sim, triangle, out, terrain::detail(sim), |p| {
        terrain::point(sim, p).y
    })
}
pub(super) fn drape_on(
    sim: &SimState,
    triangle: [Vertex; 3],
    out: &mut Vec<Vertex>,
    detail: u32,
    height: impl Fn(Position) -> f32,
) {
    conform(sim, triangle, out, detail, |p| (height(p), 0.), false);
}

/// Intersect a separate water plane with the visible bed, retaining real depth.
#[cfg(test)]
pub(super) fn intersect_water(
    sim: &SimState,
    triangle: [Vertex; 3],
    out: &mut Vec<Vertex>,
    detail: u32,
    height: impl Fn(Position) -> f32,
    simplification: impl Fn(Position) -> f32,
) {
    conform(
        sim,
        triangle,
        out,
        detail,
        |p| (height(p), simplification(p)),
        true,
    );
}

pub(super) fn channel_depth(sim: &SimState, p: Position) -> f32 {
    use genesis_sim_core::surface::channels;
    let Some(section) = channels::section(sim, p) else {
        return 0.;
    };
    let base = genesis_sim_core::spatial::elevation(&sim.grid, p);
    let bed = channels::carve(sim, p, base);
    (channels::level(sim, section.center) - bed).max(0.)
}

fn conform(
    sim: &SimState,
    triangle: [Vertex; 3],
    out: &mut Vec<Vertex>,
    detail: u32,
    height: impl Fn(Position) -> (f32, f32),
    water: bool,
) {
    let d = detail as f32;
    let offset = [sim.grid.width as f32 * 0.5, sim.grid.height as f32 * 0.5];
    let grid = |v: &Vertex| {
        [
            (v.position[0] + offset[0]) * d,
            (v.position[2] + offset[1]) * d,
        ]
    };
    let mut emit = |mut face: Vec<Vertex>| {
        for v in &mut face {
            let (ground, simplify) = height(Position::new(
                v.position[0] + offset[0],
                v.position[2] + offset[1],
            ));
            if water {
                v.position[1] += (ground + metres(0.04) - v.position[1]).max(0.) * simplify;
                v.color[3] = -(1. + (v.position[1] - ground) * 20.);
            } else {
                v.position[1] = ground + metres(0.18);
            }
        }
        if water {
            face = clip(&face, |v| -v.color[3] - 1.);
        }
        for i in 1..face.len().saturating_sub(1) {
            out.extend_from_slice(&[face[0], face[i], face[i + 1]]);
        }
    };
    let cells = triangle.map(|v| {
        let [x, z] = grid(&v);
        (
            x.floor() as i32,
            z.floor() as i32,
            x.fract() + z.fract() <= 1.,
        )
    });
    if cells.iter().all(|cell| *cell == cells[0]) {
        emit(triangle.to_vec());
        return;
    }
    let xmin = cells.iter().map(|c| c.0).min().unwrap_or(0);
    let xmax = cells.iter().map(|c| c.0).max().unwrap_or(0);
    let zmin = cells.iter().map(|c| c.1).min().unwrap_or(0);
    let zmax = cells.iter().map(|c| c.1).max().unwrap_or(0);
    for z in zmin..=zmax {
        for x in xmin..=xmax {
            let polygon = clip(&triangle, |v| grid(v)[0] - x as f32);
            let polygon = clip(&polygon, |v| x as f32 + 1. - grid(v)[0]);
            let polygon = clip(&polygon, |v| grid(v)[1] - z as f32);
            let polygon = clip(&polygon, |v| z as f32 + 1. - grid(v)[1]);
            for sign in [-1., 1.] {
                let face = clip(&polygon, |v| {
                    let [px, pz] = grid(v);
                    (px + pz - (x + z + 1) as f32) * sign
                });
                emit(face);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_water_matches_receiver_planes_without_duplicate_or_missing_area() {
        use glam::Vec3;
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        sim.geography.version = 8;
        for z in 0..16 {
            for x in 0..16 {
                sim.grid.get_mut(x, z).elevation = 2. + ((x * 3 + z * 7) % 11) as f32 * 0.06;
            }
        }
        let triangle =
            [[-7.2, 0., -5.7], [-6.9, 0., -2.1], [-4.8, 0., -4.2]].map(|position| Vertex {
                position,
                color: [-1., 0., 0., -1.],
            });
        let area = |t: &[Vertex]| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|v| Vec3::from_array(v.position));
            (b - a).cross(c - a).y * 0.5
        };
        for eye in [Vec3::new(0., 2.1, 0.), Vec3::new(0., 100., 0.)] {
            let cache = terrain::Cache::at(&sim, terrain::lod::Key::at(eye));
            let mut mesh = Vec::new();
            let mut sheet = super::super::sheet::Sheet::default();
            sheet.add(&sim, &cache, triangle);
            sheet.finish(
                &sim,
                &super::super::slopes::Slopes::new(&sim),
                &mut mesh,
                |_| 1.,
            );
            assert!(!mesh.is_empty() && mesh.len() < 200_000);
            assert!((mesh.chunks_exact(3).map(area).sum::<f32>() - area(&triangle)).abs() < 1e-4);
            for tri in mesh.chunks_exact(3) {
                assert!(area(tri) > 0.);
                let [a, b, c] = [tri[0], tri[1], tri[2]].map(|v| Vec3::from_array(v.position));
                for weights in [[0.2, 0.3, 0.5], [0.6, 0.3, 0.1]] {
                    let p = a * weights[0] + b * weights[1] + c * weights[2];
                    let h = cache.height_at(&sim, Position::new(p.x + 8., p.z + 8.));
                    assert!((p.y - h - metres(0.04)).abs() < 2e-5);
                }
                assert!(
                    tri.iter()
                        .all(|v| v.position.iter().chain(&v.color).all(|x| x.is_finite()))
                );
            }
        }
    }
    #[test]
    fn water_plane_clips_at_banks_and_encodes_the_actual_depth() {
        let sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 8,
            height: 8,
            ..Default::default()
        });
        // Bed rises through a level plane halfway across this triangle.
        let triangle = [[-1., 2., 0.], [1., 2., 0.], [-1., 2., 1.]].map(|position| Vertex {
            position,
            color: [-1., 1., 0., -1.],
        });
        let mut out = Vec::new();
        intersect_water(
            &sim,
            triangle,
            &mut out,
            2,
            |p| 2. + (p.x - 4.) * 0.1,
            |_| 0.,
        );
        assert!(!out.is_empty());
        let mut area = 0.;
        for t in out.chunks_exact(3) {
            let [a, b, c] = [t[0].position, t[1].position, t[2].position];
            area += ((b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])).abs() * 0.5;
            for v in t {
                assert!(v.position[0] <= 1e-5);
                assert_eq!(v.position[1], 2.);
                assert!((-v.color[3] - 1. + v.position[0] * 2.).abs() < 1e-5);
            }
        }
        assert!((area - 0.75).abs() < 1e-5);
    }
    #[test]
    fn channel_crossing_terrain_creases_stays_above_ground_without_losing_area() {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 8,
            height: 8,
            ..Default::default()
        });
        sim.grid.get_mut(3, 3).elevation = 8.;
        let triangle = [(2.2, 2.4), (2.5, 4.4), (4.6, 2.6)].map(|(x, z)| {
            let mut p = terrain::point(&sim, Position::new(x, z));
            p.y += metres(0.18);
            Vertex {
                position: p.to_array(),
                color: [0.2, 0.3, 0.2, -2.],
            }
        });
        let area = |t: &[Vertex]| {
            let [a, b, c] = [t[0].position, t[1].position, t[2].position];
            ((b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])).abs() * 0.5
        };
        let mut out = Vec::new();
        drape(&sim, triangle, &mut out);
        assert!((out.chunks_exact(3).map(area).sum::<f32>() - area(&triangle)).abs() < 1e-4);
        for t in out.chunks_exact(3) {
            for weights in [[0.33333334; 3], [0.2, 0.6, 0.2], [0.1, 0.1, 0.8]] {
                let p: [f32; 3] =
                    std::array::from_fn(|i| (0..3).map(|j| t[j].position[i] * weights[j]).sum());
                let ground = terrain::point(&sim, Position::new(p[0] + 4., p[2] + 4.)).y;
                assert!((p[1] - ground - metres(0.18)).abs() < 1e-5);
            }
        }
    }
}
