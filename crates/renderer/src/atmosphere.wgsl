// Shared directional scattering lookup for sky, water and regional aerial light.
// Cloud integration is evaluated in the reduced-resolution sky environment pass.
@group(2) @binding(3) var sky_scattering: texture_2d<f32>;
@group(2) @binding(4) var sky_lookup_sampler: sampler;
fn sky_radiance(direction: vec3<f32>) -> vec3<f32> {
    let sun_xz = normalize(vec2<f32>(-0.5, 0.3));
    let horizontal = direction.xz / max(length(direction.xz), 0.00001);
    let phi = acos(clamp(dot(horizontal, sun_xz), -1.0, 1.0)) / 3.14159265;
    let height = sqrt(clamp(direction.y, 0.0, 1.0));
    let uv = (vec2<f32>(phi, height) * vec2<f32>(127.0, 63.0) + 0.5) / vec2<f32>(128.0, 64.0);
    return textureSampleLevel(sky_scattering, sky_lookup_sampler, uv, 0.0).rgb;
}

fn aerial_perspective(color: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    if camera.haze.x <= 0.0 { return color; }
    let delta = world - camera.eye_sea_level.xyz;
    let distance = length(delta) * 20.0;
    let h0 = max((camera.eye_sea_level.y - camera.eye_sea_level.w) * 20.0, 0.0) / camera.haze.y;
    let h1 = max((world.y - camera.eye_sea_level.w) * 20.0, 0.0) / camera.haze.y;
    let dh = h1 - h0;
    var density = exp(-h0);
    if abs(dh) > 0.001 {
        density = (exp(-h0) - exp(-h1)) / dh;
    }
    let transmission = exp(-max(distance - 25.0, 0.0) * camera.haze.x * density);
    return mix(sky_radiance(normalize(delta + vec3<f32>(0.000001))), color, transmission);
}

struct SkyOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ray: vec3<f32>,
    @location(1) uv: vec2<f32>,
};
@vertex fn sky_vertex(@builtin(vertex_index) index: u32) -> SkyOutput {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    var out: SkyOutput;
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    // z=0 is a direction at infinity in our reversed infinite projection.
    out.ray = (camera.inverse_projection * vec4<f32>(p, 0.0, 1.0)).xyz;
    return out;
}
// RGB stores sky + cloud radiance; alpha stores cloud transmission so the
// composite can draw the small solar disk at full display resolution.
@fragment fn sky_environment(in: SkyOutput) -> @location(0) vec4<f32> {
    let ray = normalize(in.ray);
    let jitter = 0.1 + 0.8 * hash(floor(in.position.xy));
    let cloud = cloud_layer(ray, camera.eye_sea_level.xyz * 20.0, length(fwidth(ray)), 64u, jitter);
    return vec4<f32>(mix(sky_radiance(ray), cloud.rgb, cloud.a), 1.0 - cloud.a);
}
