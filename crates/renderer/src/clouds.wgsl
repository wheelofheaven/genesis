// A single world-anchored sheet, not volumetric ray marching. The same periodic
// density field drives sky appearance, water reflections and broad sun shadows.
@group(2) @binding(0) var cloud_density: texture_2d<f32>;
@group(2) @binding(1) var cloud_sampler: sampler;

fn cloud_uv(metres: vec2<f32>) -> vec2<f32> {
    return (metres - camera.cloud_motion.xy) / camera.clouds.w;
}

fn cloud_lod(footprint: f32) -> f32 {
    return clamp(log2(max(footprint * 512.0 / camera.clouds.w, 1.0)), 0.0, 9.0);
}

fn cloud_amount(uv: vec2<f32>, lod: f32) -> f32 {
    let density = textureSampleLevel(cloud_density, cloud_sampler, uv, lod).r;
    let threshold = mix(0.82, 0.22, camera.clouds.y);
    return smoothstep(threshold - 0.10, threshold + 0.16, density);
}

fn cloud_sunlight(world_metres: vec3<f32>, footprint: f32) -> f32 {
    if camera.clouds.y <= 0.0 || world_metres.y >= camera.clouds.x { return 1.0; }
    let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let intersection = world_metres.xz + sun.xz * ((camera.clouds.x - world_metres.y) / sun.y);
    // At least 125 m filtering keeps cloud shadows broad, even at human scale.
    let amount = cloud_amount(cloud_uv(intersection), max(cloud_lod(footprint), 2.0));
    return 1.0 - amount * camera.clouds.z;
}

// Returns cloud radiance and opacity. Footprint is angular size per pixel (or a
// wider reflection cone). Explicit mip levels remain valid after leaf discard.
fn cloud_layer(direction: vec3<f32>, origin: vec3<f32>, angular_pixel: f32) -> vec4<f32> {
    if camera.clouds.y <= 0.0 || abs(direction.y) < 0.01 { return vec4<f32>(0.0); }
    let distance = (camera.clouds.x - origin.y) / direction.y;
    if distance <= 0.0 || distance >= 80000.0 { return vec4<f32>(0.0); }
    let p = origin.xz + direction.xz * distance;
    let lod = cloud_lod(angular_pixel * distance / max(abs(direction.y), 0.03));
    let uv = cloud_uv(p);
    let amount = cloud_amount(uv, lod);
    // Sample toward the sun within the sheet for a soft lit rim and a cooler,
    // denser interior. This suggests thickness without a volume or extra pass.
    let sunward = cloud_amount(uv + vec2<f32>(-0.5, 0.3) * (260.0 / camera.clouds.w), lod);
    let lighting = clamp(0.58 + (amount - sunward) * 1.5 - amount * 0.36, 0.0, 1.0);
    let lit = mix(vec3<f32>(0.40, 0.49, 0.61), vec3<f32>(1.05, 1.02, 0.96), lighting);
    // Distant sheets merge into the same horizon haze as the landscape, rather
    // than leaving high-contrast stripes all the way to the vanishing point.
    let radiance = mix(sky_radiance(direction), lit, exp(-distance * 0.000025));
    let horizon_fade = 1.0 - smoothstep(24000.0, 80000.0, distance);
    let opacity = amount * 0.94 * horizon_fade * smoothstep(0.01, 0.045, abs(direction.y));
    return vec4<f32>(radiance, opacity);
}

fn reflected_sky(direction: vec3<f32>, origin: vec3<f32>, angular_pixel: f32) -> vec3<f32> {
    let cloud = cloud_layer(direction, origin, max(angular_pixel, 0.025));
    return mix(sky_radiance(direction), cloud.rgb, cloud.a);
}
