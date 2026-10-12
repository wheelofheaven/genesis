// Regional finite-segment single scattering. The three exponential layers are
// molecules, aerosols and local haze. Closed-form columns replace a view march;
// unlike blending toward the horizon, extinction and added light are separate.
fn aerial_density(h0: f32, h1: f32, scale: vec3<f32>) -> vec3<f32> {
    let low = min(h0, h1);
    let high = max(h0, h1);
    let span = (max(high, 0.0) - max(low, 0.0)) / scale;
    // Stable (1-exp(-x))/x near horizontal sight lines. Neither branch divides
    // by zero, even on backends that evaluate both sides of select.
    let mean = select((vec3<f32>(1.0) - exp(-span)) / max(span, vec3<f32>(0.00001)),
        vec3<f32>(1.0) - span * 0.5 + span * span / 6.0, span < vec3<f32>(0.01));
    let above = clamp(high / max(high - low, 0.00001), 0.0, 1.0);
    // Below sea level density stays bounded. Split crossing segments instead of
    // clamping endpoints before integration (important for depressed basins).
    return mix(vec3<f32>(1.0), exp(-max(low, 0.0) / scale) * mean, above);
}

fn aerial_phase(mu: f32, g: f32) -> f32 {
    let denominator = max(1.0 + g * g - 2.0 * g * mu, 0.001);
    return (1.0 - g * g) / (denominator * sqrt(denominator));
}

fn aerial_perspective(color: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    if camera.haze.z > 0.5 || camera.haze.x <= 0.0 { return color; }
    let delta = (world - camera.eye_sea_level.xyz) * 20.0;
    let distance = length(delta);
    // Preserve nearby silhouettes and material detail, continuously at 25 m.
    if distance <= 25.0 { return color; }
    let ray = delta / distance;
    let h0 = (camera.eye_sea_level.y - camera.eye_sea_level.w) * 20.0 + ray.y * 25.0;
    let h1 = (world.y - camera.eye_sea_level.w) * 20.0;
    let scale = vec3<f32>(camera.air_layers.zw * 1000.0, camera.haze.y);
    let density = aerial_density(h0, h1, scale);
    let column = density * (distance - 25.0);
    let molecular = camera.rayleigh.rgb * (column.x / scale.x);
    let aerosol = camera.mie.rgb * (column.y / scale.y);
    let haze = vec3<f32>(camera.haze.x * column.z);
    let optical = molecular + aerosol + haze;
    let transmission = exp(-optical);

    let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let mu = clamp(dot(ray, sun), -1.0, 1.0);
    // Representative solar attenuation through the same molecular/aerosol air
    // as the sky LUT. Constant illumination along the segment is an intentional
    // approximation: no terrain/cloud volumetric shadows or multiple scattering.
    let solar = 2.4 * exp(-(camera.rayleigh.rgb * density.x + camera.mie.rgb * density.y) / sun.y);
    let source_r = solar * (0.75 * (1.0 + mu * mu)) + vec3<f32>(0.025, 0.04, 0.065);
    let source_m = solar * aerial_phase(mu, 0.76);
    // A broad forward lobe and restrained diffuse sky fill keep valley haze
    // luminous toward the sun without whitening the whole view from above.
    let source_h = (solar * aerial_phase(mu, 0.65) + vec3<f32>(0.16, 0.20, 0.26)) * 0.9;
    let source = (molecular * source_r + aerosol * source_m + haze * source_h)
        / max(optical, vec3<f32>(0.000001));
    return color * transmission + source * (vec3<f32>(1.0) - transmission);
}
