use super::*;

#[test]
fn brush_stays_finite_and_bounded_even_for_zero_radius() {
    let mut grid = Grid::new(8, 8);
    apply_terraform(&mut grid, 3, 3, 0, 2.);
    assert_eq!(grid.get(3, 3).elevation, 1.);
    for strength in [f32::NAN, f32::INFINITY, f32::MAX, -f32::MAX] {
        apply_terraform(&mut grid, 3, 3, 2, strength);
        assert!(
            grid.cells().iter().all(|c| c.elevation.is_finite()
                && (MIN_ELEVATION..=MAX_ELEVATION).contains(&c.elevation))
        );
    }
}

#[test]
fn test_terraform_raises_center() {
    let mut grid = Grid::new(16, 16);
    let before = grid.get(8, 8).elevation;
    apply_terraform(&mut grid, 8, 8, 2, 1.0);
    assert!(grid.get(8, 8).elevation > before);
}

#[test]
fn test_terraform_falloff() {
    let mut grid = Grid::new(16, 16);
    apply_terraform(&mut grid, 8, 8, 3, 1.0);
    let center = grid.get(8, 8).elevation;
    let edge = grid.get(8 + 2, 8).elevation;
    assert!(center > edge, "Center should be higher than edge");
}

#[test]
fn test_water_depth_clamped() {
    let mut grid = Grid::new(4, 4);
    grid.get_mut(0, 0).elevation = 5.0; // Well above sea level
    update_water_depth(&mut grid, 0.0);
    assert_eq!(grid.get(0, 0).water_depth, 0.0);
}

#[test]
fn clipped_and_wrapped_brushes_preserve_falloff_and_repeated_visits() {
    let mut wrapped = Grid::new(2, 2);
    assert_eq!(try_apply_terraform(&mut wrapped, 0, 0, 2, 1.), Ok(4));
    // The four diagonal stencil visits all address the same wrapped sample.
    for (cell, expected) in wrapped.cells().iter().zip([0., 0., 0., 0.17157292]) {
        assert!((cell.elevation - expected).abs() < 1e-7);
    }
    let mut clipped = Grid::new(3, 3);
    clipped.boundary = Boundary::Clamp;
    assert_eq!(try_apply_terraform(&mut clipped, -1, 1, 2, 1.), Ok(3));
    assert_eq!(clipped.get(0, 1).elevation, -0.5);
    assert_eq!(clipped.get(2, 1).elevation, -1., "bounded edges never wrap");
    assert!(clipped.cells().iter().all(|c| c.elevation >= -1.));
}

#[test]
fn rejected_and_zero_effect_requests_leave_the_grid_untouched() {
    let mut grid = Grid::new(8, 8);
    grid.boundary = Boundary::Clamp;
    let before = format!("{grid:?}");
    for (x, y, radius, strength, expected) in [
        (3, 3, 2, f32::NAN, Err(TerraformError::NonFiniteStrength)),
        (
            3,
            3,
            2,
            f32::INFINITY,
            Err(TerraformError::NonFiniteStrength),
        ),
        (i32::MAX, 3, u32::MAX, 1., Err(TerraformError::OutsideMap)),
        (3, i32::MIN, u32::MAX, 1., Err(TerraformError::OutsideMap)),
        (-2, -2, 2, 1., Err(TerraformError::OutsideMap)),
        (3, 3, 2, 0., Ok(0)),
    ] {
        assert_eq!(
            try_apply_terraform(&mut grid, x, y, radius, strength),
            expected
        );
        assert_eq!(before, format!("{grid:?}"));
    }
    for cell in grid.cells_mut() {
        cell.elevation = MAX_ELEVATION;
    }
    assert_eq!(try_apply_terraform(&mut grid, 3, 3, 2, 1.), Ok(0));
}

#[test]
fn extreme_wrapped_coordinates_match_their_canonical_location() {
    let mut a = Grid::new(7, 5);
    let mut b = a.clone();
    let result = try_apply_terraform(&mut a, i32::MIN, i32::MAX, u32::MAX, 0.25);
    assert_eq!(
        result,
        try_apply_terraform(
            &mut b,
            i32::MIN.rem_euclid(7),
            i32::MAX.rem_euclid(5),
            7,
            0.25
        )
    );
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}

