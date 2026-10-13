//! Retessellate changed roots and their one-root seam neighborhood. Flat buffers
//! are compacted in leaf/triangle order; storage IDs may change, receiver order may not.
use super::*;
use std::collections::{BTreeMap, HashMap, HashSet};

pub(super) struct Geometry {
    pub samples: Vec<Sample>,
    pub indices: Arc<Vec<[usize; 3]>>,
    pub patches: Arc<lod::Patches>,
    pub leaves: Arc<Vec<lod::Leaf>>,
}

struct Assembly<'a> {
    cache: &'a Cache,
    sim: &'a SimState,
    affected: Affected,
    remap: Vec<usize>,
    local: HashMap<(u32, u32), usize>,
    added: HashMap<(u32, u32), usize>,
    samples: Vec<Sample>,
}
impl Assembly<'_> {
    fn old(&mut self, id: usize) -> usize {
        if self.remap[id] == usize::MAX {
            let sample = self.cache.samples[id];
            self.remap[id] = self.samples.len();
            self.samples.push(if self.affected.contains(sample.0) {
                sample_geometry(self.sim, self.cache.key, sample.0)
            } else {
                sample
            });
        }
        self.remap[id]
    }
    fn point(&mut self, p: (u32, u32)) -> usize {
        if let Some(&id) = self.local.get(&p) {
            return self.old(id);
        }
        if let Some(&id) = self.added.get(&p) {
            return id;
        }
        let id = self.samples.len();
        self.samples.push(sample_geometry(
            self.sim,
            self.cache.key,
            Position::new(p.0 as f32 / 32., p.1 as f32 / 32.),
        ));
        self.added.insert(p, id);
        id
    }
}

pub(super) fn rebuild(
    cache: &Cache,
    sim: &SimState,
    affected: Affected,
    changed: BTreeMap<(u32, u32), Vec<lod::Leaf>>,
) -> Geometry {
    let min = changed
        .keys()
        .fold([u32::MAX; 2], |a, &(x, z)| [a[0].min(x), a[1].min(z)]);
    let max = changed
        .keys()
        .fold([0; 2], |a, &(x, z)| [a[0].max(x), a[1].max(z)]);
    let inside = |leaf: lod::Leaf, halo: u32| {
        let (x, z) = leaf.root();
        x >= min[0].saturating_sub(halo)
            && z >= min[1].saturating_sub(halo)
            && x <= max[0].saturating_add(halo)
            && z <= max[1].saturating_add(halo)
    };
    let mut leaves = Vec::with_capacity(cache.leaves.len());
    let mut previous = None;
    for &leaf in cache.leaves.iter() {
        let root = leaf.root();
        if let Some(replacement) = changed.get(&root) {
            if previous != Some(root) {
                leaves.extend_from_slice(replacement);
            }
        } else {
            leaves.push(leaf);
        }
        previous = Some(root);
    }
    // A rebuilt leaf sees corners from all touching neighbors, including the
    // outer edge of the seam ring. No triangulation-generated centers enter this set.
    let corners: HashSet<_> = leaves
        .iter()
        .filter(|&&leaf| inside(leaf, 2))
        .flat_map(|leaf| leaf.corners())
        .collect();
    let local = cache
        .samples
        .iter()
        .enumerate()
        .filter_map(|(id, sample)| {
            let p = sample.0;
            (p.x >= min[0].saturating_sub(1) as f32
                && p.z >= min[1].saturating_sub(1) as f32
                && p.x <= (max[0] + 2) as f32
                && p.z <= (max[1] + 2) as f32)
                .then_some((((p.x * 32.) as u32, (p.z * 32.) as u32), id))
        })
        .collect();
    let mut assembly = Assembly {
        cache,
        sim,
        affected,
        local,
        remap: vec![usize::MAX; cache.samples.len()],
        added: HashMap::new(),
        samples: Vec::with_capacity(cache.samples.len()),
    };
    let mut indices = Vec::with_capacity(cache.indices.len());
    let mut patches = HashMap::with_capacity(leaves.len());
    for &leaf in &leaves {
        let start = indices.len();
        if inside(leaf, 1) {
            lod::triangulate(leaf, &corners, |p| assembly.point(p), &mut indices);
        } else {
            for tri in &cache.indices[cache.patches[&leaf.key()].clone()] {
                indices.push(tri.map(|id| assembly.old(id)));
            }
        }
        patches.insert(leaf.key(), start..indices.len());
    }
    Geometry {
        samples: assembly.samples,
        indices: Arc::new(indices),
        patches: Arc::new(patches),
        leaves: Arc::new(leaves),
    }
}
