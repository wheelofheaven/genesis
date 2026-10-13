mod climate;
mod life;
mod seeding;
mod terraform;

pub use climate::update_climate;
pub use life::update_life;
pub(crate) use seeding::seed_life;
pub(crate) use terraform::terraform_patch;
pub use terraform::{TerraformError, apply_terraform, try_apply_terraform, update_water_depth};
