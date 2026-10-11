// World-metre triplanar detail avoids stretched grain on steep boulder faces.
// Scenery transforms are static; shared world coordinates also vary each instance.
fn stone_gradient(p: vec3<f32>, weights: vec3<f32>) -> vec3<f32> {
    let x = noise_slope(p.yz);
    let y = noise_slope(p.xz);
    let z = noise_slope(p.xy);
    return vec3<f32>(0.0, x.x, x.y) * weights.x
        + vec3<f32>(y.x, 0.0, y.y) * weights.y
        + vec3<f32>(z.x, z.y, 0.0) * weights.z;
}

fn stone_material(base: vec3<f32>, p: vec3<f32>, normal: vec3<f32>, height: f32, material: f32, pixel: f32) -> vec3<f32> {
    let power = normal * normal * normal * normal;
    let weights = power / max(dot(power, vec3<f32>(1.0)), 0.00001);
    let patch_weight = 1.0 - smoothstep(0.3, 1.5, pixel);
    let relief_weight = 1.0 - smoothstep(0.025, 0.16, pixel);
    let grain_weight = 1.0 - smoothstep(0.006, 0.055, pixel);
    var response = 0.94;
    if patch_weight > 0.0 {
        response += (rock_noise(p * 1.2, weights) - 0.5) * 0.20 * patch_weight;
    }
    var n = normal;
    if relief_weight > 0.0 {
        // Tangential height gradient: millimetre-scale weathering leaves the
        // silhouette in geometry, and fades before subpixel grain can sparkle.
        let gradient = stone_gradient(p * 6.0, weights) * 0.20 * relief_weight;
        n = normalize(normal - (gradient - normal * dot(gradient, normal)));
    }
    if grain_weight > 0.0 {
        let grain = rock_noise(p * 38.0, weights);
        response *= 1.0 + (grain - 0.5) * 0.16 * grain_weight;
        if material > 8.5 {
            // Sparse small cavities, without giving every basalt a vesicular mesh.
            response *= 1.0 - smoothstep(0.68, 0.85, grain) * 0.22 * grain_weight;
        }
    }
    if material < 8.5 && patch_weight > 0.0 {
        let warp = noise(p.xz * 0.75) * 0.45;
        let frequency = select(2.4, 5.5, material > 7.5);
        let band = sin((p.y + p.x * 0.06 + p.z * 0.025 + warp) * frequency);
        let resolved = 1.0 - smoothstep(0.08, 0.5, pixel);
        let strength = select(0.05, 0.12, material > 7.5);
        response *= 1.0 + (smoothstep(-0.65, 0.45, band) - 0.5) * strength * resolved;
    }
    let light = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let diffuse = 0.38 + 0.62 * max(dot(n, light), 0.0);
    let contact = 0.80 + 0.20 * smoothstep(-0.2, 0.65, height);
    return base * response * diffuse * contact;
}
