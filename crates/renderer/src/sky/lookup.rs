//! Small directional clear-sky radiance table, refreshed only when its inputs change.
//! Cloud blending and water reflections sample this table; finite-segment aerial
//! perspective uses the same optical parameters in a separate analytic integral.
mod key;
use key::Key;
pub(crate) struct Lookup {
    key: Option<Key>,
    pub view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
}
impl Lookup {
    pub fn new(device: &wgpu::Device, camera: &wgpu::BindGroupLayout) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("directional sky scattering lookup"),
            size: wgpu::Extent3d {
                width: 128,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: crate::tonemap::WORLD_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sky scattering lookup"),
            bind_group_layouts: &[camera],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sky scattering integral"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("../frame.wgsl"),
                    include_str!("../sky_lookup.wgsl")
                )
                .into(),
            ),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky scattering integral"),
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
                    format: crate::tonemap::WORLD_FORMAT,
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
            key: None,
            view: texture.create_view(&Default::default()),
            pipeline,
        }
    }
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        camera: &wgpu::BindGroup,
        frame: &crate::atmosphere::FrameUniform,
    ) -> bool {
        let key = Key::new(frame);
        if self.key == Some(key) {
            return false;
        }
        self.key = Some(key);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear sky scattering table"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.draw(0..3, 0..1);
        true
    }
}
