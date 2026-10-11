// Two-sided blade lighting. Height coordinates provide local self-occlusion;
// the existing shadow/cloud visibility gates transmitted sunlight afterwards.
fn grass_material(in: Output) -> FoliageResponse {
    let sun = normalize(vec3<f32>(-0.5, 0.75, 0.3));
    let view = normalize(camera.eye_sea_level.xyz - in.world);
    let normal = normalize(in.normal);
    let facing = select(-normal, normal, dot(normal, view) >= 0.0);
    let ndl = dot(facing, sun);
    let wrap = clamp((ndl + 0.4) / 1.4, 0.0, 1.0);
    let height = clamp(in.uv.y, 0.0, 1.0);
    let openness = 0.62 + 0.38 * smoothstep(0.0, 0.7, height);
    let alignment = max(dot(view, -sun), 0.0);
    let forward = alignment * alignment;
    var result: FoliageResponse;
    result.diffuse = in.color.rgb * openness * (0.52 + 0.40 * wrap);
    result.transmission = in.color.rgb * vec3<f32>(1.10, 1.12, 0.72)
        * (max(-ndl, 0.0) * (0.10 + 0.25 * forward * forward) * height);
    return result;
}
