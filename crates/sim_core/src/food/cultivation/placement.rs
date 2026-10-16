use super::*;
use crate::{paths, settlement::Building, surface, surface::walking};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    InvalidPosition,
    OutsideMap,
    Ground,
    Occupied,
    Capacity,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidPosition => "Invalid plot position",
            Self::OutsideMap => "Keep the whole plot inside the region",
            Self::Ground => "Choose gentle, dry ground clear of trees, rocks and rivers",
            Self::Occupied => "Keep plots, food patches, buildings, paths and wells clear",
            Self::Capacity => "Crop plot capacity exhausted",
        })
    }
}
/// Circular footprint; continuous positions, independent of environmental sample cells.
pub fn footprint(p: Position) -> [Position; 33] {
    std::array::from_fn(|i| {
        let a = i as f32 * std::f32::consts::TAU / 32.;
        Position::new(p.x + RADIUS * a.cos(), p.z + RADIUS * a.sin())
    })
}
pub(super) fn ground(sim: &SimState, p: Position) -> Result<(), Error> {
    if !p.x.is_finite() || !p.z.is_finite() {
        return Err(Error::InvalidPosition);
    }
    if p.x < RADIUS
        || p.z < RADIUS
        || p.x + RADIUS >= sim.grid.width as f32
        || p.z + RADIUS >= sim.grid.height as f32
    {
        return Err(Error::OutsideMap);
    }
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    let query = walking::Query::local(
        sim,
        [p.x - RADIUS, p.z - RADIUS, p.x + RADIUS, p.z + RADIUS],
    );
    // Half-metre sampling across the entire small disc, including its interior.
    for x in -4..=4_i32 {
        for z in -4..=4_i32 {
            if x * x + z * z > 16 {
                continue;
            }
            let q = Position::new(p.x + x as f32 * metres(0.5), p.z + z as f32 * metres(0.5));
            if !query.standable(q) {
                return Err(Error::Ground);
            }
            let h = surface::elevation(sim, q);
            low = low.min(h);
            high = high.max(h);
        }
    }
    if high - low > metres(0.8) {
        return Err(Error::Ground);
    }
    Ok(())
}
/// Circle against rotated building rectangle, used in both placement directions.
pub fn overlaps_building(p: Position, b: &Building) -> bool {
    let (s, c) = b.rotation.sin_cos();
    let dx = p.x - b.position.x;
    let dz = p.z - b.position.z;
    let x = ((dx * c + dz * s).abs() - Building::HALF_WIDTH).max(0.);
    let z = ((dz * c - dx * s).abs() - Building::HALF_DEPTH).max(0.);
    x.hypot(z) < RADIUS + metres(0.5)
}
pub fn validate(sim: &SimState, p: Position) -> Result<(), Error> {
    ground(sim, p)?;
    if sim.food.sources.len() >= 10_000
        || sim
            .food
            .sources
            .iter()
            .filter(|s| matches!(s.kind, Kind::Barley(_)))
            .count()
            >= MAX_PLOTS
    {
        return Err(Error::Capacity);
    }
    if sim.food.sources.iter().any(|s| {
        paths::distance(s.position, p)
            < RADIUS
                + if matches!(s.kind, Kind::Barley(_)) {
                    RADIUS
                } else {
                    metres(1.5)
                }
                + metres(0.5)
    }) || sim.buildings.iter().any(|b| overlaps_building(p, b))
        || sim
            .paths
            .iter()
            .any(|path| paths::on_path(path, p, RADIUS + metres(0.5)))
        || sim
            .subsurface
            .wells
            .iter()
            .any(|w| paths::distance(w.position, p) < RADIUS + metres(0.8))
    {
        return Err(Error::Occupied);
    }
    Ok(())
}
