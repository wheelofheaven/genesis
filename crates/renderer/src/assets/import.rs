use super::*;
use glam::{Mat4, Vec4};

pub(super) fn load(bytes: &[u8]) -> Result<Model> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("GLB exceeds 32 MiB import limit".into());
    }
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let blob = gltf
        .blob
        .as_deref()
        .ok_or("an embedded GLB buffer is required")?;
    let doc = &gltf.document;
    if doc.buffers().len() != 1
        || doc
            .buffers()
            .any(|b| !matches!(b.source(), gltf::buffer::Source::Bin) || b.length() > blob.len())
    {
        return Err("only one embedded, complete GLB buffer is supported".into());
    }
    if doc.extensions_used().next().is_some() || doc.animations().len() > 0 || doc.skins().len() > 0
    {
        return Err("extensions, animation and skinning are not supported by static import".into());
    }
    // Validate ranges before gltf's zero-copy readers touch buffer slices.
    let buffer_len = doc.buffers().next().ok_or("buffer required")?.length();
    for view in doc.views() {
        if view
            .offset()
            .checked_add(view.length())
            .is_none_or(|end| end > buffer_len)
        {
            return Err("buffer view exceeds declared buffer".into());
        }
    }
    for accessor in doc.accessors() {
        let view = accessor.view().ok_or("accessor needs a buffer view")?;
        if accessor.sparse().is_some() || accessor.count() == 0 || accessor.count() > 3_000_000 {
            return Err("sparse, empty or oversized accessor".into());
        }
        let stride = view.stride().unwrap_or(accessor.size());
        let end = (accessor.count() - 1)
            .checked_mul(stride)
            .and_then(|n| n.checked_add(accessor.size()))
            .and_then(|n| n.checked_add(accessor.offset()));
        if stride < accessor.size() || end.is_none_or(|end| end > view.length()) {
            return Err("accessor exceeds buffer view".into());
        }
    }
    if doc.materials().len() > 32 || doc.nodes().len() > 1024 {
        return Err("static import exceeds material/node budget".into());
    }
    let mut materials = Vec::new();
    let mut decoded_bytes = 0;
    for material in doc.materials() {
        let material = read_material(material, blob)?;
        decoded_bytes += material
            .images
            .iter()
            .map(|i| i.as_raw().len())
            .sum::<usize>();
        if decoded_bytes > 64 * 1024 * 1024 {
            return Err("decoded textures exceed 64 MiB budget".into());
        }
        materials.push(material);
    }
    if materials.is_empty() {
        return Err("explicit materials are required".into());
    }
    let scene = doc
        .default_scene()
        .or_else(|| {
            (doc.scenes().len() == 1)
                .then(|| doc.scenes().next())
                .flatten()
        })
        .ok_or("GLB needs an unambiguous default scene")?;
    let mut model = Model {
        primitives: Vec::new(),
        materials,
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };
    for node in scene.nodes() {
        visit(node, Mat4::IDENTITY, blob, &mut model, 0)?;
    }
    if model.primitives.is_empty() {
        return Err("scene contains no triangle meshes".into());
    }
    Ok(model)
}
fn read_material(material: gltf::Material<'_>, blob: &[u8]) -> Result<Material> {
    if material.alpha_mode() != gltf::material::AlphaMode::Opaque || material.double_sided() {
        return Err("static assets currently require opaque, single-sided materials".into());
    }
    let pbr = material.pbr_metallic_roughness();
    let normal = material.normal_texture();
    let occlusion = material.occlusion_texture();
    let maps = [
        pbr.base_color_texture()
            .map(|t| (t.texture(), t.tex_coord())),
        normal.as_ref().map(|t| (t.texture(), t.tex_coord())),
        pbr.metallic_roughness_texture()
            .map(|t| (t.texture(), t.tex_coord())),
        occlusion.as_ref().map(|t| (t.texture(), t.tex_coord())),
        material
            .emissive_texture()
            .map(|t| (t.texture(), t.tex_coord())),
    ];
    let fallback = [[255; 4], [128, 128, 255, 255], [255; 4], [255; 4], [255; 4]];
    let mut images = Vec::new();
    for (kind, (map, fallback)) in maps.into_iter().zip(fallback).enumerate() {
        images.push(if let Some((texture, uv)) = map {
            if uv != 0 && !(kind == 3 && uv == 1) {
                return Err("UV1 is supported only for occlusion; other maps require UV0".into());
            }
            let sampler = texture.sampler();
            if sampler.wrap_s() != gltf::texture::WrappingMode::Repeat
                || sampler.wrap_t() != gltf::texture::WrappingMode::Repeat
                || sampler
                    .mag_filter()
                    .is_some_and(|v| v != gltf::texture::MagFilter::Linear)
                || sampler
                    .min_filter()
                    .is_some_and(|v| v != gltf::texture::MinFilter::LinearMipmapLinear)
            {
                return Err("only repeat, linear/trilinear asset samplers are supported".into());
            }
            match texture.source().source() {
                gltf::image::Source::View {
                    view,
                    mime_type: "image/png",
                } => {
                    let data = blob
                        .get(
                            view.offset()
                                ..view
                                    .offset()
                                    .checked_add(view.length())
                                    .ok_or("image range overflow")?,
                        )
                        .ok_or("image exceeds GLB buffer")?;
                    let mut reader = image::ImageReader::with_format(
                        std::io::Cursor::new(data),
                        image::ImageFormat::Png,
                    );
                    let mut limits = image::Limits::default();
                    limits.max_image_width = Some(2048);
                    limits.max_image_height = Some(2048);
                    limits.max_alloc = Some(32 * 1024 * 1024);
                    reader.limits(limits);
                    reader.decode()?.into_rgba8()
                }
                _ => return Err("only embedded PNG textures are supported".into()),
            }
        } else {
            image::RgbaImage::from_pixel(1, 1, image::Rgba(fallback))
        });
    }
    let base = pbr.base_color_factor();
    let occlusion_uv = occlusion.as_ref().map_or(0, |t| t.tex_coord());
    let surface = [
        pbr.metallic_factor(),
        pbr.roughness_factor(),
        normal.map_or(1., |t| t.scale()),
        occlusion.map_or(1., |t| t.strength()),
    ];
    let emissive = Vec3::from_array(material.emissive_factor())
        .extend(0.)
        .to_array();
    if base
        .iter()
        .chain(surface.iter())
        .chain(emissive.iter())
        .any(|v| !v.is_finite())
    {
        return Err("non-finite material factor".into());
    }
    Ok(Material {
        base,
        surface,
        emissive,
        occlusion_uv,
        images: images.try_into().map_err(|_| "material image count")?,
    })
}
fn visit(
    node: gltf::Node<'_>,
    parent: Mat4,
    blob: &[u8],
    model: &mut Model,
    depth: usize,
) -> Result<()> {
    if depth > 64 {
        return Err("node hierarchy exceeds import depth limit".into());
    }
    let transform = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
    if !transform.is_finite() || transform.determinant() <= 1e-8 {
        return Err("singular or mirrored node transforms are unsupported".into());
    }
    let normal_transform = transform.inverse().transpose();
    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles
                || primitive.morph_targets().len() > 0
            {
                return Err("only static triangle primitives are supported".into());
            }
            for (semantic, accessor) in primitive.attributes() {
                if accessor.sparse().is_some()
                    || !matches!(
                        semantic,
                        gltf::Semantic::Positions
                            | gltf::Semantic::Normals
                            | gltf::Semantic::Tangents
                            | gltf::Semantic::TexCoords(0 | 1)
                    )
                {
                    return Err("unsupported vertex attribute or sparse accessor".into());
                }
            }
            let reader = primitive.reader(|_| Some(blob));
            let positions: Vec<_> = reader
                .read_positions()
                .ok_or("POSITION required")?
                .collect();
            let normals: Vec<_> = reader.read_normals().ok_or("NORMAL required")?.collect();
            let tangents: Vec<_> = reader.read_tangents().ok_or("TANGENT required")?.collect();
            let uv: Vec<_> = reader
                .read_tex_coords(0)
                .ok_or("TEXCOORD_0 required")?
                .into_f32()
                .collect();
            let material = primitive
                .material()
                .index()
                .ok_or("explicit primitive material required")?;
            let occlusion_uv: Vec<_> = if model.materials[material].occlusion_uv == 1 {
                reader
                    .read_tex_coords(1)
                    .ok_or("occlusion material requires TEXCOORD_1")?
                    .into_f32()
                    .collect()
            } else {
                uv.clone()
            };
            let indices: Vec<_> = reader
                .read_indices()
                .ok_or("indexed meshes required")?
                .into_u32()
                .collect();
            if positions.is_empty()
                || positions.len() > 1_000_000
                || normals.len() != positions.len()
                || tangents.len() != positions.len()
                || uv.len() != positions.len()
                || occlusion_uv.len() != positions.len()
                || indices.is_empty()
                || indices.len() % 3 != 0
                || indices.iter().any(|&i| i as usize >= positions.len())
            {
                return Err("invalid vertex/index counts or index range".into());
            }
            if model.primitives.len() >= 1024
                || positions.len()
                    + model
                        .primitives
                        .iter()
                        .map(|p| p.vertices.len())
                        .sum::<usize>()
                    > 1_000_000
            {
                return Err("expanded geometry exceeds import budget".into());
            }
            let mut vertices = Vec::with_capacity(positions.len());
            for i in 0..positions.len() {
                let position = transform.transform_point3(Vec3::from_array(positions[i])) / 20.;
                let normal = normal_transform
                    .transform_vector3(Vec3::from_array(normals[i]))
                    .normalize_or_zero();
                let t = transform.transform_vector3(Vec4::from_array(tangents[i]).truncate());
                let tangent = (t - normal * normal.dot(t))
                    .normalize_or_zero()
                    .extend(tangents[i][3]);
                if !position.is_finite()
                    || !normal.is_finite()
                    || !tangent.is_finite()
                    || normal.length_squared() < 0.9
                    || tangent.truncate().length_squared() < 0.9
                    || tangent.w.abs() != 1.
                    || uv[i]
                        .iter()
                        .chain(occlusion_uv[i].iter())
                        .any(|v| !v.is_finite())
                {
                    return Err("invalid imported vertex basis or coordinate".into());
                }
                model.min = model.min.min(position);
                model.max = model.max.max(position);
                vertices.push(Vertex {
                    position: position.to_array(),
                    normal: normal.to_array(),
                    tangent: tangent.to_array(),
                    uv: uv[i],
                    occlusion_uv: occlusion_uv[i],
                });
            }
            model.primitives.push(Primitive {
                vertices,
                indices,
                material: primitive
                    .material()
                    .index()
                    .ok_or("explicit primitive material required")?,
            });
        }
    }
    for child in node.children() {
        visit(child, transform, blob, model, depth + 1)?;
    }
    Ok(())
}
