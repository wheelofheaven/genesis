struct Camera {
    view_projection: mat4x4<f32>,
    direction_time: vec4<f32>,
    light_projection: mat4x4<f32>,
    far_light_projection: mat4x4<f32>,
    eye_sea_level: vec4<f32>,
    inverse_projection: mat4x4<f32>,
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    haze: vec4<f32>,
    clouds: vec4<f32>,
    cloud_motion: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var sun_depth: texture_depth_2d_array;
@group(1) @binding(1) var sun_sampler: sampler_comparison;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world: vec3<f32>,
    @location(2) local: vec3<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) uv: vec2<f32>,
    @location(5) @interpolate(flat) coverage: vec2<f32>,
};
@vertex fn vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> Output {
    var out: Output;
    out.position = camera.view_projection * vec4<f32>(position, 1.0);
    out.color = color;
    out.world = position;
    out.local = position;
    out.normal = vec3<f32>(0.,1.,0.);
    out.uv = vec2<f32>(0.5);
    out.coverage = vec2<f32>(0., 1.);
    return out;
}
fn hash(p: vec2<f32>) -> f32 {
    // Integer avalanche avoids large-coordinate precision bands from sin(dot()).
    let q = bitcast<vec2<u32>>(vec2<i32>(p));
    var h = q.x * 1597334677u ^ q.y * 3812015801u;
    h = (h ^ (h >> 16u)) * 2246822519u;
    h = (h ^ (h >> 13u)) * 3266489917u;
    return f32((h ^ (h >> 16u)) >> 8u) / 16777216.0;
}
fn scenery_output(position: vec3<f32>, color: vec4<f32>, placement: vec4<f32>, tint_rotation: vec4<f32>, normal: vec3<f32>, uv: vec2<f32>, coverage: vec2<f32>) -> Output {
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
    out.coverage = coverage;
    if (color.a >= 10.0 && color.a < 11.0) || (color.a >= 12.0 && color.a < 13.0) {
        // Ground blades and litter follow a local tangent plane without enlarging every
        // scenery instance. In this material, the coverage pair carries slopes.
        out.world.y += dot(rotated.xz * placement.w, coverage);
        out.normal = normalize(vec3<f32>(out.normal.x - coverage.x * out.normal.y,
            out.normal.y, out.normal.z - coverage.y * out.normal.y));
        out.coverage = vec2<f32>(0.0, 1.0);
    }
    if color.a >= 10.0 {
        // Root distance is constant across a tuft and across camera/shadow passes.
        let distance = length((placement.xyz - camera.eye_sea_level.xyz) * 20.0);
        out.coverage.y *= 1.0 - smoothstep(28.0, 38.0, distance);
        if color.a < 11.0 {
            let height = max(position.y * placement.w * 20.0, 0.0);
            let phase = camera.direction_time.w * 1.6 + placement.x * 2.1 + placement.z * 1.4;
            out.world.x += sin(phase) * height * height * 0.0018;
            out.world.z += cos(phase * 0.8) * height * height * 0.0008;
        }
    }
    out.position = camera.view_projection * vec4<f32>(out.world, 1.);
    out.color = vec4<f32>(color.rgb * tint_rotation.rgb, color.a);
    return out;
}
@vertex fn instanced_vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>,
    @location(2) placement: vec4<f32>, @location(3) tint_rotation: vec4<f32>,
    @location(4) normal: vec3<f32>, @location(5) uv: vec2<f32>,
    @location(6) coverage: vec2<f32>) -> Output {
    return scenery_output(position, color, placement, tint_rotation, normal, uv, coverage);
}
@vertex fn shadow_instanced_vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>,
    @location(2) placement: vec4<f32>, @location(3) tint_rotation: vec4<f32>,
    @location(4) normal: vec3<f32>, @location(5) uv: vec2<f32>,
    @location(6) coverage: vec2<f32>) -> Output {
    var out = scenery_output(position, color, placement, tint_rotation, normal, uv, coverage);
    out.position = camera.light_projection * vec4<f32>(out.world, 1.0);
    return out;
}
@vertex fn shadow_vertex(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return camera.light_projection * vec4<f32>(position,1.0);
}
// A signed field lets MSAA resolve leaf edges instead of binary pixel cutouts.
fn leaf_field(uv: vec2<f32>, material: f32) -> f32 {
    if material < 5.0 || material >= 6.0 { return 1.0; }
    let q = (uv - vec2<f32>(0.5)) * 2.0;
    let stem = min(0.025-abs(q.y), 0.88-abs(q.x));
    if material > 5.2 {
        let wave = abs(fract((q.x + abs(q.y)*0.55)*8.0)-0.24);
        return max(stem, min((1.0-abs(q.x))*0.98-abs(q.y), (0.24-wave)/8.0));
    }
    var field = stem;
    for (var i=0; i<6; i=i+1) {
        let x = -0.60 + f32(i/2)*0.55;
        let side = select(-1.0, 1.0, i%2==0);
        let p = q - vec2<f32>(x, side*0.35);
        let u = (p.x*0.8+p.y*side*0.6)/0.38;
        let v = (p.y*0.8-p.x*side*0.6)/0.27;
        field = max(field, (0.96-u*u-v*v)*0.13);
    }
    return field;
}
fn coverage_visible(in: Output) -> bool {
    // Identical thresholds on adjacent LODs avoid double coverage or empty bands.
    // No time input: holding the camera still cannot cause temporal flicker.
    let threshold = fract(52.9829189 * fract(dot(floor(in.position.xy), vec2<f32>(0.06711056, 0.00583715))));
    return threshold >= in.coverage.x && threshold < in.coverage.y;
}
@fragment fn shadow_fragment(in: Output) {
    if !coverage_visible(in) { discard; }
    if in.color.a >= 5.0 && in.color.a < 6.0 && leaf_field(in.uv,in.color.a) <= 0.0 { discard; }
}
fn receiver_slope(dx: vec4<f32>, dy: vec4<f32>) -> vec2<f32> {
    let determinant = dx.x * dy.y - dx.y * dy.x;
    if abs(determinant) < 1e-16 { return vec2<f32>(0.0); }
    return clamp(vec2<f32>(dx.z * dy.y - dy.z * dx.y,
        dx.x * dy.z - dy.x * dx.z) / determinant, vec2<f32>(-2.0), vec2<f32>(2.0));
}
fn shadow_sample(p: vec4<f32>, layer: i32, slope: vec2<f32>) -> f32 {
    let uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    var shadow = 0.0;
    // Both maps are 2048 square. Explicit levels permit foliage discard.
    // Project each PCF tap onto the receiver plane. The remaining half-texel
    // allowance covers hardware bilinear comparison without broad wall stripes.
    let bias = select(0.00002, 0.00001, layer == 1) + dot(abs(slope), vec2<f32>(1.0 / 2048.0));
    for (var y = -1; y <= 1; y += 1) {
        for (var x = -1; x <= 1; x += 1) {
            shadow += textureSampleCompareLevel(sun_depth, sun_sampler,
                uv + vec2<f32>(f32(x), f32(y)) / 2048.0, layer,
                p.z + dot(slope, vec2<f32>(f32(x), -f32(y)) * (2.0 / 2048.0)) - bias);
        }
    }
    return 0.58 + 0.42 * shadow / 9.0;
}
fn shadow_inside(p: vec4<f32>) -> bool {
    return max(abs(p.x), abs(p.y)) < 0.98 && p.z > 0.0 && p.z < 1.0;
}
fn sunlight(world: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>) -> f32 {
    let p = camera.light_projection * vec4<f32>(world, 1.0);
    let slope = receiver_slope(camera.light_projection * vec4<f32>(dx, 0.0), camera.light_projection * vec4<f32>(dy, 0.0));
    let edge = max(abs(p.x), abs(p.y));
    // Most close pixels sample only the detailed map; only the overlap needs both.
    if shadow_inside(p) && edge <= 0.72 { return shadow_sample(p, 0, slope); }
    let far = camera.far_light_projection * vec4<f32>(world, 1.0);
    var wide = 1.0;
    if shadow_inside(far) {
        let fade = smoothstep(0.82, 0.98, max(abs(far.x), abs(far.y)));
        let far_slope = receiver_slope(camera.far_light_projection * vec4<f32>(dx, 0.0), camera.far_light_projection * vec4<f32>(dy, 0.0));
        wide = mix(shadow_sample(far, 1, far_slope), 1.0, fade);
    }
    if shadow_inside(p) {
        return mix(shadow_sample(p, 0, slope), wide, smoothstep(0.72, 0.94, edge));
    }
    return wide;
}
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.,0.)), u.x),
        mix(hash(i + vec2<f32>(0.,1.)), hash(i + vec2<f32>(1.,1.)), u.x), u.y);
}
// Analytic gradient of smooth value noise: irregular surface slopes without
// finite-difference texture fetches or long, repeating sinusoidal crests.
fn noise_slope(p: vec2<f32>) -> vec2<f32> {
    let cell = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let du = 6.0 * f * (1.0 - f);
    let a = hash(cell);
    let b = hash(cell + vec2<f32>(1.0, 0.0));
    let c = hash(cell + vec2<f32>(0.0, 1.0));
    let d = hash(cell + vec2<f32>(1.0, 1.0));
    return du * vec2<f32>(mix(b - a, d - c, u.y), mix(c - a, d - b, u.x));
}

