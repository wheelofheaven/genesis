//! Procedural foliage lobes and weathered rocks at human scale.
use crate::terrain;
use genesis_renderer::geometry::{Vertex, cuboid};
use genesis_sim_core::SimState;
use glam::Vec3;

fn stone(out: &mut Vec<Vertex>, center: Vec3, radius: f32, height: f32, color: Vec3) {
    // Irregular latitude rings remove the old diamond/bipyramid silhouette.
    let vertex = |ring: usize, segment: usize| {
        let a = segment as f32 * std::f32::consts::TAU / 7.;
        let t = ring as f32 * std::f32::consts::PI / 3.;
        let variation = 1. + 0.10 * (a * 3. + center.x * 7.).sin() * t.sin();
        let normal = Vec3::new(t.sin() * a.cos(), t.cos(), t.sin() * a.sin());
        let p = center + normal * Vec3::new(radius * variation, height, radius * variation);
        let shade = 0.53 + 0.47 * normal.dot(Vec3::new(-0.5, 0.75, 0.3).normalize()).max(0.);
        let tint = color * shade;
        Vertex {
            position: p.to_array(),
            color: [tint.x, tint.y, tint.z, 1.],
        }
    };
    for ring in 0..3 {
        for i in 0..7 {
            let a = vertex(ring, i);
            let b = vertex(ring, i + 1);
            let c = vertex(ring + 1, i);
            let d = vertex(ring + 1, i + 1);
            if ring > 0 {
                out.extend_from_slice(&[a, b, c]);
            }
            if ring < 2 {
                out.extend_from_slice(&[b, d, c]);
            }
        }
    }
}

pub fn cover(sim: &SimState, out: &mut Vec<Vertex>) {
    for plant in &sim.geography.cover {
        let p = terrain::point(sim, plant.position);
        if p.y < sim.sea_level + 0.1
            || sim.buildings.iter().any(|b| {
                (b.position.x - plant.position.x).powi(2)
                    + (b.position.z - plant.position.z).powi(2)
                    < genesis_sim_core::settlement::Building::TREE_CLEARANCE.powi(2)
            })
        {
            continue;
        }
        let s = plant.size * if plant.rock { 0.18 } else { 0.35 };
        if plant.rock {
            stone(
                out,
                p + Vec3::Y * s * 0.16,
                s * 0.65,
                s * 0.42,
                Vec3::new(0.49, 0.43, 0.33),
            );
        } else {
            cuboid(
                out,
                (p + Vec3::Y * s * 0.55).to_array(),
                [s * 0.12, s * 1.1, s * 0.12],
                [0.25, 0.21, 0.13],
            );
            for lobe in 0..5 {
                let angle = lobe as f32 * 2.399963 + p.x;
                let reach = if lobe == 0 { 0. } else { s * 0.43 };
                let center = p + Vec3::new(
                    angle.cos() * reach,
                    s * (1.05 + 0.16 * (angle * 1.3).sin()),
                    angle.sin() * reach,
                );
                stone(
                    out,
                    center,
                    s * (0.42 + 0.06 * angle.sin()),
                    s * 0.38,
                    Vec3::new(0.20, 0.28, 0.115) * (0.93 + 0.07 * angle.cos()),
                );
            }
        }
    }
}
