//! Destination/scenario metadata, independent of player role and presentation.
//! Earth previews are the only implemented source. Future planets need their own
//! scenario providers and physical models, not an Earth region with a new name.
use crate::worldgen::Region;

pub struct PlanetDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub system: &'static str,
    pub environment: &'static str,
}
pub const EARTH: PlanetDefinition = PlanetDefinition {
    id: "earth",
    name: "Earth",
    system: "Sol",
    environment: "Terrestrial world · oceans and continents",
};
pub const PLANETS: &[PlanetDefinition] = &[EARTH];

pub struct ScenarioDefinition {
    pub id: &'static str,
    pub planet_id: &'static str,
    pub region: Region,
    /// None means an undated procedural preview, not a historical reconstruction.
    pub start_year: Option<i32>,
    pub conditions: &'static str,
}
pub const SCENARIOS: &[ScenarioDefinition] = &[
    ScenarioDefinition {
        id: "levant-coast-preview",
        planet_id: "earth",
        region: Region::LevantCoast,
        start_year: None,
        conditions: "Coastal lowlands, inland slopes, and freshwater corridors. Survey aquifers before choosing where the settlement grows.",
    },
    ScenarioDefinition {
        id: "levant-highlands-preview",
        planet_id: "earth",
        region: Region::LevantHighlands,
        start_year: None,
        conditions: "Ridges and connected valleys. Follow gentle ground, examine drainage, and find space for a settlement to expand.",
    },
    ScenarioDefinition {
        id: "mediterranean-island-preview",
        planet_id: "earth",
        region: Region::Island,
        start_year: None,
        conditions: "An isolated island and its surrounding shelf. Inspect freshwater supplies and the terrain beneath the sea.",
    },
];
pub fn scenario(planet_id: &str, region: Region) -> Option<&'static ScenarioDefinition> {
    SCENARIOS
        .iter()
        .find(|s| s.planet_id == planet_id && s.region == region)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scenarios_resolve_only_for_their_own_destination() {
        for region in Region::ALL {
            let scenario = scenario(EARTH.id, region).unwrap();
            assert!(PLANETS.iter().any(|p| p.id == scenario.planet_id));
            assert!(scenario.start_year.is_none());
            assert!(super::scenario("unknown-exoplanet", region).is_none());
        }
        for (i, a) in SCENARIOS.iter().enumerate() {
            assert!(SCENARIOS.iter().skip(i + 1).all(|b| a.id != b.id));
        }
    }
}
