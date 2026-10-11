//! Continuous terrain presentation and triangle-accurate picking.
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{
    SimState,
    spatial::{self, Position},
};
use glam::Vec3;
pub mod ground_cover;
pub mod lod;

#[cfg(test)]
pub(crate) fn detail(sim: &SimState) -> u32 {
    if sim.geography.version > 0 {
        genesis_sim_core::surface::SUBDIVISIONS
    } else {
        2
    }
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
    genesis_sim_core::surface::elevation(sim, p)
}
pub fn surface_point(sim: &SimState, p: Position) -> Vec3 {
    let mut v = point(sim, p);
    v.y = v.y.max(sim.sea_level);
    v
}
pub(crate) fn colored_vertex(
    sim: &SimState,
    p: Position,
    v: Vec3,
    normal: Vec3,
    channel_detail: f32,
    layers: crate::survey::Survey,
) -> Vertex {
    let shade = 0.48 + 0.52 * normal.dot(Vec3::new(-0.5, 0.75, 0.3).normalize()).max(0.);
    let green = spatial::sample(&sim.grid, p, |c| c.plants).clamp(0., 1.);
    let moisture = spatial::sample(&sim.grid, p, |c| c.moisture).clamp(0., 1.);
    let inland = ((v.y - sim.sea_level - 0.07) / 0.6).clamp(0., 1.);
    let sand = Vec3::new(0.55, 0.48, 0.34);
    let slope = (1. - normal.y).clamp(0., 1.);
    let stone = match sim.subsurface.reading(p).rock {
        genesis_sim_core::subsurface::Rock::Limestone => Vec3::new(0.53, 0.49, 0.39),
        genesis_sim_core::subsurface::Rock::Sandstone => Vec3::new(0.48, 0.33, 0.21),
        genesis_sim_core::subsurface::Rock::Basalt => Vec3::new(0.22, 0.23, 0.21),
    };
    // Climate and living cover control the broad palette; cosmetic shader detail
    // supplies centimetre-scale variation without resampling the simulation.
    let dry = ((1. - moisture) * sim.geography.climate.summer_drought).clamp(0., 1.);
    let earth = stone.lerp(Vec3::new(0.25, 0.19, 0.12), 0.65) * (1. - moisture * 0.22);
    let grass = Vec3::new(0.19, 0.25, 0.085).lerp(Vec3::new(0.38, 0.31, 0.15), dry);
    let ground = earth.lerp(grass, green * (1. - dry * 0.35));
    let exposure = ((v.y - 18.) / 26.)
        .clamp(0., 1.)
        .max((slope * 2.5).clamp(0., 1.));
    let mut color = sand.lerp(ground, inland).lerp(stone, exposure) * shade;
    if channel_detail > 0.
        && let Some(section) = genesis_sim_core::surface::channels::section(sim, p)
    {
        let bank = ((section.distance - section.half_width)
            / genesis_sim_core::surface::channels::BANK)
            .clamp(0., 1.);
        let damp = Vec3::new(0.24, 0.22, 0.15).lerp(sand * 0.82, bank);
        color = color.lerp(
            damp * shade,
            (1. - bank * bank * (3. - 2. * bank)) * channel_detail,
        );
    }
    let seabed = layers.seafloor && v.y < sim.sea_level;
    if seabed {
        let depth = (sim.sea_level - v.y) * 20.;
        color = Vec3::new(0.31, 0.45, 0.42)
            .lerp(Vec3::new(0.075, 0.19, 0.27), (depth / 500.).clamp(0., 1.))
            * shade;
    }
    let color = crate::survey::terrain_color(sim, p, color, layers);
    Vertex {
        position: v.to_array(),
        color: [
            color.x,
            color.y,
            color.z,
            if seabed {
                4.
            } else if layers.water || layers.minerals {
                1.
            } else {
                // Fractional material data blends soil into exposed geology
                // without adding attributes to every terrain vertex.
                2. + exposure * 0.4
            },
        ],
    }
}

