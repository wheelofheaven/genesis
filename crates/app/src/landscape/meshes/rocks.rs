//! Seeded weathered solids sampled from the same surface at every detail level.
use super::{CoverKind, Mesh, Vec3, unit, vertex};
use genesis_sim_core::worldgen::noise::noise;

struct Form {
    seed: u64,
    extent: Vec3,
    power: f32,
    planes: [(Vec3, f32); 4],
}

impl Form {
    fn new(kind: CoverKind, seed: u64) -> Self {
        let (extent, power) = match kind {
            CoverKind::Limestone => (Vec3::new(1.45, 0.80, 1.05), 4.0),
            CoverKind::Sandstone => (Vec3::new(1.40, 0.65, 1.0), 5.5),
            _ => (Vec3::new(1.15, 1.0, 0.95), 4.8),
        };
        let planes = std::array::from_fn(|i| {
            let angle = unit(seed, 401 + i as u64) * super::TAU;
            let y = unit(seed, 411 + i as u64) * 1.6 - 0.8;
            let r = (1. - y * y).sqrt();
            (
                Vec3::new(angle.cos() * r, y, angle.sin() * r),
                0.65 + unit(seed, 421 + i as u64) * 0.22,
            )
        });
        Self {
            seed,
            extent,
            power,
            planes,
        }
    }

    fn point(&self, direction: Vec3) -> Vec3 {
        let d = direction.normalize();
        // A rounded block rather than independently jittered triangle vertices.
        let a = d.abs().powf(self.power);
        let mut radius = (a.x + a.y + a.z).powf(-1. / self.power);
        let erosion = (noise(self.seed, d.x * 1.7 + 5., d.y * 1.7)
            + noise(self.seed ^ 83, d.z * 1.7, d.y * 1.7 + 7.))
            * 0.5;
        radius *= 0.94 + erosion * 0.12;
        for &(normal, offset) in &self.planes {
            let facing = normal.dot(d);
            if facing > 0.1 {
                let cut = offset / facing;
                // Smooth minimum rounds fracture junctions without erasing the
                // broad planar faces. The radius field is independent of LOD.
                let h = (0.025 - (radius - cut).abs()).max(0.) / 0.025;
                radius = radius.min(cut) - h * h * 0.025 * 0.25;
            }
        }
        let mut p = d * radius * self.extent;
        p.y +=
            p.x * (unit(self.seed, 431) - 0.5) * 0.35 + p.z * (unit(self.seed, 433) - 0.5) * 0.22;
        p + Vec3::Y * self.extent.y * 0.25
    }

    fn normal(&self, d: Vec3) -> Vec3 {
        let d = d.normalize();
        let u = d
            .cross(if d.y.abs() < 0.9 { Vec3::Y } else { Vec3::X })
            .normalize();
        let v = d.cross(u);
        let du = self.point(d + u * 0.001) - self.point(d - u * 0.001);
        let dv = self.point(d + v * 0.001) - self.point(d - v * 0.001);
        du.cross(dv).normalize()
    }
}

pub(super) fn build(out: &mut Mesh, kind: CoverKind, seed: u64, lod: usize) {
    let form = Form::new(kind, seed);
    let (color, material) = match kind {
        CoverKind::Limestone => (Vec3::new(0.55, 0.51, 0.42), 7.),
        CoverKind::Sandstone => (Vec3::new(0.48, 0.32, 0.19), 8.),
        _ => (Vec3::new(0.16, 0.17, 0.16), 9.),
    };
    // Nested grids share exact surface samples, including seams and corners.
    // No poles, degenerate triangles, or per-LOD random shape changes.
    let steps = [4, 2, 1][lod];
    for axis in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
        let u = axis.cross(if axis.y.abs() < 0.9 { Vec3::Y } else { Vec3::X });
        let v = axis.cross(u);
        let mut grid = Vec::new();
        for y in 0..=steps {
            for x in 0..=steps {
                let d = axis
                    + u * (2. * x as f32 / steps as f32 - 1.)
                    + v * (2. * y as f32 / steps as f32 - 1.);
                grid.push(vertex(
                    form.point(d),
                    form.normal(d),
                    color,
                    material,
                    [0.5, 0.5],
                ));
            }
        }
        if lod == 2 {
            // Keep each face's centre extremum as well as its corners. A plain
            // eight-corner proxy would visibly shrink a rounded block at range.
            let center = vertex(
                form.point(axis),
                form.normal(axis),
                color,
                material,
                [0.5, 0.5],
            );
            let corners = [0, 1, 3, 2];
            for i in 0..4 {
                out.extend_from_slice(&[grid[corners[i]], grid[corners[(i + 1) % 4]], center]);
            }
            continue;
        }
        for y in 0..steps {
            for x in 0..steps {
                let a = y * (steps + 1) + x;
                for i in [a, a + 1, a + steps + 2, a, a + steps + 2, a + steps + 1] {
                    out.push(grid[i]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rock_levels_share_samples_and_form_closed_outward_surfaces() {
        for kind in [
            CoverKind::Limestone,
            CoverKind::Sandstone,
            CoverKind::Basalt,
        ] {
            for variant in 0..2 {
                let meshes: [_; 3] =
                    std::array::from_fn(|lod| super::super::build(kind, variant, lod));
                assert_eq!(meshes.each_ref().map(|m| m.len()), [576, 144, 72]);
                for lod in 0..3 {
                    let mesh = &meshes[lod];
                    let mut edges = std::collections::BTreeMap::new();
                    let key =
                        |v: &super::super::Vertex| v.position.map(|p| (p * 1e7).round() as i32);
                    for tri in mesh.chunks_exact(3) {
                        let p =
                            [tri[0], tri[1], tri[2]].map(|v| Vec3::from_array(v.position) * 20.);
                        let n = (p[1] - p[0]).cross(p[2] - p[0]);
                        assert!(n.length() > 1e-5);
                        let center = p.into_iter().sum::<Vec3>() / 3.;
                        assert!(
                            n.dot(
                                center
                                    - Vec3::Y
                                        * if kind == CoverKind::Basalt {
                                            0.25
                                        } else if kind == CoverKind::Sandstone {
                                            0.1625
                                        } else {
                                            0.20
                                        }
                            ) > 0.
                        );
                        for i in 0..3 {
                            let a = key(&tri[i]);
                            let b = key(&tri[(i + 1) % 3]);
                            let edge = if a < b { (a, b) } else { (b, a) };
                            *edges.entry(edge).or_insert(0) += 1;
                        }
                    }
                    assert!(
                        edges.values().all(|&count| count == 2),
                        "watertight cube seams"
                    );
                    for v in mesh {
                        let n = Vec3::from_array(v.normal);
                        assert!((n.length() - 1.).abs() < 1e-4);
                        let p = Vec3::from_array(v.position) * 20.;
                        assert!(p.x.hypot(p.z) < 1.7 && p.length() < 2.5);
                        if lod > 0 {
                            assert!(
                                meshes[lod - 1]
                                    .iter()
                                    .any(|fine| fine.position == v.position
                                        && fine.normal == v.normal)
                            );
                        }
                    }
                }
            }
        }
    }
}
