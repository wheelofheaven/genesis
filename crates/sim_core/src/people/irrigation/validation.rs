use super::*;
impl People {
    pub(crate) fn irrigation_valid(&self, sim: &SimState) -> bool {
        let mut plots = std::collections::BTreeSet::new();
        self.entries.iter().all(|p| {
            if let Some(t) = &p.irrigation_task {
                p.water_task.is_none() && p.food_task.is_none() && p.carried_food == 0
                    && plots.insert(t.source)
                    && t.pickup.x.is_finite() && t.pickup.z.is_finite()
                    && t.pickup.x >= 0. && t.pickup.z >= 0.
                    && t.pickup.x < sim.grid.width as f32 && t.pickup.z < sim.grid.height as f32
                    && sim.subsurface.wells.iter().any(|w| w.id == t.well
                        && distance(w.position, t.pickup) <= metres(1.)
                        && distance(w.position, t.delivery) <= soil::RANGE)
                    && sim.food.source(t.source).is_some_and(|s| s.position == t.delivery
                        && matches!(&s.kind, Kind::Barley(c) if c.soil_water.as_ref().is_some_and(|w|
                            w.stored <= soil::CAPACITY - soil::DOSE
                            && w.applied <= 1_000_000_000_000_000 - soil::DOSE)))
                    && match p.activity {
                        Activity::FetchingIrrigationWater => p.carried_water == 0.,
                        Activity::WateringCrop => p.carried_water == households::CONTAINER,
                        _ => false,
                    }
            } else {
                !matches!(p.activity, Activity::FetchingIrrigationWater | Activity::WateringCrop)
            }
        })
    }
    pub(crate) fn legacy_irrigation(&self) -> bool {
        self.entries.iter().all(|p| {
            p.irrigation_task.is_none()
                && p.irrigation_issue.is_none()
                && !matches!(
                    p.activity,
                    Activity::FetchingIrrigationWater | Activity::WateringCrop
                )
        })
    }
}
