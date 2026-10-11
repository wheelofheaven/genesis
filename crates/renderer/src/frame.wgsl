struct Camera {
    view_projection: mat4x4<f32>,
    direction_time: vec4<f32>,
    light_projection: mat4x4<f32>,
    far_light_projection: mat4x4<f32>,
    eye_sea_level: vec4<f32>,
    inverse_projection: mat4x4<f32>,
    rayleigh: vec4<f32>,
    mie: vec4<f32>,
    air_layers: vec4<f32>,
    haze: vec4<f32>,
    clouds: vec4<f32>,
    cloud_motion: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
