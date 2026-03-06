#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::{view, lights}
#import bevy_pbr::shadows::fetch_directional_shadow
#import "shaders/common/noise.wgsl"::{simplex_noise_2d, fbm}

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::mesh_view_bindings::screen_space_ambient_occlusion_texture
#endif

struct StylizedMaterial {
    base_color: vec4<f32>,
    sky_color: vec4<f32>,
    ground_color: vec4<f32>,
    wrap_factor: f32,
    specular_strength: f32,
    specular_shininess: f32,
    contact_shadow_strength: f32,
    detail_scale: f32,
    detail_strength: f32,
    detail_roughness: f32,
    _padding: f32,
};

@group(2) @binding(0) var<uniform> material: StylizedMaterial;

// Valve half-Lambert wrap lighting.
fn wrap_diffuse(n_dot_l: f32, wrap_factor: f32) -> f32 {
    let wrapped = (n_dot_l + wrap_factor) / (1.0 + wrap_factor);
    return saturate(wrapped);
}

// Hemispheric ambient: lerp by world-space normal Y.
fn hemispheric_ambient(normal: vec3<f32>, sky_color: vec3<f32>, ground_color: vec3<f32>) -> vec3<f32> {
    let factor = 0.5 + 0.5 * normal.y;
    return mix(ground_color, sky_color, factor);
}

// Blinn-Phong specular highlight.
fn blinn_phong_specular(normal: vec3<f32>, view_dir: vec3<f32>, light_dir: vec3<f32>, strength: f32, shininess: f32) -> f32 {
    let half_dir = normalize(view_dir + light_dir);
    let spec_angle = max(dot(normal, half_dir), 0.0);
    return strength * pow(spec_angle, shininess);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);

    // View Z for shadow cascade selection.
    let view_z = dot(vec4<f32>(
        view.view_from_world[0].z,
        view.view_from_world[1].z,
        view.view_from_world[2].z,
        view.view_from_world[3].z,
    ), in.world_position);

    // Vertex color support — defaults to white when mesh has no vertex colors.
#ifdef VERTEX_COLORS
    let vertex_color = in.color;
#else
    let vertex_color = vec4<f32>(1.0);
#endif

    // Effective base color: material * vertex color.
    var effective_color = material.base_color.rgb * vertex_color.rgb;

    // Procedural noise detail: vary color by world-space noise.
    if material.detail_strength > 0.0 {
        let noise_uv = in.world_position.xz * material.detail_scale;
        let noise_val = fbm(noise_uv, 3, 2.0, 0.5);
        effective_color *= 1.0 + noise_val * material.detail_strength;
    }

    // Effective specular shininess, optionally varied by noise.
    var effective_shininess = material.specular_shininess;
    if material.detail_roughness > 0.0 {
        let rough_noise = simplex_noise_2d(in.world_position.xz * material.detail_scale * 2.0);
        effective_shininess *= 1.0 + rough_noise * material.detail_roughness;
    }

    // Start with hemispheric ambient.
    var diffuse_light = hemispheric_ambient(normal, material.sky_color.rgb, material.ground_color.rgb);
    var specular_light = vec3<f32>(0.0);

    // Modulate ambient by screen-space ambient occlusion.
#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
    let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
    diffuse_light *= ssao;
#endif

    // Accumulate directional lights.
    let n_lights = lights.n_directional_lights;
    for (var i = 0u; i < n_lights; i = i + 1u) {
        let light = lights.directional_lights[i];
        let light_dir = light.direction_to_light;
        let light_color = light.color.rgb;

        // Wrap diffuse.
        let n_dot_l = dot(normal, light_dir);
        let diffuse = wrap_diffuse(n_dot_l, material.wrap_factor);

        // Shadow map sampling.
        let shadow = fetch_directional_shadow(i, in.world_position, normal, view_z);

        // Blinn-Phong specular with effective shininess.
        let spec = blinn_phong_specular(normal, view_dir, light_dir, material.specular_strength, effective_shininess);

        diffuse_light += diffuse * shadow * light_color;
        specular_light += spec * shadow * light_color;
    }

    // Base color modulates diffuse; specular reflects light color directly.
    let final_color = effective_color * diffuse_light + specular_light;
    let final_alpha = material.base_color.a * vertex_color.a;

    return vec4<f32>(final_color, final_alpha);
}
