//! Continue the real valley water check with both supplies and food-cargo saves.
use crate::{Result, interaction::Object, session::Session, terrain};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{actions::Action, spatial::Position, units::metres};
pub fn check(session: &mut Session, camera: &mut Camera, capture: &std::path::Path) -> Result<()> {
    let source = session
        .sim
        .food
        .sources()
        .first()
        .ok_or("no generated food patch")?;
    let point = Position::new(source.position.x + metres(0.85), source.position.z);
    let ground = terrain::surface_point(&session.sim, point);
    let hit = crate::interaction::pick_at(
        &session.sim,
        ground + glam::Vec3::Y * metres(10.),
        -glam::Vec3::Y,
        Some(ground),
        |p| p.position,
    );
    if hit != Some(Object::FoodSource(source.id)) {
        return Err(format!("food source picking: {hit:?}").into());
    }
    session.queue(Action::SetWellActive {
        id: session.sim.subsurface.wells[0].id,
        active: true,
    });
    for _ in 0..1800 {
        session.step_once();
    }
    if !session.sim.food.homes().values().all(|h| h.consumed > 0)
        || !session
            .sim
            .households
            .all()
            .values()
            .all(|h| h.consumed > 0.)
    {
        return Err("not all valley households used both supplies".into());
    }
    for _ in 0..1800 {
        if session.sim.people.all().iter().any(|p| p.carried_food > 0) {
            break;
        }
        session.step_once();
    }
    let person = session
        .sim
        .people
        .all()
        .iter()
        .find(|p| p.carried_food > 0)
        .ok_or("no food carrier")?;
    let (id, home, position) = (person.id, person.home, person.position);
    let before = serde_json::to_vec(&*session.sim)?;
    let total_before = session
        .sim
        .food
        .home(home)
        .map_or(0, |h| h.consumed + u64::from(h.stored));
    session.queue(Action::CancelFoodTask { person: id });
    let path = capture.with_extension("food.save.json");
    session.save_to(&path)?;
    for _ in 0..600 {
        session.step_once();
    }
    if session
        .sim
        .food
        .home(home)
        .is_none_or(|h| h.consumed + u64::from(h.stored) < total_before + 4)
    {
        return Err("cancelled food cargo was not delivered".into());
    }
    let expected = serde_json::to_vec(&*session.sim)?;
    session.load_from(&path)?;
    if serde_json::to_vec(&*session.sim)? != before {
        return Err("food cargo load changed state".into());
    }
    for _ in 0..600 {
        session.step_once();
    }
    if serde_json::to_vec(&*session.sim)? != expected {
        return Err("food cargo continuation diverged".into());
    }
    session.load_from(&path)?;
    session.select_object(Some(Object::Person(id)), position);
    camera.center(terrain::surface_point(&session.sim, position));
    camera.set_visible_height(metres(24.));
    session.set_water_overview(true);
    session.supply_food = true;
    Ok(())
}
