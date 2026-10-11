use super::noise::unit;
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
    assert_valid_drainage(&sim);
}

fn assert_valid_drainage(sim: &SimState) {
    let mut grid = sim.grid.clone();
    let flow = drainage::route(&mut grid, sim.sea_level);
    let mut rank = vec![0; grid.cells().len()];
    for (order, &i) in flow.order.iter().enumerate() {
        rank[i] = order;
    }
    let mut outlets = 0.;
    let mut rainfall = 0.;
    for (i, c) in grid.cells().iter().enumerate() {
        if c.elevation > sim.sea_level {
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
    // Accumulation uses f32; inland catchments combine rainfall from almost the
    // entire field. Permit one part per million of rounding error, still less
    // than a single cell's rainfall in these fixtures.
    let tolerance = (rainfall * 1e-6).max(0.01);
    assert!(
        (rainfall - outlets).abs() < tolerance,
        "rainfall {rainfall}, outlets {outlets}, difference {}",
        (rainfall - outlets).abs()
    );
}

#[test]
fn seed_sweep_has_land_sea_rivers_resources_and_valid_settlements() {
    for seed in [0, 1, 42, DEFAULT_SEED, DEFAULT_SEED + 1, u64::MAX] {
        let mut sim = generate(seed);
        let count = sim.grid.cells().len() as f32;
        let land = sim.grid.cells().iter().filter(|c| c.elevation > 0.).count() as f32 / count;
        assert!((0.15..0.75).contains(&land), "seed {seed}: land {land}");
        assert!(!sim.geography.rivers.is_empty(), "seed {seed}: no rivers");
        for river in &sim.geography.rivers {
            assert_eq!(river.points.len(), river.widths.len());
            assert!(river.widths.iter().all(|w| w.is_finite()
                && (crate::units::metres(0.8)..=crate::units::metres(12.)).contains(w)));
            assert!(river.widths.windows(2).all(|w| w[1] + 1e-6 >= w[0]));
            assert_eq!(river.width, river.widths.iter().copied().fold(0., f32::max));
        }
        for plant in &sim.geography.cover {
            assert!((0. ..WIDTH as f32 - 1.).contains(&plant.position.x));
            assert!((0. ..HEIGHT as f32 - 1.).contains(&plant.position.z));
            assert!(
                crate::spatial::elevation(&sim.grid, plant.position)
                    >= sim.sea_level + crate::units::metres(1.5)
            );
            assert!(river_distance(&sim, plant.position) >= crate::units::metres(8.));
            assert!(
                sim.buildings
                    .iter()
                    .all(|b| (b.position.x - plant.position.x).powi(2)
                        + (b.position.z - plant.position.z).powi(2)
                        >= crate::units::metres(if plant.kind.tree() { 6. } else { 4. }).powi(2))
            );
        }
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
            assert!(
                c.elevation.is_finite()
                    && (crate::units::MIN_ELEVATION..crate::units::MAX_ELEVATION)
                        .contains(&c.elevation)
            );
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
    let mut sim = generate_region(DEFAULT_SEED, Region::Island);
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

#[test]
fn mainland_windows_have_continuous_seafloor_and_no_ocean_moat() {
    let coast = generate_region(DEFAULT_SEED, Region::LevantCoast);
    assert_eq!(coast.grid.boundary, crate::grid::Boundary::Clamp);
    for z in 0..HEIGHT as i32 {
        assert!(coast.grid.get(0, z).elevation < -2.);
        assert!(coast.grid.get(WIDTH as i32 - 1, z).elevation > 0.);
    }
    assert!(coast.grid.get(0, 160).elevation < coast.grid.get(90, 160).elevation);
    let hills = generate_region(DEFAULT_SEED, Region::LevantHighlands);
    assert!(
        hills
            .grid
            .cells()
            .iter()
            .all(|c| c.elevation > hills.sea_level)
    );
    assert_eq!(hills.buildings.len(), 6);
    assert!(!hills.geography.rivers.is_empty());
    assert_eq!(hills.subsurface.deposits.len(), 9);
    let p = Position::new(WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5);
    let center = Region::LevantCoast
        .geographic_position(p, WIDTH, HEIGHT)
        .unwrap();
    assert_eq!(Some(center), Region::LevantCoast.anchor());
    let ne = Region::LevantCoast
        .geographic_position(Position::new(p.x + 50., p.z - 50.), WIDTH, HEIGHT)
        .unwrap();
    assert!(ne.latitude > center.latitude && ne.longitude > center.longitude);
}

#[test]
fn uninhabited_scenario_keeps_the_same_terrain_without_settlement() {
    let populated = super::generate_region(7319, super::Region::LevantCoast);
    let empty = super::generate_scenario(7319, super::Region::LevantCoast, false);
    assert!(!populated.buildings.is_empty());
    assert!(empty.buildings.is_empty() && empty.paths.is_empty());
    assert_eq!(populated.geography.rivers, empty.geography.rivers);
    for (a, b) in populated.grid.cells().iter().zip(empty.grid.cells()) {
        assert_eq!(a.elevation, b.elevation);
    }
}

fn longest_major_straight_reach(rivers: &[River]) -> f32 {
    let mut longest: f32 = 0.;
    for river in rivers {
        let mut direction = (0., 0.);
        let mut run = 0.;
        for (index, pair) in river.points.windows(2).enumerate() {
            if river.widths[index] < crate::units::metres(4.) {
                run = 0.;
                direction = (0., 0.);
                continue;
            }
            let dx = pair[1].x - pair[0].x;
            let dz = pair[1].z - pair[0].z;
            let length = dx.hypot(dz);
            if length < 1e-6 {
                continue;
            }
            let next = (dx / length, dz / length);
            if next.0 * direction.0 + next.1 * direction.1 < 0.99999 {
                run = 0.;
            }
            run += length;
            longest = longest.max(run);
            direction = next;
        }
    }
    longest * crate::units::METRES_PER_UNIT
}

#[test]
fn highland_valleys_preserve_drainage_and_settlement_invariants() {
    for seed in [0, 42, DEFAULT_SEED] {
        let sim = generate_region(seed, Region::LevantHighlands);
        assert_valid_drainage(&sim);
        assert_eq!(sim.buildings.len(), 6, "seed {seed}: settlement");
        for building in &sim.buildings {
            let mut empty = sim.clone();
            empty.buildings.retain(|b| b.id != building.id);
            assert!(empty.validate_building(building).is_ok(), "seed {seed}");
        }
        assert!(sim.grid.cells().iter().all(|c| c.elevation.is_finite()
            && c.elevation > sim.sea_level
            && c.elevation < crate::units::MAX_ELEVATION));
        assert!(!sim.geography.rivers.is_empty());
        for river in &sim.geography.rivers {
            assert_eq!(river.points.len(), river.widths.len());
            assert!(river.widths.iter().all(|w| w.is_finite()
                && (crate::units::metres(0.8)..=crate::units::metres(12.)).contains(w)));
            assert!(river.widths.windows(2).all(|w| w[1] + 1e-6 >= w[0]));
        }
        if seed == DEFAULT_SEED {
            // Visual regression for the default inland valley: the old recipe
            // produced a kilometre-long ruler-straight trunk. This is an art
            // constraint for this fixture, not a hydrological law for all seeds.
            let straight = longest_major_straight_reach(&sim.geography.rivers);
            assert!(straight < 600., "default valley: {straight} m straight");
            let repeated = generate_region(seed, Region::LevantHighlands);
            assert_eq!(sim.geography, repeated.geography);
            assert_eq!(sim.buildings, repeated.buildings);
            for (a, b) in sim.grid.cells().iter().zip(repeated.grid.cells()) {
                assert_eq!(a.elevation, b.elevation);
                assert_eq!(a.drainage, b.drainage);
            }
        }
    }
}
