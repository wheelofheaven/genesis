//! A small, periodic coverage field shared by sky, reflections and sun attenuation.
//! Cosmetic atmosphere only; it does not generate weather in the simulation.
pub(crate) const SIZE: u32 = 512;
pub(crate) const TILE_METRES: f32 = 16_000.;
pub(crate) mod shadow;
mod shape;

pub(crate) struct Clouds {
    pub layout: wgpu::BindGroupLayout,
    pub group: wgpu::BindGroup,
}

impl Clouds {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, sky: &wgpu::TextureView) -> Self {
        let levels = mip_chain();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("periodic cloud density"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, data) in levels.iter().enumerate() {
            let size = SIZE >> level;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
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
        let shape_levels = shape::levels();
        let volume = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("periodic cloud lobes"),
            size: wgpu::Extent3d {
                width: shape::SIZE,
                height: shape::SIZE,
                depth_or_array_layers: shape::SIZE,
            },
            mip_level_count: shape_levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, data) in shape_levels.iter().enumerate() {
            let size = shape::SIZE >> level;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &volume,
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
                    depth_or_array_layers: size,
                },
            );
        }
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("filtered repeating clouds"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("clamped sky lookup"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cloud field"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloud field"),
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
                    resource: wgpu::BindingResource::TextureView(
                        &volume.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(sky),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sky_sampler),
                },
            ],
        });
        Self { layout, group }
    }
}

fn hash(x: i32, y: i32, period: i32) -> f32 {
    let mut h = (x.rem_euclid(period) as u32).wrapping_mul(1597334677)
        ^ (y.rem_euclid(period) as u32).wrapping_mul(3812015801)
        ^ 7319;
    h = (h ^ (h >> 16)).wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(3266489917);
    ((h ^ (h >> 16)) >> 8) as f32 / 16777216.
}

fn noise(x: f32, y: f32, period: i32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let u = fx * fx * (3. - 2. * fx);
    let v = fy * fy * (3. - 2. * fy);
    let a = hash(ix, iy, period) * (1. - u) + hash(ix + 1, iy, period) * u;
    let b = hash(ix, iy + 1, period) * (1. - u) + hash(ix + 1, iy + 1, period) * u;
    a * (1. - v) + b * v
}

fn density(u: f32, v: f32) -> f32 {
    let warp_x = (noise(u * 8., v * 8., 8) - 0.5) * 0.055;
    let warp_y = (noise(u * 8. + 3.1, v * 8. + 1.7, 8) - 0.5) * 0.055;
    let (u, v) = (u + warp_x, v + warp_y);
    [(8, 0.38), (16, 0.30), (32, 0.18), (64, 0.09), (128, 0.05)]
        .into_iter()
        .map(|(period, weight)| noise(u * period as f32, v * period as f32, period) * weight)
        .sum()
}

fn mip_chain() -> Vec<Vec<u8>> {
    let mut levels = vec![
        (0..SIZE * SIZE)
            .map(|i| {
                let u = (i % SIZE) as f32 + 0.5;
                let v = (i / SIZE) as f32 + 0.5;
                (density(u / SIZE as f32, v / SIZE as f32) * 255.).round() as u8
            })
            .collect::<Vec<_>>(),
    ];
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
    levels
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coverage_is_periodic_bounded_and_mips_preserve_average() {
        for i in -40..40 {
            let (u, v) = (i as f32 * 0.013, i as f32 * -0.047);
            let d = density(u, v);
            assert!((0.0..=1.0).contains(&d));
            assert!((d - density(u + 1., v - 1.)).abs() < 1e-5);
            assert!((density(-0.00001, v) - density(0.00001, v)).abs() < 0.001);
        }
        let levels = mip_chain();
        assert_eq!(levels, mip_chain());
        assert_eq!(levels.len(), 10);
        assert_eq!(levels.iter().map(Vec::len).sum::<usize>(), 349525);
        let base_mean = levels[0].iter().map(|x| *x as f32).sum::<f32>() / levels[0].len() as f32;
        for (level, data) in levels.iter().enumerate() {
            assert_eq!(data.len(), ((SIZE >> level) * (SIZE >> level)) as usize);
            let mean = data.iter().map(|x| *x as f32).sum::<f32>() / data.len() as f32;
            assert!((mean - base_mean).abs() < 2.);
        }
        assert!(levels[0].iter().any(|v| *v < 90));
        assert!(levels[0].iter().any(|v| *v > 175));
    }
}
