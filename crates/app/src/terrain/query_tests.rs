//! Keep the former exhaustive traversal as a reference for the adaptive query.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn exhaustive(cache: &Cache, min: Position, max: Position) -> Vec<usize> {
    let mut ranges = BTreeMap::new();
    for z in (min.z * 32.).floor() as i32..=(max.z * 32.).floor() as i32 {
        for x in (min.x * 32.).floor() as i32..=(max.x * 32.).floor() as i32 {
            if let Some((_, range)) = cache.patch(Position::new(
                (x as f32 + 0.5) / 32.,
                (z as f32 + 0.5) / 32.,
            )) {
                ranges.insert(range.start, range.end);
            }
        }
    }
    ranges.into_iter().flat_map(|(a, b)| a..b).collect()
}

#[test]
fn adaptive_receiver_query_matches_exhaustive_at_lod_and_map_edges() {
    let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
        width: 8,
        height: 8,
        ..Default::default()
    });
    sim.geography.version = 9;
    sim.geography.region = genesis_sim_core::worldgen::Region::Island;
    for cell in sim.grid.cells_mut() {
        cell.elevation = 2.;
    }
    let cache = Cache::at(&sim, lod::Key::at(Vec3::new(0., 2.1, 0.)));
    let sizes: BTreeSet<_> = cache.patches.keys().map(|&(_, _, size)| size).collect();
    assert!(sizes.len() >= 3, "exercise mixed LOD: {sizes:?}");
    let check = |min, max| {
        let mut actual = Vec::new();
        cache.indexed_triangles_in(min, max, |id, triangle| {
            assert_eq!(triangle, cache.indices[id].map(|i| cache.samples[i].1));
            actual.push(id);
        });
        assert_eq!(actual, exhaustive(&cache, min, max), "{min:?}..{max:?}");
    };
    for (min, max) in [
        ((-0.2, -0.1), (8.1, 8.2)),
        ((-0.2, -0.1), (-0.1, -0.05)),
        ((8., 8.), (8.1, 8.1)),
        ((4., 4.), (4., 4.)),
        ((3., 2.), (2., 3.)),
        ((2., 3.), (3., 2.)),
        ((-0.1, 2.), (0., 5.)),
        ((2., -0.1), (5., 0.)),
    ] {
        check(Position::new(min.0, min.1), Position::new(max.0, max.1));
    }
    for i in 0..160 {
        let x = (i as f32 * 0.618034).fract() * 8.;
        let z = (i as f32 * 0.414214).fract() * 8.;
        let width = [0., 1. / 32., 0.17, 0.73, 1.5][i % 5];
        let height = [1.5, 0.73, 0.17, 1. / 32., 0.][i % 5];
        check(Position::new(x, z), Position::new(x + width, z + height));
    }
}
