//! Exact surface-palette dependencies. Survey overlays deliberately use the full
//! extractor; their water/deposit inputs are independent of the normal palette.
use super::*;
use genesis_sim_core::{grid::Boundary, subsurface::Rock};
use std::borrow::Cow;

pub(crate) struct Inputs {
    size: [u32; 2],
    boundary: Boundary,
    sea: u32,
    drought: u32,
    rocks: Vec<Rock>,
    fields: Vec<[u32; 2]>,
}
impl Inputs {
    pub fn storage_bytes(&self) -> usize {
        self.rocks.capacity() * std::mem::size_of::<Rock>()
            + self.fields.capacity() * std::mem::size_of::<[u32; 2]>()
    }
    pub fn new(sim: &SimState) -> Self {
        Self {
            size: [sim.grid.width, sim.grid.height],
            boundary: sim.grid.boundary,
            sea: sim.sea_level.to_bits(),
            drought: sim.geography.climate.summer_drought.to_bits(),
            rocks: sim.subsurface.columns.iter().map(|c| c.rock).collect(),
            fields: sim
                .grid
                .cells()
                .iter()
                .map(|c| [c.plants.to_bits(), c.moisture.to_bits()])
                .collect(),
        }
    }
    pub fn dirty(&self, sim: &SimState) -> Dirty {
        let mut result = Dirty {
            size: self.size,
            boundary: self.boundary,
            all: false,
            roots: Vec::new(),
        };
        if self.size != [sim.grid.width, sim.grid.height]
            || self.boundary != sim.grid.boundary
            || self.sea != sim.sea_level.to_bits()
            || self.drought != sim.geography.climate.summer_drought.to_bits()
            || self.rocks.len() != sim.subsurface.columns.len()
            || self
                .rocks
                .iter()
                .zip(&sim.subsurface.columns)
                .any(|(a, b)| *a != b.rock)
        {
            result.all = true;
            return result;
        }
        let changed: Vec<_> = self
            .fields
            .iter()
            .zip(sim.grid.cells())
            .map(|(a, b)| *a != [b.plants.to_bits(), b.moisture.to_bits()])
            .collect();
        if !changed.iter().any(|&c| c) {
            return result;
        }
        let [w, h] = self.size;
        result.roots = (0..w * h)
            .map(|i| {
                let (x, z) = ((i % w) as i32, (i / w) as i32);
                (-1..=2).any(|dz| {
                    (-1..=2).any(|dx| {
                        let at = |v: i32, n: u32| match self.boundary {
                            Boundary::Wrap => v.rem_euclid(n as i32) as usize,
                            Boundary::Clamp => v.clamp(0, n as i32 - 1) as usize,
                        };
                        changed[at(z + dz, h) * w as usize + at(x + dx, w)]
                    })
                })
            })
            .collect();
        result
    }
}

pub(crate) struct Dirty {
    size: [u32; 2],
    boundary: Boundary,
    all: bool,
    roots: Vec<bool>,
}
impl Dirty {
    pub fn empty(&self) -> bool {
        !self.all && self.roots.is_empty()
    }
    pub fn at(&self, p: Position) -> bool {
        if self.all {
            return true;
        }
        if self.roots.is_empty() {
            return false;
        }
        let at = |v: f32, n: u32| match self.boundary {
            Boundary::Wrap => v.rem_euclid(n as f32) as usize,
            Boundary::Clamp => v.clamp(0., n as f32 - 1.) as usize,
        };
        self.roots[at(p.z, self.size[1]) * self.size[0] as usize + at(p.x, self.size[0])]
    }
}

pub(super) struct Paint {
    inputs: Inputs,
    colors: Vec<Vertex>,
    pub refreshed: usize,
}
fn color(sim: &SimState, s: Sample) -> Vertex {
    colored_vertex(sim, s.0, s.1, s.2, s.3, Default::default())
}
impl Paint {
    pub fn storage_bytes(&self) -> usize {
        self.inputs.storage_bytes() + self.colors.capacity() * std::mem::size_of::<Vertex>()
    }
    pub fn new(sim: &SimState, samples: &[Sample]) -> Self {
        Self {
            inputs: Inputs::new(sim),
            colors: samples.iter().map(|&s| color(sim, s)).collect(),
            refreshed: samples.len(),
        }
    }
    pub fn refresh(
        sim: &SimState,
        samples: &[Sample],
        old: &Cache,
        source_ids: Option<&[usize]>,
    ) -> Self {
        let dirty = old.paint.inputs.dirty(sim);
        let mut refreshed = 0;
        let colors = samples
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let id = source_ids.map_or(i, |ids| ids[i]);
                if id != usize::MAX && old.samples[id] == s && !dirty.at(s.0) {
                    old.paint.colors[id]
                } else {
                    refreshed += 1;
                    color(sim, s)
                }
            })
            .collect();
        Self {
            inputs: Inputs::new(sim),
            colors,
            refreshed,
        }
    }
    pub fn vertices<'a>(&'a self, sim: &SimState, samples: &[Sample]) -> Cow<'a, [Vertex]> {
        let dirty = self.inputs.dirty(sim);
        if dirty.empty() {
            return Cow::Borrowed(&self.colors);
        }
        Cow::Owned(
            samples
                .iter()
                .zip(&self.colors)
                .map(|(&s, &v)| if dirty.at(s.0) { color(sim, s) } else { v })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_sim_core::config::SimConfig;
    #[test]
    fn palette_cache_tracks_interpolation_edges_and_global_inputs() {
        for boundary in [Boundary::Clamp, Boundary::Wrap] {
            let mut sim = SimState::new(SimConfig {
                width: 16,
                height: 12,
                ..Default::default()
            });
            sim.grid.boundary = boundary;
            for c in sim.grid.cells_mut() {
                c.plants = 0.5;
                c.moisture = 0.5;
                c.elevation = 2.;
            }
            let samples: Vec<_> = (-4..=36)
                .flat_map(|x| (-4..=28).map(move |z| Position::new(x as f32 * 0.5, z as f32 * 0.5)))
                .map(|p| (p, Vec3::new(p.x, 2., p.z), Vec3::Y, 0.))
                .collect();
            let paint = Paint::new(&sim, &samples);
            assert!(matches!(paint.vertices(&sim, &samples), Cow::Borrowed(_)));
            for stage in 0..6 {
                match stage {
                    0 => {
                        sim.grid.get_mut(0, 0).plants = 0.;
                        sim.grid.get_mut(15, 11).moisture = 1.;
                    }
                    1 => {
                        sim.step(0.1, &[]);
                    }
                    2 => sim.sea_level = 2.,
                    3 => sim.geography.climate.summer_drought = 0.9,
                    4 => sim
                        .subsurface
                        .columns
                        .iter_mut()
                        .for_each(|c| c.rock = Rock::Basalt),
                    _ => sim.grid.boundary = Boundary::Clamp,
                }
                for (&s, v) in samples.iter().zip(paint.vertices(&sim, &samples).iter()) {
                    assert_eq!(
                        v.color.map(f32::to_bits),
                        color(&sim, s).color.map(f32::to_bits),
                        "stage {stage} at {:?}",
                        s.0
                    );
                }
            }
        }
    }
}
