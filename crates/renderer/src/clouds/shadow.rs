//! A periodic sun-transmission map in the cloud's moving coordinate system.
//! Wind only changes sampling coordinates. Regenerate when the volume changes.
use crate::atmosphere::FrameUniform;

pub(crate) struct CloudShadow {
    pub view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    empty_group: wgpu::BindGroup,
    key: Option<[f32; 4]>,
}

impl CloudShadow {
    pub fn new(
        device: &wgpu::Device,
        camera: &wgpu::BindGroupLayout,
        clouds: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cloud volume sun transmission"),
            size: wgpu::Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        // Do not bind the world shadow group: it contains this render target.
        let empty = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("unused cloud projection slot"),
            entries: &[],
        });
        let empty_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("unused cloud projection slot"),
            layout: &empty,
            entries: &[],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cloud shadow integral"),
            bind_group_layouts: &[camera, &empty, clouds],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cloud shadow integral"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}",
                    include_str!("../frame.wgsl"),
                    include_str!("../cloud_field.wgsl"),
                    include_str!("../cloud_shadow_integral.wgsl")
                )
                .into(),
            ),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cloud shadow integral"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R16Float,
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
            view: texture.create_view(&Default::default()),
            pipeline,
            empty_group,
            key: None,
        }
    }

    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        camera: &wgpu::BindGroup,
        clouds: &wgpu::BindGroup,
        frame: &FrameUniform,
    ) {
        let key = [
            frame.clouds[0],
            frame.clouds[1],
            frame.cloud_motion[2],
            frame.cloud_motion[3],
        ];
        if self.key == Some(key) {
            return;
        }
        self.key = Some(key);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cloud volume shadow cache"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(1, &self.empty_group, &[]);
        pass.set_bind_group(2, clouds, &[]);
        pass.draw(0..3, 0..1);
    }
}
