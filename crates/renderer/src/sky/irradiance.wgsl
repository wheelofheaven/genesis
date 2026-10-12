@group(0) @binding(0) var sky:texture_2d<f32>;
@group(0) @binding(1) var filtering:sampler;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{
    return vec4<f32>(vec2<f32>(f32((i<<1u)&2u),f32(i&2u))*2.0-1.0,0.0,1.0);
}
fn radiance(ray:vec3<f32>)->vec3<f32>{
    let sun_xz=normalize(vec2<f32>(-0.5,0.3));
    let horizontal=ray.xz/max(length(ray.xz),0.00001);
    let phi=acos(clamp(dot(horizontal,sun_xz),-1.0,1.0))/3.14159265;
    let uv=(vec2<f32>(phi,sqrt(max(ray.y,0.0)))*vec2<f32>(127.0,63.0)+0.5)/vec2<f32>(128.0,64.0);
    return textureSampleLevel(sky,filtering,uv,0.0).rgb;
}
@fragment fn fragment(@builtin(position) position:vec4<f32>)->@location(0) vec4<f32>{
    let uv=(position.xy-0.5)/vec2<f32>(31.0,15.0);
    let y=uv.y*2.0-1.0;let r=sqrt(max(1.0-y*y,0.0));let phi=uv.x*6.2831853;
    let n=vec3<f32>(r*cos(phi),y,r*sin(phi));
    let axis=select(vec3<f32>(0.0,1.0,0.0),vec3<f32>(1.0,0.0,0.0),abs(y)>0.99);
    let t=normalize(cross(axis,n));let b=cross(n,t);
    var sum=vec3<f32>(0.0);
    let ground=radiance(vec3<f32>(0.0,1.0,0.0))*vec3<f32>(0.18,0.15,0.12);
    for(var i=0u;i<32u;i+=1u){
        let a=(f32(i)+0.5)/32.0;let angle=f32(i)*2.39996323;
        let ray=t*(sqrt(a)*cos(angle))+b*(sqrt(a)*sin(angle))+n*sqrt(1.0-a);
        var light=ground;if ray.y>0.0 {light=radiance(ray);}
        sum+=light;
    }
    return vec4<f32>(sum*(3.14159265/32.0),1.0);
}