@fragment fn fragment(in: Output) -> @location(0) vec4<f32> {
    let metres = in.world.xz * 20.0;
    let pixel = max(length(fwidth(in.world * 20.0)), 0.001);
    let local_pixel = max(length(fwidth(in.local)), 0.001);
    let world_dx = dpdx(in.world);
    let world_dy = dpdy(in.world);
    let uv_dx = dpdx(in.uv);
    let uv_dy = dpdy(in.uv);
    let face = cross(world_dx, world_dy);
    // Normalize before applying any fallback. At close range the cross product
    // can be much smaller than a fixed epsilon in our 20-metre world units.
    let magnitude = length(face);
    let surface_normal = select(vec3<f32>(0.0, 1.0, 0.0),
        face * select(-1.0, 1.0, face.y >= 0.0) / max(magnitude, 1e-20), magnitude > 1e-20);
    let field = leaf_field(in.uv, in.color.a);
    let edge_width = max(fwidth(field), 0.0001);
    let leaf_coverage = smoothstep(-edge_width*0.5, edge_width*0.5, field);
    if !coverage_visible(in) { discard; }
    var alpha = 1.0;
    var color = in.color.rgb;
    var transmission = vec3<f32>(0.0);
    if in.color.a >= 14.0 {
        // Coverage is resolved against the underlying terrain by world MSAA.
        // World anchoring keeps intersecting paths on the same material pattern.
        let wear = clamp(in.color.a - 14.0, 0.0, 1.0);
        let edge_detail = (noise(metres * 13.0) - 0.5) * 0.22
            * (1.0 - smoothstep(0.02, 0.20, pixel));
        alpha = smoothstep(0.02, 0.95, wear + edge_detail * (1.0 - wear));
        if alpha <= 0.0 { discard; }
        let patches = noise(metres * 1.8);
        let fine_weight = 1.0 - smoothstep(0.015, 0.12, pixel);
        color *= 0.94 + (patches - 0.5) * 0.14 * (1.0 - smoothstep(0.3, 1.5, pixel));
        color *= 1.0 + (noise(metres * 38.0) - 0.5) * 0.18 * fine_weight;
        if fine_weight > 0.0 {
            let chips = soil_chips(metres, pixel);
            color = mix(color, color * (1.08 * chips.y), chips.x * fine_weight * 0.35);
        }
    } else if in.color.a >= 10.0 {
        let light = normalize(vec3<f32>(-0.5,0.75,0.3));
        let n = normalize(in.normal + vec3<f32>(0.00001));
        if in.color.a < 11.0 {
            let grass = grass_material(in);
            color = grass.diffuse;
            transmission = grass.transmission;
        } else if in.color.a < 12.0 {
            color *= 0.40 + 0.60 * max(dot(n, light), 0.0);
            let grain = (noise(metres * 25.0)-0.5) * (1.0-smoothstep(0.015,0.12,pixel));
            color *= 0.95 + grain * 0.12;
        } else {
            color *= 0.56 + 0.44 * abs(dot(n, light));
        }
    } else if in.color.a >= 5.0 {
        let n = normalize(in.normal + vec3<f32>(0.00001));
        let local = in.local;
        if in.color.a < 6.0 {
            // Actual leaf silhouettes and veins; opaque cutouts write depth normally.
            alpha = leaf_coverage;
            if alpha <= 0.0 { discard; }
            let foliage = foliage_material(in, local_pixel);
            color = foliage.diffuse;
            transmission = foliage.transmission;
        } else if in.color.a < 7.0 {
            color = bark_material(in, world_dx, world_dy, uv_dx, uv_dy);
        } else {
            color = stone_material(color, in.world * 20.0, n, local.y, in.color.a, pixel);
        }
    } else if in.color.a < 0.0 {
        color = water_material(in, pixel);
        if in.color.r < 0.0 {
            // Resolve shallow river margins over their receiver using the existing
            // MSAA coverage; the ocean keeps its separate shore/foam treatment.
            alpha = smoothstep(0.0, 0.08, max(-in.color.a - 1.0, 0.0));
        }
    } else if in.color.a > 3.5 {
        let depth = -in.world.y * 20.0;
        let interval = depth / 25.0;
        let spacing = max(fwidth(interval), 0.002);
        let line = 1.0 - smoothstep(spacing * 0.5, spacing * 1.5, abs(fract(interval + 0.5) - 0.5));
        color *= 0.92 + noise(metres * 0.04) * 0.12;
        color = mix(color, color * 0.55, line * 0.45);
    } else if in.color.a > 2.5 {
        color = building_material(color, in.world * 20.0, surface_normal, in.color.a, pixel);
    } else if in.color.a > 1.5 {
        color = terrain_material(color, in.world * 20.0, surface_normal,
            clamp((in.color.a - 2.0) / 0.4, 0.0, 1.0), pixel);
    }
    if in.color.a >= 2.0 {
        let shadow = sunlight(in.world, world_dx, world_dy);
        if camera.haze.z > 0.5 {
            color *= shadow;
        } else {
            // Artistic sky fill and warm sun. Preserve the existing baked normal
            // response; this separates cast-shadow colour without claiming GI.
            let direct = clamp((shadow - 0.58) / 0.42, 0.0, 1.0) * cloud_sunlight(in.world * 20.0, pixel);
            color *= mix(vec3<f32>(0.40, 0.49, 0.62), vec3<f32>(1.08, 1.035, 0.95), direct);
            if (in.color.a >= 2.0 && in.color.a < 2.5) || in.color.a >= 14.0 {
                // Local scenery blocks sky fill more than direct sunlight.
                // Only ground receivers use this footprint approximation.
                color *= 1.0 - ground_occlusion(in.world, pixel) * (1.0 - direct * 0.65);
            }
            color += transmission * direct * vec3<f32>(1.08, 1.035, 0.95);
        }
    }
    // Selection markings remain legible. Surface, scenery and water share haze.
    if in.color.a >= 2.0 || in.color.a < 0.0 { color = aerial_perspective(color, in.world); }
    return vec4<f32>(color, alpha);
}
