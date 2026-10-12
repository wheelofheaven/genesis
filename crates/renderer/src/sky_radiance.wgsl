// Shared directional scattering lookup for sky, water and regional aerial light.
// Cloud integration is evaluated in the reduced-resolution sky environment pass.
@group(2) @binding(3) var sky_scattering: texture_2d<f32>;
@group(2) @binding(4) var sky_lookup_sampler: sampler;
fn sky_radiance(direction: vec3<f32>) -> vec3<f32> {
    let sun_xz = normalize(vec2<f32>(-0.5, 0.3));
    let horizontal = direction.xz / max(length(direction.xz), 0.00001);
    let phi = acos(clamp(dot(horizontal, sun_xz), -1.0, 1.0)) / 3.14159265;
    let height = sqrt(clamp(direction.y, 0.0, 1.0));
    let uv = (vec2<f32>(phi, height) * vec2<f32>(127.0, 63.0) + 0.5) / vec2<f32>(128.0, 64.0);
    return textureSampleLevel(sky_scattering, sky_lookup_sampler, uv, 0.0).rgb;
}

@group(2) @binding(5) var sky_irradiance:texture_2d<f32>;
fn asset_irradiance(normal:vec3<f32>)->vec3<f32>{
    let u=fract(atan2(normal.z,normal.x)/6.2831853+1.0);
    let uv=(vec2<f32>(u,normal.y*0.5+0.5)*vec2<f32>(31.0,15.0)+0.5)/vec2<f32>(32.0,16.0);
    return textureSampleLevel(sky_irradiance,sky_lookup_sampler,uv,0.0).rgb;
}
