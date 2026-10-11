@group(3) @binding(0) var contact_field: texture_2d<f32>;
@group(3) @binding(1) var contact_sampler: sampler;
@group(3) @binding(2) var<uniform> contact_mapping: vec4<f32>;

fn ground_occlusion(world: vec3<f32>, pixel: f32) -> f32 {
    if contact_mapping.z <= 0.0 { return 0.0; }
    let uv = (world.xz * 20.0 - contact_mapping.xy) * contact_mapping.z;
    let edge = min(min(uv.x, uv.y), min(1.0 - uv.x, 1.0 - uv.y));
    if edge <= 0.0 { return 0.0; }
    let lod = clamp(log2(max(pixel / contact_mapping.w, 1.0)), 0.0, 10.0);
    let amount = textureSampleLevel(contact_field, contact_sampler, uv, lod).r;
    // Hide the finite field boundary and leave room for asynchronous cache moves.
    return amount * smoothstep(0.0, 24.0 * contact_mapping.z, edge);
}
