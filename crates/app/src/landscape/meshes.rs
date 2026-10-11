//! Shared metric mesh primitives, shrubs and fractured lithologies.
mod foliage;
mod trees;
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
    if kind.tree() {
        trees::build(&mut out, kind, seed, variant, lod);
        return out;
    }
    let height = if kind == CoverKind::EvergreenShrub {
        1.5
    } else {
        0.65
    } * (0.90 + variant as f32 * 0.16);
    let color = if kind == CoverKind::EvergreenShrub {
        Vec3::new(0.18, 0.23, 0.095)
    } else {
        Vec3::new(0.30, 0.28, 0.13)
    };
    let root = Vec3::Y * height * 0.12;
    branch(
        &mut out,
        Vec3::new(0., -0.15, 0.),
        root,
        0.025,
        0.016,
        [7, 5, 4][lod],
    );
    for i in 0..5 {
        let a = i as f32 * 2.399963 + variant as f32;
        let tip = Vec3::new(a.cos() * 0.52, 0.64 + unit(seed, i) * 0.19, a.sin() * 0.52) * height;
        branch(&mut out, root, tip, 0.013, 0.005, [5, 4, 3][lod]);
        foliage::Crown {
            center: tip,
            extent: Vec3::new(0.36, 0.32, 0.36) * height,
            color,
            spray_size: (height * 0.09).clamp(0.055, 0.14),
            counts: [130, 40, 12],
        }
        .emit(&mut out, seed + i * 73, lod);
    }
    out
}
