struct Camera {
    view_projection: mat4x4<f32>,
    direction_time: vec4<f32>,
    light_projection: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var sun_depth: texture_depth_2d;
@group(1) @binding(1) var sun_sampler: sampler_comparison;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world: vec3<f32>,
    @location(2) local: vec3<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) uv: vec2<f32>,
};
@vertex fn vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> Output {
    var out: Output;
    out.position = camera.view_projection * vec4<f32>(position, 1.0);
    out.color = color;
    out.world = position;
    out.local = position;
    out.normal = vec3<f32>(0.,1.,0.);
    out.uv = vec2<f32>(0.5);
    return out;
}
fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
fn scenery_output(position: vec3<f32>, color: vec4<f32>, placement: vec4<f32>, tint_rotation: vec4<f32>, normal: vec3<f32>, uv: vec2<f32>) -> Output {
    let s = sin(tint_rotation.w);
    let c = cos(tint_rotation.w);
    let rotated = vec3<f32>(c * position.x - s * position.z, position.y, s * position.x + c * position.z);
    var out: Output;
    out.world = placement.xyz + rotated * placement.w;
    if color.a >= 5.0 && color.a < 6.0 {
        let phase = camera.direction_time.w * 1.1 + placement.x * 1.7 + placement.z * 2.3;
        let flex = max(position.y * 20.0 - 0.3, 0.0);
        out.world.x += sin(phase + position.y * 2.0) * flex * 0.00035;
        out.world.z += cos(phase * 0.7) * flex * 0.00015;
    }
    out.local = position * 20.0;
    out.normal = vec3<f32>(c * normal.x - s * normal.z, normal.y, s * normal.x + c * normal.z);
    out.uv = uv;
    out.position = camera.view_projection * vec4<f32>(out.world, 1.);
    out.color = vec4<f32>(color.rgb * tint_rotation.rgb, color.a);
    return out;
}
@vertex fn instanced_vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>,
    @location(2) placement: vec4<f32>, @location(3) tint_rotation: vec4<f32>,
    @location(4) normal: vec3<f32>, @location(5) uv: vec2<f32>) -> Output {
    return scenery_output(position, color, placement, tint_rotation, normal, uv);
}
@vertex fn shadow_instanced_vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>,
    @location(2) placement: vec4<f32>, @location(3) tint_rotation: vec4<f32>,
    @location(4) normal: vec3<f32>, @location(5) uv: vec2<f32>) -> Output {
    var out = scenery_output(position, color, placement, tint_rotation, normal, uv);
    out.position = camera.light_projection * vec4<f32>(out.world, 1.0);
    return out;
}
@vertex fn shadow_vertex(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return camera.light_projection * vec4<f32>(position,1.0);
}
fn leaf_mask(uv: vec2<f32>, material: f32) -> bool {
    let q = (uv - vec2<f32>(0.5)) * 2.0;
    if abs(q.y)<0.025 && abs(q.x)<0.88 { return true; }
    if material > 5.2 {
        // Needle sprays carried on a twig, rather than broad leaves on pines.
        let wave = fract((q.x + abs(q.y)*0.55)*8.0);
        return abs(q.y)<(1.0-abs(q.x))*0.98 && wave<0.48;
    }
    for (var i=0; i<6; i=i+1) {
        let x = -0.60 + f32(i/2)*0.55;
        let side = select(-1.0, 1.0, i%2==0);
        let p = q - vec2<f32>(x, side*0.35);
        let u = (p.x*0.8+p.y*side*0.6)/0.38;
        let v = (p.y*0.8-p.x*side*0.6)/0.27;
        if u*u+v*v < 0.96-0.035*sin(u*40.0) { return true; }
    }
    return false;
}
@fragment fn shadow_fragment(in: Output) {
    if in.color.a >= 5.0 && in.color.a < 6.0 && !leaf_mask(in.uv,in.color.a) { discard; }
}
fn sunlight(world: vec3<f32>) -> f32 {
    let p = camera.light_projection * vec4<f32>(world,1.0);
    let uv = vec2<f32>(p.x*0.5+0.5,0.5-p.y*0.5);
    // Explicit level supports foliage discard and branching without derivative assumptions.
    var shadow = 0.0;
    for (var y=-1; y<=1; y=y+1) {
        for (var x=-1; x<=1; x=x+1) {
            shadow += textureSampleCompareLevel(sun_depth,sun_sampler,uv+vec2<f32>(f32(x),f32(y))/2048.0,p.z-0.00012);
        }
    }
    let edge = max(abs(p.x),abs(p.y));
    let fade = smoothstep(0.82,0.98,edge);
    if p.z<=0.0 || p.z>=1.0 { return 1.0; }
    return mix(0.42+0.58*shadow/9.0,1.0,fade);
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
    if in.color.a >= 5.0 {
        let light = normalize(vec3<f32>(-0.5,0.75,0.3));
        let n = normalize(in.normal + vec3<f32>(0.00001));
        let local = in.local;
        if in.color.a < 6.0 {
            // Actual leaf silhouettes and veins; opaque cutouts write depth normally.
            let q = (in.uv - vec2<f32>(0.5)) * 2.0;
            if !leaf_mask(in.uv,in.color.a) { discard; }
            let vein = 1.0-smoothstep(0.015,0.07,abs(q.y));
            color *= 0.83 + 0.17*noise(local.xz*16.0) + vein*0.10;
            color *= 0.48 + 0.42*abs(dot(n,light)) + 0.10*max(n.y,0.0);
            // Thin foliage receives a little transmitted light on its reverse side.
            color += in.color.rgb*0.12*max(dot(-n,light),0.0);
        } else if in.color.a < 7.0 {
            let bark = noise(vec2<f32>(atan2(local.z,local.x)*22.0,local.y*0.7));
            let cracks = smoothstep(0.28,0.40,bark);
            color *= 0.58 + cracks*0.46 + noise(local.xz*30.0)*0.12;
            color *= 0.32+0.68*max(dot(n,light),0.0);
        } else {
            let coarse = noise(local.xz*1.9 + local.y*0.7);
            let fine = noise(local.xz*35.0+local.y*17.0);
            let fine_weight = 1.0-smoothstep(0.03,0.35,pixel);
            color *= 0.76 + coarse*0.35 + (fine-0.5)*0.22*fine_weight;
            if in.color.a < 9.0 {
                let band = sin(local.y*22.0 + noise(local.xz*0.8)*3.0);
                color *= 0.92 + 0.08*smoothstep(-0.5,0.4,band);
            }
            color *= 0.30+0.70*max(dot(n,light),0.0);
            color *= 0.80+0.20*smoothstep(-0.2,0.65,local.y);
        }
    } else if in.color.a < 0.0 {
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
    } else if in.color.a > 3.5 {
        let depth = -in.world.y * 20.0;
        let interval = depth / 25.0;
        let spacing = max(fwidth(interval), 0.002);
        let line = 1.0 - smoothstep(spacing * 0.5, spacing * 1.5, abs(fract(interval + 0.5) - 0.5));
        color *= 0.92 + noise(metres * 0.04) * 0.12;
        color = mix(color, color * 0.55, line * 0.45);
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
    if in.color.a >= 2.0 { color *= sunlight(in.world); }
    return vec4<f32>(color, 1.0);
}
