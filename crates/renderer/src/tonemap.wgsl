// Genesis display pass. SDR encoding is performed once by the sRGB attachment.
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var<uniform> settings: vec4<f32>;

// WGSL adaptation of Khronos PBR Neutral, Copyright 2024 The Khronos Group, Inc.
// SPDX-License-Identifier: Apache-2.0
// https://github.com/KhronosGroup/ToneMapping/blob/main/PBR_Neutral/pbrNeutral.glsl
// Licence: ../../../licenses/KhronosToneMapping-Apache-2.0.txt
fn neutral_tonemap(input: vec3<f32>) -> vec3<f32> {
    var color = max(input, vec3<f32>(0.0));
    let lowest = min(color.r, min(color.g, color.b));
    let offset = select(0.04, lowest - 6.25 * lowest * lowest, lowest < 0.08);
    color -= vec3<f32>(offset);
    let peak = max(color.r, max(color.g, color.b));
    if peak < 0.76 { return color; }
    let new_peak = 1.0 - 0.24 * 0.24 / (peak + 0.24 - 0.76);
    color *= new_peak / peak;
    let desaturation = 1.0 - 1.0 / (0.15 * (peak - new_peak) + 1.0);
    return mix(color, vec3<f32>(new_peak), desaturation);
}

@vertex fn fullscreen(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    return vec4<f32>(p, 0.0, 1.0);
}
@fragment fn display(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let color = textureLoad(scene, vec2<i32>(position.xy), 0).rgb;
    if settings.y > 0.5 { return vec4<f32>(color, 1.0); }
    return vec4<f32>(neutral_tonemap(color * settings.x), 1.0);
}
