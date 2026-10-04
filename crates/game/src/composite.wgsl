// Puts the finished world picture on the window, pixel for pixel.

@group(0) @binding(0) var world: texture_2d<f32>;
@group(0) @binding(1) var world_sampler: sampler;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// One triangle that covers the viewport.
@vertex
fn vs_composite(@builtin(vertex_index) vi: u32) -> Out {
    let c = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var out: Out;
    out.pos = vec4<f32>(c * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = c;
    return out;
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

// The window's own format is sRGB: it wants linear light and encodes it
// back to the numbers the world picture already holds.
@fragment
fn fs_to_srgb(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(to_linear(textureSample(world, world_sampler, in.uv).rgb), 1.0);
}

// A window format that stores what it is given.
@fragment
fn fs_as_is(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(world, world_sampler, in.uv).rgb, 1.0);
}
