// Quarter-resolution separable blur; UI text is composited afterwards.
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var linear_sampler: sampler;
@group(0) @binding(2) var frosted: texture_2d<f32>;

struct Fullscreen {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@vertex fn fullscreen(@builtin(vertex_index) i: u32) -> Fullscreen {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return Fullscreen(vec4<f32>(uv * vec2<f32>(2., -2.) + vec2<f32>(-1., 1.), 0., 1.), uv);
}
fn blur(uv: vec2<f32>, step: vec2<f32>) -> vec4<f32> {
    var color = textureSample(source, linear_sampler, uv) * 0.227027;
    color += textureSample(source, linear_sampler, uv + step * 1.384615) * 0.316216;
    color += textureSample(source, linear_sampler, uv - step * 1.384615) * 0.316216;
    color += textureSample(source, linear_sampler, uv + step * 3.230769) * 0.070270;
    color += textureSample(source, linear_sampler, uv - step * 3.230769) * 0.070270;
    return color;
}
@fragment fn horizontal(in: Fullscreen) -> @location(0) vec4<f32> {
    return blur(in.uv, vec2<f32>(16., 0.) / vec2<f32>(textureDimensions(source)));
}
@fragment fn vertical(in: Fullscreen) -> @location(0) vec4<f32> {
    return blur(in.uv, vec2<f32>(0., 4.) / vec2<f32>(textureDimensions(source)));
}
@fragment fn copy(in: Fullscreen) -> @location(0) vec4<f32> {
    return textureSample(source, linear_sampler, in.uv);
}
struct Panel {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) bounds: vec4<f32>,
    @location(2) @interpolate(flat) corners: vec4<f32>,
    @location(3) @interpolate(flat) clip: vec4<f32>,
};
@vertex fn panel(@builtin(vertex_index) i: u32,
    @location(0) bounds: vec4<f32>, @location(1) corners: vec4<f32>,
    @location(2) clip: vec4<f32>) -> Panel {
    let positions = array<vec2<f32>, 6>(vec2<f32>(0., 0.), vec2<f32>(1., 0.),
        vec2<f32>(0., 1.), vec2<f32>(0., 1.), vec2<f32>(1., 0.), vec2<f32>(1., 1.));
    let local = positions[i] * (bounds.zw - bounds.xy);
    let uv = (bounds.xy + local) / vec2<f32>(textureDimensions(source));
    return Panel(vec4<f32>(uv * vec2<f32>(2., -2.) + vec2<f32>(-1., 1.), 0., 1.), local, bounds, corners, clip);
}
@fragment fn glass(in: Panel) -> @location(0) vec4<f32> {
    let p = in.position.xy;
    if any(p < in.clip.xy) || any(p > in.clip.zw) { discard; }
    let half_size = (in.bounds.zw - in.bounds.xy) * 0.5;
    let local = in.local - half_size;
    let radii = select(in.corners.xy, in.corners.zw, local.y > 0.);
    let radius = min(select(radii.x, radii.y, local.x > 0.), min(half_size.x, half_size.y));
    let q = abs(local) - half_size + radius;
    let distance = length(max(q, vec2<f32>(0.))) + min(max(q.x, q.y), 0.) - radius;
    let alpha = 1. - smoothstep(-0.8, 0., distance);
    let color = textureSample(frosted, linear_sampler, p / vec2<f32>(textureDimensions(source))).rgb;
    // Diffuse a little ambient light through the frost so ink remains legible
    // over deep water as well as sunlit ground, without a dark backing plate.
    return vec4<f32>(mix(color, vec3<f32>(0.88, 0.94, 0.97), 0.12), alpha);
}
