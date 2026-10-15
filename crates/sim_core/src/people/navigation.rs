//! Bounded local A*, fixed neighbor/tie ordering, no asynchronous readiness.
use crate::{SimState, paths::distance, spatial::Position, surface::walking::Query, units::metres};
use std::{cmp::Reverse, collections::BinaryHeap};
pub(super) const MAX_NODES: usize = 128 * 128;
const SPACING: f32 = metres(0.5);

pub(super) fn route(sim: &SimState, from: Position, to: Position) -> Option<Vec<Position>> {
    let margin = metres(5.);
    let x0 = from.x.min(to.x) - margin;
    let z0 = from.z.min(to.z) - margin;
    let width = ((from.x - to.x).abs() / SPACING).ceil() as usize + 21;
    let height = ((from.z - to.z).abs() / SPACING).ceil() as usize + 21;
    if width > 128 || height > 128 {
        return None;
    }
    let query = Query::local(
        sim,
        [
            x0,
            z0,
            x0 + width as f32 * SPACING,
            z0 + height as f32 * SPACING,
        ],
    );
    let clear = |a, b| distance(query.advance(a, b), b) < metres(0.001);
    if !query.standable(from) || !query.standable(to) {
        return None;
    }
    if distance(from, to) <= metres(10.) && clear(from, to) {
        return Some(vec![to]);
    }
    let point = |i: usize| {
        Position::new(
            x0 + (i % width) as f32 * SPACING,
            z0 + (i / width) as f32 * SPACING,
        )
    };
    let index = |p: Position| {
        ((p.z - z0) / SPACING).round() as usize * width + ((p.x - x0) / SPACING).round() as usize
    };
    let start = index(from);
    let goal = index(to);
    if !clear(from, point(start)) || !clear(point(goal), to) {
        return None;
    }
    let heuristic = |i: usize| {
        let dx = (i % width).abs_diff(goal % width);
        let dz = (i / width).abs_diff(goal / width);
        (10 * dx.max(dz) + 4 * dx.min(dz)) as u32
    };
    let count = width * height;
    let mut costs = vec![u32::MAX; count];
    let mut parents = vec![usize::MAX; count];
    let mut open = BinaryHeap::new();
    costs[start] = 0;
    open.push(Reverse((heuristic(start), 0u32, start)));
    while let Some(Reverse((_, cost, i))) = open.pop() {
        if costs[i] != cost {
            continue;
        }
        if i == goal {
            let mut path = vec![to];
            let mut cursor = goal;
            loop {
                path.push(point(cursor));
                if cursor == start {
                    break;
                }
                cursor = parents[cursor];
            }
            path.reverse();
            return Some(path);
        }
        for (dx, dz) in [
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (-1, -1),
            (1, -1),
        ] {
            let x = (i % width) as i32 + dx;
            let z = (i / width) as i32 + dz;
            if x < 0 || z < 0 || x >= width as i32 || z >= height as i32 {
                continue;
            }
            let next = z as usize * width + x as usize;
            let candidate = cost + if dx == 0 || dz == 0 { 10 } else { 14 };
            if candidate < costs[next] && clear(point(i), point(next)) {
                costs[next] = candidate;
                parents[next] = i;
                open.push(Reverse((candidate + heuristic(next), candidate, next)));
            }
        }
    }
    None
}
