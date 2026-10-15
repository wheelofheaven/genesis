use super::*;
impl People {
    pub(crate) fn food_valid(&self, sim: &SimState) -> bool {
        let mut homes = std::collections::BTreeSet::new();
        self.entries.len() <= 10_000
            && self.entries.iter().all(|p| {
                if p.last_supply == SupplyKind::Uninitialized {
                    return false;
                }
                if let Some(t) = &p.food_task {
                    p.water_task.is_none()
                        && p.carried_water == 0.
                        && homes.insert(p.home)
                        && t.amount == stock::LOAD
                        && sim
                            .food
                            .source(t.source)
                            .is_some_and(|s| s.position == t.pickup)
                        && sim
                            .food
                            .home(p.home)
                            .is_some_and(|h| h.stored <= stock::CAPACITY - t.amount)
                        && match p.activity {
                            Activity::FetchingFood => p.carried_food == 0,
                            Activity::GatheringFood => {
                                p.carried_food == 0
                                    && p.remaining_ms <= stock::GATHER_MS
                                    && distance(p.position, t.pickup) < metres(0.01)
                            }
                            Activity::CarryingFood => p.carried_food == t.amount,
                            _ => false,
                        }
                } else {
                    p.carried_food == 0
                        && !matches!(
                            p.activity,
                            Activity::FetchingFood
                                | Activity::GatheringFood
                                | Activity::CarryingFood
                        )
                }
            })
            && sim
                .food
                .sources()
                .iter()
                .all(|s| self.reserved_food(s.id) <= s.remaining)
    }
    pub(crate) fn legacy_food(&self) -> bool {
        self.entries.iter().all(|p| {
            p.food_task.is_none()
                && p.carried_food == 0
                && p.food_issue.is_none()
                && p.last_supply == SupplyKind::Uninitialized
                && !matches!(
                    p.activity,
                    Activity::FetchingFood | Activity::GatheringFood | Activity::CarryingFood
                )
        })
    }
    pub(crate) fn initialize_food_priority(&mut self) {
        for p in &mut self.entries {
            p.last_supply = SupplyKind::default();
        }
    }
}
