//! Local geometry refresh for a complete terrain-only revision range.
//! Changed LOD roots rebuild their shared seams before publication.
mod topology;
use super::*;
use genesis_sim_core::grid::{Boundary, SampleBounds};

#[derive(Clone, Copy)]
pub(crate) struct Affected {
    min: [f32; 2],
    max: [f32; 2],
    size: [f32; 2],
    wrapped: bool,
}

impl Affected {
    fn new(sim: &SimState, bounds: SampleBounds) -> Self {
        // B-spline reads floor(p)-1..floor(p)+2. Add normal offsets and
        // channel-center projection reach; use a deliberately conservative halo.
        let width = sim
            .geography
            .rivers
            .iter()
            .map(|r| r.width)
            .fold(0., f32::max);
        let halo = 3. + width * 0.5 + genesis_sim_core::surface::channels::BANK;
        Self {
            min: bounds.min.map(|v| v as f32 - halo),
            max: bounds.max.map(|v| v as f32 + halo),
            size: [sim.grid.width as f32, sim.grid.height as f32],
            wrapped: sim.grid.boundary == Boundary::Wrap,
        }
    }

    fn intersects(self, min: [f32; 2], max: [f32; 2]) -> bool {
        (0..2).all(|i| {
            let overlap = |offset| max[i] + offset >= self.min[i] && min[i] + offset <= self.max[i];
            overlap(0.) || (self.wrapped && (overlap(-self.size[i]) || overlap(self.size[i])))
        })
    }

    fn contains(self, p: Position) -> bool {
        self.intersects([p.x, p.z], [p.x, p.z])
    }

    fn touches_rivers(self, sim: &SimState) -> bool {
        sim.geography.rivers.iter().any(|river| {
            let mut min = [f32::INFINITY; 2];
            let mut max = [f32::NEG_INFINITY; 2];
            for p in &river.points {
                min[0] = min[0].min(p.x);
                min[1] = min[1].min(p.z);
                max[0] = max[0].max(p.x);
                max[1] = max[1].max(p.z);
            }
            // Include ribbon banks, coarse receiver corners and slope support.
            let margin = river.width * 0.5 + 2.;
            self.intersects(min.map(|v| v - margin), max.map(|v| v + margin))
        })
    }
}

impl Cache {
    /// The caller must establish a same-world, terrain-only revision chain.
    /// New camera keys reconstruct the full reference; changed splits rebuild locally.
    pub(crate) fn refresh(
        &self,
        sim: &SimState,
        key: lod::Key,
        bounds: SampleBounds,
    ) -> (Self, Option<Affected>) {
        self.refresh_profiled(sim, key, bounds, |_| {})
    }

    pub(crate) fn refresh_profiled(
        &self,
        sim: &SimState,
        key: lod::Key,
        bounds: SampleBounds,
        mut completed: impl FnMut(&'static str),
    ) -> (Self, Option<Affected>) {
        if key != self.key || self.elevations.len() != sim.grid.cells().len() {
            return (Self::at_profiled(sim, key, completed), None);
        }
        let affected = Affected::new(sim, bounds);
        let changed = lod::changed_splits(sim, key, &self.elevations, bounds);
        completed("refresh_lod");
        let geometry = if changed.is_empty() {
            topology::Geometry {
                source_ids: None,
                samples: self
                    .samples
                    .iter()
                    .map(|&sample| {
                        if affected.contains(sample.0) {
                            sample_geometry(sim, key, sample.0)
                        } else {
                            sample
                        }
                    })
                    .collect(),
                indices: Arc::clone(&self.indices),
                patches: Arc::clone(&self.patches),
                leaves: Arc::clone(&self.leaves),
            }
        } else {
            topology::rebuild(self, sim, affected, changed)
        };
        completed("refresh_geometry");
        let paint = appearance::Paint::refresh(
            sim,
            &geometry.samples,
            self,
            geometry.source_ids.as_deref(),
        );
        completed("refresh_appearance");
        let context = self.context.refresh(sim, bounds);
        completed("refresh_horizon");
        let ground_cover = ground_cover::placements(sim, key.eye());
        completed("refresh_ground_cover");
        let contacts = contacts::build(sim, key.eye());
        completed("refresh_contacts");
        let mut cache = Self {
            paint,
            context,
            samples: geometry.samples,
            indices: geometry.indices,
            patches: geometry.patches,
            leaves: geometry.leaves,
            elevations: sim.grid.cells().iter().map(|c| c.elevation).collect(),
            key,
            rivers: Arc::clone(&self.rivers),
            // Density/tint and exposed scenery can change with the ordinary tick.
            ground_cover,
            contacts,
        };
        completed("refresh_assembly");
        let rivers_changed = affected.touches_rivers(sim);
        completed("refresh_river_gate");
        if rivers_changed {
            let mut rivers = Vec::new();
            crate::water::rivers_profiled(sim, &cache, &mut rivers, &mut completed);
            cache.rivers = Arc::new(rivers);
        }
        completed("refresh_river_publish");
        (cache, Some(affected))
    }
}

impl Picker {
    /// Rebuild affected spatial buckets; retained chunks are immutable and can
    /// remain referenced by an old view until the whole update is published.
    pub(crate) fn refresh(&self, sim: &SimState, cache: &Cache, affected: Affected) -> Self {
        let columns = sim.grid.width.div_ceil(16) as usize;
        let mut buckets: Vec<_> = (0..self.chunks.len())
            .map(|i| {
                let x = (i % columns * 16) as f32;
                let z = (i / columns * 16) as f32;
                // A receiver rooted on one side may use a changed shared vertex.
                affected
                    .intersects([x - 1., z - 1.], [x + 17., z + 17.])
                    .then(Vec::new)
            })
            .collect();
        for ids in cache.indices.iter() {
            let tri = ids.map(|i| cache.samples[i].0);
            let x = (tri[0].x.min(tri[1].x).min(tri[2].x) / 16.) as usize;
            let z = (tri[0].z.min(tri[1].z).min(tri[2].z) / 16.) as usize;
            if let Some(bucket) = &mut buckets[z * columns + x] {
                bucket.push(ids.map(|i| cache.samples[i].1));
            }
        }
        let chunks = buckets
            .into_iter()
            .zip(&self.chunks)
            .map(|(bucket, old)| {
                let Some(triangles) = bucket else {
                    return Arc::clone(old);
                };
                let min = triangles
                    .iter()
                    .flatten()
                    .fold(Vec3::splat(f32::INFINITY), |a, &b| a.min(b));
                let max = triangles
                    .iter()
                    .flatten()
                    .fold(Vec3::splat(f32::NEG_INFINITY), |a, &b| a.max(b));
                Arc::new(Chunk {
                    triangles,
                    min: min - Vec3::splat(1e-5),
                    max: max + Vec3::splat(1e-5),
                })
            })
            .collect();
        Self {
            chunks,
            width: self.width,
            height: self.height,
            sea: sim.sea_level,
        }
    }
}

#[cfg(test)]
mod tests;
