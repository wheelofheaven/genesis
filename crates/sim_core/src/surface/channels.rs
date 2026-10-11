//! Local channel cross sections shared by terrain and water presentation.
//! This is an incision recipe, not a discharge or sediment transport solver.
use crate::{
    SimState,
    spatial::{self, Position},
    units::metres,
    worldgen::River,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Sparse one-cell bins of river/segment references. Rebuild when routes change.
/// Stored with geography so cloned and serialized worlds keep identical queries.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Index {
    columns: u32,
    rows: u32,
    bins: BTreeMap<u32, Vec<[usize; 2]>>,
}

pub const BANK: f32 = metres(2.);

impl Index {
    pub fn new(columns: u32, rows: u32, rivers: &[River]) -> Self {
        let mut result = Self {
            columns,
            rows,
            bins: BTreeMap::new(),
        };
        for (r, river) in rivers.iter().enumerate() {
            let margin = river.width * 0.5 + BANK;
            for (s, pair) in river.points.windows(2).enumerate() {
                let x0 = (pair[0].x.min(pair[1].x) - margin).floor().max(0.) as u32;
                let z0 = (pair[0].z.min(pair[1].z) - margin).floor().max(0.) as u32;
                let x1 = ((pair[0].x.max(pair[1].x) + margin).floor() as u32)
                    .min(columns.saturating_sub(1));
                let z1 = ((pair[0].z.max(pair[1].z) + margin).floor() as u32)
                    .min(rows.saturating_sub(1));
                for z in z0..=z1 {
                    for x in x0..=x1 {
                        result.bins.entry(z * columns + x).or_default().push([r, s]);
                    }
                }
            }
        }
        result
    }

    /// Broad-phase cell occupancy, also useful for preserving channels in LOD.
    pub fn contains(&self, p: Position) -> bool {
        !self.candidates(p).is_empty()
    }

    fn candidates(&self, p: Position) -> &[[usize; 2]] {
        if p.x < 0. || p.z < 0. || p.x >= self.columns as f32 || p.z >= self.rows as f32 {
            return &[];
        }
        self.bins
            .get(&(p.z.floor() as u32 * self.columns + p.x.floor() as u32))
            .map_or(&[], Vec::as_slice)
    }
}

pub fn width(river: &River, segment: usize, t: f32) -> f32 {
    let a = river.widths.get(segment).copied().unwrap_or(river.width);
    let b = river
        .widths
        .get(segment + 1)
        .copied()
        .unwrap_or(river.width);
    a + (b - a) * t
}

/// Surface follows the smoothed valley profile, level across each cross section.
/// Reading the current field keeps local terraforming coherent with the bed.
pub fn level(sim: &SimState, center: Position) -> f32 {
    (spatial::elevation(&sim.grid, center) - metres(0.25)).max(sim.sea_level)
}

#[derive(Clone, Copy)]
pub struct Section {
    pub center: Position,
    pub distance: f32,
    pub half_width: f32,
}

/// Nearest normalized cross section, bounded to the channel and its two banks.
pub fn section(sim: &SimState, p: Position) -> Option<Section> {
    let mut nearest = 1.;
    let mut result = None;
    for &[r, s] in sim.geography.channels.candidates(p) {
        let Some(river) = sim.geography.rivers.get(r) else {
            continue;
        };
        let Some(pair) = river.points.get(s..s + 2) else {
            continue;
        };
        let dx = pair[1].x - pair[0].x;
        let dz = pair[1].z - pair[0].z;
        let t = (((p.x - pair[0].x) * dx + (p.z - pair[0].z) * dz) / (dx * dx + dz * dz).max(1e-9))
            .clamp(0., 1.);
        let center = Position::new(pair[0].x + dx * t, pair[0].z + dz * t);
        let distance = ((p.x - center.x).powi(2) + (p.z - center.z).powi(2)).sqrt();
        let half_width = width(river, s, t) * 0.5;
        let normalized = distance / (half_width + BANK);
        if normalized < nearest {
            nearest = normalized;
            result = Some(Section {
                center,
                distance,
                half_width,
            });
        }
    }
    result
}

