//! Adaptive dyadic patches. Every shared edge includes the same vertices on
//! both sides, including transitions; no skirts or cracks conceal mismatches.
use genesis_sim_core::{SimState, spatial::Position, surface::SUBDIVISIONS};
use glam::Vec3;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key(pub [i32; 3]);

/// Filter sub-metre channel detail before it aliases into coarse regional cells.
/// Shared by terrain height, bank colour and the distant water approximation.
pub fn channel_simplification(sim: &SimState, eye: Vec3, p: Position) -> f32 {
    let x = eye.x + sim.grid.width as f32 * 0.5;
    let z = eye.z + sim.grid.height as f32 * 0.5;
    let base = sim.grid.get(p.x as i32, p.z as i32).elevation;
    let distance = ((p.x - x).powi(2) + (p.z - z).powi(2) + (base - eye.y).powi(2)).sqrt();
    let t = ((distance - 8.) / 3.).clamp(0., 1.);
    let edge =
        p.x.min(p.z)
            .min(sim.grid.width as f32 - p.x)
            .min(sim.grid.height as f32 - p.z)
            .clamp(0., 1.);
    (t * t * (3. - 2. * t)).max(1. - edge)
}
impl Key {
    pub fn at(eye: Vec3) -> Self {
        // 20 m horizontal buckets keep topology stable during small movements.
        Self([
            eye.x.floor() as i32,
            (eye.y / 2.).floor() as i32,
            eye.z.floor() as i32,
        ])
    }
    pub fn eye(self) -> Vec3 {
        Vec3::new(
            self.0[0] as f32 + 0.5,
            self.0[1] as f32 * 2. + 1.,
            self.0[2] as f32 + 0.5,
        )
    }
}

#[derive(Clone, Copy)]
struct Leaf {
    x: u32,
    z: u32,
    size: u32,
}

fn split(leaf: Leaf, eye: Vec3, base: f32, channel: bool, leaves: &mut Vec<Leaf>) {
    let d = SUBDIVISIONS as f32;
    let dx = (leaf.x as f32 / d - eye.x)
        .max(eye.x - (leaf.x + leaf.size) as f32 / d)
        .max(0.);
    let dz = (leaf.z as f32 / d - eye.z)
        .max(eye.z - (leaf.z + leaf.size) as f32 / d)
        .max(0.);
    let distance = dx * dx + dz * dz + (eye.y - base).powi(2);
    if leaf.size > 1
        && (distance < (leaf.size as f32 / d * 60.).powi(2)
            || (channel && leaf.size > 2 && distance < 12_f32.powi(2)))
    {
        let size = leaf.size / 2;
        for (x, z) in [(0, 0), (size, 0), (0, size), (size, size)] {
            split(
                Leaf {
                    x: leaf.x + x,
                    z: leaf.z + z,
                    size,
                },
                eye,
                base,
                channel,
                leaves,
            );
        }
    } else {
        leaves.push(leaf);
    }
}

fn edge(a: (u32, u32), b: (u32, u32), corners: &HashSet<(u32, u32)>, out: &mut Vec<(u32, u32)>) {
    let middle = ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
    if middle != a && middle != b && corners.contains(&middle) {
        edge(a, middle, corners, out);
        edge(middle, b, corners, out);
    } else {
        out.push(a);
    }
}

