use super::*;
use crate::{actions::Action, food::SourceId, scenarios, subsurface::PumpMode};

#[test]
fn uninhabited_worlds_have_no_invented_demand_and_queries_are_read_only() {
    let sim = scenarios::coastal_outpost();
    assert!(!sim.buildings.is_empty());
    let outlook = Outlook::read(&sim);
    assert!(outlook.homes.is_empty());
    assert_eq!(outlook.next_food_ms, None);
    assert_eq!(outlook.next_water_ms, None);
    assert_eq!(outlook.short_homes(), 0);
    let sim = scenarios::living_settlement();
    let outlook = Outlook::read(&sim);
    assert_eq!(outlook.short_homes(), 6);
    assert_eq!(outlook.food_short_homes(), 6);
    assert_eq!(outlook.water_short_homes(), 6);
    assert_eq!(outlook.next_food_ms, Some(30_000));
}

#[test]
fn clocks_use_each_resource_phase_and_prediction_matches_next_consumption() {
    let mut sim = scenarios::living_settlement();
    sim.food.advance(&sim.people, 29.9);
    sim.households.advance(&sim.people, 15.);
    let outlook = Outlook::read(&sim);
    assert_eq!(outlook.next_food_ms, Some(100));
    assert_eq!(outlook.next_water_ms, Some(15_000));
    sim.food.advance(&sim.people, 0.1);
    assert_eq!(Outlook::read(&sim).next_food_ms, Some(30_000));
    for home in &outlook.homes {
        assert_eq!(
            sim.food.home(home.id).unwrap().missed_portions,
            u64::from(home.food_shortfall())
        );
    }
    sim.households.advance(&sim.people, 15.);
    assert_eq!(Outlook::read(&sim).next_water_ms, Some(30_000));
    assert!(
        sim.households
            .all()
            .values()
            .all(|h| h.missed_portions == 1)
    );
}

#[test]
fn source_reservations_and_cargo_do_not_clear_shortages_until_delivery() {
    let mut sim = scenarios::supplied_settlement();
    let mut saw_reserved = false;
    let mut saw_food_cargo = false;
    let mut saw_water_cargo = false;
    let mut saw_delivery = false;
    for _ in 0..1800 {
        sim.step(0.1, &[]);
        let outlook = Outlook::read(&sim);
        for p in sim.people.all() {
            let h = outlook.homes.iter().find(|h| h.id == p.home).unwrap();
            if p.water_task.is_some() && p.carried_water == 0. {
                saw_reserved = true;
                assert_eq!(h.carried_water, 0.);
            }
            if p.carried_food > 0 && h.food == 0 {
                saw_food_cargo = true;
                assert_eq!(h.carried_food, 4);
                assert_eq!(h.food_shortfall(), 1);
            }
            if p.water_task.is_some() && p.carried_water > 0. && h.water == 0. {
                saw_water_cargo = true;
                assert_eq!(h.carried_water, 2.);
                assert_eq!(h.water_shortfall(), 0.5);
            }
            if h.water > 0. && h.food > 0 {
                saw_delivery = true;
                assert!(!h.short());
            }
        }
    }
    assert!(saw_reserved && saw_food_cargo && saw_water_cargo && saw_delivery);
}

#[test]
fn crop_water_never_appears_as_household_delivery() {
    let mut sim = scenarios::dry_cultivated_settlement();
    sim.step(
        0.1,
        &[
            Action::SetWellPumpMode {
                id: 1,
                mode: PumpMode::FillBuffer,
            },
            Action::SetCropTending {
                source: SourceId(1),
                enabled: true,
            },
        ],
    );
    for _ in 0..2000 {
        sim.step(0.1, &[]);
        if let Some(p) = sim
            .people
            .all()
            .iter()
            .find(|p| p.irrigation_task.is_some() && p.carried_water > 0.)
        {
            let outlook = Outlook::read(&sim);
            let h = outlook.homes.iter().find(|h| h.id == p.home).unwrap();
            assert_eq!(p.carried_water, 2.);
            assert_eq!(h.carried_water, 0.);
            return;
        }
    }
    panic!("fixture never carried crop water");
}
