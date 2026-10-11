//! Cosmetic growth forms in metres; habitat and persistent placement live in sim_core.
use super::{CoverKind, Mesh, TAU, Vec3, branch, foliage::Crown, unit};

pub(super) fn build(out: &mut Mesh, kind: CoverKind, seed: u64, variant: usize, lod: usize) {
    let height = match kind {
        CoverKind::Pine => 11.,
        CoverKind::Riparian => 12.,
        _ => 7.5,
    } * (0.90 + variant as f32 * 0.16);
    let trunk_radius = if kind == CoverKind::Broadleaf {
        0.26
    } else {
        0.20
    };
    let sides = [9, 6, 4][lod];
    let bend = Vec3::new(unit(seed, 61) - 0.5, 0., unit(seed, 63) - 0.5) * 0.75;
    let stem = |t: f32| Vec3::Y * height * t + bend * (t * t * 2.);
    // Buried flare and several tapered segments avoid a straight cylindrical pole.
    let joints = if kind == CoverKind::Broadleaf {
        [0., 0.08, 0.22, 0.38, 0.55]
    } else {
        [0., 0.10, 0.35, 0.65, 0.98]
    };
    for (i, pair) in joints.windows(2).enumerate() {
        let radius = |t: f32| trunk_radius * (1. - t).powf(1.2).max(0.06);
        branch(
            out,
            stem(pair[0]) - Vec3::Y * if i == 0 { 0.18 } else { 0. },
            stem(pair[1]),
            radius(pair[0]) * if i == 0 { 1.5 } else { 1. },
            radius(pair[1]),
            sides,
        );
    }
    let mut crowns = Vec::new();
    if kind == CoverKind::Pine {
        pine(out, height, seed, lod, stem, &mut crowns);
    } else {
        deciduous(out, height, kind, seed, lod, stem, &mut crowns);
    }
    super::foliage::emit(&crowns, out, lod, kind == CoverKind::Pine);
}

fn pine(
    out: &mut Mesh,
    height: f32,
    seed: u64,
    lod: usize,
    stem: impl Fn(f32) -> Vec3,
    crowns: &mut Vec<(Crown, u64)>,
) {
    // Four asymmetric limbs around each tier make a complete tapered crown.
    for tier in 0..6 {
        let t = tier as f32 / 5.;
        for limb in 0..4 {
            let id = (tier * 4 + limb) as u64;
            let anchor = stem(0.32 + t * 0.58 + (unit(seed, id + 71) - 0.5) * 0.035);
            let a = limb as f32 * TAU / 4. + tier as f32 * 1.17 + (unit(seed, id + 90) - 0.5) * 0.6;
            let direction = Vec3::new(a.cos(), 0., a.sin());
            let reach = height * (0.26 - t * 0.20) * (0.80 + unit(seed, id + 140) * 0.35);
            let elbow = anchor + direction * reach * 0.55 - Vec3::Y * height * 0.018;
            let tip = anchor + direction * reach + Vec3::Y * height * (0.035 + t * 0.025);
            let radius = 0.085 * (1. - t * 0.65);
            branch(out, anchor, elbow, radius, radius * 0.50, [6, 4, 3][lod]);
            if lod < 2 {
                branch(out, elbow, tip, radius * 0.50, 0.008, [5, 3][lod]);
            }
            crowns.push((
                Crown {
                    center: elbow.lerp(tip, 0.55),
                    extent: Vec3::new(reach * 0.60 + 0.18, 0.65 - t * 0.22, reach * 0.60 + 0.18),
                    color: Vec3::new(0.12, 0.20, 0.085),
                    spray_size: 0.28 - t * 0.10,
                    counts: [90, 24, 5],
                },
                seed + id * 73,
            ));
        }
    }
    crowns.push((
        Crown {
            center: stem(0.96),
            extent: Vec3::new(0.40, 0.70, 0.40),
            color: Vec3::new(0.15, 0.23, 0.10),
            spray_size: 0.20,
            counts: [90, 24, 5],
        },
        seed ^ 812,
    ));
}

fn deciduous(
    out: &mut Mesh,
    height: f32,
    kind: CoverKind,
    seed: u64,
    lod: usize,
    stem: impl Fn(f32) -> Vec3,
    crowns: &mut Vec<(Crown, u64)>,
) {
    let riparian = kind == CoverKind::Riparian;
    for limb in 0..7 {
        let id = limb as u64;
        let a = limb as f32 * 2.399963 + unit(seed, 80) * TAU;
        let direction = Vec3::new(a.cos(), 0., a.sin());
        let rise = if riparian {
            0.42 + limb as f32 * 0.075
        } else {
            0.59 + unit(seed, id + 81) * 0.15
        };
        let reach =
            height * if riparian { 0.16 } else { 0.29 } * (0.75 + unit(seed, id + 101) * 0.4);
        let root = stem(if riparian {
            rise - 0.16
        } else {
            0.30 + limb as f32 * 0.025
        });
        let end = stem(rise) + direction * reach;
        let elbow = root.lerp(end, 0.55) + Vec3::Y * height * 0.055;
        branch(out, root, elbow, 0.105, 0.060, [7, 5, 3][lod]);
        branch(out, elbow, end, 0.060, 0.020, [6, 4, 3][lod]);
        for twig in 0..3 {
            let k = id * 3 + twig as u64;
            let angle = a + (twig as f32 - 1.) * 1.1;
            let tip = end
                + Vec3::new(angle.cos(), 0.7 + unit(seed, k + 191) * 0.5, angle.sin())
                    * height
                    * 0.095;
            if lod < 2 {
                branch(out, elbow.lerp(end, 0.75), tip, 0.030, 0.005, [4, 3][lod]);
            }
            crowns.push((
                Crown {
                    center: tip,
                    extent: if riparian {
                        Vec3::new(0.90, 1.45, 0.90)
                    } else {
                        Vec3::new(1.15, 0.95, 1.15)
                    },
                    color: if riparian {
                        Vec3::new(0.18, 0.29, 0.105)
                    } else {
                        Vec3::new(0.14, 0.23, 0.09)
                    },
                    spray_size: if riparian { 0.26 } else { 0.29 },
                    counts: [90, 24, 5],
                },
                seed + k * 73,
            ));
        }
    }
    // Upright leaders close the canopy above the fork; radial limbs alone leave
    // an artificial ring-shaped hole when the tree is viewed from above.
    for leader in 0..if riparian { 1 } else { 3 } {
        let a = leader as f32 * TAU / 3. + unit(seed, 341) * TAU;
        let root = stem(if riparian { 0.82 } else { 0.50 });
        let tip = stem(if riparian { 0.95 } else { 0.82 })
            + Vec3::new(a.cos(), 0., a.sin()) * if riparian { 0.15 } else { 0.65 };
        branch(out, root, tip, 0.055, 0.008, [6, 4, 3][lod]);
        crowns.push((
            Crown {
                center: tip,
                extent: if riparian {
                    Vec3::new(0.90, 1.25, 0.90)
                } else {
                    Vec3::new(1.15, 1.0, 1.15)
                },
                color: if riparian {
                    Vec3::new(0.18, 0.29, 0.105)
                } else {
                    Vec3::new(0.14, 0.23, 0.09)
                },
                spray_size: if riparian { 0.26 } else { 0.29 },
                counts: [90, 24, 5],
            },
            seed + 853 + leader as u64 * 73,
        ));
    }
}
