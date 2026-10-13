//! Visual-only horizon: relief and seam recipes survive interior terrain edits.
mod geometry;
use crate::terrain::appearance::Inputs;
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{
    SimState,
    grid::{Boundary, SampleBounds},
    worldgen::Region,
};
use geometry::*;
use glam::Vec3;
use std::sync::Arc;

const PAGE_TRIANGLES: usize = 2048;
#[derive(PartialEq)]
struct Key {
    seed: u64,
    region: Region,
    size: [u32; 2],
    sea: u32,
    boundary: Boundary,
}
impl Key {
    fn new(sim: &SimState) -> Self {
        Self {
            seed: sim.config.seed,
            region: sim.geography.region,
            size: [sim.grid.width, sim.grid.height],
            sea: sim.sea_level.to_bits(),
            boundary: sim.grid.boundary,
        }
    }
}
pub struct Cache {
    key: Key,
    geometry: Arc<Geometry>,
    inputs: Inputs,
    colors: Vec<[f32; 4]>,
    ground: Vec<Arc<Vec<Vertex>>>,
    pub(crate) reused_geometry: bool,
    pub(crate) refreshed_pages: usize,
}
impl Cache {
    /// Additional retained data beyond the pre-existing expanded ground/water.
    pub(crate) fn metadata_bytes(&self) -> usize {
        self.inputs.storage_bytes()
            + self.colors.capacity() * std::mem::size_of::<[f32; 4]>()
            + self.geometry.nodes.capacity() * std::mem::size_of::<Node>()
            + self.geometry.triangles.capacity() * std::mem::size_of::<[usize; 3]>()
            + self.ground.capacity() * std::mem::size_of::<Arc<Vec<Vertex>>>()
    }
    pub fn new(sim: &SimState) -> Self {
        Self::paint(sim, Arc::new(Geometry::new(sim)), None)
    }
    /// Caller supplies a complete same-world terrain-only edit range. Edge
    /// elevations influence the first ring and its normals; wrapped worlds use
    /// a conservative full rebuild. Sea/seed/region changes also invalidate it.
    pub fn refresh(&self, sim: &SimState, bounds: SampleBounds) -> Self {
        let interior =
            (0..2).all(|i| bounds.min[i] > 3 && bounds.max[i].saturating_add(4) < self.key.size[i]);
        if self.key != Key::new(sim) || !interior || sim.grid.boundary != Boundary::Clamp {
            return Self::new(sim);
        }
        Self::paint(sim, Arc::clone(&self.geometry), Some(self))
    }
    fn paint(sim: &SimState, geometry: Arc<Geometry>, old: Option<&Self>) -> Self {
        let dirty = old.map(|c| c.inputs.dirty(sim));
        let mut colors: Vec<[f32; 4]> = Vec::with_capacity(geometry.nodes.len());
        let mut changed = Vec::with_capacity(geometry.nodes.len());
        for (i, node) in geometry.nodes.iter().enumerate() {
            let color = match node.paint {
                Paint::Empty => [0.; 4],
                Paint::Sample(p, normal) => {
                    if let Some((old, dirty)) = old.zip(dirty.as_ref())
                        && !dirty.at(p)
                    {
                        old.colors[i]
                    } else {
                        crate::terrain::colored_vertex(
                            sim,
                            p,
                            Vec3::from_array(node.position),
                            normal,
                            0.,
                            Default::default(),
                        )
                        .color
                    }
                }
                Paint::Average(ids) => {
                    std::array::from_fn(|a| ids.iter().map(|&j| colors[j][a]).sum::<f32>() * 0.25)
                }
                Paint::Lerp(a, b, t) => {
                    std::array::from_fn(|axis| colors[a][axis] * (1. - t) + colors[b][axis] * t)
                }
            };
            changed
                .push(old.is_none_or(|c| c.colors[i].map(f32::to_bits) != color.map(f32::to_bits)));
            colors.push(color);
        }
        let mut refreshed_pages = 0;
        let ground = geometry
            .triangles
            .chunks(PAGE_TRIANGLES)
            .enumerate()
            .map(|(i, triangles)| {
                if let Some(old) = old
                    && !triangles.iter().flatten().any(|&id| changed[id])
                {
                    return Arc::clone(&old.ground[i]);
                }
                refreshed_pages += 1;
                Arc::new(
                    triangles
                        .iter()
                        .flatten()
                        .map(|&id| Vertex {
                            position: geometry.nodes[id].position,
                            color: colors[id],
                        })
                        .collect(),
                )
            })
            .collect();
        Self {
            key: Key::new(sim),
            geometry,
            inputs: Inputs::new(sim),
            colors,
            ground,
            reused_geometry: old.is_some(),
            refreshed_pages,
        }
    }
    pub fn append(&self, sim: &SimState, out: &mut Vec<Vertex>, layers: crate::survey::Survey) {
        for &v in self.ground.iter().flat_map(|page| page.iter()) {
            let mut v = v;
            if layers.seafloor && v.position[1] < sim.sea_level {
                let depth = (sim.sea_level - v.position[1]) * 20.;
                let shade = (v.color[0] / 0.55).clamp(0.35, 1.);
                let c = Vec3::new(0.31, 0.45, 0.42)
                    .lerp(Vec3::new(0.075, 0.19, 0.27), (depth / 500.).clamp(0., 1.))
                    * shade;
                v.color = [c.x, c.y, c.z, 4.];
            }
            out.push(v);
        }
        if !layers.seafloor {
            out.extend_from_slice(&self.geometry.water);
        }
    }
}

#[cfg(test)]
mod reference;
#[cfg(test)]
mod tests;
