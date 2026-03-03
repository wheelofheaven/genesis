mod climate;
mod life;
mod terraform;

pub use climate::update_climate;
pub use life::update_life;
pub use terraform::{apply_terraform, update_water_depth};
