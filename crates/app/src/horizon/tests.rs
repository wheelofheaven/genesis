use super::*;
use genesis_sim_core::config::SimConfig;

fn fixture() -> SimState {
    let mut sim = SimState::new(SimConfig {
        width: 64,
        height: 48,
        ..Default::default()
    });
    sim.grid.boundary = Boundary::Clamp;
    sim.geography.region = Region::LevantCoast;
    for z in 0..48 {
        for x in 0..64 {
            let cell = sim.grid.get_mut(x, z);
            cell.elevation = (x as f32 - 20.) * 0.04;
            cell.plants = 0.7;
            cell.moisture = 0.3;
        }
    }
    sim
}
fn equivalent(sim: &SimState, cache: &Cache) {
    let reference = super::reference::Cache::new(sim);
    for seafloor in [false, true] {
        let layers = crate::survey::Survey {
            seafloor,
            ..Default::default()
        };
        let (mut a, mut b) = (Vec::new(), Vec::new());
        cache.append(sim, &mut a, layers);
        reference.append(sim, &mut b, layers);
        assert_eq!(a.len(), b.len());
        for (i, (a, b)) in a.iter().zip(&b).enumerate() {
            assert_eq!(
                a.position.map(f32::to_bits),
                b.position.map(f32::to_bits),
                "position {i}"
            );
            assert_eq!(
                a.color.map(f32::to_bits),
                b.color.map(f32::to_bits),
                "color {i}"
            );
        }
    }
}
#[test]
fn reused_horizon_matches_original_with_distant_palette_changes() {
    let mut sim = fixture();
    let old = Cache::new(&sim);
    equivalent(&sim, &old);
    let bounds = SampleBounds {
        min: [30, 20],
        max: [34, 24],
    };
    sim.grid.get_mut(32, 22).elevation += 0.4;
    let unchanged = old.refresh(&sim, bounds);
    assert!(Arc::ptr_eq(&old.geometry, &unchanged.geometry));
    assert_eq!(unchanged.refreshed_pages, 0);
    equivalent(&sim, &unchanged);
    // This distant ecological edit is deliberately outside the terrain brush.
    sim.grid.get_mut(63, 24).plants = 0.;
    sim.grid.get_mut(63, 24).moisture = 1.;
    let next = unchanged.refresh(&sim, bounds);
    assert!(Arc::ptr_eq(&old.geometry, &next.geometry));
    assert!(next.refreshed_pages > 0);
    assert!(
        next.ground
            .iter()
            .zip(&old.ground)
            .any(|(a, b)| Arc::ptr_eq(a, b))
    );
    equivalent(&sim, &next);
    sim.step(0.1, &[]);
    let next = next.refresh(&sim, bounds);
    equivalent(&sim, &next);
    // A reader retaining an older cache must still see its original snapshot.
    equivalent(&fixture(), &old);
}
#[test]
fn edges_wrapping_and_changed_world_parameters_rebuild_horizon() {
    let mut sim = fixture();
    let mut cache = Cache::new(&sim);
    for (x, z) in [(0, 24), (63, 24), (32, 0), (32, 47)] {
        sim.grid.get_mut(x, z).elevation += 1.;
        let next = cache.refresh(
            &sim,
            SampleBounds {
                min: [x as u32, z as u32],
                max: [x as u32, z as u32],
            },
        );
        assert!(!Arc::ptr_eq(&cache.geometry, &next.geometry));
        equivalent(&sim, &next);
        cache = next;
    }
    let bounds = SampleBounds {
        min: [30, 20],
        max: [34, 24],
    };
    sim.sea_level += 0.1;
    let next = cache.refresh(&sim, bounds);
    assert!(!next.reused_geometry);
    equivalent(&sim, &next);
    cache = next;
    sim.grid.boundary = Boundary::Wrap;
    let next = cache.refresh(&sim, bounds);
    assert!(!next.reused_geometry);
    equivalent(&sim, &next);
    cache = next;
    sim.grid.get_mut(32, 22).elevation += 2.;
    let next = cache.refresh(&sim, bounds);
    assert!(!next.reused_geometry);
    equivalent(&sim, &next);
    sim.geography.region = Region::Island;
    let next = next.refresh(&sim, bounds);
    equivalent(&sim, &next);
    assert!(next.geometry.nodes.is_empty());
}
