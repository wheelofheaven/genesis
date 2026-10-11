@group(1) @binding(0) var sky_image: texture_2d<f32>;
@group(1) @binding(1) var sky_sampler: sampler;
struct SkyVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) ray: vec3<f32>,
    @location(1) uv: vec2<f32>,
};
@vertex fn vertex(@builtin(vertex_index) index: u32) -> SkyVertex {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    var out: SkyVertex;
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.ray = (camera.inverse_projection * vec4<f32>(p, 0.0, 1.0)).xyz;
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}
@fragment fn fragment(in: SkyVertex) -> @location(0) vec4<f32> {
    let ray = normalize(in.ray);
    let alignment = dot(ray, normalize(vec3<f32>(-0.5, 0.75, 0.3)));
    let edge = max(fwidth(alignment), 0.000001);
    let disk = smoothstep(0.999989 - edge, 0.999989 + edge, alignment);
    let sky = textureSampleLevel(sky_image, sky_sampler, in.uv, 0.0);
    return vec4<f32>(sky.rgb + vec3<f32>(12.0, 11.04, 8.88) * disk * sky.a, 1.0);
}
