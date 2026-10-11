// Periodic cylindrical value noise and its analytic gradient. Wrapping lattice
// indices, rather than UVs, keeps both colour and relief continuous at the seam.
fn bark_noise(p: vec2<f32>, period: f32) -> vec3<f32> {
    let cell = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let du = 6.0 * f * (1.0 - f);
    let x = cell.x - floor(cell.x / period) * period;
    let next = x + 1.0 - floor((x + 1.0) / period) * period;
    let a = hash(vec2<f32>(x, cell.y));
    let b = hash(vec2<f32>(next, cell.y));
    let c = hash(vec2<f32>(x, cell.y + 1.0));
    let d = hash(vec2<f32>(next, cell.y + 1.0));
    let value = mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
    let gradient = du * vec2<f32>(mix(b - a, d - c, u.y), mix(c - a, d - b, u.x));
    return vec3<f32>(value, gradient);
}

// The nearest two jittered cells give a broken plate boundary and its gradient.
// Nine neighbours suffice for these bounded offsets; elongated mapping gives
// longitudinal plates without long, uniformly spaced stripes.
fn bark_plate(p: vec2<f32>, period: f32) -> vec4<f32> {
    let cell = floor(p);
    let f = fract(p);
    var first = 100.0;
    var second = 100.0;
    var nearest = vec2<f32>(0.0);
    var runner = vec2<f32>(0.0);
    var variation = 0.5;
    for (var y = -1; y <= 1; y += 1) {
        for (var x = -1; x <= 1; x += 1) {
            let offset = vec2<f32>(f32(x), f32(y));
            let id = cell + offset;
            let wrapped = vec2<f32>(id.x - floor(id.x / period) * period, id.y);
            let seed = vec2<f32>(hash(wrapped), hash(wrapped + vec2<f32>(37.0, 91.0)));
            let delta = f - offset - (0.2 + seed * 0.6);
            let distance = dot(delta, delta);
            if distance < first {
                second = first;
                runner = nearest;
                first = distance;
                nearest = delta;
                variation = seed.x;
            } else if distance < second {
                second = distance;
                runner = delta;
            }
        }
    }
    let separation = max(length(runner - nearest), 0.001);
    return vec4<f32>((second - first) / (2.0 * separation),
        (runner - nearest) / separation, variation);
}

fn bark_material(in: Output, world_dx: vec3<f32>, world_dy: vec3<f32>,
    uv_dx: vec2<f32>, uv_dy: vec2<f32>) -> vec3<f32> {
    let period = clamp(round((in.color.a - 6.0) * 256.0), 3.0, 128.0);
    let p = in.uv;
    let bend = (sin(p.y * 1.7) + 0.4 * sin(p.y * 4.1 + period)) * 0.32;
    let bend_slope = (1.7 * cos(p.y * 1.7) + 1.64 * cos(p.y * 4.1 + period)) * 0.32;
    let q = vec2<f32>(p.x * period + bend, p.y * 8.0);
    let q_dx = vec2<f32>(uv_dx.x * period + uv_dx.y * bend_slope, uv_dx.y * 8.0);
    let q_dy = vec2<f32>(uv_dy.x * period + uv_dy.y * bend_slope, uv_dy.y * 8.0);
    let footprint = max(length(q_dx), length(q_dy));
    let resolved = 1.0 - smoothstep(0.15, 0.7, footprint);
    var response = 0.9;
    let normal = normalize(in.normal);
    var n = normal;
    if resolved > 0.0 {
        // Perturb the plate edges without breaking cylindrical periodicity.
        // The chain rule keeps relief aligned with these irregular fissures.
        let warp = bark_noise(q * 2.0, period * 2.0);
        let displacement = vec2<f32>(0.65, 0.45);
        let field = bark_plate(q + displacement * (warp.x - 0.5), period);
        let warped_dx = q_dx + displacement * dot(warp.yz * 2.0, q_dx);
        let warped_dy = q_dy + displacement * dot(warp.yz * 2.0, q_dy);
        let start = 0.015 - footprint * 0.25;
        let end = 0.10 + footprint * 0.25;
        let plate = smoothstep(start, end, field.x);
        response = mix(response, 0.73 + plate * (0.20 + field.w * 0.12), resolved);
        // Millimetre relief gives bark plates subtle light-facing edges.
        // Convert the analytic UV gradient into a world-space surface gradient.
        let t = clamp((field.x - start) / (end - start), 0.0, 1.0);
        let relief = 0.0012 * min(period / 24.0, 1.0);
        let gradient = field.yz * (6.0 * t * (1.0 - t) / (end - start)) * (relief * resolved);
        let r1 = cross(world_dy * 20.0, normal);
        let r2 = cross(normal, world_dx * 20.0);
        let det = dot(world_dx * 20.0, r1);
        if abs(det) > 1e-12 {
            let surface = (r1 * dot(gradient, warped_dx) + r2 * dot(gradient, warped_dy)) / det;
            n = normalize(normal - surface / max(length(surface), 1.0));
        }
        let fine = 1.0 - smoothstep(0.10, 0.55, footprint * 4.0);
        if fine > 0.0 {
            let grain = bark_noise(vec2<f32>(q.x * 4.0, p.y * 9.0), period * 4.0).x;
            response *= 1.0 + (grain - 0.5) * 0.18 * fine;
        }
    }
    let light = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let diffuse = 0.40 + 0.60 * max(dot(n, light), 0.0);
    return in.color.rgb * response * diffuse;
}
