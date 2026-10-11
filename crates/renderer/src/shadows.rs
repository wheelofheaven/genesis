//! Two nested, stable sun-shadow maps: detail around the observer and settlement context.
pub const CASCADES: usize = 2;
pub const RESOLUTION: u32 = 2048;
// World units are 20 metres. Fixed extents prevent resolution pumping on zoom.
pub const RADII: [f32; CASCADES] = [8., 32.];
const DEPTHS: [f32; CASCADES] = [40., 128.];
pub struct Shadows {
    pub views: [wgpu::TextureView; CASCADES],
    pub layout: wgpu::BindGroupLayout,
    pub group: wgpu::BindGroup,
}
impl Shadows {
    pub fn new(device: &wgpu::Device, cloud_shadow: &wgpu::TextureView) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nested sunlight depth"),
            size: wgpu::Extent3d {
                width: RESOLUTION,
                height: RESOLUTION,
                depth_or_array_layers: CASCADES as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let views = std::array::from_fn(|i| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: i as u32,
                array_layer_count: Some(1),
                ..Default::default()
            })
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sun comparison"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sun shadow sampling"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sun shadow sampling"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(cloud_shadow),
                },
            ],
        });
        Self {
            views,
            layout,
            group,
        }
    }
    pub fn matrix(center: glam::Vec3) -> glam::Mat4 {
        Self::cascade_matrix(center, 0)
    }
    pub fn cascade_matrix(center: glam::Vec3, cascade: usize) -> glam::Mat4 {
        let radius = RADII[cascade];
        let depth = DEPTHS[cascade];
        let texel = radius * 2. / RESOLUTION as f32;
        let sun = glam::Vec3::new(-0.5, 0.75, 0.3).normalize();
        // Snap in the light's image plane; world-axis snapping still makes
        // projected shadow edges crawl as the camera pans diagonally.
        let basis = glam::Mat4::look_at_rh(sun * 40., glam::Vec3::ZERO, glam::Vec3::Y);
        let mut light_center = basis.transform_point3(center);
        light_center.x = (light_center.x / texel).round() * texel;
        light_center.y = (light_center.y / texel).round() * texel;
        let center = basis.inverse().transform_point3(light_center);
        glam::Mat4::orthographic_rh(-radius, radius, -radius, radius, 0.1, depth * 2.)
            * glam::Mat4::look_at_rh(center + sun * depth, center, glam::Vec3::Y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shadow_projection_stays_still_inside_a_light_texel() {
        let sun = glam::Vec3::new(-0.5, 0.75, 0.3).normalize();
        let right = glam::Vec3::Y.cross(sun).normalize();
        let point = glam::Vec3::new(1., 2., 3.);
        for (cascade, radius) in RADII.into_iter().enumerate() {
            let texel = radius * 2. / RESOLUTION as f32;
            let a = Shadows::cascade_matrix(glam::Vec3::ZERO, cascade).project_point3(point);
            let b = Shadows::cascade_matrix(right * texel * 0.2, cascade).project_point3(point);
            assert!((a.truncate() - b.truncate()).length() < 1e-6);
            let c = Shadows::cascade_matrix(right * texel, cascade).project_point3(point);
            assert!(((a.x - c.x).abs() - 2. / RESOLUTION as f32).abs() < 1e-6);
        }
    }
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    #[test]
    fn wide_map_contains_near_receivers_and_surrounding_settlement() {
        use glam::{Vec3, Vec4};
        for focus in [Vec3::ZERO, Vec3::new(-140., 21., 90.)] {
            let near = Shadows::matrix(focus);
            let far = Shadows::cascade_matrix(focus, 1);
            for x in [-0.95, 0., 0.95] {
                for y in [-0.95, 0., 0.95] {
                    for z in [0.01, 0.5, 0.99] {
                        let point = near.inverse() * Vec4::new(x, y, z, 1.);
                        let q = far * point;
                        assert!(q.x.abs() < 0.3 && q.y.abs() < 0.3);
                        assert!(q.z > 0. && q.z < 1.);
                    }
                }
            }
            let sun = Vec3::new(-0.5, 0.75, 0.3).normalize();
            let receiver = focus + Vec3::Y.cross(sun).normalize() * 20.;
            assert!(near.project_point3(receiver).x.abs() > 1.);
            let q = far.project_point3(receiver);
            assert!(q.x.abs() < 0.8 && q.y.abs() < 0.8 && q.z > 0. && q.z < 1.);
        }
    }
}
