use bevy::prelude::*;
use genesis_sim_core::{SimState, actions::Action, config::SimConfig};

use crate::input::tool::PendingActions;
use crate::ui::overlay::SimSpeed;

/// Bevy resource wrapping the sim core state.
#[derive(Resource)]
pub struct Simulation(pub SimState);

/// Accumulated time for fixed-step simulation.
#[derive(Resource, Default)]
pub struct SimAccumulator(pub f32);

/// Recent events from the sim, for the expedition log.
#[derive(Resource, Default)]
pub struct SimEventLog(pub Vec<String>);

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        let config = SimConfig::default();
        let state = SimState::new(config);
        app.insert_resource(Simulation(state))
            .insert_resource(SimAccumulator::default())
            .insert_resource(SimEventLog::default())
            .insert_resource(PendingActions::default())
            .insert_resource(SimSpeed::default())
            .add_systems(Update, tick_simulation);
    }
}

fn tick_simulation(
    mut sim: ResMut<Simulation>,
    mut accumulator: ResMut<SimAccumulator>,
    mut pending: ResMut<PendingActions>,
    mut log: ResMut<SimEventLog>,
    speed: Res<SimSpeed>,
    time: Res<Time>,
) {
    let dt = time.delta_secs() * speed.multiplier();
    accumulator.0 += dt;

    let tick_dt = sim.0.config.tick_dt;

    while accumulator.0 >= tick_dt {
        accumulator.0 -= tick_dt;

        let actions: Vec<Action> = pending.0.drain(..).collect();
        let events = sim.0.step(tick_dt, &actions);

        for event in events {
            let msg = format!("[t={:.0}] {:?}", sim.0.time, event);
            log.0.push(msg);
            // Keep log bounded.
            if log.0.len() > 100 {
                log.0.remove(0);
            }
        }
    }
}
