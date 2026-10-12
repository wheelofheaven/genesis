use super::{SaveError, Snapshot};
use crate::identity::Allocator;

pub(super) fn upgrade(save: &mut Snapshot) -> Result<(), SaveError> {
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
    save.schema = super::SCHEMA;
    Ok(())
}