pub type Patches = HashMap<(u32, u32, u32), std::ops::Range<usize>>;
pub fn topology(sim: &SimState, key: Key) -> (Vec<Position>, Vec<[usize; 3]>, Patches) {
    let eye = key.eye() + Vec3::new(sim.grid.width as f32 / 2., 0., sim.grid.height as f32 / 2.);
    let mut leaves = Vec::new();
    let step = if sim.geography.version == 0 {
        SUBDIVISIONS / 2
    } else {
        SUBDIVISIONS
    };
    for z in (0..sim.grid.height * SUBDIVISIONS).step_by(step as usize) {
        for x in (0..sim.grid.width * SUBDIVISIONS).step_by(step as usize) {
            let leaf = Leaf { x, z, size: step };
            // Keep regional boundary vertices compatible with the outer context.
            if sim.geography.version == 0
                || x == 0
                || z == 0
                || x + step == sim.grid.width * SUBDIVISIONS
                || z + step == sim.grid.height * SUBDIVISIONS
            {
                leaves.push(leaf);
            } else {
                let base = sim
                    .grid
                    .get((x / SUBDIVISIONS) as i32, (z / SUBDIVISIONS) as i32)
                    .elevation;
                let channel = sim.geography.channels.contains(Position::new(
                    x as f32 / step as f32,
                    z as f32 / step as f32,
                ));
                split(leaf, eye, base, channel, &mut leaves);
            }
        }
    }
    let mut corners = HashSet::new();
    for leaf in &leaves {
        for (x, z) in [
            (0, 0),
            (0, leaf.size),
            (leaf.size, leaf.size),
            (leaf.size, 0),
        ] {
            corners.insert((leaf.x + x, leaf.z + z));
        }
    }
    let mut ids = HashMap::new();
    let mut points = Vec::new();
    let mut index = |p: (u32, u32)| {
        *ids.entry(p).or_insert_with(|| {
            let id = points.len();
            points.push(Position::new(
                p.0 as f32 / SUBDIVISIONS as f32,
                p.1 as f32 / SUBDIVISIONS as f32,
            ));
            id
        })
    };
    let mut triangles = Vec::new();
    let mut patches = HashMap::new();
    for leaf in leaves {
        let start = triangles.len();
        let p = [
            (leaf.x, leaf.z),
            (leaf.x, leaf.z + leaf.size),
            (leaf.x + leaf.size, leaf.z + leaf.size),
            (leaf.x + leaf.size, leaf.z),
        ];
        let mut perimeter = Vec::new();
        for i in 0..4 {
            edge(p[i], p[(i + 1) % 4], &corners, &mut perimeter);
        }
        if perimeter.len() == 4 {
            let q = p.map(&mut index);
            triangles.extend([[q[0], q[1], q[3]], [q[3], q[1], q[2]]]);
        } else {
            let center = index((leaf.x + leaf.size / 2, leaf.z + leaf.size / 2));
            for i in 0..perimeter.len() {
                triangles.push([
                    center,
                    index(perimeter[i]),
                    index(perimeter[(i + 1) % perimeter.len()]),
                ]);
            }
        }
        patches.insert((leaf.x, leaf.z, leaf.size), start..triangles.len());
    }
    (points, triangles, patches)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adaptive_surface_has_shared_edges_complete_coverage_and_a_bounded_budget() {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 128,
            height: 96,
            ..Default::default()
        });
        sim.geography.version = 6;
        for c in sim.grid.cells_mut() {
            c.elevation = 2.;
        }
        for eye in [
            Vec3::new(0., 2.085, 0.),
            Vec3::new(1.1, 2.085, -0.9),
            Vec3::new(0., 100., 0.),
        ] {
            let (points, tris, _) = topology(&sim, Key::at(eye));
            assert!(tris.len() < 400_000, "{} triangles", tris.len());
            let mut edges = HashMap::new();
            let mut area = 0.;
            for t in tris {
                let [a, b, c] = t.map(|i| points[i]);
                let signed = (b.z - a.z) * (c.x - a.x) - (b.x - a.x) * (c.z - a.z);
                assert!(signed > 0.);
                area += signed * 0.5;
                for i in 0..3 {
                    let a = t[i].min(t[(i + 1) % 3]);
                    let b = t[i].max(t[(i + 1) % 3]);
                    *edges.entry((a, b)).or_insert(0) += 1;
                }
            }
            assert!((area - sim.grid.width as f32 * sim.grid.height as f32).abs() < 0.01);
            for ((a, b), count) in edges {
                let a = points[a];
                let b = points[b];
                let boundary = (a.x == b.x && (a.x == 0. || a.x == sim.grid.width as f32))
                    || (a.z == b.z && (a.z == 0. || a.z == sim.grid.height as f32));
                assert_eq!(count, if boundary { 1 } else { 2 });
            }
        }
    }
}
