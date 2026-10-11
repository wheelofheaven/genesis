//! Coarse spatial rejection before per-object tests. Shadow casters are independent
//! of camera visibility, so turning away never removes a tree's visible shadow.
use super::*;
use genesis_renderer::{
    shadows::{CASCADES, Shadows},
    visibility::Frustum,
};
use std::collections::BTreeMap;

pub(super) struct Cluster {
    min: Vec3,
    max: Vec3,
    items: Vec<Item>,
}

pub(super) fn cluster(items: Vec<Item>) -> Vec<Cluster> {
    let mut groups = BTreeMap::<(i32, i32), Vec<Item>>::new();
    for item in items {
        let p = item.position / metres(160.);
        groups
            .entry((p.x.floor() as i32, p.z.floor() as i32))
            .or_default()
            .push(item);
    }
    groups
        .into_values()
        .map(|items| {
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            for item in &items {
                let (center, radius) = bounds(item);
                min = min.min(center - Vec3::splat(radius));
                max = max.max(center + Vec3::splat(radius));
            }
            Cluster { min, max, items }
        })
        .collect()
}

fn bounds(item: &Item) -> (Vec3, f32) {
    (
        item.position + Vec3::Y * item.radius * 0.4,
        item.radius * 1.4,
    )
}

#[derive(Default, Debug)]
pub struct VisibilityStats {
    pub total_items: usize,
    pub tested_items: usize,
    pub main_instances: usize,
    pub shadow_instances: [usize; CASCADES],
}

pub struct VisibleScenery {
    pub main: Vec<Vec<Instance>>,
    pub shadows: [Vec<Vec<Instance>>; CASCADES],
    pub stats: VisibilityStats,
}

