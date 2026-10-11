//! Split water at terrain triangle edges so narrow channels cannot disappear
//! below creases. A finer strip alone cannot guarantee this at every crossing.
#[cfg(test)]
use crate::terrain;
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{SimState, spatial::Position, units::metres};

fn clip(polygon: &[Vertex], distance: impl Fn(&Vertex) -> f32) -> Vec<Vertex> {
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
    let d = detail as f32;
    let offset = [sim.grid.width as f32 * 0.5, sim.grid.height as f32 * 0.5];
    let grid = |v: &Vertex| {
        [
            (v.position[0] + offset[0]) * d,
            (v.position[2] + offset[1]) * d,
        ]
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
        out.extend_from_slice(&triangle);
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
                let mut face = clip(&polygon, |v| {
                    let [px, pz] = grid(v);
                    (px + pz - (x + z + 1) as f32) * sign
                });
                for v in &mut face {
                    v.position[1] = height(Position::new(
                        v.position[0] + offset[0],
                        v.position[2] + offset[1],
                    )) + metres(0.18);
                }
                for i in 1..face.len().saturating_sub(1) {
                    out.extend_from_slice(&[face[0], face[i], face[i + 1]]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
