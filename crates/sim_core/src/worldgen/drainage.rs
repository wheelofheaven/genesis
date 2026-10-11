//! Deterministic eight-neighbour Priority-Flood with strict downhill receivers.
//! Fills synthetic pits, then accumulates prescribed rainfall in reverse order.
use crate::{grid::Grid, spatial::Position};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BinaryHeap};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct River {
    pub points: Vec<Position>,
    /// Maximum bank-to-bank width, in world units.
    pub width: f32,
    /// Local bank-to-bank widths, aligned with points. Empty in legacy data.
    #[serde(default)]
    pub widths: Vec<f32>,
    #[serde(default)]
    pub bounds: Option<[f32; 4]>,
}
pub struct Drainage {
    pub receivers: Vec<Option<usize>>,
    pub order: Vec<usize>,
    pub accumulation: Vec<f32>,
}
#[derive(Clone, Copy, PartialEq)]
struct Entry(f32, usize);
impl Eq for Entry {}
impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .total_cmp(&self.0)
            .then_with(|| other.1.cmp(&self.1))
    }
}
impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
pub fn neighbours(i: usize, width: usize, height: usize) -> impl Iterator<Item = usize> {
    let x = (i % width) as i32;
    let z = (i / width) as i32;
    (-1..=1).flat_map(move |dz| {
        (-1..=1).filter_map(move |dx| {
            let nx = x + dx;
            let nz = z + dz;
            ((dx != 0 || dz != 0) && nx >= 0 && nz >= 0 && nx < width as i32 && nz < height as i32)
                .then_some((nz as usize).wrapping_mul(width).wrapping_add(nx as usize))
        })
    })
}
pub fn route(grid: &mut Grid, sea: f32) -> Drainage {
    let n = grid.cells().len();
    let width = grid.width as usize;
    let height = grid.height as usize;
    let mut heap = BinaryHeap::new();
    let mut visited = vec![false; n];
    for (i, c) in grid.cells().iter().enumerate() {
        if c.elevation <= sea
            || i % width == 0
            || i % width == width - 1
            || i < width
            || i >= n - width
        {
            heap.push(Entry(c.elevation, i));
            visited[i] = true;
        }
    }
    let mut order = Vec::with_capacity(n);
    let mut receivers = vec![None; n];
    while let Some(Entry(elevation, i)) = heap.pop() {
        order.push(i);
        for j in neighbours(i, width, height) {
            if visited[j] {
                continue;
            }
            visited[j] = true;
            let level = grid.cells()[j].elevation.max(elevation + 0.002);
            grid.cells_mut()[j].elevation = level;
            receivers[j] = Some(i);
            heap.push(Entry(level, j));
        }
    }
    // Choose the steepest available downhill neighbour. All such neighbours
    // precede this node in flood order, so no flow cycles are possible.
    for &i in &order {
        if receivers[i].is_none() {
            continue;
        }
        let mut best = 0.;
        for j in neighbours(i, width, height) {
            let diagonal = i % width != j % width && i / width != j / width;
            let slope = (grid.cells()[i].elevation - grid.cells()[j].elevation)
                / if diagonal {
                    std::f32::consts::SQRT_2
                } else {
                    1.
                };
            if slope > best {
                best = slope;
                receivers[i] = Some(j);
            }
        }
    }
    let mut accumulation: Vec<_> = grid
        .cells()
        .iter()
        .map(|c| if c.elevation > sea { c.rainfall } else { 0. })
        .collect();
    for &i in order.iter().rev() {
        if let Some(j) = receivers[i] {
            accumulation[j] += accumulation[i];
        }
    }
    for (cell, &flow) in grid.cells_mut().iter_mut().zip(&accumulation) {
        cell.drainage = flow;
    }
    Drainage {
        receivers,
        order,
        accumulation,
    }
}

pub fn rivers(grid: &Grid, drainage: &Drainage, sea: f32, threshold: f32) -> Vec<River> {
    let n = grid.cells().len();
    let mut upstream = vec![0; n];
    for (i, receiver) in drainage.receivers.iter().enumerate() {
        if drainage.accumulation[i] >= threshold
            && let Some(j) = receiver
        {
            upstream[*j] += 1;
        }
    }
    let mut result = Vec::new();
    let mut used = vec![false; n];
    for &start in drainage.order.iter().rev() {
        if upstream[start] != 0
            || drainage.accumulation[start] < threshold
            || grid.cells()[start].elevation <= sea
        {
            continue;
        }
        let mut points = Vec::new();
        let mut widths = Vec::new();
        let mut i = start;
        loop {
            points.push(Position::new(
                (i % grid.width as usize) as f32,
                (i / grid.width as usize) as f32,
            ));
            // A metric visual recipe based on contributing rainfall-weighted
            // catchment area, not distance along an arbitrarily split polyline.
            let area_km2 =
                drainage.accumulation[i] * crate::units::METRES_PER_UNIT.powi(2) / 1_000_000.;
            widths.push(crate::units::metres(
                (0.8 + 4. * area_km2.sqrt()).clamp(0.8, 12.),
            ));
            if used[i] || grid.cells()[i].elevation <= sea {
                break;
            }
            used[i] = true;
            match drainage.receivers[i] {
                Some(j) => i = j,
                None => break,
            }
        }
        if points.len() > 2 {
            // Corner cutting rounds D8 bends while preserving confluences and
            // mouths exactly. Flow accumulation still uses the routing graph.
            for _ in 0..2 {
                let mut smooth = vec![points[0]];
                let mut smooth_widths = vec![widths[0]];
                for (index, pair) in points.windows(2).enumerate() {
                    for t in [0.25, 0.75] {
                        smooth_widths.push(widths[index] * (1. - t) + widths[index + 1] * t);
                        smooth.push(Position::new(
                            pair[0].x * (1. - t) + pair[1].x * t,
                            pair[0].z * (1. - t) + pair[1].z * t,
                        ));
                    }
                }
                smooth.push(points[points.len() - 1]);
                smooth_widths.push(widths[widths.len() - 1]);
                points = smooth;
                widths = smooth_widths;
            }
            let bounds = points.iter().fold(
                [
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                ],
                |b, p| [b[0].min(p.x), b[1].min(p.z), b[2].max(p.x), b[3].max(p.z)],
            );
            result.push(River {
                bounds: Some(bounds),
                points,
                width: widths.iter().copied().fold(0., f32::max),
                widths,
            });
        }
    }
    result
}
