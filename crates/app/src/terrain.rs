//! Continuous terrain presentation and triangle-accurate picking.
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{
    SimState,
    spatial::{self, Position},
};
use glam::Vec3;

fn detail(sim: &SimState) -> u32 {
    if sim.grid.width > 128 { 1 } else { 2 }
}
pub fn home_camera(sim: &SimState, camera: &mut genesis_renderer::camera::Camera) {
    *camera = genesis_renderer::camera::Camera::default();
    if sim.geography.version > 0 {
        camera.frame_region(sim.grid.width.max(sim.grid.height) as f32 * 0.5);
        camera.center(Vec3::new(0., 5., 0.));
    }
}
pub fn point(sim: &SimState, p: Position) -> Vec3 {
    Vec3::new(
        p.x - sim.grid.width as f32 / 2.,
        rendered_elevation(sim, p),
        p.z - sim.grid.height as f32 / 2.,
    )
}
/// Attach small objects to the actual triangle surface, not the curved field
/// hidden between its vertices. This prevents floating houses and broken rivers.
fn rendered_elevation(sim: &SimState, p: Position) -> f32 {
    let d = detail(sim) as f32;
    let x = (p.x * d).floor() / d;
    let z = (p.z * d).floor() / d;
    let tx = (p.x - x) * d;
    let tz = (p.z - z) * d;
    let a = spatial::elevation(&sim.grid, Position::new(x, z));
    if tx == 0. && tz == 0. {
        return a;
    }
    let b = spatial::elevation(&sim.grid, Position::new(x + 1. / d, z));
    let c = spatial::elevation(&sim.grid, Position::new(x, z + 1. / d));
    if tx + tz <= 1. {
        a + tx * (b - a) + tz * (c - a)
    } else {
        let e = spatial::elevation(&sim.grid, Position::new(x + 1. / d, z + 1. / d));
        e + (1. - tx) * (c - e) + (1. - tz) * (b - e)
    }
}
pub fn surface_point(sim: &SimState, p: Position) -> Vec3 {
    let mut v = point(sim, p);
    v.y = v.y.max(sim.sea_level);
    v
}
fn colored_vertex(
    sim: &SimState,
    p: Position,
    v: Vec3,
    normal: Vec3,
    layers: crate::survey::Survey,
) -> Vertex {
    let shade = 0.48 + 0.52 * normal.dot(Vec3::new(-0.5, 0.75, 0.3).normalize()).max(0.);
    let green = spatial::sample(&sim.grid, p, |c| c.plants * 0.7 + c.fungus * 0.3);
    let inland = ((v.y - sim.sea_level - 0.07) / 0.6).clamp(0., 1.);
    let sand = Vec3::new(0.55, 0.48, 0.34);
    let grass = Vec3::new(0.30 - green * 0.10, 0.30 + green * 0.025, 0.13);
    let slope = (1. - normal.y).clamp(0., 1.);
    let stone = Vec3::new(0.39, 0.37, 0.30);
    let exposure = ((v.y - 18.) / 26.)
        .clamp(0., 1.)
        .max((slope * 2.5).clamp(0., 1.));
    let color = sand.lerp(grass, inland).lerp(stone, exposure) * shade;
    let color = crate::survey::terrain_color(sim, p, color, layers);
    Vertex {
        position: v.to_array(),
        color: [
            color.x,
            color.y,
            color.z,
            if layers.active() { 1. } else { 2. },
        ],
    }
}

/// Winding faces up; neighboring patches share exactly the same vertices.
pub fn triangles(sim: &SimState, mut visit: impl FnMut([Position; 3])) {
    let detail = detail(sim);
    for z in 0..sim.grid.height * detail {
        for x in 0..sim.grid.width * detail {
            let x = x as f32 / detail as f32;
            let z = z as f32 / detail as f32;
            let d = 1. / detail as f32;
            let a = Position::new(x, z);
            let b = Position::new(x + d, z);
            let c = Position::new(x, z + d);
            let e = Position::new(x + d, z + d);
            visit([a, c, b]);
            visit([b, c, e]);
        }
    }
}

