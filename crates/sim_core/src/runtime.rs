//! Fixed logical advancement shared by the interactive host and CPU runner.
//!
//! `SimState` remains mutable for generation and legacy callers. Once handed to
//! a `Simulation`, clients can only inspect it or submit actions at a tick.
use crate::{SimState, actions::Action, events::SimEvent};
use std::{ops::Deref, time::Duration};

/// Existing game cadence. Wall-clock pacing belongs to the host, not the rules.
pub const TICK: Duration = Duration::from_millis(100);

/// One running world and its integer logical timeline.
///
/// Tick zero is the supplied initial state, including any fixture setup steps.
/// Use `save::Snapshot::restore` to load a saved timeline rather than `new`.
/// Immutable dereferencing preserves the existing borrowed extraction/query APIs;
/// deliberately no `DerefMut` or mutable state accessor is provided.
///
/// ```compile_fail
/// use genesis_sim_core::{runtime::Simulation, scenarios};
/// let mut sim = Simulation::new(scenarios::coastal_outpost());
/// sim.grid.get_mut(0, 0).elevation = 20.;
/// ```
#[derive(Debug)]
pub struct Simulation {
    pub(crate) state: SimState,
    pub(crate) completed_ticks: u64,
}

impl Simulation {
    /// Take ownership of an initialized world, before its first gameplay tick.
    pub fn new(state: SimState) -> Self {
        Self {
            state,
            completed_ticks: 0,
        }
    }

    /// Number of completed gameplay ticks since this world was handed off.
    pub fn completed_ticks(&self) -> u64 {
        self.completed_ticks
    }

    /// Execute ordered actions, then the existing rules, at the next boundary.
    ///
    /// Actions are validated by `SimState::step` in slice order. The caller owns
    /// delivery and must not resubmit an already consumed batch. No wall clock,
    /// graphical state, or worker completion affects this operation.
    pub fn advance(&mut self, actions: &[Action]) -> Vec<SimEvent> {
        let events = self.state.step(TICK.as_secs_f32(), actions);
        self.completed_ticks += 1;
        events
    }
}

impl Deref for Simulation {
    type Target = SimState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{scenarios, spatial::Position};

    #[test]
    fn fixed_runtime_preserves_existing_rules_and_ordered_conflicts() {
        let initial = scenarios::coastal_outpost();
        let mut old = initial.clone();
        let mut run = Simulation::new(initial);
        assert_eq!(run.completed_ticks(), 0, "setup is not gameplay time");
        let build = Action::PlaceBuilding {
            position: Position::new(25.31, 20.27),
            rotation: 0.47,
        };
        for tick in 1..=100 {
            let actions = match tick {
                1 => vec![Action::Terraform {
                    x: 15,
                    y: 12,
                    radius: 2,
                    strength: 0.4,
                }],
                2 => vec![build.clone(), build.clone()],
                3 => vec![Action::DrillWell {
                    position: Position::new(18.25, 21.5),
                }],
                50 => vec![Action::SetWellActive {
                    id: 1,
                    active: false,
                }],
                _ => vec![],
            };
            let expected = old.step(0.1, &actions);
            let actual = run.advance(&actions);
            assert_eq!(run.completed_ticks(), tick);
            // Full debug equality is a local regression oracle, not a durable
            // fingerprint or a serialization/replay compatibility contract.
            assert_eq!(format!("{expected:?}"), format!("{actual:?}"));
            if tick == 2 {
                assert!(matches!(actual[0], SimEvent::BuildingPlaced { .. }));
                assert!(matches!(actual[1], SimEvent::BuildingRejected { .. }));
            }
        }
        assert_eq!(format!("{old:?}"), format!("{:?}", run.state));
        assert_eq!(run.buildings.len(), 7);
        assert!(run.subsurface.wells[0].recovered > 0.);
        assert!(!run.subsurface.wells[0].active);
    }
}
