struct TiltShiftSettings {
    focus_center: f32,
    focus_width: f32,
    max_blur_radius: f32,
    _padding: f32,
};

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: TiltShiftSettings;

const GOLDEN_ANGLE: f32 = 2.39996;

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(screen_texture));
    let uv = position.xy / texture_size;

    // Distance from horizontal focus band, mapped to blur amount via smoothstep.
    let dist = abs(uv.y - settings.focus_center) - settings.focus_width;
    let blur_amount = smoothstep(0.0, settings.focus_width, max(dist, 0.0));
    let radius = blur_amount * settings.max_blur_radius;

    // Skip blur in the sharp focus band.
    if radius < 0.5 {
        return textureSample(screen_texture, screen_sampler, uv);
    }

    // 12-tap golden angle spiral + center sample for smooth, artifact-free blur.
    var color = textureSample(screen_texture, screen_sampler, uv);
    let texel = 1.0 / texture_size;

    for (var i = 0u; i < 12u; i = i + 1u) {
        let r = sqrt(f32(i + 1u) / 12.0) * radius;
        let angle = f32(i) * GOLDEN_ANGLE;
        let offset = vec2<f32>(cos(angle), sin(angle)) * r * texel;
        color += textureSample(screen_texture, screen_sampler, uv + offset);
    }

    color /= 13.0;
    return color;
}
