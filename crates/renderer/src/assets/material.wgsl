@group(3) @binding(0) var base_map: texture_2d<f32>;
@group(3) @binding(1) var normal_map: texture_2d<f32>;
@group(3) @binding(2) var surface_map: texture_2d<f32>;
@group(3) @binding(3) var occlusion_map: texture_2d<f32>;
@group(3) @binding(4) var emissive_map: texture_2d<f32>;
@group(3) @binding(5) var material_sampler: sampler;
@group(3) @binding(7) var occlusion_sampler: sampler;
struct Material { base:vec4<f32>, surface:vec4<f32>, emissive:vec4<f32> };
@group(3) @binding(6) var<uniform> material:Material;
struct AssetOutput {
    @builtin(position) position:vec4<f32>,
    @location(0) world:vec3<f32>,
    @location(1) normal:vec3<f32>,
    @location(2) tangent:vec4<f32>,
    @location(3) uv:vec2<f32>,
    @location(4) tint:vec3<f32>,
    @location(5) occlusion_uv:vec2<f32>,
};
fn asset_rotate(v:vec3<f32>, angle:f32)->vec3<f32> {
    return vec3<f32>(cos(angle)*v.x-sin(angle)*v.z,v.y,sin(angle)*v.x+cos(angle)*v.z);
}
@vertex fn asset_vertex(@location(0) p:vec3<f32>, @location(1) n:vec3<f32>,
    @location(2) t:vec4<f32>, @location(3) uv:vec2<f32>, @location(6) occlusion_uv:vec2<f32>,
    @location(4) placement:vec4<f32>, @location(5) tint_rotation:vec4<f32>)->AssetOutput {
    let world=asset_rotate(p*placement.w,tint_rotation.w)+placement.xyz;
    return AssetOutput(camera.view_projection*vec4<f32>(world,1.0),world,
        asset_rotate(n,tint_rotation.w),vec4<f32>(asset_rotate(t.xyz,tint_rotation.w),t.w),uv,tint_rotation.rgb,occlusion_uv);
}
@vertex fn asset_shadow(@location(0) p:vec3<f32>, @location(4) placement:vec4<f32>,
    @location(5) tint_rotation:vec4<f32>)->@builtin(position) vec4<f32> {
    return camera.light_projection*vec4<f32>(asset_rotate(p*placement.w,tint_rotation.w)+placement.xyz,1.0);
}
@fragment fn asset_fragment(in:AssetOutput)->@location(0) vec4<f32> {
    let base=textureSample(base_map,material_sampler,in.uv).rgb*material.base.rgb*in.tint;
    let surface=textureSample(surface_map,material_sampler,in.uv);
    let metallic=clamp(surface.b*material.surface.x,0.0,1.0);
    let roughness=clamp(surface.g*material.surface.y,0.08,1.0);
    let ao=mix(1.0,textureSample(occlusion_map,occlusion_sampler,in.occlusion_uv).r,material.surface.w);
    let geometric=normalize(in.normal);
    let tangent=normalize(in.tangent.xyz-geometric*dot(geometric,in.tangent.xyz));
    let bitangent=cross(geometric,tangent)*in.tangent.w;
    let detail=normalize((textureSample(normal_map,material_sampler,in.uv).xyz*2.0-1.0)*vec3<f32>(material.surface.zz,1.0));
    let n=normalize(mat3x3<f32>(tangent,bitangent,geometric)*detail);
    let view=normalize(camera.eye_sea_level.xyz-in.world);
    let sun=normalize(vec3<f32>(-0.5,0.75,0.3));
    let half_light=(view+sun)/max(length(view+sun),0.00001);
    let nl=max(dot(n,sun),0.0); let nv=max(dot(n,view),0.001);
    let nh=max(dot(n,half_light),0.0);let vh=max(dot(view,half_light),0.0);
    let dx=dpdx(n);let dy=dpdy(n);
    let a2=min(pow(roughness,4.0)+min(0.25*(dot(dx,dx)+dot(dy,dy)),0.18),1.0);
    let d=nh*nh*(a2-1.0)+1.0;
    let distribution=a2/(3.14159265*d*d);
    let visibility=0.5/max(nl*sqrt(nv*nv*(1.0-a2)+a2)+nv*sqrt(nl*nl*(1.0-a2)+a2),0.00001);
    let f0=mix(vec3<f32>(0.04),base,metallic);
    let fresnel=f0+(1.0-f0)*pow(1.0-vh,5.0);
    let shadow=clamp((sunlight(in.world,dpdx(in.world),dpdy(in.world))-0.58)/0.42,0.0,1.0);
    let direct=shadow*cloud_sunlight(in.world*20.0,length(fwidth(in.world*20.0)));
    let diffuse=base*(1.0-metallic);
    // Clear-sky irradiance comes from a cached hemispherical convolution of the
    // same sky LUT used by water/atmosphere. Ground bounce remains an approximation.
    let sky=asset_irradiance(n)*0.32;
    let ambient=diffuse*sky*ao;
    let sunlight_color=vec3<f32>(2.6,2.47,2.23);
    var color=ambient+((1.0-fresnel)*diffuse/3.14159265+distribution*visibility*fresnel)*nl*direct*sunlight_color;
    color+=textureSample(emissive_map,material_sampler,in.uv).rgb*material.emissive.rgb;
    if camera.haze.z>0.5 {color=base*(0.42+0.58*max(dot(geometric,sun),0.0))* (0.58+0.42*shadow);}
    return vec4<f32>(aerial_perspective(color,in.world),1.0);
}
