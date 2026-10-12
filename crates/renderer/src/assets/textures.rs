use super::Material;
use wgpu::util::DeviceExt;

pub(super) fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let mut entries = Vec::new();
    for binding in 0..5 {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
    }
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 5,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    });
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 6,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    });
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 7,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    });
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("imported material"),
        entries: &entries,
    })
}
fn decode(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn encode(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
pub(super) fn mips(image: &image::RgbaImage, kind: usize) -> Vec<image::RgbaImage> {
    let mut levels = vec![image.clone()];
    while let Some(src) = levels.last() {
        if src.width() == 1 && src.height() == 1 {
            break;
        }
        let (w, h) = ((src.width() / 2).max(1), (src.height() / 2).max(1));
        let dst = image::RgbaImage::from_fn(w, h, |x, y| {
            let mut values = [0.; 4];
            let mut count = 0.;
            // Area averaging also handles odd dimensions without losing the last edge.
            for sy in y * src.height() / h..(y + 1) * src.height() / h {
                for sx in x * src.width() / w..(x + 1) * src.width() / w {
                    let p = src.get_pixel(sx, sy);
                    for c in 0..4 {
                        let v = f32::from(p[c]) / 255.;
                        values[c] += if (kind == 0 || kind == 4) && c < 3 {
                            decode(v)
                        } else if kind == 2 && c == 1 {
                            v.powi(4)
                        } else {
                            v
                        };
                    }
                    count += 1.;
                }
            }
            for v in &mut values {
                *v /= count;
            }
            if kind == 1 {
                let n = glam::Vec3::new(
                    values[0] * 2. - 1.,
                    values[1] * 2. - 1.,
                    values[2] * 2. - 1.,
                )
                .normalize_or_zero();
                for c in 0..3 {
                    values[c] = n[c] * 0.5 + 0.5;
                }
            }
            image::Rgba(std::array::from_fn(|c| {
                let v = if (kind == 0 || kind == 4) && c < 3 {
                    encode(values[c])
                } else if kind == 2 && c == 1 {
                    values[c].sqrt().sqrt()
                } else {
                    values[c]
                };
                (v.clamp(0., 1.) * 255.).round() as u8
            }))
        });
        levels.push(dst);
    }
    levels
}
pub(super) type Cache = Vec<(usize, image::RgbaImage, wgpu::TextureView)>;

pub(super) fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    material: &Material,
    cache: &mut Cache,
) -> wgpu::BindGroup {
    let views: Vec<_> = material
        .images
        .iter()
        .enumerate()
        .map(|(kind, image)| {
            if let Some((_, _, view)) = cache
                .iter()
                .find(|(k, pixels, _)| *k == kind && pixels == image)
            {
                return view.clone();
            }
            let levels = mips(image, kind);
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("asset material mip chain"),
                size: wgpu::Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: levels.len() as u32,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: if kind == 3 {
                    wgpu::TextureFormat::R8Unorm
                } else if kind == 0 || kind == 4 {
                    wgpu::TextureFormat::Rgba8UnormSrgb
                } else {
                    wgpu::TextureFormat::Rgba8Unorm
                },
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            for (mip, data) in levels.iter().enumerate() {
                let red: Vec<_> = if kind == 3 {
                    data.pixels().map(|p| p[0]).collect()
                } else {
                    Vec::new()
                };
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: mip as u32,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    if kind == 3 { &red } else { data.as_raw() },
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(data.width() * if kind == 3 { 1 } else { 4 }),
                        rows_per_image: Some(data.height()),
                    },
                    wgpu::Extent3d {
                        width: data.width(),
                        height: data.height(),
                        depth_or_array_layers: 1,
                    },
                );
            }
            let view = texture.create_view(&Default::default());
            cache.push((kind, image.clone(), view.clone()));
            view
        })
        .collect();
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("asset trilinear anisotropy"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        anisotropy_clamp: 8,
        ..Default::default()
    });
    // Atlas charts have three extruded texels. Bound the filter footprint so
    // adjacent charts cannot contaminate each other through coarse mip levels.
    let occlusion_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("occlusion chart filtering"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_max_clamp: if material.occlusion_uv == 1 { 1. } else { 32. },
        ..Default::default()
    });
    let factors = [material.base, material.surface, material.emissive];
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("material factors"),
        contents: bytemuck::cast_slice(&factors),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let mut entries: Vec<_> = views
        .iter()
        .enumerate()
        .map(|(i, v)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: wgpu::BindingResource::TextureView(v),
        })
        .collect();
    entries.push(wgpu::BindGroupEntry {
        binding: 5,
        resource: wgpu::BindingResource::Sampler(&sampler),
    });
    entries.push(wgpu::BindGroupEntry {
        binding: 6,
        resource: uniform.as_entire_binding(),
    });
    entries.push(wgpu::BindGroupEntry {
        binding: 7,
        resource: wgpu::BindingResource::Sampler(&occlusion_sampler),
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("imported material"),
        layout,
        entries: &entries,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn texture_mips_preserve_linear_light_and_roughness() {
        let image = image::RgbaImage::from_fn(3, 1, |x, _| {
            image::Rgba(if x == 2 { [255; 4] } else { [0, 0, 0, 255] })
        });
        let color = mips(&image, 0);
        assert_eq!(color.last().unwrap().dimensions(), (1, 1));
        assert!((i32::from(color[1].get_pixel(0, 0)[0]) - 156).abs() <= 1);
        let data = mips(&image, 3);
        assert_eq!(data[1].get_pixel(0, 0)[0], 85);
        let rough = mips(&image, 2);
        assert!(rough[1].get_pixel(0, 0)[1] > 190);
    }
}
