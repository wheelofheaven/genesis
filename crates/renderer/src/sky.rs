//! Reduced-resolution sky radiance, composited behind full-resolution terrain.
//! No temporal history: camera movement, resize and weather edits cannot ghost.
mod irradiance;
mod lookup;
pub(crate) use lookup::Lookup;
pub(crate) struct Sky {
    pub generation: wgpu::RenderPipeline,
    pub view: wgpu::TextureView,
    composite: wgpu::RenderPipeline,
    group: wgpu::BindGroup,
    layout: wgpu::BindGroupLayout,
    size: [u32; 2],
}

fn resolution(size: [u32; 2]) -> [u32; 2] {
    [
        size[0].div_ceil(2).clamp(1, 1024),
        size[1].div_ceil(2).clamp(1, 768),
    ]
}
fn target(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    size: [u32; 2],
) -> (wgpu::TextureView, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("half resolution sky radiance and transmission"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: crate::tonemap::WORLD_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sky composition"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    (view, group)
}
impl Sky {
    pub fn new(
        device: &wgpu::Device,
        camera: &wgpu::BindGroupLayout,
        world: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky composition"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sky composite"),
            bind_group_layouts: &[camera, &layout],
            push_constant_ranges: &[],
        });
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sky composite"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("frame.wgsl"),
                    include_str!("sky_composite.wgsl")
                )
                .into(),
            ),
        });
        let pipeline = |name, layout, shader, vertex, fragment, composite| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(name),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: crate::tonemap::WORLD_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: if composite {
                    Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::GreaterEqual,
                        stencil: Default::default(),
                        bias: Default::default(),
                    })
                } else {
                    None
                },
                multisample: wgpu::MultisampleState {
                    count: if composite {
                        crate::gpu::SAMPLE_COUNT
                    } else {
                        1
                    },
                    ..Default::default()
                },
                multiview: None,
                cache: None,
            })
        };
        let generation = pipeline(
            "cloud volume and clear sky",
            world,
            shader,
            "sky_vertex",
            "sky_environment",
            false,
        );
        let composite = pipeline(
            "full resolution sky composition",
            &composite_layout,
            &composite_shader,
            "vertex",
            "fragment",
            true,
        );
        let size = [1, 1];
        let (view, group) = target(device, &layout, size);
        Self {
            generation,
            view,
            composite,
            group,
            layout,
            size,
        }
    }
    pub fn resize(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        let size = resolution(size);
        if size != self.size {
            (self.view, self.group) = target(device, &self.layout, size);
            self.size = size;
        }
    }
    pub fn composite<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, camera: &'a wgpu::BindGroup) {
        pass.set_pipeline(&self.composite);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(1, &self.group, &[]);
        pass.draw(0..3, 0..1);
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn resolution_handles_resize_retina_and_minimized_surfaces_with_a_fixed_budget() {
        for (input, expected) in [
            ([0, 0], [1, 1]),
            ([1280, 800], [640, 400]),
            ([1997, 1249], [999, 625]),
            ([7680, 4320], [1024, 768]),
        ] {
            assert_eq!(super::resolution(input), expected);
        }
    }
}
