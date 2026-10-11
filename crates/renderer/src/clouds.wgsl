fn cloud_phase(cosine: f32, g: f32) -> f32 {
    return (1.0 - g * g) / pow(1.0 + g * g - 2.0 * g * cosine, 1.5);
}

// Front-to-back Beer extinction. Static spatial jitter breaks aligned sample
// bands; ray-step and angular footprints filter unresolved lobes. Reflections use fewer
// steps and a column-light approximation, keeping water cost bounded.
fn cloud_layer(direction: vec3<f32>, origin: vec3<f32>, angular_pixel: f32, steps: u32, jitter: f32) -> vec4<f32> {
    if camera.clouds.y <= 0.0 || camera.cloud_motion.w <= 0.0 || abs(direction.y) < 0.0001 { return vec4<f32>(0.0); }
    let a = (camera.clouds.x - origin.y) / direction.y;
    let b = (camera.clouds.x + camera.cloud_motion.z - origin.y) / direction.y;
    let start = max(min(a, b), 0.0);
    let end = min(max(a, b), 48000.0);
    if end <= start { return vec4<f32>(0.0); }
    let step_length = (end - start) / f32(steps);
    let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let alignment = dot(direction, sun);
    let phase = 0.65 + 0.12 * cloud_phase(alignment, 0.65) + 0.08 * cloud_phase(alignment, -0.2);
    var transmission = 1.0;
    var light = vec3<f32>(0.0);
    var weighted_distance = 0.0;
    for (var i = 0u; i < steps; i += 1u) {
        let distance = start + (f32(i) + jitter) * step_length;
        let p = origin + direction * distance;
        let footprint = max(angular_pixel * distance, step_length * 0.65);
        let density = cloud_volume(p, footprint);
        if density > 0.001 {
            let h = clamp((p.y - camera.clouds.x) / camera.cloud_motion.z, 0.0, 1.0);
            var optical_sun = density * camera.cloud_motion.z * (1.0 - h) * 0.5;
            if steps > 8u {
                optical_sun = cloud_volume(p + sun * 60.0, max(footprint, 50.0)) * 120.0
                    + cloud_volume(p + sun * 220.0, max(footprint, 120.0)) * 300.0
                    + cloud_volume(p + sun * 640.0, max(footprint, 200.0)) * 600.0;
            }
            let sun_transmission = exp(-optical_sun * camera.cloud_motion.w);
            let ambient = vec3<f32>(0.18, 0.23, 0.30) * (0.6 + 0.4 * h);
            let source = ambient + vec3<f32>(1.4, 1.32, 1.20) * phase * (0.25 + 0.75 * sun_transmission);
            let opacity = 1.0 - exp(-density * camera.cloud_motion.w * step_length);
            let weight = transmission * opacity;
            light += source * weight;
            weighted_distance += distance * weight;
            transmission *= 1.0 - opacity;
            if transmission < 0.01 { break; }
        }
    }
    let opacity = 1.0 - transmission;
    if opacity < 0.0001 { return vec4<f32>(0.0); }
    let distance = weighted_distance / opacity;
    let radiance = mix(sky_radiance(direction), light / opacity, exp(-distance * 0.000025));
    return vec4<f32>(radiance, opacity);
}
fn reflected_sky(direction: vec3<f32>, origin: vec3<f32>, angular_pixel: f32) -> vec3<f32> {
    let cloud = cloud_layer(direction, origin, max(angular_pixel, 0.025), 8u, 0.5);
    return mix(sky_radiance(direction), cloud.rgb, cloud.a);
}
