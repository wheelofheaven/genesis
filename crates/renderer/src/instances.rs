//! Repeated scenery shares geometry, with complementary coverage for LOD fades.
use crate::{geometry::Vertex, shadows::CASCADES};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub position_scale: [f32; 4],
    pub tint_rotation: [f32; 4],
    /// Half-open dither interval [low, high); full coverage is [0, 1].
    /// Materials 10 (blades) and 12 (litter) instead store world-space ground slopes
    /// (dh/dx, dh/dz); its vertex shader restores full coverage before fading.
    pub coverage: [f32; 2],
}
/// Scenery carries normals and material coordinates; regional terrain stays compact.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneryVertex {
    pub position: [f32; 3],
    /// RGB tint and material tag. Leaves encode crown occlusion in the fraction:
    /// broadleaf 5..=5.15, needles 5.3..=5.45 (larger means less sky access).
    /// Bark uses 6 + circumference_cells / 256, with 3..=128 cells.
    pub color: [f32; 4],
    pub normal: [f32; 3],
    /// Leaf cutouts, blade (across, height), or bark (turns, axial prototype metres).
    pub uv: [f32; 2],
}
#[derive(Default, Debug)]
pub struct ShadowStats {
    pub batches: usize,
    pub instances: u64,
    pub vertices: u64,
}
struct Batch {
    mesh: wgpu::Buffer,
    vertices: u32,
    instances: wgpu::Buffer,
    count: u32,
    shadow_instances: [wgpu::Buffer; CASCADES],
    shadow_count: [Option<u32>; CASCADES],
}
pub struct Instances {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    batches: Vec<Batch>,
}
impl Instances {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        layout: &wgpu::PipelineLayout,
        shadow_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
    ) -> Self {
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("instanced scenery"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("instanced_vertex"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: size_of::<SceneryVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 4 => Float32x3, 5 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 6 => Float32x2],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: crate::gpu::SAMPLE_COUNT,
                alpha_to_coverage_enabled: true,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("instanced scenery"),
            layout: Some(shadow_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("shadow_instanced_vertex"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: size_of::<SceneryVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 4 => Float32x3, 5 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 6 => Float32x2],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("shadow_fragment"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2., clamp: 0. },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self {
            shadow_pipeline,
            pipeline,
            batches: Vec::new(),
        }
    }
    pub fn add_mesh(&mut self, device: &wgpu::Device, vertices: &[Vertex]) -> usize {
        let vertices: Vec<_> = vertices
            .iter()
            .map(|v| SceneryVertex {
                position: v.position,
                color: v.color,
                normal: [0., 1., 0.],
                uv: [0., 0.],
            })
            .collect();
        self.add_scenery(device, &vertices)
    }
    pub fn add_scenery(&mut self, device: &wgpu::Device, vertices: &[SceneryVertex]) -> usize {
        let index = self.batches.len();
        self.batches.push(Batch {
            mesh: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shared scenery mesh"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            vertices: vertices.len() as u32,
            instances: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scenery placements"),
                size: size_of::<Instance>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            count: 0,
            shadow_instances: std::array::from_fn(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("shadow placements"),
                    size: size_of::<Instance>() as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            }),
            shadow_count: [None; CASCADES],
        });
        index
    }
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: usize,
        instances: &[Instance],
    ) {
        let batch = &mut self.batches[index];
        let bytes = bytemuck::cast_slice(instances);
        if bytes.len() as u64 > batch.instances.size() {
            batch.instances = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scenery placements"),
                size: (bytes.len() as u64).next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !bytes.is_empty() {
            queue.write_buffer(&batch.instances, 0, bytes);
        }
        batch.count = instances.len() as u32;
    }
    pub fn render_shadow(&self, pass: &mut wgpu::RenderPass<'_>, cascade: usize) {
        pass.set_pipeline(&self.shadow_pipeline);
        for batch in &self.batches {
            let count =
                batch.shadow_count[cascade].unwrap_or(if cascade == 0 { batch.count } else { 0 });
            if count == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, batch.mesh.slice(..));
            pass.set_vertex_buffer(
                1,
                if batch.shadow_count[cascade].is_some() {
                    batch.shadow_instances[cascade].slice(..)
                } else {
                    batch.instances.slice(..)
                },
            );
            pass.draw(0..batch.vertices, 0..count);
        }
    }
    pub fn main_stats(&self) -> ShadowStats {
        let mut stats = ShadowStats::default();
        for batch in &self.batches {
            if batch.count > 0 {
                stats.batches += 1;
                stats.instances += u64::from(batch.count);
                stats.vertices += u64::from(batch.vertices) * u64::from(batch.count);
            }
        }
        stats
    }
    pub fn shadow_stats(&self, cascade: usize) -> ShadowStats {
        let mut stats = ShadowStats::default();
        for batch in &self.batches {
            let count =
                batch.shadow_count[cascade].unwrap_or(if cascade == 0 { batch.count } else { 0 })
                    as u64;
            if count > 0 {
                stats.batches += 1;
                stats.instances += count;
                stats.vertices += count * batch.vertices as u64;
            }
        }
        stats
    }
    pub fn upload_shadows(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: usize,
        cascade: usize,
        instances: &[Instance],
    ) {
        let batch = &mut self.batches[index];
        let bytes = bytemuck::cast_slice(instances);
        if bytes.len() as u64 > batch.shadow_instances[cascade].size() {
            batch.shadow_instances[cascade] = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shadow placements"),
                size: (bytes.len() as u64).next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !bytes.is_empty() {
            queue.write_buffer(&batch.shadow_instances[cascade], 0, bytes);
        }
        batch.shadow_count[cascade] = Some(instances.len() as u32);
    }
    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        self.draw_batches(pass);
    }
    fn draw_batches(&self, pass: &mut wgpu::RenderPass<'_>) {
        for batch in &self.batches {
            if batch.count == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, batch.mesh.slice(..));
            pass.set_vertex_buffer(1, batch.instances.slice(..));
            pass.draw(0..batch.vertices, 0..batch.count);
        }
    }
}
