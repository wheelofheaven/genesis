use super::{SaveError, Snapshot};
use crate::identity::Allocator;

pub(super) fn upgrade(save: &mut Snapshot) -> Result<(), SaveError> {
    if matches!(save.schema, 1 | 2) {
        if save.rules != "genesis:rules/1"
            || (save.state.people != crate::people::People::default()
                && save.state.people != crate::people::People::legacy_missing())
        {
            return Err(SaveError(
                "invalid legacy rules or unexpected population".into(),
            ));
        }
        if save.schema == 1 {
            upgrade_identities(save)?;
        }
        save.state.people = crate::people::People::default();
        save.schema = 3;
        save.rules = "genesis:rules/2".into();
    }
    if save.schema == 3 {
        if save.rules != "genesis:rules/2"
            || !save.state.households.is_missing()
            || !save.state.people.legacy_water()
            || save
                .state
                .subsurface
                .wells
                .iter()
                .any(|w| w.supply.is_some())
        {
            return Err(SaveError(
                "invalid V3 rules or unexpected water commitments".into(),
            ));
        }
        save.state.households = crate::households::Households::default();
        save.state.households.initialize(&save.state.people);
        for well in &mut save.state.subsurface.wells {
            well.supply = Some(crate::subsurface::supply::Supply {
                released: well.recovered,
                ..Default::default()
            });
        }
        save.schema = 4;
        save.rules = "genesis:rules/3".into();
    }
    if save.schema == 4 {
        if save.rules != "genesis:rules/3"
            || !save.state.food.is_missing()
            || !save.state.people.legacy_food()
        {
            return Err(SaveError(
                "invalid V4 rules or unexpected food commitments".into(),
            ));
        }
        save.state.food = crate::food::Food::default();
        save.state.food.initialize(&save.state.people);
        save.state.people.initialize_food_priority();
        save.schema = 5;
        save.rules = "genesis:rules/4".into();
    }
    if save.schema == 5 {
        if save.rules != "genesis:rules/4"
            || save
                .state
                .food
                .sources()
                .iter()
                .any(|s| s.kind != crate::food::cultivation::Kind::Legacy)
        {
            return Err(SaveError("invalid V5 rules or unexpected crops".into()));
        }
        for s in &mut save.state.food.sources {
            s.kind = crate::food::cultivation::Kind::Wild;
        }
        save.schema = super::SCHEMA;
        save.rules = super::RULES.into();
    }
    Ok(())
}
fn upgrade_identities(save: &mut Snapshot) -> Result<(), SaveError> {
    if save.schema != 1 {
        return Ok(()); // Validation reports unsupported versions.
    }
    let s = &mut save.state;
    // Only the original append-only layout can be interpreted safely. Never
    // repair duplicate identities or guess an allocator from a damaged V2 save.
    if s.buildings.len() > 10_000
        || s.paths.len() > 10_000
        || s.subsurface.wells.len() > 10_000
        || s.building_ids != Allocator::default()
        || s.path_ids != Allocator::default()
        || s.subsurface.well_ids != Allocator::default()
        || s.buildings
            .iter()
            .enumerate()
            .any(|(i, b)| b.id != i as u64 + 1)
        || s.subsurface
            .wells
            .iter()
            .enumerate()
            .any(|(i, w)| w.id != i as u64 + 1)
        || s.paths.iter().any(|p| p.id != 0)
    {
        return Err(SaveError("invalid V1 append-only identities".into()));
    }
    s.building_ids = Allocator::legacy(s.buildings.len());
    s.path_ids = Allocator::legacy(s.paths.len());
    s.subsurface.well_ids = Allocator::legacy(s.subsurface.wells.len());
    for (i, path) in s.paths.iter_mut().enumerate() {
        path.id = i as u64 + 1;
    }
    save.schema = 2;
    Ok(())
}
