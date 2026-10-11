// A finite cloud slab, driven by a periodic regional coverage map and 3D lobes.
// Cosmetic presentation; no weather simulation or precipitation is implied.
@group(2) @binding(0) var cloud_density: texture_2d<f32>;
@group(2) @binding(1) var cloud_sampler: sampler;
@group(2) @binding(2) var cloud_shape: texture_3d<f32>;

fn cloud_uv(metres: vec2<f32>) -> vec2<f32> {
    return (metres - camera.cloud_motion.xy) / camera.clouds.w;
}
fn cloud_lod(footprint: f32) -> f32 {
    return clamp(log2(max(footprint * 512.0 / camera.clouds.w, 1.0)), 0.0, 9.0);
}
fn cloud_amount(uv: vec2<f32>, lod: f32) -> f32 {
    let density = textureSampleLevel(cloud_density, cloud_sampler, uv, lod).r;
    let threshold = mix(0.82, 0.22, camera.clouds.y);
    return smoothstep(threshold - 0.10, threshold + 0.16, density);
}
fn cloud_volume_field(p: vec3<f32>, footprint: f32, drift: vec2<f32>) -> f32 {
    let height = (p.y - camera.clouds.x) / camera.cloud_motion.z;
    if height <= 0.0 || height >= 1.0 { return 0.0; }
    let cover = cloud_amount((p.xz - drift) / camera.clouds.w, cloud_lod(footprint));
    if cover <= 0.001 { return 0.0; }
    let q = vec3<f32>(p.x - drift.x, p.y, p.z - drift.y) / 2000.0;
    let lod = clamp(log2(max(footprint * 64.0 / 2000.0, 1.0)), 0.0, 6.0);
    let lobe = textureSampleLevel(cloud_shape, cloud_sampler, q, lod).r;
    let fine_lod = min(lod + 1.585, 6.0);
    let detail = textureSampleLevel(cloud_shape, cloud_sampler, q * 3.0 + vec3<f32>(0.17, 0.43, 0.71), fine_lod).r;
    let base = (1.0 - lobe) * 0.12;
    let profile = smoothstep(base, base + 0.12, height) * (1.0 - smoothstep(0.35 + cover * 0.45, 1.0, height));
    let body = smoothstep(0.02, 0.35, cover - (1.0 - lobe) * 0.70);
    return max(body - (1.0 - detail) * 0.18, 0.0) * profile * (0.65 + 0.35 * lobe);
}

fn cloud_volume(p: vec3<f32>, footprint: f32) -> f32 {
    return cloud_volume_field(p, footprint, camera.cloud_motion.xy);
}
