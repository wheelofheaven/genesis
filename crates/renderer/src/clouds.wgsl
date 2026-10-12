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
    let phase = 0.75 * cloud_phase(alignment, 0.65) + 0.25 * cloud_phase(alignment, -0.2);
    let broad_phase = cloud_phase(alignment, 0.3);
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
                // Integrate the light path all the way to cloud top. Growing
                // segments resolve nearby occlusion without fixed probe gaps.
                let exit = max(camera.clouds.x + camera.cloud_motion.z - p.y, 0.0) / sun.y;
                optical_sun = 0.0;
                var previous = 0.0;
                for (var light_step = 1u; light_step <= 4u; light_step += 1u) {
                    let t = f32(light_step) * 0.25;
                    let end = exit * t * t;
                    let segment = end - previous;
                    optical_sun += cloud_volume(p + sun * (previous + segment * 0.5),
                        max(footprint, segment * 0.25)) * segment;
                    previous = end;
                }
            }
            let optical_depth = optical_sun * camera.cloud_motion.w;
            // Two broad, attenuated scattering orders replace a constant grey
            // interior fill. This is an approximation, not a multiple-scattering solver.
            let scattered = phase * exp(-optical_depth)
                + 0.35 * broad_phase * exp(-optical_depth * 0.45)
                + 0.12 * exp(-optical_depth * 0.18);
            let ambient = vec3<f32>(0.14, 0.18, 0.24) * (0.5 + 0.5 * h);
            let source = ambient + vec3<f32>(1.8, 1.70, 1.55) * scattered;
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
