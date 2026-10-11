@group(1) @binding(2) var cloud_transmission: texture_2d<f32>;

fn cloud_sunlight(world_metres: vec3<f32>, footprint: f32) -> f32 {
    let top = camera.clouds.x + camera.cloud_motion.z;
    if camera.clouds.y <= 0.0 || camera.cloud_motion.w <= 0.0 || world_metres.y >= top { return 1.0; }
    let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    if world_metres.y > camera.clouds.x {
        // Rare mountain/airborne receivers inside the layer need a partial integral.
        let step_length = (top - world_metres.y) / (sun.y * 12.0);
        var optical_depth = 0.0;
        for (var i = 0u; i < 12u; i += 1u) {
            let p = world_metres + sun * ((f32(i) + 0.5) * step_length);
            optical_depth += cloud_volume(p, max(footprint, 31.25)) * step_length;
        }
        return mix(1.0, exp(-optical_depth * camera.cloud_motion.w), camera.clouds.z);
    }
    // Project each receiver up to the base of the layer along the actual sun ray.
    // Sampling in the integrated wind frame makes sky and shadow move together.
    let distance = (camera.clouds.x - world_metres.y) / sun.y;
    let uv = cloud_uv(world_metres.xz + sun.xz * distance);
    // Angular sun radius and pixel footprint soften edges, without camera snapping.
    let radius = max(footprint * 0.5, distance * 0.00465) / camera.clouds.w;
    var transmission = 0.0;
    for (var i = 0u; i < 4u; i += 1u) {
        let offset = vec2<f32>(f32(i & 1u) * 2.0 - 1.0, f32(i >> 1u) * 2.0 - 1.0) * radius * 0.7071;
        transmission += textureSampleLevel(cloud_transmission, cloud_sampler, uv + offset, 0.0).r;
    }
    return mix(1.0, transmission * 0.25, camera.clouds.z);
}
