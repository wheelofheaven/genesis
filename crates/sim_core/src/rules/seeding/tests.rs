use crate::{SimState, actions::Action, config::SimConfig, grid::Boundary};

fn dry_world(width: u32, height: u32, boundary: Boundary) -> SimState {
    let mut state = SimState::new(SimConfig {
        width,
        height,
        ..Default::default()
    });
    state.grid.boundary = boundary;
    for cell in state.grid.cells_mut() {
        cell.elevation = 1.;
        cell.water_depth = 0.;
    }
    state
}

fn seed(x: i32, y: i32, radius: u32, fungus: bool) -> Action {
    if fungus {
        Action::SeedFungus { x, y, radius }
    } else {
        Action::SeedPlants { x, y, radius }
    }
}

#[test]
fn extreme_wrapped_commands_seed_the_mathematical_neighborhood() {
    for fungus in [false, true] {
        let mut state = dry_world(5, 7, Boundary::Wrap);
        state.step(0., &[seed(i32::MAX, i32::MIN, 1, fungus)]);
        // MAX mod 5 = 2, MIN mod 7 = 5; a radius-one brush is a cross.
        for (x, y, cell) in state.grid.iter_coords() {
            let expected = if [(2, 5), (1, 5), (3, 5), (2, 4), (2, 6)].contains(&(x, y)) {
                0.3
            } else {
                0.
            };
            assert_eq!(if fungus { cell.fungus } else { cell.plants }, expected);
            assert_eq!(if fungus { cell.plants } else { cell.fungus }, 0.);
        }
    }
}

#[test]
fn extreme_bounded_commands_do_not_seed_edges_or_interrupt_later_commands() {
    for fungus in [false, true] {
        let mut state = dry_world(5, 7, Boundary::Clamp);
        let mut actions = Vec::new();
        for extreme in [i32::MIN, i32::MAX] {
            actions.push(seed(extreme, 0, u32::MAX, fungus));
            actions.push(seed(0, extreme, u32::MAX, fungus));
        }
        actions.push(seed(2, 3, 0, fungus));
        state.step(0., &actions);
        for (x, y, cell) in state.grid.iter_coords() {
            assert_eq!(
                if fungus { cell.fungus } else { cell.plants },
                if (x, y) == (2, 3) { 0.3 } else { 0. }
            );
        }
    }
}

#[test]
fn cropped_brush_preserves_underwater_exclusion_without_opposite_edge_seeding() {
    for fungus in [false, true] {
        let mut state = dry_world(5, 7, Boundary::Clamp);
        state.grid.get_mut(0, 1).elevation = -1.;
        state.grid.get_mut(0, 1).water_depth = 1.;
        state.step(0., &[seed(-1, 0, 2, fungus)]);
        for (x, y, cell) in state.grid.iter_coords() {
            assert_eq!(
                if fungus { cell.fungus } else { cell.plants },
                if [(0, 0), (1, 0)].contains(&(x, y)) {
                    0.3
                } else {
                    0.
                }
            );
        }
    }
}

#[test]
fn wrapped_repeated_visits_and_ordered_brushes_retain_saturation_and_radius_cap() {
    for fungus in [false, true] {
        let mut state = dry_world(1, 1, Boundary::Wrap);
        state.step(0., &[seed(i32::MIN, i32::MAX, u32::MAX, fungus)]);
        let cell = state.grid.get(0, 0);
        assert_eq!(if fungus { cell.fungus } else { cell.plants }, 1.);
        let mut state = dry_world(5, 7, Boundary::Wrap);
        state.step(0., &[seed(-1, -1, 0, fungus), seed(-1, -1, 0, fungus)]);
        let cell = state.grid.get(4, 6);
        assert_eq!(if fungus { cell.fungus } else { cell.plants }, 0.6);
    }
}