/// Geometry and baked normals survive ecological ticks and water animation.
pub struct Cache {
    samples: Vec<(Position, Vec3, Vec3)>,
    width: u32,
    height: u32,
}
impl Cache {
    pub fn new(sim: &SimState) -> Self {
        let detail = detail(sim);
        let width = sim.grid.width * detail + 1;
        let height = sim.grid.height * detail + 1;
        let samples = (0..height)
            .flat_map(|z| {
                (0..width).map(move |x| {
                    let p = Position::new(x as f32 / detail as f32, z as f32 / detail as f32);
                    let v = point(sim, p);
                    let dx = spatial::elevation(&sim.grid, Position::new(p.x + 0.1, p.z))
                        - spatial::elevation(&sim.grid, Position::new(p.x - 0.1, p.z));
                    let dz = spatial::elevation(&sim.grid, Position::new(p.x, p.z + 0.1))
                        - spatial::elevation(&sim.grid, Position::new(p.x, p.z - 0.1));
                    (p, v, Vec3::new(-dx, 0.2, -dz).normalize())
                })
            })
            .collect();
        Self {
            samples,
            width,
            height,
        }
    }
    pub fn mesh(&self, sim: &SimState, out: &mut Vec<Vertex>, layers: crate::survey::Survey) {
        let vertices: Vec<_> = self
            .samples
            .iter()
            .map(|&(p, v, n)| colored_vertex(sim, p, v, n, layers))
            .collect();
        for z in 0..self.height - 1 {
            for x in 0..self.width - 1 {
                let i = (z * self.width + x) as usize;
                let w = self.width as usize;
                for ids in [[i, i + w, i + 1], [i + 1, i + w, i + w + 1]] {
                    for j in ids {
                        out.push(vertices[j]);
                    }
                    if ids.iter().any(|&j| vertices[j].position[1] < sim.sea_level) {
                        for j in ids {
                            let [x, h, z] = vertices[j].position;
                            out.push(Vertex {
                                position: [x, sim.sea_level, z],
                                color: [
                                    0.24,
                                    0.32,
                                    0.24,
                                    -(1. + (sim.sea_level - h).max(0.) * 20.),
                                ],
                            });
                        }
                    }
                }
            }
        }
        // Four ocean patches extend the shelf beyond the simulation boundary.
        let x = sim.grid.width as f32 * 0.5;
        let z = sim.grid.height as f32 * 0.5;
        for [x0, z0, x1, z1] in [
            [-2000., -2000., -x, 2000.],
            [x, -2000., 2000., 2000.],
            [-x, -2000., x, -z],
            [-x, z, x, 2000.],
        ] {
            for [px, pz] in [[x0, z0], [x0, z1], [x1, z0], [x1, z0], [x0, z1], [x1, z1]] {
                out.push(Vertex {
                    position: [px, sim.sea_level, pz],
                    color: [0.24, 0.32, 0.24, -481.],
                });
            }
        }
    }
}
#[cfg(test)]
pub fn mesh(sim: &SimState, out: &mut Vec<Vertex>, layers: crate::survey::Survey) {
    Cache::new(sim).mesh(sim, out, layers);
}

fn intersect(origin: Vec3, dir: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
    let e = b - a;
    let f = c - a;
    let h = dir.cross(f);
    let det = e.dot(h);
    if det.abs() < 1e-7 {
        return None;
    }
    let s = origin - a;
    let u = s.dot(h) / det;
    let q = s.cross(e);
    let v = dir.dot(q) / det;
    let t = f.dot(q) / det;
    (u >= -1e-6 && v >= -1e-6 && u + v <= 1.000001 && t >= 0.).then_some(t)
}
/// Rebuilt only when terrain height changes, reused for every pointer query.
pub struct Picker {
    chunks: Vec<Chunk>,
    width: f32,
    height: f32,
    sea: f32,
}
struct Chunk {
    triangles: Vec<[Vec3; 3]>,
    min: Vec3,
    max: Vec3,
}
impl Picker {
    pub fn new(sim: &SimState) -> Self {
        let columns = sim.grid.width.div_ceil(16) as usize;
        let mut buckets = vec![Vec::new(); columns * sim.grid.height.div_ceil(16) as usize];
        triangles(sim, |tri| {
            let x = (tri[0].x.min(tri[1].x).min(tri[2].x) / 16.) as usize;
            let z = (tri[0].z.min(tri[1].z).min(tri[2].z) / 16.) as usize;
            buckets[z * columns + x].push(tri.map(|p| point(sim, p)));
        });
        let chunks = buckets
            .into_iter()
            .map(|triangles| {
                let min = triangles
                    .iter()
                    .flatten()
                    .fold(Vec3::splat(f32::INFINITY), |a, &b| a.min(b));
                let max = triangles
                    .iter()
                    .flatten()
                    .fold(Vec3::splat(f32::NEG_INFINITY), |a, &b| a.max(b));
                Chunk {
                    triangles,
                    min: min - Vec3::splat(1e-5),
                    max: max + Vec3::splat(1e-5),
                }
            })
            .collect();
        Self {
            chunks,
            width: sim.grid.width as f32,
            height: sim.grid.height as f32,
            sea: sim.sea_level,
        }
    }
    pub fn pick(&self, origin: Vec3, dir: Vec3) -> Option<Position> {
        let mut nearest = f32::INFINITY;
        for chunk in &self.chunks {
            let mut entry: f32 = 0.;
            let mut exit = nearest;
            for axis in 0..3 {
                if dir[axis].abs() < 1e-8 {
                    if origin[axis] < chunk.min[axis] || origin[axis] > chunk.max[axis] {
                        exit = -1.;
                    }
                } else {
                    let a = (chunk.min[axis] - origin[axis]) / dir[axis];
                    let b = (chunk.max[axis] - origin[axis]) / dir[axis];
                    entry = entry.max(a.min(b));
                    exit = exit.min(a.max(b));
                }
            }
            if exit < entry {
                continue;
            }
            for tri in &chunk.triangles {
                if let Some(t) = intersect(origin, dir, *tri) {
                    nearest = nearest.min(t);
                }
            }
        }
        if dir.y.abs() > 1e-7 {
            let t = (self.sea - origin.y) / dir.y;
            let p = origin + dir * t;
            if t >= 0. && p.x.abs() < self.width / 2. && p.z.abs() < self.height / 2. {
                nearest = nearest.min(t);
            }
        }
        if !nearest.is_finite() {
            return None;
        }
        let p = origin + dir * nearest;
        Some(Position::new(
            (p.x + self.width / 2.).clamp(0., self.width - f32::EPSILON * 64.),
            (p.z + self.height / 2.).clamp(0., self.height - f32::EPSILON * 64.),
        ))
    }
}

