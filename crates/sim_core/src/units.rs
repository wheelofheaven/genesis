//! Working metric scale for geometry. Resource volumes and time retain their
//! prototype units; this conversion does not calibrate the environmental model.
pub const METRES_PER_UNIT: f32 = 20.;
pub const fn metres(length: f32) -> f32 {
    length / METRES_PER_UNIT
}

/// Safe working bounds covering Earth's relief, in the same metric scale.
pub const MIN_ELEVATION: f32 = -600.;
pub const MAX_ELEVATION: f32 = 450.;
