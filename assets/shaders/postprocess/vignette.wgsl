struct VignetteSettings {
    intensity: f32,
    radius: f32,
    softness: f32,
    _padding: f32,
};

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: VignetteSettings;

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(screen_texture));
    let uv = position.xy / texture_size;
    let color = textureSample(screen_texture, screen_sampler, uv);

    // Compute radial distance from screen center.
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center);

    // Smoothstep vignette darkening.
    let vignette = 1.0 - smoothstep(settings.radius, settings.radius + settings.softness, dist) * settings.intensity;

    return vec4<f32>(color.rgb * vignette, color.a);
}
