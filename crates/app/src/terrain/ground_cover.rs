//! Bounded cosmetic ground cover; these instances are not simulated organisms.
mod canopy;
mod meshes;
use genesis_renderer::instances::{Instance, SceneryVertex};
use genesis_sim_core::{
    SimState,
    spatial::{self, Position},
    subsurface::Rock,
    surface::channels,
    units::metres,
    worldgen::noise::{noise, unit},
};
use glam::Vec3;
pub const BATCHES: usize = 5;
pub type Placements = [Vec<Instance>; BATCHES];
// 38 m visible radius + 16 m halo covers the 20 m camera-key diagonal offset.
const RADIUS: f32 = metres(54.);
const SPACING: f32 = metres(0.8);
pub fn meshes() -> [Vec<SceneryVertex>; BATCHES] {
    meshes::build()
}

pub fn placements(sim: &SimState, eye: Vec3) -> Placements {
    let center = Position::new(
        eye.x + sim.grid.width as f32 / 2.,
        eye.z + sim.grid.height as f32 / 2.,
    );
    let mut out: Placements = Default::default();
    if eye.y - super::point(sim, center).y > metres(60.) {
        return out;
    }
    let clearance = crate::scene::Clearance::new(sim);
    let canopy = canopy::Canopy::new(sim, center, RADIUS, &clearance);
    let cx = (center.x / SPACING).floor() as i32;
    let cz = (center.z / SPACING).floor() as i32;
    let cells = (RADIUS / SPACING).ceil() as i32;
    for z in cz - cells..=cz + cells {
        for x in cx - cells..=cx + cells {
            let id = ((x as u32 as u64) << 32) | z as u32 as u64;
            let p = Position::new(
                (x as f32 + unit(sim.config.seed ^ 511, id)) * SPACING,
                (z as f32 + unit(sim.config.seed ^ 517, id)) * SPACING,
            );
            if p.x < 0.
                || p.z < 0.
                || p.x >= sim.grid.width as f32
                || p.z >= sim.grid.height as f32
                || (p.x - center.x).powi(2) + (p.z - center.z).powi(2) > RADIUS * RADIUS
            {
                continue;
            }
            let section = channels::section(sim, p);
            let bank = section.is_some();
            let patch =
                (0.5 + noise(sim.config.seed ^ 529, p.x * 4., p.z * 4.) * 0.5).clamp(0., 1.);
            let roll = unit(sim.config.seed ^ 521, id);
            let (shade, pine) = canopy.at(p);
            let density = if bank {
                0.65
            } else {
                (spatial::sample(&sim.grid, p, |c| c.plants) * 0.85).max(shade * 0.85)
            };
            if roll > density * (0.3 + patch * 0.7) {
                continue;
            }
            let ground = super::point(sim, p);
            if ground.y < sim.sea_level + metres(0.3) {
                continue;
            }
            let wet_height = section.map(|s| ground.y - channels::level(sim, s.center));
            if wet_height.is_some_and(|h| h < metres(0.04)) {
                continue;
            }
            // Read the physical surface so plants do not grow sideways out of cliffs.
            let d = metres(0.35);
            let h = |x, z| genesis_sim_core::surface::elevation(sim, Position::new(x, z));
            let dx = (h(p.x + d, p.z) - h(p.x - d, p.z)) / (2. * d);
            let dz = (h(p.x, p.z + d) - h(p.x, p.z - d)) / (2. * d);
            let slope = (dx * dx + dz * dz).sqrt();
            if slope > 0.85 {
                continue;
            }
            let variation = unit(sim.config.seed ^ 523, id);
            let temperature = sim.geography.climate.sea_level_temperature - ground.y * 20. * 0.0065;
            let kind = if shade > 0.08 && variation < shade * 0.85 && slope < 0.55 {
                if pine { 4 } else { 3 }
            } else if (bank && variation < 0.38) || (!bank && variation < 0.07 + slope * 0.12) {
                2
            } else if wet_height.is_some_and(|h| h < metres(0.85))
                && slope < 0.55
                && temperature > 5.
                && sim.geography.climate.moisture_scale > 0.2
            {
                1
            } else {
                0
            };
            if kind == 0 && unit(sim.config.seed ^ 547, id) < shade * 0.65 {
                continue;
            }
            if kind == 2 && slope > 0.25 {
                continue;
            }
            // Horizontal mesh reach, including scale and a small wind allowance.
            let reach = metres(match kind {
                1 => 0.30,
                2 => 0.11,
                3 | 4 => 0.22,
                _ => 0.19,
            }) * (0.65 + variation * 0.8)
                + metres(0.05);
            if clearance.blocks(p, reach, reach) {
                continue;
            }
            let moisture = spatial::sample(&sim.grid, p, |c| c.moisture);
            let dry = ((1. - moisture) * sim.geography.climate.summer_drought
                + (patch - 0.5) * 0.3)
                .clamp(0., 1.);
            let tint = if kind == 2 {
                match sim.subsurface.reading(p).rock {
                    Rock::Limestone => [0.48, 0.44, 0.34],
                    Rock::Sandstone => [0.43, 0.29, 0.18],
                    Rock::Basalt => [0.19, 0.20, 0.18],
                }
            } else if kind >= 3 {
                let shade = 0.78 + variation * 0.40;
                [shade, shade, shade]
            } else {
                [
                    0.90 + variation * 0.2 + dry * 0.8,
                    0.95 + variation * 0.1 - dry * 0.12,
                    0.95 + dry * 0.18,
                ]
            };
            out[kind].push(Instance {
                position_scale: [
                    ground.x,
                    ground.y
                        + metres(match kind {
                            2 => -0.04,
                            3 | 4 => 0.008,
                            _ => -0.015,
                        }),
                    ground.z,
                    0.65 + variation * 0.8,
                ],
                tint_rotation: [
                    tint[0],
                    tint[1],
                    tint[2],
                    unit(sim.config.seed ^ 541, id) * std::f32::consts::TAU,
                ],
                // Material 12 decodes this pair as ground slope, then supplies
                // its own full coverage interval before the shared distance fade.
                coverage: if kind >= 3 { [dx, dz] } else { [0., 1.] },
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> SimState {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        sim.geography.version = 8;
        for c in sim.grid.cells_mut() {
            c.elevation = 2.;
            c.plants = 0.7;
            c.moisture = 0.6;
        }
        sim.geography
            .rivers
            .push(genesis_sim_core::worldgen::River {
                points: vec![Position::new(2., 8.), Position::new(14., 8.)],
                width: metres(4.),
                widths: vec![],
                bounds: None,
            });
        sim.geography.channels = channels::Index::new(16, 16, &sim.geography.rivers);
        for (x, kind) in [
            (7., genesis_sim_core::ecology::CoverKind::Broadleaf),
            (9., genesis_sim_core::ecology::CoverKind::Pine),
        ] {
            sim.geography.cover.push(genesis_sim_core::worldgen::Cover {
                position: Position::new(x, 9.5),
                size: 1.,
                kind,
                seed: 1,
            });
        }
        sim
    }
    #[test]
    fn cover_is_bounded_repeatable_and_keeps_roots_out_of_water_and_construction() {
        let mut sim = fixture();
        sim.buildings.push(genesis_sim_core::settlement::Building {
            id: 1,
            position: Position::new(8., 8.5),
            rotation: 0.,
        });
        sim.paths.push(genesis_sim_core::settlement::Path {
            points: vec![Position::new(6., 7.5), Position::new(10., 7.5)],
            width: metres(2.),
        });
        let eye = Vec3::new(0., 2.085, 0.5);
        let first = placements(&sim, eye);
        let second = placements(&sim, eye);
        assert!(first.iter().all(|b| !b.is_empty()));
        assert!(first.iter().map(Vec::len).sum::<usize>() < 19_000);
        let clearance = crate::scene::Clearance::new(&sim);
        let crowns = canopy::Canopy::new(&sim, Position::new(8., 8.5), RADIUS, &clearance);
        for (kind, (a, b)) in first.iter().zip(&second).enumerate() {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                assert_eq!(a.position_scale, b.position_scale);
                assert_eq!(a.tint_rotation, b.tint_rotation);
                assert_eq!(a.coverage, b.coverage);
                let p = Position::new(a.position_scale[0] + 8., a.position_scale[2] + 8.);
                if kind >= 3 {
                    let (shade, pine) = crowns.at(p);
                    assert!(shade > 0.08);
                    assert_eq!(pine, kind == 4);
                    assert!(a.coverage[0].hypot(a.coverage[1]) < 0.55);
                }
                assert!(!clearance.blocks(p, 0., 0.));
                if let Some(s) = channels::section(&sim, p) {
                    assert!(
                        genesis_sim_core::surface::elevation(&sim, p)
                            >= channels::level(&sim, s.center) + metres(0.039)
                    );
                }
            }
        }
        assert!(
            placements(&sim, Vec3::new(0., 100., 0.))
                .iter()
                .all(Vec::is_empty)
        );
        // Moving the cached patch does not reseed positions still inside it.
        let moved = placements(&sim, eye + Vec3::X);
        for (a, b) in first.iter().zip(&moved) {
            let positions: std::collections::HashSet<_> = b
                .iter()
                .map(|v| v.position_scale.map(f32::to_bits))
                .collect();
            for v in a.iter().filter(|v| {
                (v.position_scale[0] - 1.).hypot(v.position_scale[2] - 0.5) < metres(38.)
            }) {
                assert!(positions.contains(&v.position_scale.map(f32::to_bits)));
            }
        }
        sim.geography.cover.clear();
        let cleared = placements(&sim, eye);
        assert!(cleared[3].is_empty() && cleared[4].is_empty());
        assert!(!cleared[0].is_empty());
    }
    #[test]
    fn shared_meshes_have_finite_normals_and_human_scale_bounds() {
        for (i, mesh) in meshes().iter().enumerate() {
            assert!(!mesh.is_empty() && mesh.len() <= 200);
            for v in mesh {
                assert!(
                    v.position
                        .iter()
                        .chain(&v.normal)
                        .chain(&v.color)
                        .all(|x| x.is_finite())
                );
                assert!(
                    Vec3::from_array(v.position).length() <= metres(if i == 1 { 1.1 } else { 0.4 })
                );
                assert!((Vec3::from_array(v.normal).length() - 1.).abs() < 1e-4);
            }
        }
    }
}
