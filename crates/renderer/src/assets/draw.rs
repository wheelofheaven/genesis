use super::{Model, Vertex, textures};
use crate::{camera::Camera, instances::Instance, visibility::Frustum};
use glam::Vec3;
use wgpu::util::DeviceExt;

struct Primitive {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    material: usize,
}
struct Stream {
    buffer: wgpu::Buffer,
    previous: Vec<Instance>,
    next: Vec<Instance>,
}
struct ModelGpu {
    primitives: Vec<Primitive>,
    materials: Vec<wgpu::BindGroup>,
    streams: [Stream; 3],
    center: Vec3,
    radius: f32,
}
#[derive(Default, Clone, Copy, Debug)]
pub struct AssetStats {
    pub instances: u32,
    pub triangles: u64,
    pub draws: u32,
    pub upload_bytes: usize,
}
pub struct Assets {
    pipeline: wgpu::RenderPipeline,
    shadow: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    models: Vec<ModelGpu>,
    pub stats: AssetStats,
    /// Texture payload bytes, including mips, excluding driver allocation overhead.
    pub texture_bytes: u64,
}
impl Assets {
    pub(crate) fn new(
        device: &wgpu::Device,
        frame: &wgpu::BindGroupLayout,
        shadows: &wgpu::BindGroupLayout,
        clouds: &wgpu::BindGroupLayout,
    ) -> Self {
        let layout = textures::layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("textured asset layouts"),
            bind_group_layouts: &[frame, shadows, clouds, &layout],
            push_constant_ranges: &[],
        });
        let source = [
            include_str!("../frame.wgsl"),
            include_str!("../sunlight.wgsl"),
            include_str!("../sky_radiance.wgsl"),
            include_str!("../aerial.wgsl"),
            include_str!("../cloud_field.wgsl"),
            include_str!("../cloud_shadow.wgsl"),
            include_str!("material.wgsl"),
        ]
        .join("\n");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("imported asset shading"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let buffers = [
            wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Float32x4,3=>Float32x2,6=>Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: size_of::<Instance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![4=>Float32x4,5=>Float32x4],
            },
        ];
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("asset shadow layout"),
            bind_group_layouts: &[frame],
            push_constant_ranges: &[],
        });
        let make = |shadow: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if shadow {
                    "imported asset shadows"
                } else {
                    "imported material world"
                }),
                layout: Some(if shadow {
                    &shadow_layout
                } else {
                    &pipeline_layout
                }),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(if shadow {
                        "asset_shadow"
                    } else {
                        "asset_vertex"
                    }),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                fragment: if shadow {
                    None
                } else {
                    Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("asset_fragment"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: crate::tonemap::WORLD_FORMAT,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    })
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: if shadow {
                        wgpu::CompareFunction::LessEqual
                    } else {
                        wgpu::CompareFunction::Greater
                    },
                    stencil: Default::default(),
                    bias: if shadow {
                        wgpu::DepthBiasState {
                            constant: 2,
                            slope_scale: 2.,
                            clamp: 0.,
                        }
                    } else {
                        Default::default()
                    },
                }),
                multisample: wgpu::MultisampleState {
                    count: if shadow { 1 } else { crate::gpu::SAMPLE_COUNT },
                    ..Default::default()
                },
                multiview: None,
                cache: None,
            })
        };
        Self {
            pipeline: make(false),
            shadow: make(true),
            layout,
            models: Vec::new(),
            stats: Default::default(),
            texture_bytes: 0,
        }
    }
    /// Publish a complete high/medium/far chain after CPU import succeeds.
    pub fn replace_lods(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        models: &[Model; 3],
    ) {
        let mut texture_cache = textures::Cache::new();
        self.models = models
            .iter()
            .map(|m| ModelGpu {
                center: (m.min + m.max) * 0.5,
                radius: (m.max - m.min).length() * 0.5,
                materials: m
                    .materials
                    .iter()
                    .map(|v| textures::upload(device, queue, &self.layout, v, &mut texture_cache))
                    .collect(),
                primitives: m
                    .primitives
                    .iter()
                    .map(|p| Primitive {
                        vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("asset indexed vertices"),
                            contents: bytemuck::cast_slice(&p.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                        indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("asset indices"),
                            contents: bytemuck::cast_slice(&p.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        }),
                        count: p.indices.len() as u32,
                        material: p.material,
                    })
                    .collect(),
                streams: std::array::from_fn(|_| Stream {
                    buffer: device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("asset placement stream"),
                        size: size_of::<Instance>() as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }),
                    previous: Vec::new(),
                    next: Vec::new(),
                }),
            })
            .collect();
        self.texture_bytes = texture_cache
            .iter()
            .map(|(kind, image, _)| {
                let (mut width, mut height) = image.dimensions();
                let mut pixels = 0_u64;
                loop {
                    pixels += u64::from(width) * u64::from(height);
                    if width == 1 && height == 1 {
                        break;
                    }
                    width = (width / 2).max(1);
                    height = (height / 2).max(1);
                }
                pixels * if *kind == 3 { 1 } else { 4 }
            })
            .sum();
    }
    /// Independent camera/caster culling. Unchanged streams skip GPU uploads;
    /// mesh and material resources remain resident.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        placements: &[Instance],
        camera: &Camera,
        aspect: f32,
        height: u32,
    ) {
        self.stats = Default::default();
        let Some(prototype) = self.models.first() else {
            return;
        };
        let (local_center, local_radius) = (prototype.center, prototype.radius);
        for model in &mut self.models {
            for stream in &mut model.streams {
                stream.next.clear();
            }
        }
        let frusta = [
            Frustum::new(camera.view_projection(aspect)),
            Frustum::new(crate::shadows::Shadows::cascade_matrix(camera.focus, 0)),
            Frustum::new(crate::shadows::Shadows::cascade_matrix(camera.focus, 1)),
        ];
        for p in placements {
            let (s, c) = p.tint_rotation[3].sin_cos();
            let center = Vec3::new(
                c * local_center.x - s * local_center.z,
                local_center.y,
                s * local_center.x + c * local_center.z,
            ) * p.position_scale[3]
                + Vec3::from_slice(&p.position_scale[..3]);
            let radius = local_radius * p.position_scale[3].abs();
            let pixels = camera.projected_diameter(center, radius, height);
            let lod = if pixels > 120. {
                0
            } else if pixels > 35. {
                1
            } else {
                2
            };
            for (i, f) in frusta.iter().enumerate() {
                if f.sphere(center, radius) {
                    // Caster detail is independent of the viewing direction.
                    let draw_lod = if i == 0 { lod } else { i };
                    self.models[draw_lod].streams[i].next.push(*p);
                }
            }
        }
        for model in &mut self.models {
            for (i, stream) in model.streams.iter_mut().enumerate() {
                let bytes = bytemuck::cast_slice::<_, u8>(&stream.next);
                if bytes != bytemuck::cast_slice::<_, u8>(&stream.previous) {
                    if bytes.len() as u64 > stream.buffer.size() {
                        stream.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("asset placement growth"),
                            size: (bytes.len() as u64).next_power_of_two(),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        });
                    }
                    if !bytes.is_empty() {
                        queue.write_buffer(&stream.buffer, 0, bytes);
                        self.stats.upload_bytes += bytes.len();
                    }
                    std::mem::swap(&mut stream.previous, &mut stream.next);
                }
                if i == 0 && !stream.previous.is_empty() {
                    self.stats.instances += stream.previous.len() as u32;
                    self.stats.draws += model.primitives.len() as u32;
                    self.stats.triangles += model
                        .primitives
                        .iter()
                        .map(|p| u64::from(p.count) / 3)
                        .sum::<u64>()
                        * stream.previous.len() as u64;
                }
            }
        }
    }
    pub(crate) fn render(&self, pass: &mut wgpu::RenderPass<'_>, shadow: Option<usize>) {
        pass.set_pipeline(if shadow.is_some() {
            &self.shadow
        } else {
            &self.pipeline
        });
        for model in &self.models {
            let stream = &model.streams[shadow.map_or(0, |c| c + 1)];
            if stream.previous.is_empty() {
                continue;
            }
            pass.set_vertex_buffer(1, stream.buffer.slice(..));
            for primitive in &model.primitives {
                if shadow.is_none() {
                    pass.set_bind_group(3, &model.materials[primitive.material], &[]);
                }
                pass.set_vertex_buffer(0, primitive.vertices.slice(..));
                pass.set_index_buffer(primitive.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..primitive.count, 0, 0..stream.previous.len() as u32);
            }
        }
    }
}
