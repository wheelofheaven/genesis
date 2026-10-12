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
    let shape = textureSampleLevel(cloud_shape, cloud_sampler, q, lod);
    // Large lobes contain smaller billows. The packed channels preserve variation
    // without a second 3D lookup at every view/light/shadow sample.
    let billow = shape.r * 0.45 + shape.g * 0.35 + shape.b * 0.20;
    let base = 0.02 + shape.a * 0.035;
    let top = mix(0.48, 1.0, cover) * mix(0.82, 1.08, shape.a);
    let profile = smoothstep(base, base + 0.12, height)
        * (1.0 - smoothstep(top - 0.40, top, height));
    let body = clamp((cover * profile - (1.0 - billow) * 0.65 - 0.04) * 5.0, 0.0, 1.0);
    // Small-scale erosion acts on the boundary; dense interiors stay connected.
    let erosion = (1.0 - shape.b) * 0.18 * (1.0 - body);
    return max(body - erosion, 0.0) * (0.72 + shape.a * 0.28);
}

fn cloud_volume(p: vec3<f32>, footprint: f32) -> f32 {
    return cloud_volume_field(p, footprint, camera.cloud_motion.xy);
}
