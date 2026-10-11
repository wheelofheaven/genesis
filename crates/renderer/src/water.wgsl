// Metric water shading with shared river slopes and world-anchored ripples.
// Advection is a visual current approximation, not a flow solver.
fn water_wave(uv: vec2<f32>, footprint: vec2<f32>, frequency: vec2<f32>, amplitude: f32, phase: f32, distortion: vec3<f32>) -> vec2<f32> {
    // The gradient includes phase distortion, keeping the bent swell's lighting
    // and pixel filter consistent with its height field.
    let gradient = frequency + distortion.yz;
    let radians_per_pixel = dot(abs(gradient), footprint) + fwidth(phase);
    let resolved = 1.0 - smoothstep(0.8, 2.8, radians_per_pixel);
    return gradient * (amplitude * cos(dot(uv, frequency) + distortion.x - phase) * resolved);
}

// World-anchored phases do not restart at ribbon boundaries. Two overlapping
// advection cycles prevent unbounded stretching of the varying direction field.
fn water_current(uv: vec2<f32>, footprint: vec2<f32>, time: f32, velocity: vec2<f32>) -> vec2<f32> {
    let phase = fract(time / 6.0);
    let next = fract(phase + 0.5);
    let rotation = mat2x2<f32>(vec2<f32>(0.8, 0.6), vec2<f32>(-0.6, 0.8));
    let scale = vec2<f32>(1.4, 2.3);
    // Each field has zero weight as its phase wraps.
    let blend = 0.5 + 0.5 * cos(phase * 6.2831853);
    let resolved = 1.0 - smoothstep(0.3, 1.2, (footprint.x + footprint.y) * 2.3);
    var broad = vec2<f32>(0.0);
    if resolved > 0.0 {
        let a = rotation * (uv - velocity * phase * 6.0);
        let b = rotation * (uv - velocity * next * 6.0);
        let first = noise_slope(a * scale) * scale;
        let second = noise_slope(b * scale) * scale;
        broad = transpose(rotation) * mix(first, second, blend) * 0.022 * resolved;
    }
    // A separately rotated, shorter scale interrupts the value-noise lattice.
    // Filter it independently so close ripples do not become distant shimmer.
    let fine_rotation = mat2x2<f32>(vec2<f32>(0.28, -0.96), vec2<f32>(0.96, 0.28));
    let fine_scale = vec2<f32>(7.0, 9.0);
    let fine_resolved = 1.0 - smoothstep(0.3, 1.2, (footprint.x + footprint.y) * 9.0);
    if fine_resolved > 0.0 {
        let fine_a = noise_slope(fine_rotation * (uv - velocity * phase * 6.0) * fine_scale);
        let fine_b = noise_slope(fine_rotation * (uv - velocity * next * 6.0) * fine_scale);
        broad += transpose(fine_rotation) * (mix(fine_a, fine_b, blend) * fine_scale) * 0.003 * fine_resolved;
    }
    return broad;
}

