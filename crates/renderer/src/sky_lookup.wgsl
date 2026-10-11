// Ground/regional single scattering in a configurable spherical atmosphere.
// Kilometres keep sphere intersections well conditioned in f32. Scale heights,
// molecular colour and aerosol loading are separate from the cloud volume.
struct LookupVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@vertex fn vertex(@builtin(vertex_index) index: u32) -> LookupVertex {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    var out: LookupVertex;
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}
@fragment fn fragment(in: LookupVertex) -> @location(0) vec4<f32> {
    // Endpoint samples; the consumer maps endpoints to texel centres.
    let uv = clamp((in.uv * vec2<f32>(128.0, 64.0) - 0.5) / vec2<f32>(127.0, 63.0), vec2<f32>(0.0), vec2<f32>(1.0));
    let mu = uv.y * uv.y;
    let phi = uv.x * 3.14159265;
    let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let alignment = mu * sun.y + sqrt(max(1.0 - mu * mu, 0.0)) * length(sun.xz) * cos(phi);
    let phase_r = 0.75 * (1.0 + alignment * alignment);
    let g = 0.76;
    let phase_m = (1.0 - g * g) / pow(1.0 + g * g - 2.0 * g * alignment, 1.5);
    let beta_r = camera.rayleigh.rgb / camera.air_layers.z;
    let beta_m = camera.mie.rgb / camera.air_layers.w;
    let altitude = clamp((camera.eye_sea_level.y - camera.eye_sea_level.w) * 0.02, 0.0, camera.air_layers.y * 0.9);
    let radius = camera.air_layers.x + altitude;
    let top = camera.air_layers.x + camera.air_layers.y;
    let distance = sqrt(radius * radius * mu * mu + (top - radius) * (top + radius)) - radius * mu;
    var optical = vec3<f32>(0.0);
    var radiance = vec3<f32>(0.0);
    // Quadratic spacing resolves the dense air near the viewer and horizon.
    for (var i = 0u; i < 32u; i += 1u) {
        let a = f32(i) / 32.0;
        let b = f32(i + 1u) / 32.0;
        let step_length = (b * b - a * a) * distance;
        let t = (a * a + b * b) * 0.5 * distance;
        let radial_delta = t * t + 2.0 * radius * mu * t;
        // Rationalized difference avoids subtracting two large, close radii.
        let height = altitude + radial_delta / (sqrt(radius * radius + radial_delta) + radius);
        let densities = exp(-height / camera.air_layers.zw);
        let scattering_r = beta_r * densities.x;
        let scattering_m = beta_m * densities.y;
        let delta = (scattering_r + scattering_m) * step_length;
        // Fixed high daytime sun: local plane-parallel solar optical depth.
        // Moving low sun/ozone/multiple scattering need a fuller transport model.
        let solar_depth = (camera.rayleigh.rgb * densities.x + camera.mie.rgb * densities.y) / sun.y;
        let transmission = exp(-optical - delta * 0.5 - solar_depth);
        radiance += transmission * (scattering_r * phase_r + scattering_m * phase_m) * step_length;
        optical += delta;
    }
    let fill = vec3<f32>(0.025, 0.04, 0.065) * (1.0 - exp(-optical));
    return vec4<f32>(radiance * 2.4 + fill, 1.0);
}
