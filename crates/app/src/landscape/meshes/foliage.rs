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
    pub fn emit(&self, out: &mut Mesh, seed: u64, lod: usize) {
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
            let tint =
                self.color * (0.82 + unit(seed ^ 31, id) * 0.30) * (0.78 + 0.22 * (y + 1.) * 0.5);
            spray(out, self.center + offset, seed ^ (id * 101), size, tint);
        }
    }
}

fn spray(out: &mut Mesh, p: Vec3, seed: u64, size: f32, color: Vec3) {
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
    for j in [0, 1, 2, 0, 2, 3] {
        out.push(vertex(ps[j], n, color, 5., uv[j]));
    }
}