fn water_material(in: Output, pixel: f32) -> vec3<f32> {
    let world = in.world * 20.0;
    let time = camera.direction_time.w;
    let river = in.color.r < 0.0;
    let depth = max(-in.color.a - 1.0, 0.0);
    let uv = world.xz;
    let footprint = fwidth(uv);
    let dx = dpdx(world);
    let dy = dpdy(world);
    var plane = cross(dx, dy);
    plane *= select(-1.0, 1.0, plane.y >= 0.0);
    plane = normalize(plane + vec3<f32>(0.0, 0.0000001, 0.0));
    if river {
        plane = normalize(vec3<f32>(-in.color.g, 1.0, -in.color.b));
    }
    var slope = vec2<f32>(0.0);
    if river {
        // A bounded continuous downhill drift blends to light wind on flat pools.
        // Avoid normalizing a nearly zero slope, which flips direction abruptly.
        let drift = -in.color.gb / max(0.025, length(in.color.gb)) * 0.6;
        let velocity = drift + vec2<f32>(0.08, 0.04);
        slope = water_current(uv, footprint, time, velocity) * smoothstep(0.0, 0.18, depth);
    } else {
        // Slowly varying phase bends ocean crests instead of drawing perfectly
        // straight, parallel stripes across the entire regional view.
        let bend = vec3<f32>(noise(uv * 0.012) * 5.0, noise_slope(uv * 0.012) * 0.06);
        slope = water_wave(uv, footprint, vec2<f32>(0.12, 0.05), 0.10, time * 1.1, bend)
              + water_wave(uv, footprint, vec2<f32>(0.62, -0.35), 0.035, time * 1.9 + 2.1, bend)
              + water_wave(uv, footprint, vec2<f32>(2.1, 0.7), 0.006, time * 2.8 + 4.3, bend);
        slope += water_current(uv, footprint, time, vec2<f32>(0.22, 0.09)) * 0.7;
    }
    let normal = normalize(plane - vec3<f32>(slope.x, 0.0, slope.y));
    let view = normalize(camera.eye_sea_level.xyz - in.world);
    let facing = max(dot(normal, view), 0.0);
    // Snell's law bounds the submerged ray even at grazing view angles. Depth
    // is vertical separation from the terrain; plane.y converts it to normal
    // separation for the local parallel-bed approximation.
    let ray = refract(-view, normal, 1.0 / 1.333);
    let optical_depth = depth * plane.y / max(-dot(ray, plane), 0.25);
    let bed_uv = uv + ray.xz * min(optical_depth, 12.0);
    let bed_pixel = max(length(fwidth(bed_uv)), pixel);
    let contact = smoothstep(0.0, 0.10, depth);
    let fresnel = water_fresnel(facing) * contact;
    // Artist-selected extinction per metre, not measured local turbidity.
    // Red attenuates first, preserving warm gravel in the very shallow margins.
    let extinction = select(vec3<f32>(0.32, 0.12, 0.075), vec3<f32>(0.72, 0.38, 0.28), river);
    let transmitted = exp(-optical_depth * extinction);
    let scattering = select(vec3<f32>(0.021, 0.075, 0.105), vec3<f32>(0.055, 0.115, 0.09), river);
    var bed = vec3<f32>(0.30, 0.285, 0.225);
    if max(transmitted.r, max(transmitted.g, transmitted.b)) > 0.005 {
        let material = select(0.0, clamp(-in.color.r - 1.0, 0.0, 2.0), river);
        bed = water_bed(bed_uv, bed_pixel, material);
    }
    // Recover direct visibility from the shared shadow helper's ambient floor.
    // Occlusion suppresses sunlight; the reflected sky remains independently lit.
    let shadow = clamp((sunlight(in.world, dx / 20.0, dy / 20.0) - 0.58) / 0.42, 0.0, 1.0);
    let direct = shadow * cloud_sunlight(world, pixel);
    var color = mix(scattering, bed, transmitted) * (0.55 + 0.45 * direct);
    let normal_dx = dpdx(normal);
    let normal_dy = dpdy(normal);
    let variance = 0.25 * (dot(normal_dx, normal_dx) + dot(normal_dy, normal_dy));
    let unresolved = smoothstep(0.03, 0.30, pixel) * 0.001;
    let alpha_squared = 0.0025 + min(variance, 0.025) + unresolved;
    let reflected = reflect(-view, normal);
    let reflection_pixel = max(length(fwidth(reflected)), sqrt(alpha_squared) * 0.5);
    color = mix(color, reflected_sky(reflected, world, reflection_pixel), fresnel);
    color += vec3<f32>(2.0, 1.86, 1.58) * water_sun(normal, view, alpha_squared) * contact * direct;
    if !river {
        let wash = 0.18 + 0.12 * sin(time * 1.4 + noise(world.xz * 0.07) * 5.0);
        let breakup = noise(world.xz * 0.8 + vec2<f32>(time * 0.06, -time * 0.04));
        let detail = 1.0 - smoothstep(0.5, 2.5, pixel);
        let patches = mix(0.22, smoothstep(0.46, 0.72, breakup), detail);
        let foam = (1.0 - smoothstep(wash, wash + 0.38, depth)) * patches * contact;
        color = mix(color, vec3<f32>(0.66, 0.71, 0.65) * (0.55 + 0.45 * direct), foam * 0.55);
    }
    return color;
}