#[cfg(test)]
pub fn pick(sim: &SimState, origin: Vec3, dir: Vec3) -> Option<Position> {
    let mut nearest = f32::INFINITY;
    triangles(sim, |tri| {
        if let Some(t) = intersect(origin, dir, tri.map(|p| point(sim, p))) {
            nearest = nearest.min(t);
        }
    });
    if dir.y.abs() > 1e-7 {
        let t = (sim.sea_level - origin.y) / dir.y;
        let p = origin + dir * t;
        if t >= 0.
            && p.x.abs() < sim.grid.width as f32 / 2.
            && p.z.abs() < sim.grid.height as f32 / 2.
        {
            nearest = nearest.min(t);
        }
    }
    if !nearest.is_finite() {
        return None;
    }
    let p = origin + dir * nearest;
    Some(Position::new(
        (p.x + sim.grid.width as f32 / 2.).clamp(0., sim.grid.width as f32 - f32::EPSILON * 64.),
        (p.z + sim.grid.height as f32 / 2.).clamp(0., sim.grid.height as f32 - f32::EPSILON * 64.),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_picker_matches_reference_for_oblique_rays_and_edits() {
        let mut sim = genesis_sim_core::scenarios::coastal_outpost();
        for edit in [false, true] {
            if edit {
                sim.step(
                    0.,
                    &[genesis_sim_core::actions::Action::Terraform {
                        x: 24,
                        y: 20,
                        radius: 3,
                        strength: 3.,
                    }],
                );
            }
            let picker = Picker::new(&sim);
            for i in 0..16 {
                let origin = Vec3::new(i as f32 - 8., 15., 12.);
                let dir = Vec3::new(0.15, -1., -0.6).normalize();
                assert_eq!(picker.pick(origin, dir), pick(&sim, origin, dir));
            }
        }
    }
    #[test]
    #[ignore = "manual interaction timing; no timing assertions"]
    fn pointer_timing() {
        use std::{hint::black_box, time::Instant};
        let sim = genesis_sim_core::scenarios::coastal_outpost();
        let picker = Picker::new(&sim);
        let origin = Vec3::new(2., 20., 10.);
        let dir = Vec3::new(0., -1., -0.4).normalize();
        let start = Instant::now();
        for _ in 0..100 {
            black_box(pick(&sim, black_box(origin), dir));
        }
        let original = start.elapsed().as_secs_f64() * 10.;
        let start = Instant::now();
        for _ in 0..100 {
            black_box(picker.pick(black_box(origin), dir));
        }
        eprintln!(
            "Pointer CPU mean: original {original:.3} ms; cached {:.3} ms",
            start.elapsed().as_secs_f64() * 10.
        );
    }
    #[test]
    fn picking_preserves_subcell_positions_and_uses_visible_surface() {
        let sim = genesis_sim_core::scenarios::coastal_outpost();
        for p in [Position::new(24.23, 20.67), Position::new(1.32, 1.87)] {
            let v = surface_point(&sim, p);
            let hit = pick(&sim, v + Vec3::Y * 20., -Vec3::Y).unwrap();
            assert!((hit.x - p.x).abs() < 1e-4 && (hit.z - p.z).abs() < 1e-4);
        }
        assert!(pick(&sim, Vec3::new(80., 20., 0.), -Vec3::Y).is_none());
        let mut out = Vec::new();
        mesh(&sim, &mut out, crate::survey::Survey::default());
        assert!(out.iter().all(|v| {
            v.position
                .iter()
                .chain(v.color.iter())
                .all(|v| v.is_finite())
        }));
    }
}
