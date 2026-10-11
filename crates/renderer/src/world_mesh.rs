//! Resident spatial batches for terrain, water and settlement geometry.
//! Whole triangles are assigned by centroid; bounds include every vertex, so
//! large coast/context triangles remain conservatively visible across cells.
use crate::{geometry::Vertex, visibility::Frustum};
use glam::Vec3;
use std::{
    collections::{BTreeMap, hash_map::DefaultHasher},
    hash::Hasher,
};

const CHUNK_SIZE: f32 = 16.; // 320 metres at the current scene scale.
type Key = (u8, i32, i32);

#[derive(Clone, Copy)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}
impl Default for Bounds {
    fn default() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }
}
impl Bounds {
    fn extend(&mut self, v: &Vertex) {
        let p = Vec3::from_array(v.position);
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }
    fn visible(self, frustum: &Frustum) -> bool {
        frustum.bounds(self.min, self.max)
    }
}

#[derive(Default)]
pub struct ChunkMesh {
    pub vertices: Vec<Vertex>,
    pub bounds: Bounds,
    pub shadow_bounds: Bounds,
    pub shadow_vertices: u32,
    uncast: Vec<Vertex>,
}
impl ChunkMesh {
    pub fn visible_vertices(&self, frustum: &Frustum, shadow: bool) -> u32 {
        visible_count(
            self.bounds,
            self.shadow_bounds,
            self.vertices.len() as u32,
            self.shadow_vertices,
            frustum,
            shadow,
        )
    }
    pub fn fingerprint(&self) -> u64 {
        let mut hash = DefaultHasher::new();
        hash.write_u32(self.shadow_vertices);
        hash.write(bytemuck::cast_slice(&self.vertices));
        hash.finish()
    }
}

pub fn partition(vertices: &[Vertex]) -> BTreeMap<Key, ChunkMesh> {
    debug_assert_eq!(vertices.len() % 3, 0);
    let mut chunks = BTreeMap::<Key, ChunkMesh>::new();
    for tri in vertices.chunks_exact(3) {
        let center = tri
            .iter()
            .map(|v| Vec3::from_array(v.position))
            .sum::<Vec3>()
            / 3.;
        // Coarse horizon rings must not become thousands of two-triangle draws.
        // Group them in larger pages; detailed playable terrain keeps small cells.
        let min = tri.iter().fold(Vec3::splat(f32::INFINITY), |a, v| {
            a.min(Vec3::from_array(v.position))
        });
        let max = tri.iter().fold(Vec3::splat(f32::NEG_INFINITY), |a, v| {
            a.max(Vec3::from_array(v.position))
        });
        let span = (max.x - min.x).max(max.z - min.z);
        let (tier, size) = if span >= 8. {
            (2, CHUNK_SIZE * 8.)
        } else if span >= 2. {
            (1, CHUNK_SIZE * 4.)
        } else {
            (0, CHUNK_SIZE)
        };
        let key = (
            tier,
            (center.x / size).floor() as i32,
            (center.z / size).floor() as i32,
        );
        let chunk = chunks.entry(key).or_default();
        let uncast = tri.iter().any(|v| v.color[3] < 0. || v.color[3] >= 14.);
        for v in tri {
            chunk.bounds.extend(v);
        }
        if uncast {
            chunk.uncast.extend_from_slice(tri);
        } else {
            for v in tri {
                chunk.shadow_bounds.extend(v);
            }
            chunk.vertices.extend_from_slice(tri);
        }
    }
    for chunk in chunks.values_mut() {
        chunk.shadow_vertices = chunk.vertices.len() as u32;
        chunk.vertices.append(&mut chunk.uncast);
    }
    chunks
}

fn visible_count(
    bounds: Bounds,
    shadow_bounds: Bounds,
    vertices: u32,
    casters: u32,
    frustum: &Frustum,
    shadow: bool,
) -> u32 {
    if shadow {
        if casters > 0 && shadow_bounds.visible(frustum) {
            casters
        } else {
            0
        }
    } else if bounds.visible(frustum) {
        vertices
    } else {
        0
    }
}

