//! Metric branching skeletons, individual leaf sprays and fractured lithologies.
use genesis_renderer::instances::SceneryVertex as Vertex;
use genesis_sim_core::{ecology::CoverKind, worldgen::noise::unit};
use glam::Vec3;
const TAU: f32 = std::f32::consts::TAU;
type Mesh = Vec<Vertex>;
fn vertex(p: Vec3, n: Vec3, color: Vec3, material: f32, uv: [f32; 2]) -> Vertex {
    Vertex {
        position: (p / 20.).to_array(),
        normal: n.to_array(),
        color: [color.x, color.y, color.z, material],
        uv,
    }
}
fn branch(out: &mut Mesh, a: Vec3, b: Vec3, r0: f32, r1: f32, sides: usize) {
    let axis = (b - a).normalize();
    let u = axis
        .cross(if axis.y.abs() < 0.95 {
            Vec3::Y
        } else {
            Vec3::X
        })
        .normalize();
    let v = axis.cross(u);
    for i in 0..sides {
        let n0 =
            u * (i as f32 * TAU / sides as f32).cos() + v * (i as f32 * TAU / sides as f32).sin();
        let n1 = u * ((i + 1) as f32 * TAU / sides as f32).cos()
            + v * ((i + 1) as f32 * TAU / sides as f32).sin();
        let ps = [a + n0 * r0, a + n1 * r0, b + n0 * r1, b + n1 * r1];
        for j in [0, 1, 2, 2, 1, 3] {
            out.push(vertex(
                ps[j],
                if j % 2 == 0 { n0 } else { n1 },
                Vec3::new(0.23, 0.18, 0.115),
                6.,
                [0., 0.],
            ));
        }
    }
}
fn leaf(out: &mut Mesh, p: Vec3, seed: u64, size: f32, color: Vec3) {
    let a = unit(seed, 1) * TAU;
    let u = Vec3::new(a.cos(), 0.15, a.sin()).normalize() * size;
    let v = Vec3::new(-a.sin() * 0.45, 0.75, a.cos() * 0.45).normalize() * size * 0.52;
    let n = u.cross(v).normalize();
    let ps = [p - u - v, p + u - v, p + u + v, p - u + v];
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    for j in [0, 1, 2, 0, 2, 3] {
        out.push(vertex(ps[j], n, color, 5., uv[j]));
    }
}
fn crown(out: &mut Mesh, p: Vec3, extent: Vec3, seed: u64, lod: usize, color: Vec3) {
    if lod == 2 {
        solid(out, p, extent, seed, 5., color, (2, 5));
        return;
    }
    let count = if lod == 0 {
        if extent.min_element() < 0.5 { 130 } else { 220 }
    } else {
        65
    };
    for i in 0..count {
        let id = i as u64;
        let a = unit(seed ^ 17, id) * TAU;
        let y = unit(seed ^ 23, id) * 2. - 1.;
        let radial = (1. - y * y).sqrt();
        let volume = 0.55 + unit(seed ^ 27, id) * 0.45;
        let offset = Vec3::new(a.cos() * radial, y, a.sin() * radial) * extent * volume;
        let tint = color * (0.72 + unit(seed ^ 31, id) * 0.5) * (0.85 + 0.15 * (y + 1.) * 0.5);
        leaf(
            out,
            p + offset,
            seed ^ id,
            if lod == 0 {
                (extent.min_element() * 0.25).clamp(0.055, 0.24)
            } else {
                (extent.min_element() * 0.40).clamp(0.08, 0.43)
            },
            tint,
        );
    }
}
fn solid(
    out: &mut Mesh,
    center: Vec3,
    extent: Vec3,
    seed: u64,
    material: f32,
    color: Vec3,
    detail: (usize, usize),
) {
    let (rings, sides) = detail;
    let point = |r: usize, s: usize| {
        let a = s as f32 * TAU / sides as f32;
        let t = r as f32 * std::f32::consts::PI / rings as f32;
        let n = Vec3::new(t.sin() * a.cos(), t.cos(), t.sin() * a.sin());
        let irregular = 0.80 + unit(seed, (r * sides + s % sides) as u64) * 0.35;
        center + n * extent * irregular
    };
    for r in 0..rings {
        for s in 0..sides {
            let ps = [
                point(r, s),
                point(r, s + 1),
                point(r + 1, s),
                point(r + 1, s + 1),
            ];
            for ids in [[0, 2, 1], [1, 2, 3]] {
                let tri = ids.map(|i| ps[i]);
                let mut n = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize_or_zero();
                if n.dot(tri[0] - center) < 0. {
                    n = -n;
                }
                for p in tri {
                    out.push(vertex(p, n, color, material, [0.5, 0.5]));
                }
            }
        }
    }
}
pub fn build(kind: CoverKind, variant: usize, lod: usize) -> Mesh {
    let mut out = Vec::new();
    let seed = 271 + variant as u64 * 113 + kind.index() as u64 * 47;
    if kind.rock() {
        let (color, material, extent) = match kind {
            CoverKind::Limestone => (Vec3::new(0.55, 0.51, 0.42), 7., Vec3::new(1.45, 0.80, 1.05)),
            CoverKind::Sandstone => (Vec3::new(0.48, 0.32, 0.19), 8., Vec3::new(1.40, 0.65, 1.0)),
            _ => (Vec3::new(0.16, 0.17, 0.16), 9., Vec3::new(1.15, 1.0, 0.95)),
        };
        solid(
            &mut out,
            Vec3::Y * extent.y * 0.25,
            extent,
            seed,
            material,
            color,
            if lod == 0 { (7, 11) } else { (4, 7) },
        );
        return out;
    }
    let shrub = !kind.tree();
    let height = match kind {
        CoverKind::Broadleaf => 7.5,
        CoverKind::Pine => 11.,
        CoverKind::Riparian => 12.,
        CoverKind::EvergreenShrub => 1.5,
        _ => 0.65,
    } * (0.90 + variant as f32 * 0.16);
    let foliage = match kind {
        CoverKind::Broadleaf => Vec3::new(0.14, 0.22, 0.085),
        CoverKind::Pine => Vec3::new(0.11, 0.18, 0.075),
        CoverKind::Riparian => Vec3::new(0.20, 0.32, 0.095),
        CoverKind::EvergreenShrub => Vec3::new(0.18, 0.23, 0.095),
        _ => Vec3::new(0.30, 0.28, 0.13),
    };
    let trunk = if shrub { 0.025 } else { 0.22 };
    let lean = Vec3::new(0.24 * (seed as f32).sin(), 0., 0.16) * if shrub { 0.25 } else { 1. };
    let fork = Vec3::Y * height * if shrub { 0.12 } else { 0.43 } + lean;
    branch(
        &mut out,
        Vec3::new(0., -0.15, 0.),
        fork,
        trunk,
        trunk * 0.65,
        if lod == 0 { 9 } else { 5 },
    );
    if kind == CoverKind::Pine {
        let top = Vec3::Y * height + lean * 2.;
        branch(&mut out, fork, top, trunk * 0.65, 0.025, 6);
    }
    let limbs = if kind == CoverKind::Pine {
        9
    } else if shrub {
        5
    } else {
        7
    };
    for i in 0..limbs {
        let a = i as f32 * 2.399963 + variant as f32;
        let t = i as f32 / limbs as f32;
        let reach = if shrub {
            height * 0.52
        } else if kind == CoverKind::Riparian {
            height * 0.18
        } else {
            height * 0.28
        };
        let y = if kind == CoverKind::Pine {
            height * (0.50 + t * 0.40)
        } else {
            height * (0.64 + unit(seed, i as u64) * 0.19)
        };
        let reach = reach
            * if kind == CoverKind::Pine {
                1. - t * 0.62
            } else {
                1.
            };
        let end = Vec3::new(a.cos() * reach, y, a.sin() * reach) + lean;
        let root = if kind == CoverKind::Pine {
            Vec3::Y * (y - height * 0.08) + lean
        } else {
            fork
        };
        branch(
            &mut out,
            root,
            end,
            trunk * 0.5,
            0.015,
            if lod == 0 { 6 } else { 4 },
        );
        if lod == 0 && !shrub {
            for j in 0..3 {
                let dir = Vec3::new((a + j as f32).cos(), 0.35, (a + j as f32).sin());
                branch(
                    &mut out,
                    root.lerp(end, 0.65),
                    end + dir * 0.7,
                    0.045,
                    0.006,
                    4,
                );
            }
        }
        let extent = if shrub {
            Vec3::new(height * 0.36, height * 0.32, height * 0.36)
        } else if kind == CoverKind::Pine {
            Vec3::new(reach * 0.70, 0.7, reach * 0.70)
        } else if kind == CoverKind::Riparian {
            Vec3::new(1.4, 2.2, 1.4)
        } else {
            Vec3::new(1.7, 1.45, 1.7)
        };
        crown(&mut out, end, extent, seed + i as u64 * 73, lod, foliage);
    }
    if kind == CoverKind::Pine {
        for v in &mut out {
            if v.color[3] == 5. {
                v.color[3] = 5.3;
            }
        }
    }
    out
}
