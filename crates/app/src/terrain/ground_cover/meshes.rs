//! Small shared geometry in metres: blades, pebbles and curled fallen leaves.
use genesis_renderer::instances::SceneryVertex;
use genesis_sim_core::units::metres;
use glam::Vec3;

pub fn build() -> [Vec<SceneryVertex>; super::BATCHES] {
    [
        blades(false),
        blades(true),
        pebble(),
        litter(false),
        litter(true),
    ]
}
fn triangle(out: &mut Vec<SceneryVertex>, points: [Vec3; 3], color: [f32; 3], material: f32) {
    let normal = (points[1] - points[0])
        .cross(points[2] - points[0])
        .normalize_or_zero();
    for p in points {
        let light = if material < 11. {
            0.76 + (p.y / metres(0.9)).clamp(0., 1.) * 0.35
        } else {
            1.
        };
        out.push(SceneryVertex {
            position: p.to_array(),
            normal: normal.to_array(),
            color: [
                color[0] * light,
                color[1] * light,
                color[2] * light,
                material,
            ],
            uv: [0., 0.],
        });
    }
}
fn blades(rush: bool) -> Vec<SceneryVertex> {
    let mut out = Vec::new();
    let count = if rush { 9 } else { 6 };
    let steps = if rush { 4 } else { 3 };
    for i in 0..count {
        let angle = i as f32 * 2.399963;
        let direction = Vec3::new(angle.cos(), 0., angle.sin());
        let side = Vec3::new(-angle.sin(), 0., angle.cos());
        let root = direction * metres(0.015 + i as f32 * 0.006);
        let height = metres(if rush {
            0.55 + (i % 4) as f32 * 0.13
        } else {
            0.17 + (i % 3) as f32 * 0.055
        });
        let bend = metres(if rush { 0.2 } else { 0.12 });
        let width = metres(if rush { 0.012 } else { 0.018 });
        let cross = |t: f32| {
            let center = root + Vec3::Y * height * t + direction * bend * t * t;
            [
                center - side * width * (1. - t),
                center + side * width * (1. - t),
            ]
        };
        for j in 0..steps {
            let a = cross(j as f32 / steps as f32);
            let b = cross((j + 1) as f32 / steps as f32);
            let color = if rush {
                [0.20, 0.29, 0.09]
            } else {
                [0.22, 0.29, 0.09]
            };
            triangle(&mut out, [a[0], a[1], b[0]], color, 10.);
            if j + 1 < steps {
                triangle(&mut out, [a[1], b[1], b[0]], color, 10.);
            }
        }
    }
    out
}
fn pebble() -> Vec<SceneryVertex> {
    let mut out = Vec::new();
    let ring = |i: usize, upper: bool| {
        let a = i as f32 * std::f32::consts::TAU / 8.;
        let radius = metres(if upper { 0.07 } else { 0.11 });
        Vec3::new(
            a.cos() * radius,
            metres(if upper { 0.065 } else { 0.008 }),
            a.sin() * radius * 0.73,
        )
    };
    for i in 0..8 {
        let a = ring(i, false);
        let b = ring((i + 1) % 8, false);
        let c = ring(i, true);
        let d = ring((i + 1) % 8, true);
        for tri in [[a, c, b], [b, c, d], [c, Vec3::Y * metres(0.084), d]] {
            triangle(&mut out, tri, [1.; 3], 11.);
        }
    }
    out
}

fn litter(needles: bool) -> Vec<SceneryVertex> {
    let mut out = Vec::new();
    for i in 0..if needles { 12 } else { 6 } {
        let scatter = i as f32 * 2.399963;
        let a = genesis_sim_core::worldgen::noise::unit(919, i) * std::f32::consts::TAU;
        let direction = Vec3::new(a.cos(), 0., a.sin());
        let side = Vec3::new(-a.sin(), 0., a.cos());
        let center =
            Vec3::new(scatter.cos(), 0., scatter.sin()) * metres(0.035 + (i % 4) as f32 * 0.04);
        let along = direction * metres(if needles { 0.055 } else { 0.045 });
        let across = side * metres(if needles { 0.0025 } else { 0.026 });
        let ridge = center + Vec3::Y * metres(if needles { 0.004 } else { 0.012 });
        let tip = center + along + Vec3::Y * metres(0.005);
        let base = center - along;
        let tint = 0.80 + (i % 3) as f32 * 0.17;
        let color = [0.30 * tint, 0.19 * tint, 0.075 * tint];
        if needles {
            triangle(&mut out, [base, center - across, tip], color, 12.);
            triangle(&mut out, [base, tip, center + across], color, 12.);
        } else {
            let edge = |j: usize| {
                let t = j as f32 * std::f32::consts::TAU / 8.;
                center
                    + along * t.cos()
                    + across * t.sin()
                    + Vec3::Y * metres(0.005 * t.cos().max(0.))
            };
            for j in 0..8 {
                triangle(&mut out, [edge(j), edge(j + 1), ridge], color, 12.);
            }
        }
    }
    out
}