struct Resident {
    buffer: wgpu::Buffer,
    vertices: u32,
    casters: u32,
    bounds: Bounds,
    shadow_bounds: Bounds,
    fingerprint: u64,
}
impl Resident {
    fn visible(&self, frustum: &Frustum, shadow: bool) -> u32 {
        visible_count(
            self.bounds,
            self.shadow_bounds,
            self.vertices,
            self.casters,
            frustum,
            shadow,
        )
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct UploadStats {
    pub chunks: usize,
    pub changed: usize,
    pub bytes: usize,
}
#[derive(Default, Debug)]
pub struct DrawStats {
    pub chunks: usize,
    pub vertices: u64,
}

#[derive(Default)]
pub struct WorldMesh {
    chunks: BTreeMap<Key, Resident>,
    pub last_upload: UploadStats,
}

/// CPU-only geometry preparation, safe to perform away from the render thread.
/// Fingerprints are computed here, not while submitting replacement GPU buffers.
pub struct PreparedWorld {
    chunks: BTreeMap<Key, (ChunkMesh, u64)>,
}
impl PreparedWorld {
    pub fn new(vertices: &[Vertex]) -> Self {
        Self {
            chunks: partition(vertices)
                .into_iter()
                .map(|(key, chunk)| {
                    let fingerprint = chunk.fingerprint();
                    (key, (chunk, fingerprint))
                })
                .collect(),
        }
    }
}
impl WorldMesh {
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[Vertex]) {
        self.upload_prepared(device, queue, &PreparedWorld::new(vertices));
    }
    pub fn upload_prepared(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &PreparedWorld,
    ) {
        let prepared = &world.chunks;
        self.chunks.retain(|key, _| prepared.contains_key(key));
        let mut stats = UploadStats {
            chunks: prepared.len(),
            ..Default::default()
        };
        for (&key, (chunk, fingerprint)) in prepared {
            let fingerprint = *fingerprint;
            let bytes = bytemuck::cast_slice(&chunk.vertices);
            let resident = self.chunks.entry(key).or_insert_with(|| Resident {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("resident world chunk"),
                    size: (bytes.len() as u64).next_power_of_two(),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                vertices: 0,
                casters: 0,
                bounds: chunk.bounds,
                shadow_bounds: chunk.shadow_bounds,
                fingerprint: 0,
            });
            if resident.fingerprint == fingerprint
                && resident.vertices == chunk.vertices.len() as u32
                && resident.casters == chunk.shadow_vertices
            {
                continue;
            }
            if resident.buffer.size() < bytes.len() as u64 {
                resident.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("resident world chunk"),
                    size: (bytes.len() as u64).next_power_of_two(),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            queue.write_buffer(&resident.buffer, 0, bytes);
            resident.vertices = chunk.vertices.len() as u32;
            resident.casters = chunk.shadow_vertices;
            resident.bounds = chunk.bounds;
            resident.shadow_bounds = chunk.shadow_bounds;
            resident.fingerprint = fingerprint;
            stats.changed += 1;
            stats.bytes += bytes.len();
        }
        self.last_upload = stats;
    }
    pub fn stats(&self, frustum: &Frustum, shadow: bool) -> DrawStats {
        let mut stats = DrawStats::default();
        for chunk in self.chunks.values() {
            let count = chunk.visible(frustum, shadow);
            if count > 0 {
                stats.chunks += 1;
                stats.vertices += count as u64;
            }
        }
        stats
    }
    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>, frustum: &Frustum, shadow: bool) {
        for chunk in self.chunks.values() {
            let count = chunk.visible(frustum, shadow);
            if count == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, chunk.buffer.slice(..));
            pass.draw(0..count, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn triangle(x: f32, material: f32) -> [Vertex; 3] {
        [[x, 0., -1.], [x, 0., -2.], [x + 1., 0., -1.]].map(|position| Vertex {
            position,
            color: [0.3, 0.4, 0.2, material],
        })
    }
    #[test]
    fn partition_preserves_triangles_but_water_and_paths_never_cast_shadows() {
        let mut vertices = Vec::new();
        vertices.extend(triangle(-16., 2.));
        vertices.extend(triangle(1., -5.));
        vertices.extend(triangle(0., 3.));
        vertices.extend(triangle(90., -4.));
        vertices.extend(triangle(2., 14.5));
        let chunks = partition(&vertices);
        assert_eq!(chunks.len(), 3);
        assert_eq!(
            chunks.values().map(|c| c.vertices.len()).sum::<usize>(),
            vertices.len()
        );
        assert_eq!(chunks.values().map(|c| c.shadow_vertices).sum::<u32>(), 6);
        for c in chunks.values() {
            assert!(
                c.vertices[..c.shadow_vertices as usize]
                    .iter()
                    .all(|v| v.color[3] >= 0. && v.color[3] < 14.)
            );
            assert!(
                c.vertices[c.shadow_vertices as usize..]
                    .iter()
                    .all(|v| v.color[3] < 0. || v.color[3] >= 14.)
            );
            for v in &c.vertices {
                let p = Vec3::from_array(v.position);
                assert!(p.cmpge(c.bounds.min).all() && p.cmple(c.bounds.max).all());
            }
        }
    }
    #[test]
    fn changed_geometry_or_colour_invalidates_only_its_chunk() {
        let mut vertices = triangle(0., 2.).to_vec();
        vertices.extend(triangle(32., 2.));
        let a = partition(&vertices);
        assert_eq!(
            a[&(0, 0, -1)].fingerprint(),
            partition(&vertices)[&(0, 0, -1)].fingerprint()
        );
        vertices[0].color[0] += 0.01;
        vertices[1].position[1] += 1.;
        let b = partition(&vertices);
        assert_ne!(a[&(0, 0, -1)].fingerprint(), b[&(0, 0, -1)].fingerprint());
        assert_eq!(a[&(0, 2, -1)].fingerprint(), b[&(0, 2, -1)].fingerprint());
    }
    #[test]
    fn crossing_triangles_use_geometry_bounds_not_cell_bounds() {
        let vertices =
            [[-100., 0., -10.], [100., 0., -10.], [100., 1., -10.]].map(|position| Vertex {
                position,
                color: [1.; 4],
            });
        let chunks = partition(&vertices);
        let frustum = Frustum::new(glam::Mat4::orthographic_rh(-1., 1., -1., 1., 0.1, 20.));
        assert_eq!(
            chunks
                .values()
                .map(|c| c.visible_vertices(&frustum, false))
                .sum::<u32>(),
            3
        );
    }
}
