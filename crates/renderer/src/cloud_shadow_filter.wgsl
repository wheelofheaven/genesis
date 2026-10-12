@group(0) @binding(0) var source: texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    return vec4<f32>(p, 0.0, 1.0);
}
@fragment fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) f32 {
    let p = vec2<i32>(pixel.xy) * 2;
    // Average transmission, not optical depth: this preserves mean irradiance.
    return (textureLoad(source, p, 0).r
        + textureLoad(source, p + vec2<i32>(1, 0), 0).r
        + textureLoad(source, p + vec2<i32>(0, 1), 0).r
        + textureLoad(source, p + vec2<i32>(1, 1), 0).r) * 0.25;
}
