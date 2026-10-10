struct Camera {
    view_projection: mat4x4<f32>,
    direction_time: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world: vec3<f32>,
};
@vertex fn vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> Output {
    var out: Output;
    out.position = camera.view_projection * vec4<f32>(position, 1.0);
    out.color = color;
    out.world = position;
    return out;
}
fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.,0.)), u.x),
        mix(hash(i + vec2<f32>(0.,1.)), hash(i + vec2<f32>(1.,1.)), u.x), u.y);
}
@fragment fn fragment(in: Output) -> @location(0) vec4<f32> {
    let metres = in.world.xz * 20.0;
    let pixel = max(length(fwidth(metres)), 0.001);
    let time = camera.direction_time.w;
    var color = in.color.rgb;
    if in.color.a < 0.0 {
        let depth = max(-in.color.a - 1.0, 0.0);
        let swell = dot(metres, vec2<f32>(0.10,0.055)) - time * 1.25
            + noise(metres * 0.013) * 8.0;
        let ripple = dot(metres, vec2<f32>(0.8,-0.4)) + time * 2.4
            + noise(metres * 0.12) * 4.0;
        let ripple_weight = 1.0 - smoothstep(0.8, 5.0, pixel);
        let swell_weight = 1.0 - smoothstep(8.0, 35.0, pixel);
        let nx = sin(swell) * 0.035 * swell_weight + sin(ripple) * 0.035 * ripple_weight;
        let nz = cos(swell * 0.73 + noise(metres * 0.025) * 2.0) * 0.03 * swell_weight
            + cos(ripple * 1.3) * 0.025 * ripple_weight;
        let normal = normalize(vec3<f32>(nx, 1.0, nz));
        let view = camera.direction_time.xyz;
        let fresnel = 0.025 + 0.45 * pow(1.0 - max(dot(normal, view),0.0), 4.0);
        let shallow = vec3<f32>(0.17, 0.29, 0.24);
        let deep = vec3<f32>(0.021, 0.075, 0.105);
        color = mix(deep, shallow, exp(-depth * 0.16));
        // Attenuated bed detail remains visible only in the shallows.
        color *= 0.97 + noise(metres * 0.12) * 0.06 * exp(-depth * 0.25);
        color = mix(color, vec3<f32>(0.38,0.49,0.56), fresnel);
        let half_light = normalize(view + normalize(vec3<f32>(-0.5,0.75,0.3)));
        let glint = pow(max(dot(normal, half_light),0.0), 180.0);
        color += vec3<f32>(0.65,0.62,0.48) * glint * 0.35;
        color += (nx + nz) * vec3<f32>(0.05,0.08,0.08);
        let wash = 0.18 + 0.12 * sin(time * 1.4 + noise(metres * 0.07) * 5.0);
        let foam = (1.0 - smoothstep(wash, wash + 0.48, depth))
            * (0.25 + 0.30 * noise(metres * 0.6 + time * 0.1));
        color = mix(color, vec3<f32>(0.66,0.71,0.65), foam);
    } else if in.color.a > 2.5 {
        let surface = in.world.xz * 20.0 + vec2<f32>(in.world.y * 13.0, in.world.y * 19.0);
        let grit = (noise(surface * 5.0) - 0.5) * (1.0 - smoothstep(0.05,0.7,pixel));
        color *= 0.91 + noise(surface * 0.9) * 0.12 + grit * 0.16;
    } else if in.color.a > 1.5 {
        let broad = noise(metres * 0.018) * 0.10 + noise(metres * 0.085) * 0.10;
        let fine = (noise(metres * 1.3) - 0.5) * (1.0 - smoothstep(0.2, 2.0, pixel));
        let gravel = (noise(metres * 4.0) - 0.5) * (1.0 - smoothstep(0.05, 0.7, pixel));
        color *= 0.87 + broad + fine * 0.18 + gravel * 0.12;
        let dry = smoothstep(0.48,0.77, noise(metres * 0.065 + noise(metres * 0.008) * 3.0));
        color = mix(color, color * vec3<f32>(1.13,0.98,0.81), dry * 0.5);
    }
    return vec4<f32>(color, 1.0);
}
