//! Leaf-spray distributions shared by every detail level of a crown.
use super::{Mesh, TAU, Vec3, unit, vertex};

pub(super) struct Crown {
    pub center: Vec3,
    pub extent: Vec3,
    pub color: Vec3,
    pub spray_size: f32,
    pub counts: [usize; 3],
}
impl Crown {
    fn emit(&self, out: &mut Mesh, seed: u64, lod: usize, crowns: &[(Crown, u64)], material: f32) {
        let count = self.counts[lod];
        // Keep total card area constant. LOD changes complexity, not canopy mass.
        let size = self.spray_size * (self.counts[0] as f32 / count as f32).sqrt();
        for i in 0..count {
            let id = i as u64;
            let a = unit(seed ^ 17, id) * TAU;
            let y = unit(seed ^ 23, id) * 2. - 1.;
            let radial = (1. - y * y).sqrt();
            let volume = 0.35 + unit(seed ^ 27, id).cbrt() * 0.65;
            let offset = Vec3::new(a.cos() * radial, y, a.sin() * radial) * self.extent * volume;
            let tint = self.color * (0.82 + unit(seed ^ 31, id) * 0.30);
            let p = self.center + offset;
            let normal = (offset / (self.extent * self.extent)).normalize();
            let visibility = super::canopy::openness(p, crowns);
            spray(
                out,
                p,
                seed ^ (id * 101),
                size,
                tint,
                normal,
                material + (1. - visibility) * 0.15,
            );
        }
    }
}

pub(super) fn emit(crowns: &[(Crown, u64)], out: &mut Mesh, lod: usize, pine: bool) {
    for (crown, seed) in crowns {
        crown.emit(out, *seed, lod, crowns, if pine { 5.3 } else { 5. });
    }
}

fn spray(
    out: &mut Mesh,
    p: Vec3,
    seed: u64,
    size: f32,
    color: Vec3,
    crown_normal: Vec3,
    material: f32,
) {
    let a = unit(seed, 1) * TAU;
    // Mixed inclinations give the canopy volume from above and from eye height.
    let ny = 0.12 + unit(seed, 3) * 0.86;
    let radial = (1. - ny * ny).sqrt();
    let n = Vec3::new(a.cos() * radial, ny, a.sin() * radial);
    let right = Vec3::new(-a.sin(), 0., a.cos());
    let up = n.cross(right);
    let roll = unit(seed, 7) * TAU;
    let u = (right * roll.cos() + up * roll.sin()) * size;
    let v = (-right * roll.sin() + up * roll.cos()) * size * 0.52;
    let ps = [p - u - v, p + u - v, p + u + v, p - u + v];
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    // Bend only the lighting normal toward the crown surface. Leaves retain
    // their existing orientation, LOD area compensation and shadow silhouette.
    let facing = if n.dot(crown_normal) >= 0. { n } else { -n };
    let shading_normal = (crown_normal * 0.65 + facing * 0.35).normalize();
    for j in [0, 1, 2, 0, 2, 3] {
        out.push(vertex(ps[j], shading_normal, color, material, uv[j]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crown_lighting_is_stable_across_detail_levels_and_leaf_kinds() {
        let crowns = vec![(
            Crown {
                center: Vec3::new(1., 3., -2.),
                extent: Vec3::new(1.2, 0.8, 1.),
                color: Vec3::new(0.14, 0.23, 0.09),
                spray_size: 0.2,
                counts: [90, 24, 5],
            },
            71,
        )];
        for pine in [false, true] {
            let meshes: [_; 3] = std::array::from_fn(|lod| {
                let mut mesh = Vec::new();
                emit(&crowns, &mut mesh, lod, pine);
                mesh
            });
            let base = if pine { 5.3 } else { 5. };
            for coarse in &meshes[1..] {
                for (a, b) in meshes[0].chunks_exact(6).zip(coarse.chunks_exact(6)) {
                    assert_eq!(a[0].color, b[0].color);
                    assert_eq!(a[0].normal, b[0].normal);
                    let center = |card: &[super::super::Vertex]| {
                        (Vec3::from_array(card[0].position) + Vec3::from_array(card[2].position))
                            * 0.5
                    };
                    assert!((center(a) - center(b)).length() < 1e-6);
                    assert!((base..=base + 0.15001).contains(&a[0].color[3]));
                }
            }
            let min = meshes[0]
                .iter()
                .map(|v| v.color[3])
                .fold(f32::INFINITY, f32::min);
            let max = meshes[0]
                .iter()
                .map(|v| v.color[3])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(max - min > 0.01, "crown depth should affect lighting");
        }
    }
}
