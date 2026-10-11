//! A bounded, world-anchored ground occlusion field. Presentation proxies only.
use glam::Vec2;

pub const SIZE: u32 = 1024;
pub const TEXEL_METRES: f32 = 0.25;
pub const EXTENT_METRES: f32 = SIZE as f32 * TEXEL_METRES;

#[derive(Clone, Copy)]
pub struct Footprint {
    pub center: Vec2,
    pub radius: Vec2,
    pub rotation: f32,
    pub strength: f32,
}

pub struct Field {
    pub origin: Vec2,
    levels: Vec<Vec<u8>>,
}

impl Field {
    pub fn bake(center: Vec2, footprints: impl IntoIterator<Item = Footprint>) -> Self {
        // Integer texel shifts preserve every shared sample when the camera moves.
        let origin =
            (center / TEXEL_METRES).floor() * TEXEL_METRES - Vec2::splat(EXTENT_METRES * 0.5);
        let mut values = vec![0; (SIZE * SIZE) as usize];
        for footprint in footprints {
            stamp(&mut values, origin, footprint);
        }
        let mut levels = vec![values];
        let mut size = SIZE as usize;
        while size > 1 {
            let previous = &levels[levels.len() - 1];
            let next = (0..size * size / 4)
                .map(|i| {
                    let x = (i % (size / 2)) * 2;
                    let y = (i / (size / 2)) * 2;
                    let sum = previous[y * size + x] as u32
                        + previous[y * size + x + 1] as u32
                        + previous[(y + 1) * size + x] as u32
                        + previous[(y + 1) * size + x + 1] as u32;
                    ((sum + 2) / 4) as u8
                })
                .collect();
            levels.push(next);
            size /= 2;
        }
        Self { origin, levels }
    }

    /// Useful for CPU inspection; GPU sampling uses filtered mip levels.
    pub fn at(&self, world_metres: Vec2) -> u8 {
        let p = (world_metres - self.origin) / TEXEL_METRES;
        if p.min_element() < 0. || p.max_element() >= SIZE as f32 {
            return 0;
        }
        self.levels[0][p.y as usize * SIZE as usize + p.x as usize]
    }
}

fn stamp(values: &mut [u8], origin: Vec2, footprint: Footprint) {
    if !footprint.center.is_finite()
        || !footprint.radius.is_finite()
        || !footprint.rotation.is_finite()
        || !footprint.strength.is_finite()
        || footprint.radius.min_element() <= 0.
    {
        return;
    }
    let radius = footprint.radius.min(Vec2::splat(16.));
    let reach = Vec2::splat(radius.max_element());
    let lo = ((footprint.center - reach - origin) / TEXEL_METRES)
        .floor()
        .clamp(Vec2::ZERO, Vec2::splat(SIZE as f32))
        .as_uvec2();
    let hi = ((footprint.center + reach - origin) / TEXEL_METRES)
        .ceil()
        .clamp(Vec2::ZERO, Vec2::splat(SIZE as f32))
        .as_uvec2();
    let (sin, cos) = footprint.rotation.sin_cos();
    for y in lo.y..hi.y {
        for x in lo.x..hi.x {
            let p =
                origin + (Vec2::new(x as f32, y as f32) + 0.5) * TEXEL_METRES - footprint.center;
            let local = Vec2::new(p.x * cos + p.y * sin, p.y * cos - p.x * sin) / radius;
            let falloff = (1. - local.length_squared()).max(0.);
            let amount =
                (footprint.strength.clamp(0., 0.6) * falloff * falloff * 255.).round() as u8;
            let pixel = &mut values[(y * SIZE + x) as usize];
            // Max is order-independent and prevents dense stands turning black.
            *pixel = (*pixel).max(amount);
        }
    }
}

pub struct Contacts {
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) group: wgpu::BindGroup,
    texture: wgpu::Texture,
    mapping: wgpu::Buffer,
}

impl Contacts {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scenery ground occlusion"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: SIZE.ilog2() + 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mapping = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("contact field origin and scale"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("filtered ground occlusion"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ground occlusion"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ground occlusion"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mapping.as_entire_binding(),
                },
            ],
        });
        Self {
            layout,
            group,
            texture,
            mapping,
        }
    }

    pub fn upload(&self, queue: &wgpu::Queue, field: &Field) {
        let mapping = [
            field.origin.x,
            field.origin.y,
            1. / EXTENT_METRES,
            TEXEL_METRES,
        ];
        queue.write_buffer(&self.mapping, 0, bytemuck::cast_slice(&mapping));
        for (level, data) in field.levels.iter().enumerate() {
            let size = SIZE >> level;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size),
                    rows_per_image: Some(size),
                },
                wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contact_field_is_bounded_order_independent_and_stable_when_shifted() {
        let a = Footprint {
            center: Vec2::new(1.125, -2.875),
            radius: Vec2::new(4., 1.5),
            rotation: 0.6,
            strength: 0.8,
        };
        let b = Footprint {
            center: Vec2::new(-1., 0.),
            strength: 0.35,
            ..a
        };
        let field = Field::bake(Vec2::ZERO, [a, b]);
        assert_eq!(field.levels, Field::bake(Vec2::ZERO, [b, a, a]).levels);
        assert_eq!(field.at(a.center), 153);
        assert_eq!(field.at(Vec2::splat(100.)), 0);
        assert_eq!(field.at(Vec2::splat(1000.)), 0);
        assert!(field.levels.iter().flatten().all(|&value| value <= 153));
        assert_eq!(field.levels.iter().map(Vec::len).sum::<usize>(), 1_398_101);
        let shifted = Field::bake(Vec2::new(20., -20.), [a, b]);
        for y in -60..60 {
            for x in -60..60 {
                let p = (Vec2::new(x as f32, y as f32) + 0.5) * TEXEL_METRES;
                assert_eq!(field.at(p), shifted.at(p));
            }
        }
        let base_mean =
            field.levels[0].iter().map(|&x| x as f64).sum::<f64>() / field.levels[0].len() as f64;
        for (level, data) in field.levels.iter().enumerate() {
            assert_eq!(data.len(), ((SIZE >> level) * (SIZE >> level)) as usize);
            let mean = data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64;
            assert!((mean - base_mean).abs() < 2.);
        }
    }

    #[test]
    fn contact_ellipses_rotate_and_invalid_or_absent_footprints_stay_clear() {
        let a = Footprint {
            center: Vec2::splat(0.125),
            radius: Vec2::new(4., 0.5),
            rotation: 0.,
            strength: 0.5,
        };
        let horizontal = Field::bake(Vec2::ZERO, [a]);
        let vertical = Field::bake(
            Vec2::ZERO,
            [Footprint {
                rotation: std::f32::consts::FRAC_PI_2,
                ..a
            }],
        );
        assert!(horizontal.at(a.center + Vec2::X * 2.) > 0);
        assert_eq!(horizontal.at(a.center + Vec2::Y * 2.), 0);
        assert_eq!(vertical.at(a.center + Vec2::X * 2.), 0);
        assert!(vertical.at(a.center + Vec2::Y * 2.) > 0);
        let clear = Field::bake(
            Vec2::ZERO,
            [
                Footprint {
                    radius: Vec2::ZERO,
                    ..a
                },
                Footprint {
                    strength: f32::NAN,
                    ..a
                },
                Footprint {
                    center: Vec2::splat(1000.),
                    ..a
                },
            ],
        );
        assert!(clear.levels.iter().flatten().all(|&v| v == 0));
        assert_eq!(clear.levels, Field::bake(Vec2::ZERO, []).levels);
    }
}
