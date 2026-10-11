// World-anchored material detail, in metres. Pixel footprints filter each scale
// independently. These are surface colours/normals, never displacement or deposits.
fn rock_noise(p: vec3<f32>, weights: vec3<f32>) -> f32 {
    return dot(vec3<f32>(noise(p.yz), noise(p.xz), noise(p.xy)), weights);
}

// Sparse mineral chips, 2–6 cm across. Every ellipse fits inside its jittered
// 18 cm cell, so no neighbour search or texture is needed. Return coverage/light.
fn soil_chips(p: vec2<f32>, pixel: f32) -> vec2<f32> {
    let q = mat2x2<f32>(vec2<f32>(0.8, 0.6), vec2<f32>(-0.6, 0.8)) * p / 0.18;
    let cell = floor(q);
    let id = hash(cell);
    let center = vec2<f32>(hash(cell + vec2<f32>(17.0, 31.0)),
                           hash(cell + vec2<f32>(53.0, 11.0))) * 0.5 + 0.25;
    let radius = mix(0.055, 0.17, hash(cell + vec2<f32>(89.0, 47.0)));
    let delta = (fract(q) - center) / radius;
    let axis = normalize(center - vec2<f32>(0.5) + vec2<f32>(0.0001));
    let shape = vec2<f32>(dot(delta, axis), dot(delta, vec2<f32>(-axis.y, axis.x)) * 1.55);
    // Unequal clipped corners interrupt the ellipse. Every outline remains
    // contained in its cell, including the antialiased margin.
    let facet = max(abs(shape.x) + abs(shape.y) * 0.5, abs(shape.y) + abs(shape.x) * 0.22);
    let edge = mix(length(shape), facet, id);
    let aa = clamp(pixel / (0.18 * radius), 0.06, 0.40);
    let mask = (1.0 - smoothstep(1.0 - aa, 1.0 + aa, edge)) * step(0.72, id);
    let light = clamp(0.94 + dot(delta, vec2<f32>(-0.20, 0.08)), 0.68, 1.20);
    return vec2<f32>(mask, light);
}

fn terrain_material(base: vec3<f32>, p: vec3<f32>, normal: vec3<f32>, exposed: f32, pixel: f32) -> vec3<f32> {
    let fine_weight = 1.0 - smoothstep(0.08, 0.5, pixel);
    let grain_weight = 1.0 - smoothstep(0.008, 0.045, pixel);
    let patch_weight = 1.0 - smoothstep(0.8, 4.0, pixel);
    let broad_weight = 1.0 - smoothstep(8.0, 40.0, pixel);
    var broad = 0.0;
    if broad_weight > 0.0 {
        broad = (noise(p.xz * 0.045) - 0.5) * broad_weight;
    }
    let rotation = mat2x2<f32>(vec2<f32>(0.8, 0.6), vec2<f32>(-0.6, 0.8));
    let uv = rotation * p.xz;
    var patches = 0.5;
    if patch_weight > 0.0 {
        // Several-metre ground patches with a slow domain warp. A weaker,
        // differently rotated octave interrupts the lattice without a dense
        // half-metre camouflage pattern over every open area.
        let warp = vec2<f32>(noise(uv * 0.045), noise(uv * 0.045 + vec2<f32>(7.3, 19.1))) - 0.5;
        let ground = uv + warp * 9.0;
        patches = 0.5 + (noise(ground * 0.22) - 0.5) * 0.78;
        let breakup_weight = 1.0 - smoothstep(0.2, 1.2, pixel);
        if breakup_weight > 0.0 {
            patches += (noise(rotation * ground * 0.63 + vec2<f32>(13.7, 4.1)) - 0.5) * 0.22 * breakup_weight;
        }
    }
    // The vertex palette already follows living cover, moisture and geology.
    // Expose irregular brown earth, with weaker tinting on neutral rock palettes.
    let green = smoothstep(0.04, 0.30, (base.g - base.b) / max(base.r, 0.02))
        * (1.0 - smoothstep(1.20, 1.65, base.r / max(base.g, 0.02)));
    // Fade toward the mean coverage, not bare soil, as patches become subpixel.
    let bare = mix(0.5, smoothstep(0.20, 0.80, patches), patch_weight) * green;
    let earth = base * vec3<f32>(1.05, 0.78, 0.72);
    var soil = mix(base, earth, bare * 0.48) * (0.96 + broad * 0.18);
    if fine_weight > 0.0 {
        soil *= 1.0 + (noise(uv * 5.3) - 0.5) * 0.18 * fine_weight;
    }
    if grain_weight > 0.0 {
        let grain = noise(uv * 38.0);
        soil *= 1.0 + (grain - 0.5) * 0.30 * grain_weight;
        // Millimetre relief changes only the local lighting ratio: broad terrain
        // illumination is already baked into base, so do not apply it twice.
        let slope = transpose(rotation) * noise_slope(uv * 19.0) * 0.16 * grain_weight;
        let bumped = normalize(normal - vec3<f32>(slope.x, 0.0, slope.y) * normal.y);
        let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
        let ratio = (0.48 + 0.52 * max(dot(bumped, sun), 0.0))
            / (0.48 + 0.52 * max(dot(normal, sun), 0.0));
        soil *= clamp(ratio, 0.7, 1.3);
        let chips = soil_chips(p.xz, pixel);
        let mineral = mix(base, vec3<f32>(dot(base, vec3<f32>(0.3333))), 0.65);
        soil = mix(soil, mineral * ((1.12 + grain * 0.16) * chips.y), chips.x * grain_weight * 0.48);
    }
    if exposed < 0.01 { return soil; }
    let power = normal * normal * normal * normal;
    let weights = power / max(power.x + power.y + power.z, 0.0001);
    let coarse_weight = 1.0 - smoothstep(0.5, 3.0, pixel);
    let coarse = mix(0.5, rock_noise(p * 0.35, weights), coarse_weight);
    let grain = (rock_noise(p * 5.0, weights) - 0.5) * fine_weight;
    let rock_warp = rock_noise(p * 0.11, weights);
    let layers = sin(p.y * 3.2 + rock_warp * 5.0);
    let layer_weight = 1.0 - smoothstep(0.15, 1.5, pixel);
    let seams = 1.0 - smoothstep(0.015, 0.055, abs(coarse - 0.48));
    var rock = base * (0.88 + coarse * 0.22 + grain * 0.20);
    rock *= 1.0 - seams * 0.18 * fine_weight;
    rock *= 1.0 + layers * 0.065 * layer_weight;
    return mix(soil, rock, smoothstep(0.0, 1.0, exposed));
}
