//! Static, opaque GLB assets in metres. CPU import is independent of GPU upload.
mod draw;
mod import;
mod textures;
pub use draw::{AssetStats, Assets};
use glam::Vec3;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
    pub occlusion_uv: [f32; 2],
}
pub struct Primitive {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material: usize,
}
pub struct Material {
    pub base: [f32; 4],
    /// Metallic, roughness, normal scale, occlusion strength.
    pub surface: [f32; 4],
    pub emissive: [f32; 4],
    /// Occlusion alone may use a separate non-overlapping UV set.
    pub occlusion_uv: u32,
    /// Base colour, normal, metallic/roughness, occlusion, emissive.
    pub images: [image::RgbaImage; 5],
}
pub struct Model {
    pub primitives: Vec<Primitive>,
    pub materials: Vec<Material>,
    pub min: Vec3,
    pub max: Vec3,
}
impl Model {
    /// Supported subset: embedded PNGs/buffer, opaque triangle meshes with UV0,
    /// normals/tangents, optional UV1 occlusion charts, static node transforms
    /// and repeat/linear samplers.
    /// Unsupported features produce errors instead of silently changing the asset.
    pub fn from_glb(bytes: &[u8]) -> Result<Self> {
        import::load(bytes)
    }
    pub fn triangles(&self) -> usize {
        self.primitives.iter().map(|p| p.indices.len() / 3).sum()
    }
}
