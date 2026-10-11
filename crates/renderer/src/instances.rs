//! Repeated scenery shares geometry; each placement uploads only 32 bytes.
use crate::geometry::Vertex;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub position_scale: [f32; 4],
    pub tint_rotation: [f32; 4],
}
struct Batch {
    mesh: wgpu::Buffer,
    vertices: u32,
    instances: wgpu::Buffer,
    count: u32,
}
pub struct Instances {
    pipeline: wgpu::RenderPipeline,
    batches: Vec<Batch>,
}
impl Instances {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        layout: &wgpu::PipelineLayout,
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
                        array_stride: size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4],
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
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            batches: Vec::new(),
        }
    }
    pub fn add_mesh(&mut self, device: &wgpu::Device, vertices: &[Vertex]) -> usize {
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
                size: 32,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            count: 0,
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
    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
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
