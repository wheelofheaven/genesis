//! Native persistence check using an explicit capture sibling, never the user's slot.
use crate::session::Session;
use genesis_sim_core::{actions::Action, spatial::Position};
use std::path::Path;

pub fn check(session: &mut Session, capture: &Path) -> genesis_storage::Result<()> {
    let path = capture.with_extension("world.json");
    session.queue(Action::DrillWell {
        position: Position::new(18.25, 21.5),
    });
    let before = serde_json::to_vec(&*session.sim)?;
    let tick = session.sim.completed_ticks();
    let identity = session.world_id.clone();
    session.save_to(&path)?;
    session.step_once();
    if session.sim.subsurface.wells.len() != 1 {
        return Err("save smoke: queued well did not execute".into());
    }
    let expected = serde_json::to_vec(&*session.sim)?;
    session.load_from(&path)?;
    if serde_json::to_vec(&*session.sim)? != before
        || session.sim.completed_ticks() != tick
        || session.world_id != identity
        || session.pending_count() != 1
        || !session.clock.paused
    {
        return Err("save smoke: load did not restore the saved boundary".into());
    }
    session.step_once();
    if serde_json::to_vec(&*session.sim)? != expected || session.pending_count() != 0 {
        return Err("save smoke: continued state differs".into());
    }
    Ok(())
}
