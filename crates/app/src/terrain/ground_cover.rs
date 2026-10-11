//! Bounded cosmetic detail near the observer, anchored to stable world cells.
use genesis_renderer::{geometry::Vertex, instances::Instance};
use genesis_sim_core::{
    SimState,
    spatial::{self, Position},
    units::metres,
    worldgen::noise::unit,
};
use glam::Vec3;

pub fn mesh() -> Vec<Vertex> {
    let mut out = Vec::new();
    for i in 0..5 {
        let angle = i as f32 * 2.399963;
        let root = Vec3::new(angle.cos(), 0., angle.sin()) * metres(0.045);
        let side = Vec3::new(-angle.sin(), 0., angle.cos()) * metres(0.012);
        let tip = root
            + Vec3::new(
                angle.cos() * metres(0.09),
                metres(0.18 + i as f32 * 0.025),
                angle.sin() * metres(0.09),
            );
        let points = [root - side, root + side, tip];
        for index in [0, 1, 2, 2, 1, 0] {
            let color = if index == 2 {
                [0.36, 0.36, 0.15, 1.]
            } else {
                [0.20, 0.26, 0.09, 1.]
            };
            out.push(Vertex {
                position: points[index].to_array(),
                color,
            });
        }
    }
    out
}

pub fn placements(sim: &SimState, eye: Vec3) -> Vec<Instance> {
    let center = Position::new(
        eye.x + sim.grid.width as f32 / 2.,
        eye.z + sim.grid.height as f32 / 2.,
    );
    if eye.y - super::point(sim, center).y > metres(80.) {
        return Vec::new();
    }
    let spacing = metres(0.8);
    let cx = (center.x / spacing).floor() as i32;
    let cz = (center.z / spacing).floor() as i32;
    let mut out = Vec::new();
    for z in cz - 48..=cz + 48 {
        for x in cx - 48..=cx + 48 {
            let id = ((x as u32 as u64) << 32) | z as u32 as u64;
            let p = Position::new(
                (x as f32 + unit(sim.config.seed ^ 511, id)) * spacing,
                (z as f32 + unit(sim.config.seed ^ 517, id)) * spacing,
            );
            if p.x < 0. || p.z < 0. || p.x >= sim.grid.width as f32 || p.z >= sim.grid.height as f32
            {
                continue;
            }
            let distance = ((p.x - center.x).powi(2) + (p.z - center.z).powi(2)).sqrt();
            if distance > metres(38.)
                || unit(sim.config.seed ^ 521, id)
                    > spatial::sample(&sim.grid, p, |c| c.plants) * 0.8
            {
                continue;
            }
            let ground = super::point(sim, p);
            if ground.y < sim.sea_level + metres(1.)
                || sim.buildings.iter().any(|b| {
                    (p.x - b.position.x).powi(2) + (p.z - b.position.z).powi(2)
                        < metres(4.5).powi(2)
                })
                || sim
                    .paths
                    .iter()
                    .any(|path| genesis_sim_core::paths::on_path(path, p, metres(0.15)))
                || genesis_sim_core::worldgen::river_distance(sim, p) < metres(7.)
            {
                continue;
            }
            let fade = ((metres(38.) - distance) / metres(10.)).clamp(0., 1.);
            let variation = unit(sim.config.seed ^ 523, id);
            let moisture = spatial::sample(&sim.grid, p, |c| c.moisture);
            let dry = ((0.65 - moisture) * sim.geography.climate.summer_drought).clamp(0., 0.5);
            out.push(Instance {
                position_scale: [
                    ground.x,
                    ground.y - metres(0.01),
                    ground.z,
                    (0.65 + variation * 0.8) * fade,
                ],
                tint_rotation: [
                    0.85 + variation * 0.3 + dry * 0.65,
                    0.9 + variation * 0.2 - dry * 0.10,
                    1.,
                    variation * std::f32::consts::TAU,
                ],
            });
        }
    }
    out
}
