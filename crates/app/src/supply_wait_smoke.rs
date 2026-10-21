//! Native acceptance: stopped well, waiting residents, recovery, then save restore.
use crate::{Result, session::Session};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{actions::Action, people::Activity, units::metres};

pub fn check(session: &mut Session, camera: &mut Camera, capture: &std::path::Path) -> Result<()> {
    session.queue(Action::SetWellActive {
        id: 1,
        active: false,
    });
    for _ in 0..65 {
        session.step_once();
    }
    if !session
        .sim
        .people
        .all()
        .iter()
        .all(|p| p.activity == Activity::WaitingForSupplies)
    {
        return Err("empty households did not wait at home".into());
    }
    let path = capture.with_extension("waiting.json");
    session.save_to(&path)?;
    let saved = serde_json::to_vec(&*session.sim)?;
    session.queue(Action::SetWellActive {
        id: 1,
        active: true,
    });
    for _ in 0..900 {
        session.step_once();
    }
    if !session
        .sim
        .households
        .all()
        .values()
        .all(|h| h.consumed > 0.)
    {
        return Err("resumed well did not supply every household".into());
    }
    session.load_from(&path)?;
    if saved != serde_json::to_vec(&*session.sim)? {
        return Err("waiting save changed state or retry phase".into());
    }
    let id = session.sim.people.all()[0].id;
    session.inspect_person(id, camera);
    camera.set_visible_height(metres(28.));
    eprintln!(
        "Supply waiting: six households waited, recovered deliveries, and restored exact retry state"
    );
    Ok(())
}
