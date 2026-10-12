// Isotropic dielectric GGX: distribution, height-correlated Smith visibility,
// and Schlick Fresnel. Independently implemented from the standard equations;
// reference: https://google.github.io/filament/main/filament.html
// Returns reflected sunlight per unit incident radiance (including N.L), and
// the remaining diffuse fraction. Roughness inputs are perceptual, not alpha.
fn dielectric_sun(normal: vec3<f32>, view: vec3<f32>, light: vec3<f32>,
    roughness: f32, normal_variance: f32) -> vec2<f32> {
    let nl = dot(normal, light);
    let nv = dot(normal, view);
    if nl <= 0.0 || nv <= 0.0 { return vec2<f32>(0.0, 1.0); }
    let half_light = normalize(view + light);
    let nh = clamp(dot(normal, half_light), 0.0, 1.0);
    let vh = clamp(dot(view, half_light), 0.0, 1.0);
    let r = clamp(roughness, 0.35, 1.0);
    let alpha_squared = min(r * r * r * r + clamp(normal_variance, 0.0, 0.18), 1.0);
    let denominator = nh * nh * (alpha_squared - 1.0) + 1.0;
    let distribution = alpha_squared / (3.14159265 * denominator * denominator);
    let visibility = 0.5 / max(nl * sqrt(nv * nv * (1.0 - alpha_squared) + alpha_squared)
        + nv * sqrt(nl * nl * (1.0 - alpha_squared) + alpha_squared), 0.00001);
    let grazing = 1.0 - vh;
    let fresnel = 0.04 + 0.96 * grazing * grazing * grazing * grazing * grazing;
    return vec2<f32>(distribution * visibility * fresnel * nl, 1.0 - fresnel);
}
