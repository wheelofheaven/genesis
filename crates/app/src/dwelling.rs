//! Application-owned asset identity and simulation-to-render placements.
use crate::terrain;
use genesis_renderer::{assets::Model, gpu::Gpu, instances::Instance};
use genesis_sim_core::SimState;
use std::{error::Error, path::Path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const EMBEDDED: [&[u8]; 3] = [
    include_bytes!("../../../assets/models/dwelling/lod0.glb"),
    include_bytes!("../../../assets/models/dwelling/lod1.glb"),
    include_bytes!("../../../assets/models/dwelling/lod2.glb"),
];

pub fn models(directory: Option<&Path>) -> Result<[Model; 3]> {
    let mut models = Vec::new();
    for (lod, embedded) in EMBEDDED.iter().enumerate() {
        let bytes = directory
            .map(|p| std::fs::read(p.join(format!("lod{lod}.glb"))))
            .transpose()?;
        let model = Model::from_glb(bytes.as_deref().unwrap_or(embedded))?;
        // The visual asset must remain inside its simulation/picking envelope.
        let expected_min = glam::Vec3::new(-3.25, 0., -2.4) / 20.;
        let expected_max = glam::Vec3::new(3.25, 3.3, 2.53) / 20.;
        if model
            .min
            .cmplt(expected_min - glam::Vec3::splat(0.002))
            .any()
            || model
                .max
                .cmpgt(expected_max + glam::Vec3::splat(0.002))
                .any()
        {
            return Err("dwelling asset exceeds its supported metre-scale envelope".into());
        }
        models.push(model);
    }
    if models
        .windows(2)
        .any(|m| m[1].triangles() >= m[0].triangles())
    {
        return Err("dwelling LOD triangle counts must decrease".into());
    }
    models
        .try_into()
        .map_err(|_| "three dwelling LODs required".into())
}

pub fn load(gpu: &mut Gpu, directory: Option<&Path>) -> Result<()> {
    let models = models(directory)?;
    gpu.assets.replace_lods(&gpu.device, &gpu.queue, &models);
    Ok(())
}

// Cosmetic variation follows persistent identity; camera movement and building
// insertion order never change a home's palette or require another material.
fn palette(id: u64) -> [f32; 3] {
    let h = id.wrapping_mul(0x9e3779b97f4a7c15);
    match (h ^ (h >> 32)) % 4 {
        0 => [1.06, 1.035, 0.98],
        1 => [0.96, 1., 1.02],
        2 => [1., 1., 1.],
        _ => [0.94, 0.965, 0.99],
    }
}

pub fn placements(sim: &SimState) -> Vec<Instance> {
    sim.buildings
        .iter()
        .map(|b| {
            let center = terrain::point(sim, b.position);
            let high = b
                .footprint()
                .map(|p| terrain::point(sim, p).y)
                .into_iter()
                .fold(f32::NEG_INFINITY, f32::max);
            Instance {
                position_scale: [center.x, high, center.z, 1.],
                tint_rotation: {
                    let [r, g, b_tint] = palette(b.id);
                    [r, g, b_tint, b.rotation]
                },
                coverage: [0., 1.],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dwelling_palettes_are_bounded_and_vary_by_identity() {
        let colors: Vec<_> = (1..32).map(palette).collect();
        assert!(colors.windows(2).any(|p| p[0] != p[1]));
        assert!(colors.iter().flatten().all(|v| (0.9..=1.1).contains(v)));
    }
    #[test]
    fn embedded_lods_preserve_scale_materials_and_winding() {
        let models = models(None).expect("authored fixture imports");
        let counts = models.each_ref().map(|m| m.triangles());
        assert!(counts[0] <= 9000 && counts[1] <= 500 && counts[2] <= 200);
        assert!(counts[0] > counts[1] && counts[1] > counts[2]);
        for m in models {
            assert!((6.5 / 20. ..=6.57 / 20.).contains(&(m.max.x - m.min.x)));
            assert!((m.max.y - 3.3 / 20.).abs() < 1e-5);
            assert_eq!(m.materials.len(), 4);
            for p in &m.primitives {
                for tri in p.indices.chunks_exact(3) {
                    let [a, b, c] = [0, 1, 2]
                        .map(|i| glam::Vec3::from_array(p.vertices[tri[i] as usize].position));
                    let n = glam::Vec3::from_array(p.vertices[tri[0] as usize].normal);
                    assert!((b - a).cross(c - a).dot(n) > 0.);
                }
            }
        }
    }
}

#[cfg(test)]
mod import_validation {
    use super::*;
    fn changed(edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
        let original = EMBEDDED[0];
        let length = u32::from_le_bytes(original[12..16].try_into().unwrap()) as usize;
        let mut doc: serde_json::Value =
            serde_json::from_slice(&original[20..20 + length]).unwrap();
        edit(&mut doc);
        let mut json = serde_json::to_vec(&doc).unwrap();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut out = Vec::new();
        out.extend_from_slice(&original[..8]);
        out.extend_from_slice(
            &((20 + json.len() + original.len() - 20 - length) as u32).to_le_bytes(),
        );
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&json);
        out.extend_from_slice(&original[20 + length..]);
        out
    }
    #[test]
    fn rejects_malformed_and_unsupported_assets_without_panics() {
        for bytes in [
            changed(|d| d["accessors"][0]["count"] = serde_json::json!(1000000)),
            changed(|d| d["bufferViews"][0]["byteLength"] = serde_json::json!(u32::MAX)),
            changed(|d| d["nodes"][0]["scale"] = serde_json::json!([-1, 1, 1])),
            changed(|d| d["materials"][0]["alphaMode"] = serde_json::json!("BLEND")),
            changed(|d| {
                d["meshes"][0]["primitives"][0]["attributes"]
                    .as_object_mut()
                    .unwrap()
                    .remove("TANGENT");
            }),
            EMBEDDED[0][..100].to_vec(),
        ] {
            assert!(Model::from_glb(&bytes).is_err());
        }
    }
    #[test]
    fn occlusion_uv_contract_rejects_missing_charts_and_preserves_uv0_fallback() {
        let missing = changed(|d| {
            d["meshes"][0]["primitives"][0]["attributes"]
                .as_object_mut()
                .unwrap()
                .remove("TEXCOORD_1");
        });
        assert!(Model::from_glb(&missing).is_err());
        let unsupported = changed(|d| {
            d["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["texCoord"] =
                serde_json::json!(1)
        });
        assert!(Model::from_glb(&unsupported).is_err());
        let fallback = changed(|d| {
            for m in d["materials"].as_array_mut().unwrap() {
                m["occlusionTexture"]["texCoord"] = serde_json::json!(0);
            }
        });
        let model = Model::from_glb(&fallback).unwrap();
        assert!(
            model
                .primitives
                .iter()
                .flat_map(|p| &p.vertices)
                .all(|v| v.uv == v.occlusion_uv)
        );
        let model = Model::from_glb(EMBEDDED[0]).unwrap();
        assert!(
            model
                .primitives
                .iter()
                .flat_map(|p| &p.vertices)
                .any(|v| v.uv != v.occlusion_uv)
        );
        assert!(model.materials.iter().all(|m| m.occlusion_uv == 1));
        assert!(model.materials[0].images[3].pixels().any(|p| p[0] < 200));
        assert!(model.materials[0].images[3].pixels().any(|p| p[0] == 255));
    }
    #[test]
    fn bakes_node_transforms_in_metres_and_preserves_tangent_basis() {
        let bytes = changed(|d| {
            d["nodes"][0]["translation"] = serde_json::json!([10, 2, 5]);
            d["nodes"][0]["scale"] = serde_json::json!([2, 3, 1]);
        });
        let model = Model::from_glb(&bytes).unwrap();
        let original = Model::from_glb(EMBEDDED[0]).unwrap();
        assert!((model.min.x - (10. / 20. + original.min.x * 2.)).abs() < 1e-5);
        assert!((model.max.y - (2. + 3.3 * 3.) / 20.).abs() < 1e-5);
        for primitive in model.primitives {
            for v in primitive.vertices {
                let n = glam::Vec3::from_array(v.normal);
                let t = glam::Vec3::from_slice(&v.tangent[..3]);
                assert!(n.dot(t).abs() < 1e-5);
                assert!((t.length() - 1.).abs() < 1e-5);
            }
        }
    }
}
