use super::*;
impl Food {
    pub(crate) fn valid(&self, sim: &SimState) -> bool {
        let mut ids = std::collections::BTreeSet::new();
        if !self.initialized
            || self.elapsed_ms >= 30_000
            || self.homes.len() > 10_000
            || self.sources.len() > 10_000
            || self
                .sources
                .iter()
                .filter(|s| matches!(s.kind, cultivation::Kind::Barley(_)))
                .count()
                > cultivation::MAX_PLOTS
            || !self.ids.covers(self.sources.iter().map(|s| s.id.0))
            || !self.sources.iter().all(|s| {
                ids.insert(s.id)
                    && s.initial <= 1_000_000
                    && match &s.kind {
                        cultivation::Kind::Legacy => false,
                        cultivation::Kind::Wild => s.remaining <= s.initial,
                        cultivation::Kind::Barley(c) => {
                            s.initial == 0
                                && c.soil_water.as_ref().is_some_and(|w| w.valid())
                                && s.remaining <= cultivation::YIELD
                                && c.produced <= 1_000_000_000_000
                                && c.produced >= u64::from(s.remaining)
                                && c.produced % u64::from(cultivation::YIELD) == 0
                                && c.growth_units < cultivation::GROWTH_UNITS
                                && (s.remaining == 0 || c.growth_units == 0)
                                && s.position.x >= cultivation::RADIUS
                                && s.position.z >= cultivation::RADIUS
                                && s.position.x + cultivation::RADIUS < sim.grid.width as f32
                                && s.position.z + cultivation::RADIUS < sim.grid.height as f32
                        }
                    }
                    && s.position.x.is_finite()
                    && s.position.z.is_finite()
                    && s.position.x >= 0.
                    && s.position.x < sim.grid.width as f32
                    && s.position.z >= 0.
                    && s.position.z < sim.grid.height as f32
            })
            || !self.homes.iter().all(|(id, h)| {
                sim.buildings.iter().any(|b| b.id == *id)
                    && h.stored <= CAPACITY
                    && h.consumed <= 1_000_000_000_000
            })
            || !sim
                .people
                .all()
                .iter()
                .all(|p| self.homes.contains_key(&p.home))
        {
            return false;
        }
        let harvested: u64 = self
            .sources
            .iter()
            .map(|s| u64::from(s.initial) + s.produced() - u64::from(s.remaining))
            .sum();
        let accounted: u64 = self
            .homes
            .values()
            .map(|h| u64::from(h.stored) + h.consumed)
            .sum::<u64>()
            + sim
                .people
                .all()
                .iter()
                .map(|p| u64::from(p.carried_food))
                .sum::<u64>();
        harvested == accounted
    }
}
