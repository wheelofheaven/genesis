//! GPU backdrop frosting. Buffers and reduced-resolution blur targets are reused.
use crate::gpu::Gpu;

/// Physical-pixel bounds, corner radii (NW, NE, SW, SE), and clipping rectangle.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Panel {
    pub bounds: [f32; 4],
    pub corners: [f32; 4],
    pub clip: [f32; 4],
}

pub struct Glass {
    pub scene: wgpu::TextureView,
    horizontal: wgpu::TextureView,
    vertical: wgpu::TextureView,
    groups: [wgpu::BindGroup; 3],
    pipelines: [wgpu::RenderPipeline; 4],
    panels: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl Glass {
    pub fn new(gpu: &Gpu, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let device = &gpu.device;
        let texture = |w, h| {
            gpu.texture(
                w,
                h,
                format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            )
            .create_view(&Default::default())
        };
        let scene = texture(width, height);
        let horizontal = texture(width.div_ceil(4), height.div_ceil(4));
        let vertical = texture(width.div_ceil(4), height.div_ceil(4));
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glass textures"),
            entries: &[
                texture_entry(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                texture_entry(2),
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let group = |source, blurred| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("glass source"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(blurred),
                    },
                ],
            })
        };
        let groups = [
            group(&scene, &scene),
            group(&horizontal, &horizontal),
            group(&scene, &vertical),
        ];
        let shader = device.create_shader_module(wgpu::include_wgsl!("glass.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glass"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let buffers = [wgpu::VertexBufferLayout {
            array_stride: size_of::<Panel>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &attributes,
        }];
        let pipelines = ["horizontal", "vertical", "copy", "glass"].map(|entry| {
            let panel = entry == "glass";
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(if panel { "panel" } else { "fullscreen" }),
                    compilation_options: Default::default(),
                    buffers: if panel { &buffers } else { &[] },
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: panel.then_some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        });
        let panels = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glass panels"),
            size: 64 * size_of::<Panel>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            scene,
            horizontal,
            vertical,
            groups,
            pipelines,
            panels,
            layout,
            sampler,
        }
    }

    /// Only size-dependent resources change; shaders and pipelines stay warm.
    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32, format: wgpu::TextureFormat) {
        let texture = |w, h| {
            gpu.texture(
                w,
                h,
                format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            )
            .create_view(&Default::default())
        };
        self.scene = texture(width, height);
        self.horizontal = texture(width.div_ceil(4), height.div_ceil(4));
        self.vertical = texture(width.div_ceil(4), height.div_ceil(4));
        self.groups = [
            (&self.scene, &self.scene),
            (&self.horizontal, &self.horizontal),
            (&self.scene, &self.vertical),
        ]
        .map(|(source, blurred)| {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("glass source"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(blurred),
                    },
                ],
            })
        });
    }

    pub fn composite(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        panels: &[Panel],
    ) {
        if std::mem::size_of_val(panels) as u64 > self.panels.size() {
            self.panels = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("glass panels"),
                size: (std::mem::size_of_val(panels) as u64).next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        gpu.queue
            .write_buffer(&self.panels, 0, bytemuck::cast_slice(panels));
        for (index, view) in [&self.horizontal, &self.vertical, target]
            .into_iter()
            .enumerate()
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("glass backdrop"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipelines[index]);
            pass.set_bind_group(0, &self.groups[index], &[]);
            pass.draw(0..3, 0..1);
            if index == 2 {
                pass.set_pipeline(&self.pipelines[3]);
                pass.set_vertex_buffer(0, self.panels.slice(..));
                pass.draw(0..6, 0..panels.len() as u32);
            }
        }
    }
}
