//! Linear HDR world storage and a single display transform before glass and UI.
use crate::gpu::Gpu;
pub const WORLD_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct ToneMap {
    pub multisample: wgpu::TextureView,
    pub scene: wgpu::Texture,
    pub resolve: wgpu::TextureView,
    layout: wgpu::BindGroupLayout,
    group: wgpu::BindGroup,
    settings: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}
impl ToneMap {
    pub fn new(gpu: &Gpu, width: u32, height: u32, output: wgpu::TextureFormat) -> Self {
        let device = &gpu.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HDR display inputs"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });
        let settings = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("display exposure and survey bypass"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (multisample, scene, resolve) = Self::targets(gpu, width, height);
        let group = Self::group(device, &layout, &resolve, &settings);
        let shader = device.create_shader_module(wgpu::include_wgsl!("tonemap.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("HDR display"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("neutral tone mapping"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("fullscreen"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("display"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self {
            multisample,
            scene,
            resolve,
            layout,
            group,
            settings,
            pipeline,
        }
    }

    fn targets(
        gpu: &Gpu,
        width: u32,
        height: u32,
    ) -> (wgpu::TextureView, wgpu::Texture, wgpu::TextureView) {
        let multisample = gpu
            .multisample_texture(width, height, WORLD_FORMAT)
            .create_view(&Default::default());
        let scene = gpu.texture(
            width,
            height,
            WORLD_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
        );
        let resolve = scene.create_view(&Default::default());
        (multisample, scene, resolve)
    }

    fn group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        scene: &wgpu::TextureView,
        settings: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HDR display inputs"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scene),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: settings.as_entire_binding(),
                },
            ],
        })
    }

    /// Keep the shader/pipeline warm; recreate only size-dependent attachments.
    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        (self.multisample, self.scene, self.resolve) = Self::targets(gpu, width, height);
        self.group = Self::group(&gpu.device, &self.layout, &self.resolve, &self.settings);
    }

    pub fn composite(
        &self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        survey: bool,
    ) {
        // Fixed exposure avoids camera-dependent brightness pumping. Surveys are
        // data colours and bypass both exposure and the display curve.
        let settings = [1.0_f32, if survey { 1.0 } else { 0.0 }, 0.0, 0.0];
        gpu.queue
            .write_buffer(&self.settings, 0, bytemuck::cast_slice(&settings));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HDR to display"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..3, 0..1);
    }
}
