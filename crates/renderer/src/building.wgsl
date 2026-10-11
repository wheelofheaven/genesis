// The fractional building tag distinguishes plaster, stone, timber and roof
// earth. All detail is world-scale, triplanar and filtered before minification.
// Directional face lighting is baked after building rotation, not applied twice.
fn building_material(base: vec3<f32>, p: vec3<f32>, normal: vec3<f32>, material: f32, pixel: f32) -> vec3<f32> {
    let power = normal * normal * normal * normal;
    let weights = power / max(dot(power, vec3<f32>(1.0)), 0.00001);
    let patch_weight = 1.0 - smoothstep(0.12, 0.8, pixel);
    let grain_weight = 1.0 - smoothstep(0.008, 0.06, pixel);
    let coarse = rock_noise(p * 1.8, weights);
    var response = 1.0;
    if material < 3.05 {
        // Worn plaster: quiet mottling and small sandy grains, not stone blocks.
        response = 0.96 + (coarse - 0.5) * 0.10 * patch_weight;
        if grain_weight > 0.0 {
            let grain = rock_noise(p * 65.0, weights);
            response *= 1.0 + (grain - 0.5) * 0.10 * grain_weight;
        }
    } else if material < 3.15 {
        response = 0.91 + coarse * 0.16 * patch_weight;
        if grain_weight > 0.0 {
            let grain = rock_noise(p * 42.0, weights);
            response *= 1.0 - smoothstep(0.62, 0.80, grain) * 0.08 * grain_weight;
        }
    } else if material < 3.25 {
        let grain = rock_noise(p * vec3<f32>(48.0, 1.3, 48.0), weights);
        let resolved = 1.0 - smoothstep(0.008, 0.075, pixel);
        response = 0.95 + (coarse - 0.5) * 0.16 * patch_weight;
        response *= 1.0 - smoothstep(0.58, 0.80, grain) * 0.30 * resolved;
    } else {
        response = 0.94 + (coarse - 0.5) * 0.15 * patch_weight;
        if grain_weight > 0.0 {
            response *= 1.0 + (rock_noise(p * 35.0, weights) - 0.5) * 0.25 * grain_weight;
        }
    }
    return base * response;
}
