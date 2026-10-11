struct FoliageResponse {
    diffuse: vec3<f32>,
    transmission: vec3<f32>,
};

fn foliage_material(in: Output, pixel: f32) -> FoliageResponse {
    // Fractional leaf tags carry the static crown's sky occlusion. The gap
    // between broadleaf [5, 5.15] and needle [5.3, 5.45] preserves cutout routing.
    let kind = select(5.0, 5.3, in.color.a > 5.2);
    let openness = 1.0 - clamp((in.color.a - kind) / 0.15, 0.0, 1.0);
    let light = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let n = normalize(in.normal + vec3<f32>(0.00001));
    let ndl = dot(n, light);
    let wrap = clamp((ndl + 0.35) / 1.35, 0.0, 1.0);
    let sky = 0.45 + 0.55 * openness;
    let detail = 1.0 - smoothstep(0.015, 0.12, pixel);
    let q = (in.uv - vec2<f32>(0.5)) * 2.0;
    let vein = 1.0 - smoothstep(0.015, 0.07, abs(q.y));
    let variation = 0.915 + (noise(in.local.xz * 16.0) - 0.5) * 0.17 * detail + vein * 0.07 * detail;
    let albedo = in.color.rgb * variation;

    // Broad forward scattering brightens thin leaves when looking toward the
    // sun. The caller applies direct-light visibility, so shadowed leaves do
    // not emit light. Curved shading normals approximate a leafy crown volume.
    let view = normalize(camera.eye_sea_level.xyz - in.world);
    let alignment = max(dot(view, -light), 0.0);
    let phase = alignment * alignment;
    let thin = max(-ndl, 0.0) * (0.2 + 0.8 * phase * phase);
    let needle = select(1.0, 0.65, kind > 5.2);
    var result: FoliageResponse;
    result.diffuse = albedo * (0.50 * sky + 0.42 * wrap);
    result.transmission = albedo * vec3<f32>(1.08, 1.12, 0.70)
        * (0.35 * thin * (0.3 + 0.7 * openness) * needle);
    return result;
}
