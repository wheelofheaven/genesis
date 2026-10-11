//! Rendering owns GPU resources and camera math; it knows no simulation types.
pub mod atmosphere;
pub mod camera;
pub mod geometry;
pub mod glass;
pub mod gpu;
pub mod instances;
pub mod visibility;
pub mod world_mesh;

pub mod shadows;
pub mod tonemap;

mod clouds;
mod timing;