pub fn carve(sim: &SimState, p: Position, ground: f32) -> f32 {
    let Some(section) = section(sim, p) else {
        return ground;
    };
    // Undersea continuations already have an ocean bed; never raise terrain.
    if ground <= sim.sea_level - metres(2.) {
        return ground;
    }
    let water = level(sim, section.center);
    let t = (section.distance / section.half_width.max(metres(0.1))).min(1.);
    let depth = (section.half_width * 0.32).clamp(metres(0.25), metres(1.2));
    let bed = water - depth * (1. - t * t);
    let bank = ((section.distance - section.half_width) / BANK).clamp(0., 1.);
    let blend = bank * bank * (3. - 2. * bank);
    let incised = ground.min(bed + (ground - bed) * blend);
    let shore = ((ground - sim.sea_level + metres(2.)) / metres(2.)).clamp(0., 1.);
    // The context ring shares the unmodified regional boundary vertices.
    let edge =
        p.x.min(p.z)
            .min(sim.grid.width as f32 - p.x)
            .min(sim.grid.height as f32 - p.z)
            .clamp(0., 1.);
    ground + (incised - ground) * shore * edge
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> SimState {
        let mut sim = SimState::new(crate::config::SimConfig {
            width: 12,
            height: 12,
            ..Default::default()
        });
        sim.geography.version = 8;
        for cell in sim.grid.cells_mut() {
            cell.elevation = 2.;
        }
        sim.geography.rivers.push(River {
            points: vec![Position::new(2., 6.), Position::new(10., 6.)],
            width: metres(4.),
            widths: vec![],
            bounds: None,
        });
        sim.geography.channels = Index::new(12, 12, &sim.geography.rivers);
        sim
    }
    #[test]
    fn channel_has_a_submerged_bed_and_continuous_banks_without_raising_ground() {
        let sim = fixture();
        let center = Position::new(6., 6.);
        let water = level(&sim, center);
        assert!(super::super::elevation(&sim, center) < water - metres(0.2));
        let mut last = carve(&sim, center, 2.);
        for i in 1..=400 {
            let p = Position::new(6., 6. + metres(i as f32 * 0.02));
            let h = carve(&sim, p, 2.);
            assert!(h >= last - 1e-6 && h <= 2.);
            assert!((h - last).abs() < metres(0.03));
            last = h;
        }
        assert_eq!(last, 2.);
        assert_eq!(carve(&sim, center, -1.), -1.);
    }
    #[test]
    fn incision_preserves_context_boundaries_and_blends_into_the_seabed() {
        let mut sim = fixture();
        sim.geography.rivers[0].points[0] = Position::new(0., 6.);
        sim.geography.channels = Index::new(12, 12, &sim.geography.rivers);
        assert_eq!(carve(&sim, Position::new(0., 6.), 2.), 2.);
        assert!((carve(&sim, Position::new(0.0001, 6.), 2.) - 2.).abs() < 0.0001);
        let p = Position::new(6., 6.);
        let mut last = 0.;
        for i in 0..=200 {
            let h = -metres(i as f32 * 0.01);
            for cell in sim.grid.cells_mut() {
                cell.elevation = h;
            }
            let bed = carve(&sim, p, h);
            assert!(bed <= h && bed.is_finite());
            if i > 0 {
                assert!((bed - last).abs() < metres(0.02));
            }
            last = bed;
        }
        assert_eq!(last, -metres(2.));
    }
    #[test]
    fn index_matches_all_segments_and_edits_update_water_and_bed_together() {
        let mut sim = fixture();
        let indexed = sim.geography.channels.clone();
        // Expand every bin to all segments as a brute-force reference.
        for z in 0..12 {
            for x in 0..12 {
                sim.geography.channels.bins.insert(z * 12 + x, vec![[0, 0]]);
            }
        }
        let points: Vec<_> = (0..2000)
            .map(|i| Position::new((i * 73 % 1200) as f32 / 100., (i * 19 % 1200) as f32 / 100.))
            .collect();
        let expected: Vec<_> = points.iter().map(|&p| carve(&sim, p, 2.)).collect();
        sim.geography.channels = indexed;
        for (p, h) in points.into_iter().zip(expected) {
            assert_eq!(carve(&sim, p, 2.), h);
        }
        let p = Position::new(6., 6.);
        let before = super::super::elevation(&sim, p);
        let water = level(&sim, p);
        for cell in sim.grid.cells_mut() {
            cell.elevation += 1.;
        }
        assert!((super::super::elevation(&sim, p) - before - 1.).abs() < 1e-5);
        assert!((level(&sim, p) - water - 1.).abs() < 1e-5);
    }
}
