//! Place a plot in the generated valley through the normal client command path.
use crate::{Result, interaction::Object, session::Session, terrain};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{
    actions::Action,
    food::cultivation::{self, irrigation},
    spatial::Position,
    units::metres,
};
pub fn check(session: &mut Session, camera: &mut Camera, capture: &std::path::Path) -> Result<()> {
    session.clock.paused = true;
    // The preceding water smoke deliberately leaves a pump-stop command queued.
    session.step_once();
    let well = session
        .sim
        .subsurface
        .wells
        .first()
        .ok_or("crop check needs a well")?
        .id;
    let well_position = session.sim.subsurface.wells[0].position;
    session.queue(Action::SetWellActive {
        id: well,
        active: true,
    });
    session.step_once();
    let home = session
        .sim
        .people
        .all()
        .first()
        .ok_or("crop check needs residents")?
        .home_position;
    let site = [8., 12., 16., 20.]
        .into_iter()
        .flat_map(|r| {
            (0..32).map(move |i| {
                let a = i as f32 * std::f32::consts::TAU / 32.;
                Position::new(home.x + metres(r) * a.cos(), home.z + metres(r) * a.sin())
            })
        })
        .find(|&p| {
            cultivation::validate(&session.sim, p).is_ok()
                && cultivation::moisture(&session.sim, p) > 0.25
                && genesis_sim_core::paths::distance(p, well_position) <= irrigation::RANGE
        })
        .ok_or("no usable crop site")?;
    let count = session.sim.food.sources().len();
    session.set_placing(true);
    session.crop_placing = true;
    session.set_hover(Some(site));
    session.build_hovered(false);
    if session.pending_count() != 1 || session.sim.food.sources().len() != count {
        return Err("crop preview mutated world".into());
    }
    session.step_once();
    let source = session.sim.food.sources().last().ok_or("planting failed")?;
    let id = source.id;
    if !matches!(source.kind, cultivation::Kind::Barley(_)) {
        return Err("no planted crop".into());
    }
    let ground = terrain::surface_point(&session.sim, site);
    if crate::interaction::pick_at(
        &session.sim,
        ground + glam::Vec3::Y * metres(10.),
        -glam::Vec3::Y,
        Some(ground),
        |p| p.position,
    ) != Some(Object::FoodSource(id))
    {
        return Err("seedling picking failed".into());
    }
    for _ in 0..100 {
        session.step_once();
    }
    // Save both active storage and pending intent, then compare the full continuation.
    let well = irrigation::candidate(&session.sim, id)?;
    session.queue(Action::IrrigateCrop { source: id, well });
    session.step_once();
    if irrigation::total_applied(&session.sim) != 2. {
        return Err("crop irrigation transfer failed".into());
    }
    session.queue(Action::IrrigateCrop { source: id, well });
    let path = capture.with_extension("crop.save.json");
    session.save_to(&path)?;
    for _ in 0..1800 {
        session.step_once();
    }
    let expected = serde_json::to_vec(&*session.sim)?;
    session.load_from(&path)?;
    for _ in 0..1800 {
        session.step_once();
    }
    if serde_json::to_vec(&*session.sim)? != expected {
        return Err("crop save continuation diverged".into());
    }
    if session
        .sim
        .food
        .source(id)
        .is_none_or(|s| s.produced() == 0)
    {
        return Err("crop did not mature".into());
    }
    if irrigation::total_applied(&session.sim) != 4. {
        return Err("saved pending irrigation did not execute exactly once".into());
    }
    // Leave live storage visible in the captured inspector.
    let well = irrigation::candidate(&session.sim, id)?;
    session.queue(Action::IrrigateCrop { source: id, well });
    session.step_once();
    session.queue(Action::SetCropTending {
        source: id,
        enabled: true,
    });
    let tending_path = capture.with_extension("tending.save.json");
    session.save_to(&tending_path)?;
    session.step_once();
    let expected = serde_json::to_vec(&*session.sim)?;
    session.load_from(&tending_path)?;
    session.step_once();
    if serde_json::to_vec(&*session.sim)? != expected
        || !matches!(
            &session.sim.food.source(id).ok_or("missing tended plot")?.kind,
            cultivation::Kind::Barley(c) if c.tending
        )
    {
        return Err("pending tending intent did not round-trip".into());
    }
    session.select_object(Some(Object::FoodSource(id)), site);
    camera.center(terrain::surface_point(&session.sim, site));
    camera.set_visible_height(metres(24.));
    Ok(())
}
