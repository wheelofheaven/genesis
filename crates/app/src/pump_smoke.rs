//! Extend the native survey check with buffer control and pending-policy saves.
use crate::{Result, session::Session};
use genesis_sim_core::{actions::Action, subsurface::PumpMode};

pub fn check(session: &mut Session, capture: &std::path::Path) -> Result<()> {
    session.clock.paused = true;
    let id = session
        .sim
        .subsurface
        .wells
        .first()
        .ok_or("pump check needs a well")?
        .id;
    session.queue(Action::SetWellPumpMode {
        id,
        mode: PumpMode::FillBuffer,
    });
    for _ in 0..100 {
        session.step_once();
    }
    let well = &session.sim.subsurface.wells[0];
    if well.supply.as_ref().is_none_or(|s| s.stored < 19.999)
        || !well.active
        || well.last_yield != 0.
    {
        return Err("enabled pump did not pause at a full buffer".into());
    }
    let recovered = well.recovered;
    let released = well.supply.as_ref().ok_or("missing well supply")?.released;
    for _ in 0..50 {
        session.step_once();
    }
    let well = &session.sim.subsurface.wells[0];
    if well.recovered != recovered || well.supply.as_ref().is_none_or(|s| s.released != released) {
        return Err("buffer-limited pumping wasted water".into());
    }
    session.queue(Action::SetWellPumpMode {
        id,
        mode: PumpMode::Continuous,
    });
    let path = capture.with_extension("pump.save.json");
    session.save_to(&path)?;
    for _ in 0..10 {
        session.step_once();
    }
    let expected = serde_json::to_vec(&*session.sim)?;
    session.load_from(&path)?;
    for _ in 0..10 {
        session.step_once();
    }
    if serde_json::to_vec(&*session.sim)? != expected {
        return Err("saved pump-policy continuation diverged".into());
    }
    session.queue(Action::SetWellPumpMode {
        id,
        mode: PumpMode::FillBuffer,
    });
    session.step_once();
    // Loading clears transient selection/tool state; restore the survey view.
    session.select(session.sim.subsurface.wells[0].position);
    session.set_survey(true);
    Ok(())
}
