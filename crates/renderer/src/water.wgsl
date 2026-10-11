// Metric water shading. River coordinates follow the channel instead of rotating
// a world-space phase at each bend. This is surface animation, not a flow solver.
fn water_wave(uv: vec2<f32>, footprint: vec2<f32>, frequency: vec2<f32>, amplitude: f32, phase: f32) -> vec2<f32> {
    let radians_per_pixel = dot(abs(frequency), footprint) + fwidth(phase);
    let resolved = 1.0 - smoothstep(0.8, 2.8, radians_per_pixel);
    return frequency * (amplitude * cos(dot(uv, frequency) - phase) * resolved);
}

fn water_current(uv: vec2<f32>, footprint: vec2<f32>, time: f32) -> vec2<f32> {
    let drift = uv - vec2<f32>(time * 0.6, 0.0);
    let scale = vec2<f32>(1.4, 2.3);
    let first = noise_slope(drift * scale) * scale * 0.014;
    let first_weight = 1.0 - smoothstep(0.3, 1.2, max(footprint.x * scale.x, footprint.y * scale.y));
    // Rotated second layer hides the lattice of either individual noise field.
    let rotation = mat2x2<f32>(vec2<f32>(0.8, 0.6), vec2<f32>(-0.6, 0.8));
    let q = rotation * drift * 3.7 + vec2<f32>(11.3, 7.1);
    let second = transpose(rotation) * noise_slope(q) * (3.7 * 0.005);
    let second_weight = 1.0 - smoothstep(0.3, 1.2, (footprint.x + footprint.y) * 3.7);
    return first * first_weight + second * second_weight;
}

// Convert derivatives in channel coordinates to a slope in world XZ. At sharp
// bends the mapping compresses, so bound its gain; degenerate triangles fall flat.
fn water_gradient(dx: vec2<f32>, dy: vec2<f32>, field: vec2<f32>) -> vec2<f32> {
    let det = dx.x * dy.y - dx.y * dy.x;
    if abs(det) < 0.00000001 { return vec2<f32>(0.0); }
    let gradient = vec2<f32>(field.x * dy.y - field.y * dx.y,
                            dx.x * field.y - dy.x * field.x) / det;
    return gradient / max(1.0, length(gradient) / 2.0);
}

fn water_material(in: Output, pixel: f32) -> vec3<f32> {
    let world = in.world * 20.0;
    let time = camera.direction_time.w;
    let river = in.color.r < 0.0;
    let depth = max(-in.color.a - 1.0, 0.0);
    let uv = select(world.xz, in.color.gb, river);
    let footprint = fwidth(uv);
    let dx = dpdx(world);
    let dy = dpdy(world);
    let du = vec2<f32>(dpdx(uv.x), dpdy(uv.x));
    let dv = vec2<f32>(dpdx(uv.y), dpdy(uv.y));
    var plane = cross(dx, dy);
    plane *= select(-1.0, 1.0, plane.y >= 0.0);
    plane = normalize(plane + vec3<f32>(0.0, 0.0000001, 0.0));
    var slope = vec2<f32>(0.0);
    if river {
        slope = water_current(uv, footprint, time) * smoothstep(0.0, 0.18, depth);
    } else {
        slope = water_wave(uv, footprint, vec2<f32>(0.12, 0.05), 0.10, time * 1.1)
              + water_wave(uv, footprint, vec2<f32>(0.62, -0.35), 0.035, time * 1.9 + 2.1)
              + water_wave(uv, footprint, vec2<f32>(2.1, 0.7), 0.006, time * 2.8 + 4.3);
    }
    let slope_world = water_gradient(dx.xz, dy.xz, du) * slope.x
                    + water_gradient(dx.xz, dy.xz, dv) * slope.y;
    let normal = normalize(plane - vec3<f32>(slope_world.x, 0.0, slope_world.y));
    let view = normalize(camera.eye_sea_level.xyz - in.world);
    let facing = max(dot(normal, view), 0.0);
    // Limit optical path length at grazing angles. At zero depth, let the wet
    // bed meet the bank without a bright Fresnel outline.
    let optical_depth = depth / max(0.35, abs(dot(plane, view)));
    let contact = smoothstep(0.0, 0.10, depth);
    let fresnel = (0.02 + 0.65 * pow(1.0 - facing, 5.0)) * contact;
    var color = mix(vec3<f32>(0.021, 0.075, 0.105), vec3<f32>(0.17, 0.29, 0.24), exp(-optical_depth * 0.16));
    if river {
        let material = clamp(-in.color.r - 1.0, 0.0, 2.0);
        let limestone = vec3<f32>(0.30, 0.285, 0.225);
        let sandstone = vec3<f32>(0.32, 0.235, 0.15);
        let basalt = vec3<f32>(0.16, 0.175, 0.17);
        var bed = mix(mix(limestone, sandstone, min(material, 1.0)), basalt, max(material - 1.0, 0.0));
        let gravel = (noise(world.xz * 6.0) - 0.5) * (1.0 - smoothstep(0.04, 0.35, pixel));
        bed *= 0.88 + noise(world.xz * 0.7) * 0.16 + gravel * 0.3;
        color = mix(vec3<f32>(0.055, 0.14, 0.12), bed, exp(-optical_depth * 1.8));
    }
    let reflected = reflect(-view, normal);
    let reflection_pixel = length(fwidth(reflected));
    let cloud_light = cloud_sunlight(world, pixel);
    color *= mix(0.65, 1.0, cloud_light);
    color = mix(color, reflected_sky(reflected, world, reflection_pixel), fresnel);
    let half_light = normalize(view + normalize(vec3<f32>(-0.5, 0.75, 0.3)));
    // Broad, modest highlights survive minification without pinprick shimmer.
    let glint = pow(max(dot(normal, half_light), 0.0), 100.0) * contact;
    color += vec3<f32>(0.65, 0.62, 0.48) * glint * 1.8 * cloud_light;
    if !river {
        let wash = 0.18 + 0.12 * sin(time * 1.4 + noise(world.xz * 0.07) * 5.0);
        let breakup = noise(world.xz * 0.8 + vec2<f32>(time * 0.06, -time * 0.04));
        let detail = 1.0 - smoothstep(0.5, 2.5, pixel);
        let patches = mix(0.22, smoothstep(0.46, 0.72, breakup), detail);
        let foam = (1.0 - smoothstep(wash, wash + 0.38, depth)) * patches * contact;
        color = mix(color, vec3<f32>(0.66, 0.71, 0.65), foam * 0.55);
    }
    return color * sunlight(in.world, dx / 20.0, dy / 20.0);
}
