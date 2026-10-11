//! Cached metric scenery, culled and instanced at three distances.
mod meshes;
mod visibility;
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
#[derive(Clone)]
struct Item {
    kind: CoverKind,
    variant: usize,
    position: Vec3,
    instance: Instance,
    radius: f32,
}
pub struct Scenery {
    clusters: Vec<visibility::Cluster>,
}
impl Scenery {
    pub fn new(sim: &SimState) -> Self {
        let mut items = Vec::new();
        let clearance = crate::scene::Clearance::new(sim);
        for plant in &sim.geography.cover {
            let p = terrain::point(sim, plant.position);
            if p.y < sim.sea_level + metres(0.3) || clearance.hides_cover(plant) {
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
                    coverage: [0., 1.],
                },
            });
        }
        Self::from_items(items)
    }
    fn from_items(items: Vec<Item>) -> Self {
        Self {
            clusters: visibility::cluster(items),
        }
    }
}

fn smooth(start: f32, end: f32, value: f32) -> f32 {
    let t = ((value - start) / (end - start)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// Adjacent meshes share complementary screen coverage through each transition.
fn detail_weights(distance: f32) -> [f32; LODS] {
    let near = smooth(metres(65.), metres(105.), distance);
    let far = smooth(metres(260.), metres(380.), distance);
    [1. - near, near - far, far]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detail_transitions_preserve_coverage_without_scaling_objects() {
        let mut previous = detail_weights(0.);
        for m in 0..=16000 {
            let weights = detail_weights(metres(m as f32 * 0.1));
            assert!((weights.iter().sum::<f32>() - 1.).abs() < 1e-6);
            assert!(weights.iter().all(|w| (0. ..=1.).contains(w)));
            assert!(weights.iter().filter(|&&w| w > 0.).count() <= 2);
            for i in 0..LODS {
                assert!((weights[i] - previous[i]).abs() < 0.004);
            }
            previous = weights;
        }
        assert_eq!(detail_weights(0.), [1., 0., 0.]);
        assert_eq!(detail_weights(metres(400.)), [0., 0., 1.]);
    }
    #[test]
    fn visible_batches_crossfade_and_range_fade_with_fixed_scale() {
        let mut camera = Camera::default();
        camera.enter_ground(-Vec3::Y, 1.);
        for (distance, count, coverage) in
            [(85., 2, 1.), (320., 2, 1.), (364., 2, 0.5), (401., 0, 0.)]
        {
            let position = camera.eye() - camera.direction() * metres(distance);
            let scenery = Scenery::from_items(vec![Item {
                kind: CoverKind::DryShrub,
                variant: 0,
                position,
                radius: metres(2.),
                instance: Instance {
                    position_scale: [position.x, position.y, position.z, 0.8],
                    tint_rotation: [1., 1., 1., 0.],
                    coverage: [0., 1.],
                },
            }]);
            let batches = scenery.visible(&camera, 1.5);
            let instances: Vec<_> = batches.main.iter().flatten().collect();
            assert_eq!(instances.len(), count);
            let mut high = 0.;
            for instance in instances {
                assert_eq!(instance.position_scale[3], 0.8);
                assert!((instance.coverage[0] - high).abs() < 1e-6);
                high = instance.coverage[1];
            }
            assert!((high - coverage).abs() < 1e-5);
        }
    }
    #[test]
    fn scenery_meshes_have_finite_metric_bounds_and_reduce_detail() {
        for kind in CoverKind::ALL {
            for variant in 0..VARIANTS {
                let levels: [_; LODS] =
                    std::array::from_fn(|lod| meshes::build(kind, variant, lod));
                assert!(levels[0].len() > levels[1].len(), "{}", kind.name());
                assert!(levels[1].len() >= levels[2].len(), "{}", kind.name());
                for mesh in &levels {
                    for v in mesh {
                        assert!(
                            v.position
                                .iter()
                                .chain(&v.normal)
                                .chain(&v.color)
                                .chain(&v.uv)
                                .all(|v| v.is_finite())
                        );
                        let p = Vec3::from_array(v.position);
                        assert!(p.length() < metres(20.), "{}: {p:?}", kind.name());
                        if kind.tree() {
                            // Match the conservative sphere used by scenery culling,
                            // leaving room for the shader's small wind displacement.
                            assert!(
                                (p - Vec3::Y * metres(17. * 0.4)).length()
                                    < metres(17. * 1.4 - 0.2)
                            );
                        }
                    }
                }
                if kind.tree() {
                    assert!(levels[0].len() < 18_000);
                    assert!(levels[2].len() < 1_500);
                }
                if !kind.rock() {
                    let areas = levels.each_ref().map(|mesh| {
                        mesh.chunks_exact(3)
                            .filter(|t| (5. ..6.).contains(&t[0].color[3]))
                            .map(|t| {
                                let [a, b, c] =
                                    std::array::from_fn(|i| Vec3::from_array(t[i].position));
                                (b - a).cross(c - a).length() * 0.5
                            })
                            .sum::<f32>()
                    });
                    // Simplification must not make a tree lose most of its leaf area.
                    assert!(areas[0] > 0.);
                    assert!(areas.iter().all(|area| (area / areas[0] - 1.).abs() < 0.01));
                }
            }
        }
    }
}