/// Winding faces up; neighboring patches share exactly the same vertices.
#[cfg(test)]
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
    context: crate::horizon::Cache,
    samples: Vec<(Position, Vec3, Vec3, f32)>,
    indices: Vec<[usize; 3]>,
    pub key: lod::Key,
    patches: lod::Patches,
    pub rivers: Vec<Vertex>,
    pub ground_cover: ground_cover::Placements,
}
impl Cache {
    #[cfg(test)]
    pub fn new(sim: &SimState) -> Self {
        let mut camera = genesis_renderer::camera::Camera::default();
        home_camera(sim, &mut camera);
        Self::at(sim, lod::Key::at(camera.eye()))
    }
    pub fn at(sim: &SimState, key: lod::Key) -> Self {
        let (points, indices, patches) = lod::topology(sim, key);
        let height = |p: Position| {
            let simplify = lod::channel_simplification(sim, key.eye(), p);
            if simplify == 0. || sim.geography.version == 0 {
                return genesis_sim_core::surface::vertex_height(sim, p);
            }
            let base = spatial::elevation(&sim.grid, p);
            if simplify == 1. {
                return base;
            }
            let detailed = genesis_sim_core::surface::vertex_height(sim, p);
            detailed + (base - detailed) * simplify
        };
        let samples = points
            .into_iter()
            .map(|p| {
                let v = Vec3::new(
                    p.x - sim.grid.width as f32 * 0.5,
                    height(p),
                    p.z - sim.grid.height as f32 * 0.5,
                );
                let delta = genesis_sim_core::surface::SPACING;
                let dx = height(Position::new(p.x + delta, p.z))
                    - height(Position::new(p.x - delta, p.z));
                let dz = height(Position::new(p.x, p.z + delta))
                    - height(Position::new(p.x, p.z - delta));
                (
                    p,
                    v,
                    Vec3::new(-dx, delta * 2., -dz).normalize(),
                    1. - lod::channel_simplification(sim, key.eye(), p),
                )
            })
            .collect();
        let mut cache = Self {
            patches,
            rivers: Vec::new(),
            ground_cover: ground_cover::placements(sim, key.eye()),
            context: crate::horizon::Cache::new(sim),
            samples,
            indices,
            key,
        };
        let mut rivers = Vec::new();
        crate::water::rivers(sim, &cache, &mut rivers);
        cache.rivers = rivers;
        cache
    }
    fn patch(&self, p: Position) -> Option<(u32, &std::ops::Range<usize>)> {
        let x = (p.x * 32.).floor().max(0.) as u32;
        let z = (p.z * 32.).floor().max(0.) as u32;
        [1, 2, 4, 8, 16, 32].into_iter().find_map(|size| {
            self.patches
                .get(&(x / size * size, z / size * size, size))
                .map(|r| (size, r))
        })
    }
    pub fn detail_at(&self, p: Position) -> u32 {
        self.patch(p).map_or(1, |(s, _)| 32 / s)
    }
    /// Nearby visible triangles for small conforming surface decals. Query the
    /// finest lattice, deduplicate adaptive patches, then visit their triangles.
    pub(crate) fn triangles_in(
        &self,
        min: Position,
        max: Position,
        mut visit: impl FnMut([Vec3; 3]),
    ) {
        self.indexed_triangles_in(min, max, |_, triangle| visit(triangle));
    }
    /// Stable receiver IDs let overlapping surface decals share one partition.
    pub(crate) fn indexed_triangles_in(
        &self,
        min: Position,
        max: Position,
        mut visit: impl FnMut(usize, [Vec3; 3]),
    ) {
        let mut ranges = std::collections::BTreeMap::new();
        for z in (min.z * 32.).floor() as i32..=(max.z * 32.).floor() as i32 {
            for x in (min.x * 32.).floor() as i32..=(max.x * 32.).floor() as i32 {
                if let Some((_, range)) = self.patch(Position::new(
                    (x as f32 + 0.5) / 32.,
                    (z as f32 + 0.5) / 32.,
                )) {
                    ranges.insert(range.start, range.end);
                }
            }
        }
        for (start, end) in ranges {
            for id in start..end {
                visit(id, self.indices[id].map(|i| self.samples[i].1));
            }
        }
    }
    pub fn height_at(&self, sim: &SimState, p: Position) -> f32 {
        if let Some((_, range)) = self.patch(p) {
            for id in range.clone() {
                let [a, b, c] = self.indices[id].map(|i| self.samples[i]);
                let ax = b.0.x - a.0.x;
                let az = b.0.z - a.0.z;
                let bx = c.0.x - a.0.x;
                let bz = c.0.z - a.0.z;
                let dx = p.x - a.0.x;
                let dz = p.z - a.0.z;
                let det = ax * bz - az * bx;
                let u = (dx * bz - dz * bx) / det;
                let v = (ax * dz - az * dx) / det;
                if u >= -1e-5 && v >= -1e-5 && u + v <= 1.00001 {
                    return a.1.y + u * (b.1.y - a.1.y) + v * (c.1.y - a.1.y);
                }
            }
        }
        rendered_elevation(sim, p)
    }
    pub fn mesh(&self, sim: &SimState, out: &mut Vec<Vertex>, layers: crate::survey::Survey) {
        let vertices: Vec<_> = self
            .samples
            .iter()
            .map(|&(p, v, n, channel_detail)| colored_vertex(sim, p, v, n, channel_detail, layers))
            .collect();
        for ids in &self.indices {
            for &j in ids {
                out.push(vertices[j]);
            }
            if !layers.seafloor && ids.iter().any(|&j| vertices[j].position[1] < sim.sea_level) {
                for &j in ids {
                    let [x, h, z] = vertices[j].position;
                    out.push(Vertex {
                        position: [x, sim.sea_level, z],
                        color: [0.24, 0.32, 0.24, -(1. + (sim.sea_level - h).max(0.) * 20.)],
                    });
                }
            }
        }
        if sim.geography.region != genesis_sim_core::worldgen::Region::Island {
            self.context.append(sim, out, layers);
            return;
        }
        if layers.seafloor {
            return;
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

pub(crate) fn intersect(origin: Vec3, dir: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
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
    #[cfg(test)]
    pub fn new(sim: &SimState) -> Self {
        Self::from_cache(sim, &Cache::new(sim))
    }
    pub fn from_cache(sim: &SimState, cache: &Cache) -> Self {
        let columns = sim.grid.width.div_ceil(16) as usize;
        let mut buckets = vec![Vec::new(); columns * sim.grid.height.div_ceil(16) as usize];
        for ids in &cache.indices {
            let tri = ids.map(|i| cache.samples[i].0);
            let x = (tri[0].x.min(tri[1].x).min(tri[2].x) / 16.) as usize;
            let z = (tri[0].z.min(tri[1].z).min(tri[2].z) / 16.) as usize;
            buckets[z * columns + x].push(ids.map(|i| cache.samples[i].1));
        }
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
    pub fn pick(&self, origin: Vec3, dir: Vec3, include_water: bool) -> Option<Position> {
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
        if include_water && dir.y.abs() > 1e-7 {
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
    fn human_view_shares_physical_ground_and_fits_the_mesh_budget() {
        use genesis_sim_core::worldgen::{self, Region};
        for region in Region::ALL {
            let sim = worldgen::generate_region(worldgen::DEFAULT_SEED, region);
            let mut camera = genesis_renderer::camera::Camera::default();
            crate::ground::toggle(&sim, sim.buildings[0].position, &mut camera);
            assert!(camera.is_ground());
            let start = std::time::Instant::now();
            let cache = Cache::at(&sim, lod::Key::at(camera.eye()));
            let mut mesh = Vec::new();
            cache.mesh(&sim, &mut mesh, Default::default());
            let bytes = (mesh.len() + cache.rivers.len()) * std::mem::size_of::<Vertex>();
            eprintln!(
                "{}: {} terrain triangles, {} river vertices, {} MiB, {:?} build",
                region.name(),
                cache.indices.len(),
                cache.rivers.len(),
                bytes / 1048576,
                start.elapsed()
            );
            assert!(bytes < 128 * 1024 * 1024, "bounded CPU/GPU scene mesh");
            for i in 0..32 {
                let p = Position::new(
                    camera.focus.x + sim.grid.width as f32 / 2. + i as f32 * 0.013,
                    camera.focus.z + sim.grid.height as f32 / 2. + 0.137,
                );
                assert_eq!(cache.detail_at(p), genesis_sim_core::surface::SUBDIVISIONS);
                assert!(
                    (cache.height_at(&sim, p) - genesis_sim_core::surface::elevation(&sim, p))
                        .abs()
                        < 1e-4
                );
            }
            let picker = Picker::from_cache(&sim, &cache);
            let point = picker.pick(camera.eye(), Vec3::NEG_Y, false).unwrap();
            assert!((point.x - camera.focus.x - sim.grid.width as f32 / 2.).abs() < 1e-5);
        }
    }
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
                assert_eq!(picker.pick(origin, dir, true), pick(&sim, origin, dir));
            }
        }
    }
    #[test]
    fn seafloor_picking_uses_exposed_bed_instead_of_hidden_water() {
        let mut sim = genesis_sim_core::scenarios::coastal_outpost();
        for c in sim.grid.cells_mut() {
            c.elevation = -5.;
        }
        let picker = Picker::new(&sim);
        let o = Vec3::new(0., 10., 0.);
        let d = Vec3::new(0.2, -1., 0.).normalize();
        let sea = picker.pick(o, d, true).unwrap();
        let bed = picker.pick(o, d, false).unwrap();
        assert!((bed.x - sea.x - 1.).abs() < 0.001);
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
            black_box(picker.pick(black_box(origin), dir, true));
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
