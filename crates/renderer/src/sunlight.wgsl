@group(1) @binding(0) var sun_depth: texture_depth_2d_array;
@group(1) @binding(1) var sun_sampler: sampler_comparison;
fn receiver_slope(dx: vec4<f32>, dy: vec4<f32>) -> vec2<f32> {
    let determinant = dx.x * dy.y - dx.y * dy.x;
    if abs(determinant) < 1e-16 { return vec2<f32>(0.0); }
    return clamp(vec2<f32>(dx.z * dy.y - dy.z * dx.y,
        dx.x * dy.z - dy.x * dx.z) / determinant, vec2<f32>(-2.0), vec2<f32>(2.0));
}
fn shadow_sample(p: vec4<f32>, layer: i32, slope: vec2<f32>) -> f32 {
    let uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    var shadow = 0.0;
    // Both maps are 2048 square. Explicit levels permit foliage discard.
    // Project each PCF tap onto the receiver plane. The remaining half-texel
    // allowance covers hardware bilinear comparison without broad wall stripes.
    let bias = select(0.00002, 0.00001, layer == 1) + dot(abs(slope), vec2<f32>(1.0 / 2048.0));
    for (var y = -1; y <= 1; y += 1) {
        for (var x = -1; x <= 1; x += 1) {
            shadow += textureSampleCompareLevel(sun_depth, sun_sampler,
                uv + vec2<f32>(f32(x), f32(y)) / 2048.0, layer,
                p.z + dot(slope, vec2<f32>(f32(x), -f32(y)) * (2.0 / 2048.0)) - bias);
        }
    }
    return 0.58 + 0.42 * shadow / 9.0;
}
fn shadow_inside(p: vec4<f32>) -> bool {
    return max(abs(p.x), abs(p.y)) < 0.98 && p.z > 0.0 && p.z < 1.0;
}
fn sunlight(world: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>) -> f32 {
    let p = camera.light_projection * vec4<f32>(world, 1.0);
    let slope = receiver_slope(camera.light_projection * vec4<f32>(dx, 0.0), camera.light_projection * vec4<f32>(dy, 0.0));
    let edge = max(abs(p.x), abs(p.y));
    // Most close pixels sample only the detailed map; only the overlap needs both.
    if shadow_inside(p) && edge <= 0.72 { return shadow_sample(p, 0, slope); }
    let far = camera.far_light_projection * vec4<f32>(world, 1.0);
    var wide = 1.0;
    if shadow_inside(far) {
        let fade = smoothstep(0.82, 0.98, max(abs(far.x), abs(far.y)));
        let far_slope = receiver_slope(camera.far_light_projection * vec4<f32>(dx, 0.0), camera.far_light_projection * vec4<f32>(dy, 0.0));
        wide = mix(shadow_sample(far, 1, far_slope), 1.0, fade);
    }
    if shadow_inside(p) {
        return mix(shadow_sample(p, 0, slope), wide, smoothstep(0.72, 0.94, edge));
    }
    return wide;
}
