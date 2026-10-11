//! Shared metric mesh primitives, shrubs and fractured lithologies.
mod bark;
mod canopy;
mod foliage;
mod rocks;
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
use bark::branch;
pub fn build(kind: CoverKind, variant: usize, lod: usize) -> Mesh {
    let mut out = Vec::new();
    let seed = 271 + variant as u64 * 113 + kind.index() as u64 * 47;
    if kind.rock() {
        rocks::build(&mut out, kind, seed, lod);
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
    let mut crowns = Vec::new();
    for i in 0..5 {
        let a = i as f32 * 2.399963 + variant as f32;
        let tip = Vec3::new(a.cos() * 0.52, 0.64 + unit(seed, i) * 0.19, a.sin() * 0.52) * height;
        branch(&mut out, root, tip, 0.013, 0.005, [5, 4, 3][lod]);
        crowns.push((
            foliage::Crown {
                center: tip,
                extent: Vec3::new(0.36, 0.32, 0.36) * height,
                color,
                spray_size: (height * 0.09).clamp(0.055, 0.14),
                counts: [130, 40, 12],
            },
            seed + i * 73,
        ));
    }
    foliage::emit(&crowns, &mut out, lod, false);
    out
}