impl Scenery {
    pub fn visible(&self, camera: &Camera, aspect: f32) -> VisibleScenery {
        let view = Frustum::new(camera.view_projection(aspect));
        let lights = std::array::from_fn::<_, CASCADES, _>(|i| {
            Frustum::new(Shadows::cascade_matrix(camera.focus, i))
        });
        let eye = camera.eye();
        let mut out = VisibleScenery {
            main: vec![Vec::new(); BATCHES],
            shadows: std::array::from_fn(|_| vec![Vec::new(); BATCHES]),
            stats: VisibilityStats::default(),
        };
        for cluster in &self.clusters {
            out.stats.total_items += cluster.items.len();
            let main = view.bounds(cluster.min, cluster.max);
            let shadows = lights
                .each_ref()
                .map(|light| light.bounds(cluster.min, cluster.max));
            if !main && !shadows.iter().any(|s| *s) {
                continue;
            }
            out.stats.tested_items += cluster.items.len();
            for item in &cluster.items {
                let (center, radius) = bounds(item);
                for cascade in 0..CASCADES {
                    // Tiny shrubs/stones cannot produce useful far-map silhouettes.
                    if cascade == 1 && !item.kind.tree() && radius < metres(2.) {
                        continue;
                    }
                    if shadows[cascade] && lights[cascade].sphere(center, radius) {
                        // Fixed caster LODs do not change with camera-facing LOD fades.
                        let lod = if cascade == 0 { 1 } else { 2 };
                        out.shadows[cascade][batch(item.kind, item.variant, lod)]
                            .push(item.instance);
                        out.stats.shadow_instances[cascade] += 1;
                    }
                }
                let distance = item.position.distance(eye);
                let range = metres(if item.kind.tree() {
                    12000.
                } else if item.kind.rock() {
                    1400.
                } else {
                    400.
                });
                if !main || distance >= range || !view.sphere(center, radius) {
                    continue;
                }
                let visibility = 1. - smooth(range * 0.82, range, distance);
                let mut low = 0.;
                for (lod, weight) in detail_weights(distance).into_iter().enumerate() {
                    let high = low + weight * visibility;
                    if high > low {
                        let mut instance = item.instance;
                        instance.coverage = [low, high];
                        out.main[batch(item.kind, item.variant, lod)].push(instance);
                        out.stats.main_instances += 1;
                    }
                    low = high;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(position: Vec3) -> Item {
        Item {
            kind: CoverKind::Limestone,
            variant: 0,
            position,
            radius: metres(2.5),
            instance: Instance {
                position_scale: position.extend(1.).to_array(),
                tint_rotation: [1., 1., 1., 0.],
                coverage: [0., 1.],
            },
        }
    }
    fn flat(scenery: &Scenery) -> Scenery {
        Scenery {
            clusters: vec![Cluster {
                min: Vec3::splat(-100000.),
                max: Vec3::splat(100000.),
                items: scenery
                    .clusters
                    .iter()
                    .flat_map(|c| c.items.iter().cloned())
                    .collect(),
            }],
        }
    }
    fn identity(batches: &[Vec<Instance>]) -> Vec<(usize, [u32; 4], [u32; 2])> {
        let mut ids: Vec<_> = batches
            .iter()
            .enumerate()
            .flat_map(|(i, batch)| {
                batch.iter().map(move |v| {
                    (
                        i,
                        v.position_scale.map(f32::to_bits),
                        v.coverage.map(f32::to_bits),
                    )
                })
            })
            .collect();
        ids.sort_unstable();
        ids
    }
    #[test]
    fn offscreen_casters_survive_and_distant_visible_objects_skip_shadows() {
        let mut camera = Camera::default();
        camera.enter_ground(-Vec3::Y, 1.);
        let behind = item(camera.direction() * metres(40.));
        let distant = item(-camera.direction() * metres(1000.));
        let scenery = Scenery::from_items(vec![behind.clone(), distant.clone()]);
        let visible = scenery.visible(&camera, 1.5);
        assert_eq!(visible.stats.main_instances, 1);
        assert_eq!(visible.stats.shadow_instances[0], 1);
        assert_eq!(
            visible.main.iter().flatten().next().unwrap().position_scale,
            distant.instance.position_scale
        );
        assert_eq!(
            visible.shadows[0]
                .iter()
                .flatten()
                .next()
                .unwrap()
                .position_scale,
            behind.instance.position_scale
        );
        let shadows = visible.shadows.each_ref().map(|s| identity(s));
        camera.orbit(glam::Vec2::new(200., 30.));
        assert_eq!(
            shadows,
            scenery
                .visible(&camera, 1.5)
                .shadows
                .each_ref()
                .map(|s| identity(s))
        );
    }
    #[test]
    fn wider_map_retains_tree_casters_but_omits_tiny_objects() {
        let mut camera = Camera::default();
        camera.enter_ground(Vec3::ZERO, metres(1.7));
        let sun = Vec3::new(-0.5, 0.75, 0.3).normalize();
        let p = Vec3::Y.cross(sun).normalize() * metres(400.);
        let mut tree = item(p);
        tree.kind = CoverKind::Pine;
        tree.radius = metres(8.);
        let mut tiny = item(p + Vec3::X);
        tiny.radius = metres(0.3);
        let scenery = Scenery::from_items(vec![tree.clone(), tiny]);
        let visible = scenery.visible(&camera, 1.5);
        assert_eq!(visible.stats.shadow_instances, [0, 1]);
        assert_eq!(
            visible.shadows[1][batch(tree.kind, tree.variant, 2)].len(),
            1
        );
        let far = identity(&visible.shadows[1]);
        camera.orbit(glam::Vec2::new(170., 40.));
        assert_eq!(far, identity(&scenery.visible(&camera, 1.5).shadows[1]));
    }
    #[test]
    fn cluster_rejection_matches_brute_force_across_views() {
        let items = (-30..30)
            .flat_map(|z| {
                (-30..30).map(move |x| {
                    item(Vec3::new(
                        x as f32 * 2.,
                        (x as f32 * 0.1).sin(),
                        z as f32 * 2.,
                    ))
                })
            })
            .collect();
        let scenery = Scenery::from_items(items);
        let reference = flat(&scenery);
        let mut camera = Camera::default();
        camera.enter_ground(Vec3::ZERO, 0.085);
        for aspect in [0.5, 1.5, 3.] {
            for _ in 0..8 {
                camera.orbit(glam::Vec2::new(130., 2.));
                let actual = scenery.visible(&camera, aspect);
                let expected = reference.visible(&camera, aspect);
                assert_eq!(identity(&actual.main), identity(&expected.main));
                for cascade in 0..CASCADES {
                    assert_eq!(
                        identity(&actual.shadows[cascade]),
                        identity(&expected.shadows[cascade])
                    );
                }
                assert!(actual.stats.tested_items < actual.stats.total_items);
            }
        }
    }
    #[test]
    #[ignore = "manual CPU visibility benchmark; no frame-rate assertions"]
    fn visibility_timing() {
        use std::{hint::black_box, time::Instant};
        let sim = genesis_sim_core::worldgen::generate(genesis_sim_core::worldgen::DEFAULT_SEED);
        let scenery = Scenery::new(&sim);
        let reference = flat(&scenery);
        let mut camera = Camera::default();
        let home = terrain::point(&sim, sim.buildings[0].position);
        camera.frame_region(192.);
        camera.center(home);
        camera.set_visible_height(metres(160.));
        for label in ["settlement", "ground", "region"] {
            if label == "ground" {
                camera.enter_ground(home, metres(1.7));
            }
            if label == "region" {
                camera.leave_ground();
                camera.set_visible_height(250.);
            }
            for (name, data) in [("clustered", &scenery), ("flat", &reference)] {
                let start = Instant::now();
                for _ in 0..30 {
                    black_box(data.visible(&camera, 1.6));
                }
                println!(
                    "{label} {name}: {:.3} ms/call {:?}",
                    start.elapsed().as_secs_f64() * 1000. / 30.,
                    data.visible(&camera, 1.6).stats
                );
            }
        }
    }
}
