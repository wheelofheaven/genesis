#define_import_path genesis::noise

// Simplex noise utilities for procedural generation.

// Permutation helper.
fn permute3(x: vec3<f32>) -> vec3<f32> {
    return ((x * 34.0 + 1.0) * x) % 289.0;
}

// 2D simplex noise. Returns value in [-1, 1].
fn simplex_noise_2d(v: vec2<f32>) -> f32 {
    let c = vec4<f32>(
        0.211324865405187,  // (3.0 - sqrt(3.0)) / 6.0
        0.366025403784439,  // 0.5 * (sqrt(3.0) - 1.0)
        -0.577350269189626, // -1.0 + 2.0 * c.x
        0.024390243902439,  // 1.0 / 41.0
    );

    // First corner.
    var i = floor(v + dot(v, c.yy));
    let x0 = v - i + dot(i, c.xx);

    // Other corners.
    var i1: vec2<f32>;
    if x0.x > x0.y {
        i1 = vec2<f32>(1.0, 0.0);
    } else {
        i1 = vec2<f32>(0.0, 1.0);
    }
    var x12 = x0.xyxy + c.xxzz;
    x12 = vec4<f32>(x12.xy - i1, x12.zw);

    // Permutations.
    i = i % 289.0;
    let p = permute3(permute3(i.y + vec3<f32>(0.0, i1.y, 1.0)) + i.x + vec3<f32>(0.0, i1.x, 1.0));

    var m = max(0.5 - vec3<f32>(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3<f32>(0.0));
    m = m * m;
    m = m * m;

    // Gradients.
    let x = 2.0 * fract(p * c.www) - 1.0;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;

    m = m * (1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h));

    let g = vec3<f32>(
        a0.x * x0.x + h.x * x0.y,
        a0.y * x12.x + h.y * x12.y,
        a0.z * x12.z + h.z * x12.w,
    );

    return 130.0 * dot(m, g);
}

// Fractal Brownian Motion using simplex noise.
fn fbm(p: vec2<f32>, octaves: i32, lacunarity: f32, gain: f32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;

    for (var i = 0; i < octaves; i = i + 1) {
        value += amplitude * simplex_noise_2d(pos);
        pos *= lacunarity;
        amplitude *= gain;
    }

    return value;
}
