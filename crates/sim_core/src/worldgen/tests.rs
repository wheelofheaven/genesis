use super::*;

#[test]
fn seeded_regions_are_repeatable_and_different_seeds_change_all_layers() {
    let a = generate(DEFAULT_SEED);
    let b = generate(DEFAULT_SEED);
    let c = generate(DEFAULT_SEED + 1);
    assert_eq!(a.geography, b.geography);
    assert_eq!(a.buildings, b.buildings);
    assert_eq!(a.subsurface.deposits, b.subsurface.deposits);
    assert_ne!(a.geography, c.geography);
    assert_ne!(a.subsurface.deposits, c.subsurface.deposits);
    for (a, b) in a.grid.cells().iter().zip(b.grid.cells()) {
        assert_eq!(a.elevation, b.elevation);
        assert_eq!(a.rainfall, b.rainfall);
        assert_eq!(a.drainage, b.drainage);
    }
}

#[test]
fn drainage_is_acyclic_downhill_and_conserves_catchment_rainfall() {
    let sim = generate(DEFAULT_SEED);
    let mut grid = sim.grid.clone();
    let flow = drainage::route(&mut grid, 0.);
    let mut rank = vec![0; grid.cells().len()];
    for (order, &i) in flow.order.iter().enumerate() {
        rank[i] = order;
    }
    let mut outlets = 0.;
    let mut rainfall = 0.;
    for (i, c) in grid.cells().iter().enumerate() {
        if c.elevation > 0. {
            rainfall += c.rainfall as f64;
        }
        if let Some(j) = flow.receivers[i] {
            assert!(rank[j] < rank[i]);
            assert!(grid.cells()[j].elevation < c.elevation);
            assert!(flow.accumulation[j] >= flow.accumulation[i]);
        } else {
            outlets += flow.accumulation[i] as f64;
        }
    }
    assert!((rainfall - outlets).abs() < 0.01);
}

#[test]
fn seed_sweep_has_land_sea_rivers_resources_and_valid_settlements() {
    for seed in [0, 1, 42, DEFAULT_SEED, DEFAULT_SEED + 1, u64::MAX] {
        let mut sim = generate(seed);
        let count = sim.grid.cells().len() as f32;
        let land = sim.grid.cells().iter().filter(|c| c.elevation > 0.).count() as f32 / count;
        assert!((0.15..0.75).contains(&land), "seed {seed}: land {land}");
        assert!(!sim.geography.rivers.is_empty(), "seed {seed}: no rivers");
        assert_eq!(sim.buildings.len(), 6, "seed {seed}: settlement");
        assert_eq!(sim.subsurface.deposits.len(), 9, "seed {seed}: deposits");
        for b in &sim.buildings {
            let mut empty = sim.clone();
            empty.buildings.retain(|other| other.id != b.id);
            assert!(empty.validate_building(b).is_ok());
        }
        for _ in 0..20 {
            sim.step(0.1, &[]);
        }
        for c in sim.grid.cells() {
            assert!(c.elevation.is_finite() && (-40. ..80.).contains(&c.elevation));
            assert!((0. ..=1.).contains(&c.moisture));
            assert!((0. ..=1.).contains(&c.plants));
        }
        for c in &sim.subsurface.columns {
            assert!(c.aquifer_base < c.aquifer_top);
            assert!((0. ..=c.capacity).contains(&c.stored));
        }
        for d in &sim.subsurface.deposits {
            for i in 0..32 {
                assert!(
                    d.concentration(d.outline(i as f32 / 32. * std::f32::consts::TAU, 1.)) < 0.0001
                );
            }
        }
    }
}

#[test]
fn river_bounds_preserve_exact_distances_and_the_ocean_rim_wraps() {
    let mut sim = generate(DEFAULT_SEED);
    let points: Vec<_> = (0..96)
        .map(|i| Position::new(unit(59, i) * WIDTH as f32, unit(61, i) * HEIGHT as f32))
        .collect();
    let indexed: Vec<_> = points.iter().map(|&p| river_distance(&sim, p)).collect();
    for river in &mut sim.geography.rivers {
        river.bounds = None;
    }
    for (p, expected) in points.into_iter().zip(indexed) {
        assert_eq!(river_distance(&sim, p), expected);
    }
    for x in 0..WIDTH as i32 {
        assert_eq!(
            sim.grid.get(x, 0).elevation,
            sim.grid.get(x, HEIGHT as i32 - 1).elevation
        );
        assert!(sim.grid.get(x, 0).elevation < sim.sea_level);
    }
    for z in 0..HEIGHT as i32 {
        assert_eq!(
            sim.grid.get(0, z).elevation,
            sim.grid.get(WIDTH as i32 - 1, z).elevation
        );
    }
}
