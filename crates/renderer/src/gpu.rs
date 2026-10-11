use crate::geometry::Vertex;
use crate::{
    atmosphere::{Atmosphere, FrameUniform},
    camera::Camera,
};
use std::{error::Error, path::Path};
use wgpu::util::DeviceExt;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub const SAMPLE_COUNT: u32 = 4;

/// World rendering resolves before glass and UI composition.
pub struct WorldTarget<'a> {
    pub color: &'a wgpu::TextureView,
    pub resolve: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
}

pub struct Gpu {
    pub device: wgpu::Device,
    timing: Option<crate::timing::Timing>,
    pub queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    shadows: crate::shadows::Shadows,
    clouds: crate::clouds::Clouds,
    pub world: crate::world_mesh::WorldMesh,
    overlay: wgpu::Buffer,
    overlay_count: u32,
    camera: wgpu::Buffer,
    camera_group: wgpu::BindGroup,
    far_camera: wgpu::Buffer,
    far_camera_group: wgpu::BindGroup,
    pub instances: crate::instances::Instances,
}

impl Gpu {
    pub async fn new(adapter: &wgpu::Adapter) -> Result<Self> {
        let format = crate::tonemap::WORLD_FORMAT;
        eprintln!("adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                ..Default::default()
            })
            .await?;
        let overlay = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("interaction overlay"),
            contents: &[0; 4],
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera"),
            size: size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let far_camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("far shadow camera"),
            size: size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let far_camera_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("far shadow camera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: far_camera.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("world and atmosphere"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}\n{}\n{}\n{}",
                    include_str!("scene.wgsl"),
                    include_str!("atmosphere.wgsl"),
                    include_str!("terrain.wgsl"),
                    include_str!("water.wgsl"),
                    include_str!("clouds.wgsl"),
                    include_str!("building.wgsl")
                )
                .into(),
            ),
        });
        let shadows = crate::shadows::Shadows::new(&device);
        let clouds = crate::clouds::Clouds::new(&device, &queue);
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow projection"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout, &shadows.layout, &clouds.layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("genesis"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                alpha_to_coverage_enabled: true,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world sun shadows"),
            layout: Some(&shadow_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("shadow_vertex"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                }],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.,
                    clamp: 0.,
                },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("directional sky"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("sky_vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("sky_fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });
        let instances = crate::instances::Instances::new(
            &device,
            format,
            &pipeline_layout,
            &shadow_layout,
            &shader,
        );
        let timing = device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
            .then(|| crate::timing::Timing::new(&device));
        Ok(Self {
            timing,
            sky_pipeline,
            shadows,
            clouds,
            shadow_pipeline,
            instances,
            device,
            queue,
            pipeline,
            world: Default::default(),
            overlay,
            overlay_count: 0,
            camera,
            camera_group,
            far_camera,
            far_camera_group,
        })
    }

    pub fn upload_scene(&mut self, vertices: &[Vertex]) {
        self.world.upload(&self.device, &self.queue, vertices);
    }

    pub fn upload_overlay(&mut self, vertices: &[Vertex]) {
        let bytes = bytemuck::cast_slice(vertices);
        if bytes.len() as u64 > self.overlay.size() {
            self.overlay = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("interaction overlay"),
                size: (bytes.len() as u64).next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !bytes.is_empty() {
            self.queue.write_buffer(&self.overlay, 0, bytes);
        }
        self.overlay_count = vertices.len() as u32;
    }

    pub fn texture(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> wgpu::Texture {
        let view_formats = if format.is_srgb() {
            vec![format.remove_srgb_suffix()]
        } else {
            Vec::new()
        };
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &view_formats,
        })
    }

    pub fn multisample_texture(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("world multisample attachment"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    pub fn encode(
        &self,
        target: WorldTarget<'_>,
        camera: &Camera,
        aspect: f32,
        time: f32,
        atmosphere: Atmosphere,
        measure: bool,
    ) -> wgpu::CommandEncoder {
        let timing = self.timing.as_ref().filter(|_| measure);
        let uniform = FrameUniform::new(camera, aspect, time, atmosphere);
        self.queue
            .write_buffer(&self.camera, 0, bytemuck::bytes_of(&uniform));
        // Separate buffers keep both projections alive for the entire submission;
        // queue writes cannot switch a uniform between passes in one command buffer.
        let mut far_uniform = uniform;
        far_uniform.light_projection = uniform.far_light_projection;
        self.queue
            .write_buffer(&self.far_camera, 0, bytemuck::bytes_of(&far_uniform));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for cascade in 0..crate::shadows::CASCADES {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if cascade == 0 {
                    "near sun shadows"
                } else {
                    "settlement sun shadows"
                }),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadows.views[cascade],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: timing.map(|t| t.writes(cascade as u32)),
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(
                0,
                if cascade == 0 {
                    &self.camera_group
                } else {
                    &self.far_camera_group
                },
                &[],
            );
            self.world.render(
                &mut pass,
                &crate::visibility::Frustum::new(crate::shadows::Shadows::cascade_matrix(
                    camera.focus,
                    cascade,
                )),
                true,
            );
            self.instances.render_shadow(&mut pass, cascade);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.color,
                    resolve_target: Some(target.resolve),
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.72,
                            g: 0.80,
                            b: 0.83,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: target.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: timing.map(|t| t.writes(2)),
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_group, &[]);
            pass.set_bind_group(1, &self.shadows.group, &[]);
            pass.set_bind_group(2, &self.clouds.group, &[]);
            self.world.render(
                &mut pass,
                &crate::visibility::Frustum::new(camera.view_projection(aspect)),
                false,
            );
            pass.set_vertex_buffer(0, self.overlay.slice(..));
            pass.draw(0..self.overlay_count, 0..1);
            self.instances.render(&mut pass);
            // Reverse-depth zero lets early depth rejection shade only open sky.
            pass.set_pipeline(&self.sky_pipeline);
            pass.set_bind_group(0, &self.camera_group, &[]);
            pass.draw(0..3, 0..1);
        }
        if let Some(timing) = timing {
            timing.resolve(&mut encoder);
        }
        encoder
    }

    /// Call only after submitting a measured capture; never blocks ordinary frames.
    pub fn capture_timings(&self) -> Result<Option<[Option<f64>; 3]>> {
        self.timing.as_ref().map(|t| t.read(self)).transpose()
    }

    pub fn capture(&self, target: &wgpu::Texture, path: &Path) -> Result<()> {
        let width = target.width();
        let height = target.height();
        let row = (width * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: row as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            target.size(),
        );
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })?;
        rx.recv()??;
        let data = buffer.slice(..).get_mapped_range();
        let mut pixels: Vec<u8> = data
            .chunks(row as usize)
            .flat_map(|r| r[..width as usize * 4].iter().copied())
            .collect();
        if matches!(
            target.format(),
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8)?;
        drop(data);
        buffer.unmap();
        Ok(())
    }
}
