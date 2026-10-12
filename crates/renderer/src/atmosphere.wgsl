struct SkyOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ray: vec3<f32>,
    @location(1) uv: vec2<f32>,
};
@vertex fn sky_vertex(@builtin(vertex_index) index: u32) -> SkyOutput {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    var out: SkyOutput;
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    // z=0 is a direction at infinity in our reversed infinite projection.
    out.ray = (camera.inverse_projection * vec4<f32>(p, 0.0, 1.0)).xyz;
    return out;
}
// RGB stores sky + cloud radiance; alpha stores cloud transmission so the
// composite can draw the small solar disk at full display resolution.
@fragment fn sky_environment(in: SkyOutput) -> @location(0) vec4<f32> {
    let ray = normalize(in.ray);
    let jitter = 0.1 + 0.8 * hash(floor(in.position.xy));
    let cloud = cloud_layer(ray, camera.eye_sea_level.xyz * 20.0, length(fwidth(ray)), 64u, jitter);
    return vec4<f32>(mix(sky_radiance(ray), cloud.rgb, cloud.a), 1.0 - cloud.a);
}