#[test]
fn command_rejection_has_no_partial_effects_and_later_actions_observe_edits() {
    use crate::{
        SimState, actions::Action, config::SimConfig, events::SimEvent, spatial::Position,
    };
    let mut initial = SimState::new(SimConfig {
        width: 12,
        height: 12,
        ..Default::default()
    });
    initial.grid.boundary = Boundary::Clamp;
    let mut rejected = initial.clone();
    let events = rejected.step(
        0.1,
        &[Action::Terraform {
            x: 6,
            y: 6,
            radius: 3,
            strength: f32::NAN,
        }],
    );
    let mut control = initial.clone();
    control.step(0.1, &[]);
    assert_eq!(
        format!("{rejected:?}"),
        format!("{control:?}"),
        "normal tick effects are not command effects"
    );
    assert!(matches!(events[0], SimEvent::TerraformRejected { .. }));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SimEvent::TerrainChanged { .. }))
    );
    let build = Action::PlaceBuilding {
        position: Position::new(6., 6.),
        rotation: 0.,
    };
    let raise = Action::Terraform {
        x: 6,
        y: 6,
        radius: 12,
        strength: 3.,
    };
    let mut first = initial.clone();
    let events = first.step(0., &[raise.clone(), build.clone()]);
    assert!(matches!(events[0], SimEvent::TerrainChanged { .. }));
    assert!(matches!(events[1], SimEvent::BuildingPlaced { .. }));
    let events = initial.step(0., &[build, raise]);
    assert!(matches!(events[0], SimEvent::BuildingRejected { .. }));
    assert_eq!(first.buildings.len(), 1);
    assert!(initial.buildings.is_empty());
    assert!(
        first
            .grid
            .cells()
            .iter()
            .all(|c| c.water_depth == (first.sea_level - c.elevation).max(0.))
    );
}

#[test]
fn editing_existing_ground_retains_objects_and_updates_channel_queries() {
    use crate::{actions::Action, scenarios, spatial::Position, surface, worldgen::River};
    let mut sim = scenarios::coastal_outpost();
    let building = sim.buildings[0].clone();
    let paths = sim.paths.clone();
    let before = surface::elevation(&sim, building.position);
    sim.step(
        0.,
        &[Action::Terraform {
            x: building.position.x.round() as i32,
            y: building.position.z.round() as i32,
            radius: 3,
            strength: 0.4,
        }],
    );
    assert_eq!(sim.buildings[0], building);
    assert_eq!(sim.paths, paths);
    assert!(surface::elevation(&sim, building.position) > before);

    // Channel geometry is an authored incision recipe, not a live flow solver.
    sim.geography.version = 9;
    let center = Position::new(24., 20.);
    sim.geography.rivers.push(River {
        points: vec![Position::new(22., 20.), Position::new(26., 20.)],
        width: 0.2,
        widths: vec![],
        bounds: None,
    });
    sim.geography.channels =
        surface::channels::Index::new(sim.grid.width, sim.grid.height, &sim.geography.rivers);
    let routes = sim.geography.rivers.clone();
    let water = surface::channels::level(&sim, center);
    let aquifer_top = sim.subsurface.reading(center).top;
    sim.step(
        0.,
        &[Action::Terraform {
            x: 24,
            y: 20,
            radius: 3,
            strength: 0.4,
        }],
    );
    assert!(surface::channels::level(&sim, center) > water);
    assert!(surface::elevation(&sim, center) < surface::channels::level(&sim, center));
    assert_eq!(sim.geography.rivers, routes);
    assert_eq!(sim.subsurface.reading(center).top, aquifer_top);
}

#[test]
fn staging_retains_the_original_floating_point_representation() {
    let mut grid = Grid::new(1, 1);
    grid.get_mut(0, 0).elevation = -0.;
    assert_eq!(try_apply_terraform(&mut grid, 0, 0, 0, 0.), Ok(1));
    assert_eq!(grid.get(0, 0).elevation.to_bits(), 0.0f32.to_bits());
}
