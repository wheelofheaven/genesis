// Regional artistic approximation: shared sky, water reflection and aerial light.
// Linear-light values; no extra textures, ray marching or full-screen fog buffer.
fn sky_radiance(direction: vec3<f32>) -> vec3<f32> {
    let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let height = pow(clamp(direction.y, 0.0, 1.0), 0.45);
    let alignment = max(dot(direction, sun), 0.0);
    let halo = pow(alignment, 24.0) * 0.10;
    return mix(camera.horizon.rgb, camera.zenith.rgb, height)
        + vec3<f32>(1.0, 0.82, 0.58) * halo;
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
};
@vertex fn sky_vertex(@builtin(vertex_index) index: u32) -> SkyOutput {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    var out: SkyOutput;
    out.position = vec4<f32>(p, 0.0, 1.0);
    // z=0 is a direction at infinity in our reversed infinite projection.
    out.ray = (camera.inverse_projection * vec4<f32>(p, 0.0, 1.0)).xyz;
    return out;
}
@fragment fn sky_fragment(in: SkyOutput) -> @location(0) vec4<f32> {
    let ray = normalize(in.ray);
    let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let alignment = dot(ray, sun);
    let edge = max(fwidth(alignment), 0.000001);
    let disk = smoothstep(0.999989 - edge, 0.999989 + edge, alignment);
    let cloud = cloud_layer(ray, camera.eye_sea_level.xyz * 20.0, length(fwidth(ray)));
    let clear = sky_radiance(ray) + vec3<f32>(12.0, 11.04, 8.88) * disk;
    return vec4<f32>(mix(clear, cloud.rgb, cloud.a), 1.0);
}
