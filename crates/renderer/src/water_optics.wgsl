// Single-pass optical approximation: refract into a locally parallel procedural
// bed. This does not sample scene colour/depth or reflect nearby objects.
fn water_fresnel(cosine: f32) -> f32 {
    // Air/water interface, IOR 1.333, Schlick approximation.
    return 0.0204 + 0.9796 * pow(1.0 - clamp(cosine, 0.0, 1.0), 5.0);
}

fn water_bed(p: vec2<f32>, pixel: f32, material: f32) -> vec3<f32> {
    let limestone = vec3<f32>(0.30, 0.285, 0.225);
    let sandstone = vec3<f32>(0.32, 0.235, 0.15);
    let basalt = vec3<f32>(0.16, 0.175, 0.17);
    var bed = mix(mix(limestone, sandstone, min(material, 1.0)), basalt, max(material - 1.0, 0.0));
    let fine = 1.0 - smoothstep(0.015, 0.10, pixel);
    let patches = 1.0 - smoothstep(0.3, 1.5, pixel);
    if patches > 0.0 {
        bed *= 1.0 + (noise(p * 0.7) - 0.5) * 0.22 * patches;
    }
    if fine > 0.0 {
        // Use the same metric chips as the bank; tint and optics suggest wet
        // gravel without another mesh, normal map, or a full-screen copy.
        let chips = soil_chips(p, pixel);
        bed *= 1.0 + (noise(p * 19.0) - 0.5) * 0.22 * fine;
        bed *= 1.0 + chips.x * (chips.y * 1.3 - 1.0) * fine;
    }
    return bed;
}

// GGX distribution with height-correlated Smith visibility. A finite lobe and
// screen-space normal variance spread highlights instead of unstable pinpricks.
// See https://google.github.io/filament/main/filament.html (specular BRDF).
fn water_sun(normal: vec3<f32>, view: vec3<f32>, alpha_squared: f32) -> f32 {
    let light = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let half_light = normalize(view + light);
    let nv = max(dot(normal, view), 0.001);
    let nl = max(dot(normal, light), 0.0);
    let nh = max(dot(normal, half_light), 0.0);
    let vh = max(dot(view, half_light), 0.0);
    let denominator = nh * nh * (alpha_squared - 1.0) + 1.0;
    let distribution = alpha_squared / (3.14159265 * denominator * denominator);
    let visibility = 0.5 / max(nl * sqrt(nv * nv * (1.0 - alpha_squared) + alpha_squared)
        + nv * sqrt(nl * nl * (1.0 - alpha_squared) + alpha_squared), 0.0001);
    return distribution * visibility * water_fresnel(vh) * nl;
}
