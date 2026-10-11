@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    return vec4<f32>(p, 0.0, 1.0);
}

@fragment fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) f32 {
    if camera.clouds.y <= 0.0 || camera.cloud_motion.w <= 0.0 { return 1.0; }
    let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let xz = pixel.xy * (camera.clouds.w / 512.0);
    let origin = vec3<f32>(xz.x, camera.clouds.x, xz.y);
    let step_length = camera.cloud_motion.z / (sun.y * 32.0);
    var optical_depth = 0.0;
    for (var i = 0u; i < 32u; i += 1u) {
        let p = origin + sun * ((f32(i) + 0.5) * step_length);
        optical_depth += cloud_volume_field(p, camera.clouds.w / 512.0, vec2<f32>(0.0)) * step_length;
    }
    return exp(-optical_depth * camera.cloud_motion.w);
}
