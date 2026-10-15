use super::{CONTENT, RULES, SCHEMA, SaveError, Snapshot};
use crate::{actions::Action, spatial::Position, units};
use std::collections::BTreeSet;

fn require(ok: bool, context: &str) -> Result<(), SaveError> {
    if ok {
        Ok(())
    } else {
        Err(SaveError(format!("invalid save: {context}")))
    }
}
fn range(x: f32, lo: f32, hi: f32) -> bool {
    x.is_finite() && (lo..=hi).contains(&x)
}
fn scalar(x: f32) -> bool {
    range(x, -1e6, 1e6)
}
fn positive(x: f32) -> bool {
    range(x, f32::MIN_POSITIVE, 1e6)
}
fn point(p: Position) -> bool {
    scalar(p.x) && scalar(p.z)
}

pub(super) fn validate(save: &Snapshot) -> Result<(), SaveError> {
    require(save.schema == SCHEMA, "unsupported schema (expected 3)")?;
    require(save.rules == RULES, "incompatible simulation rules")?;
    require(
        save.content == CONTENT,
        "missing or incompatible builtin content",
    )?;
    require(
        !save.world_id.is_empty()
            && save.world_id.len() <= 128
            && save
                .world_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "world identity",
    )?;
    require(
        save.completed_ticks < 1_000_000_000_000,
        "logical tick limit",
    )?;
    let s = &save.state;
    let (w, h) = (s.grid.width, s.grid.height);
    require(
        (1..=512).contains(&w) && (1..=512).contains(&h),
        "grid dimensions (1..512)",
    )?;
    let cells = w as usize * h as usize;
    require(
        s.grid.cells().len() == cells && s.subsurface.columns.len() == cells,
        "grid/column counts",
    )?;
    require(
        (s.config.width, s.config.height) == (w, h) && s.subsurface.dimensions() == (w, h),
        "inconsistent field dimensions",
    )?;
    require(
        range(s.sea_level, units::MIN_ELEVATION, units::MAX_ELEVATION)
            && scalar(s.config.sea_level)
            && positive(s.config.tick_dt),
        "configuration",
    )?;
    require(
        s.time.is_finite()
            && (0.0..=1e12).contains(&s.time)
            && range(s.stability_score, 0., 1.)
            && range(s.research_points, 0., 1e12),
        "time or global quantities",
    )?;
    require(
        matches!(s.geography.version, 0 | 9),
        "unsupported surface/generation recipe",
    )?;
    for c in s.grid.cells() {
        require(
            range(c.elevation, units::MIN_ELEVATION, units::MAX_ELEVATION)
                && range(c.water_depth, 0., 2000.)
                && (c.water_depth - (s.sea_level - c.elevation).max(0.)).abs() <= 1e-4
                && range(c.rainfall, 0., 1e6)
                && range(c.drainage, 0., 1e9)
                && [
                    c.temp,
                    c.moisture,
                    c.fertility,
                    c.toxicity,
                    c.fungus,
                    c.plants,
                ]
                .into_iter()
                .all(|x| range(x, 0., 1.)),
            "environmental cell values",
        )?;
    }
    for c in &s.subsurface.columns {
        require(
            scalar(c.aquifer_base)
                && scalar(c.aquifer_top)
                && c.aquifer_base <= c.aquifer_top
                && range(c.capacity, 0., 1e9)
                && range(c.stored, 0., c.capacity)
                && range(c.recharge, 0., 1e6)
                && range(c.permeability, 0., 1.),
            "aquifer column values",
        )?;
    }
    let inside =
        |p: Position| point(p) && p.x >= 0. && p.z >= 0. && p.x < w as f32 && p.z < h as f32;
    require(
        s.buildings.len() <= 10_000
            && s.paths.len() <= 10_000
            && s.subsurface.wells.len() <= 10_000
            && s.subsurface.deposits.len() <= 10_000
            && s.outposts.len() <= cells,
        "object count limit",
    )?;
    require(
        s.building_ids.covers(s.buildings.iter().map(|b| b.id))
            && s.path_ids.covers(s.paths.iter().map(|p| p.id))
            && s.subsurface
                .well_ids
                .covers(s.subsurface.wells.iter().map(|w| w.id)),
        "identity allocation counters",
    )?;
    require(s.people.valid(s), "resident identities, homes or routine")?;
    let mut ids = BTreeSet::new();
    for b in &s.buildings {
        require(
            ids.insert(b.id) && inside(b.position) && scalar(b.rotation),
            "building identity/position",
        )?;
    }
    ids.clear();
    for well in &s.subsurface.wells {
        require(
            ids.insert(well.id)
                && inside(well.position)
                && well.recovered.is_finite()
                && (0.0..=1e15).contains(&well.recovered)
                && range(well.last_yield, 0., 1e6),
            "well identity/quantities",
        )?;
    }
    let mut ids = BTreeSet::new();
    for d in &s.subsurface.deposits {
        require(
            d.id > 0
                && ids.insert(d.id)
                && inside(d.center)
                && d.radii.into_iter().all(positive)
                && scalar(d.rotation)
                && range(d.irregularity, 0., 1.)
                && scalar(d.roof)
                && positive(d.thickness)
                && range(d.grade, 0., 1.),
            "deposit identity/shape",
        )?;
    }
    let mut outposts = BTreeSet::new();
    for &(x, z) in &s.outposts {
        require(
            x < w && z < h && outposts.insert((x, z)),
            "outpost coordinate/duplicates",
        )?;
    }
    let mut points = 0usize;
    ids.clear();
    for path in &s.paths {
        points += path.points.len();
        require(
            ids.insert(path.id)
                && path.points.len() >= 2
                && positive(path.width)
                && path.points.iter().copied().all(inside),
            "path geometry",
        )?;
    }
    require(
        s.geography.rivers.len() <= 10_000 && s.geography.cover.len() <= 500_000,
        "geography count limit",
    )?;
    let mut index_entries = 0u64;
    for river in &s.geography.rivers {
        points += river.points.len();
        require(
            river.points.len() >= 2
                && positive(river.width)
                && river.width <= w.max(h) as f32
                && river.points.iter().copied().all(inside),
            "river geometry",
        )?;
        require(
            (river.widths.is_empty() || river.widths.len() == river.points.len())
                && river
                    .widths
                    .iter()
                    .all(|&x| positive(x) && x <= river.width),
            "river widths",
        )?;
        // Bound the cache's actual broad-phase work before Index::new allocates.
        let margin = river.width * 0.5 + crate::surface::channels::BANK;
        for pair in river.points.windows(2) {
            let nx = ((pair[0].x - pair[1].x).abs() + margin * 2.).ceil() as u64 + 2;
            let nz = ((pair[0].z - pair[1].z).abs() + margin * 2.).ceil() as u64 + 2;
            index_entries += nx.min(u64::from(w)) * nz.min(u64::from(h));
            require(index_entries <= 4_000_000, "channel reconstruction budget")?;
        }
    }
    require(points <= 1_000_000, "path/river point budget")?;
    for c in &s.geography.cover {
        require(
            inside(c.position) && range(c.size, 0.001, 100.),
            "cover position/size",
        )?;
    }
    let climate = s.geography.climate;
    require(
        scalar(climate.sea_level_temperature)
            && positive(climate.moisture_scale)
            && range(climate.summer_drought, 0., 1.),
        "climate definition",
    )?;
    require(save.pending.len() <= 10_000, "pending command count limit")?;
    for action in &save.pending {
        let valid = match action {
            Action::Terraform { x, y, strength, .. } => {
                x.unsigned_abs() <= 1_000_000 && y.unsigned_abs() <= 1_000_000 && scalar(*strength)
            }
            Action::SeedFungus { x, y, .. }
            | Action::SeedPlants { x, y, .. }
            | Action::BuildOutpost { x, y } => {
                x.unsigned_abs() <= 1_000_000 && y.unsigned_abs() <= 1_000_000
            }
            Action::PlaceBuilding { position, rotation } => point(*position) && scalar(*rotation),
            Action::DrillWell { position } => point(*position),
            Action::SetWellActive { .. } => true,
            Action::BuildPath { waypoints, width } => {
                points += waypoints.len();
                scalar(*width) && waypoints.iter().copied().all(point)
            }
        };
        require(
            valid && points <= 1_000_000,
            "pending command representation",
        )?;
    }
    // Pending actions need not pass placement/target validation now: they are
    // still intent, revalidated at their original next-tick commit boundary.
    Ok(())
}
