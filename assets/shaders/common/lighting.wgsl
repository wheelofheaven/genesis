// Shared stylized lighting utilities for Genesis.
// Pure math — no Bevy bindings.

// Valve half-Lambert wrap lighting.
// wrap_factor: 0.0 = standard Lambert, 1.0 = fully wrapped (half-Lambert).
fn wrap_diffuse(n_dot_l: f32, wrap_factor: f32) -> f32 {
    let wrapped = (n_dot_l + wrap_factor) / (1.0 + wrap_factor);
    return saturate(wrapped);
}

// Hemispheric ambient lighting.
// Interpolates between ground_color (below) and sky_color (above) based on normal Y.
fn hemispheric_ambient(normal: vec3<f32>, sky_color: vec3<f32>, ground_color: vec3<f32>) -> vec3<f32> {
    let factor = 0.5 + 0.5 * normal.y;
    return mix(ground_color, sky_color, factor);
}

// Blinn-Phong specular highlight.
fn blinn_phong_specular(normal: vec3<f32>, view_dir: vec3<f32>, light_dir: vec3<f32>, strength: f32, shininess: f32) -> f32 {
    let half_dir = normalize(view_dir + light_dir);
    let spec_angle = max(dot(normal, half_dir), 0.0);
    return strength * pow(spec_angle, shininess);
}
