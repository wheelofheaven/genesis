//! Cached metric scenery, culled and instanced at three distances.
mod meshes;
use crate::terrain;
use genesis_renderer::{
    camera::Camera,
    instances::{Instance, SceneryVertex},
};
use genesis_sim_core::{SimState, ecology::CoverKind, units::metres, worldgen::noise::unit};
use glam::Vec3;
pub const VARIANTS: usize = 2;
pub const LODS: usize = 3;
pub const BATCHES: usize = 8 * VARIANTS * LODS;
fn batch(kind: CoverKind, variant: usize, lod: usize) -> usize {
    (kind.index() * VARIANTS + variant) * LODS + lod
}
pub fn meshes() -> Vec<Vec<SceneryVertex>> {
    CoverKind::ALL
        .into_iter()
        .flat_map(|kind| {
            (0..VARIANTS).flat_map(move |variant| {
                (0..LODS).map(move |lod| meshes::build(kind, variant, lod))
            })
        })
        .collect()
}
struct Item {
    kind: CoverKind,
    variant: usize,
    position: Vec3,
    instance: Instance,
    radius: f32,
}
pub struct Scenery {
    items: Vec<Item>,
}
impl Scenery {
    pub fn new(sim: &SimState) -> Self {
        let mut items = Vec::new();
        for plant in &sim.geography.cover {
            let p = terrain::point(sim, plant.position);
            if p.y < sim.sea_level + metres(0.3)
                || sim.buildings.iter().any(|b| {
                    genesis_sim_core::paths::distance(b.position, plant.position)
                        < metres(if plant.kind.tree() { 6. } else { 4. })
                })
                || sim.paths.iter().any(|path| {
                    genesis_sim_core::paths::on_path(
                        path,
                        plant.position,
                        plant.kind.collision_radius() * plant.size + metres(0.2),
                    )
                })
            {
                continue;
            }
            let variation = unit(plant.seed, 955);
            let shade = 0.86 + unit(plant.seed, 959) * 0.27;
            let radius = metres(if plant.kind.tree() {
                17.
            } else if plant.kind.rock() {
                2.5
            } else {
                2.
            }) * plant.size;
            items.push(Item {
                kind: plant.kind,
                variant: (plant.seed % VARIANTS as u64) as usize,
                position: p,
                radius,
                instance: Instance {
                    position_scale: [p.x, p.y, p.z, plant.size],
                    tint_rotation: [
                        shade,
                        shade * (0.97 + variation * 0.06),
                        shade * (0.92 + variation * 0.12),
                        variation * std::f32::consts::TAU,
                    ],
                },
            });
        }
        Self { items }
    }
    /// Large conservative bounds keep whole crowns visible at screen edges.
    pub fn visible(&self, camera: &Camera, aspect: f32) -> Vec<Vec<Instance>> {
        let mut batches = vec![Vec::new(); BATCHES];
        let matrix = camera.view_projection(aspect);
        let eye = camera.eye();
        for item in &self.items {
            let distance = item.position.distance(eye);
            let range = metres(if item.kind.tree() {
                12000.
            } else if item.kind.rock() {
                1400.
            } else {
                400.
            });
            if distance > range {
                continue;
            }
            let clip = matrix * (item.position + Vec3::Y * item.radius * 0.4).extend(1.);
            let margin = item.radius * 3.;
            if clip.w < -margin || clip.x.abs() > clip.w + margin || clip.y.abs() > clip.w + margin
            {
                continue;
            }
            let lod = if distance < metres(85.) {
                0
            } else if distance < metres(320.) {
                1
            } else {
                2
            };
            let mut instance = item.instance;
            // Shrubs fade by shrinking only over the last 60 m of their range.
            if !item.kind.tree() && !item.kind.rock() {
                instance.position_scale[3] *= ((range - distance) / metres(60.)).clamp(0., 1.);
            }
            batches[batch(item.kind, item.variant, lod)].push(instance);
        }
        batches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scenery_meshes_have_finite_metric_bounds_and_reduce_detail() {
        for kind in CoverKind::ALL {
            let detailed = meshes::build(kind, 0, 0);
            let distant = meshes::build(kind, 0, 2);
            assert!(detailed.len() > distant.len(), "{}", kind.name());
            for v in &detailed {
                assert!(
                    v.position
                        .iter()
                        .chain(&v.normal)
                        .chain(&v.color)
                        .all(|v| v.is_finite())
                );
                let p = Vec3::from_array(v.position);
                assert!(p.length() < metres(20.), "{}: {p:?}", kind.name());
            }
            if kind.tree() {
                assert!(distant.len() < 2500);
            }
        }
    }
}
