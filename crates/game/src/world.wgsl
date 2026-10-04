// The world pass: ground and sprites, into a target that holds the
// art's own sRGB numbers (nothing here converts to linear light), so
// edges and shadows blend the way they did where the art was drawn.

struct Style {
    // rgb: the fill.
    fill: vec4<f32>,
    // rgb: the rim's colour; a: how wide the rim is, in tiles.
    rim: vec4<f32>,
}

struct Globals {
    // Clip-space units per tile, y pointing down the screen.
    scale: vec2<f32>,
    // From the camera's centre to the origin tile, in tiles.
    offset: vec2<f32>,
    // Physical pixels a tile is across.
    tile_px: f32,
    // One per terrain, by rank.
    styles: array<Style, 4>,
}

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var sheet: texture_2d<f32>;
@group(1) @binding(1) var sheet_sampler: sampler;

// A quad's corner from its place in a four-vertex strip:
// (0,0) (1,0) (0,1) (1,1).
fn corner(vi: u32) -> vec2<f32> {
    return vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
}

// ---- ground ---------------------------------------------------------------

struct TerrainOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) data: vec2<u32>,
}

@vertex
fn vs_terrain(@builtin(vertex_index) vi: u32, @location(0) tile: vec2<i32>, @location(1) data: vec2<u32>) -> TerrainOut {
    let c = corner(vi);
    // Whole numbers first: a corner two quads share comes out of the
    // same sums in both, so their edges meet exactly.
    let p = (vec2<f32>(tile) + c) - 0.5 + g.offset;
    var out: TerrainOut;
    out.pos = vec4<f32>(p * g.scale, 0.0, 1.0);
    out.uv = c;
    out.data = data;
    return out;
}

// How far `p` (0..1 across the dual tile) is from the edge of the ground
// that covers the corners in `mask`: negative inside, in tiles. Corners
// are rounded with a radius of half a tile, and every shape crosses the
// quad's sides at their middles, square on, so neighbours join smoothly.
fn edge(mask: u32, p: vec2<f32>) -> f32 {
    let tl = length(p);
    let tr = length(p - vec2<f32>(1.0, 0.0));
    let bl = length(p - vec2<f32>(0.0, 1.0));
    let br = length(p - vec2<f32>(1.0, 1.0));
    switch mask {
        // One corner: a quarter disc.
        case 1u { return tl - 0.5; }
        case 2u { return tr - 0.5; }
        case 4u { return bl - 0.5; }
        case 8u { return br - 0.5; }
        // Two along a side: half the quad.
        case 3u { return p.y - 0.5; }
        case 12u { return 0.5 - p.y; }
        case 5u { return p.x - 0.5; }
        case 10u { return 0.5 - p.x; }
        // Two across: joined through the middle.
        case 9u { return 0.5 - min(tr, bl); }
        case 6u { return 0.5 - min(tl, br); }
        // Three: everything but a quarter disc.
        case 14u { return 0.5 - tl; }
        case 13u { return 0.5 - tr; }
        case 11u { return 0.5 - bl; }
        case 7u { return 0.5 - br; }
        case 15u { return -1.0; }
        default { return 1.0; }
    }
}

@fragment
fn fs_terrain(in: TerrainOut) -> @location(0) vec4<f32> {
    let style = g.styles[in.data.y];
    // Distances in pixels, so the edge is one pixel soft at any zoom.
    let d = edge(in.data.x, in.uv) * g.tile_px;
    let cover = clamp(0.5 - d, 0.0, 1.0);
    let past_rim = clamp(0.5 - (d + style.rim.a * g.tile_px), 0.0, 1.0);
    let rgb = mix(style.rim.rgb, style.fill.rgb, past_rim);
    return vec4<f32>(rgb * cover, cover);
}

// ---- sprites --------------------------------------------------------------

struct SpriteOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
}

@vertex
fn vs_sprite(
    @builtin(vertex_index) vi: u32,
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec4<f32>,
    @location(3) tint: vec4<f32>,
) -> SpriteOut {
    let c = corner(vi);
    let p = pos + c * size + g.offset;
    var out: SpriteOut;
    out.pos = vec4<f32>(p * g.scale, 0.0, 1.0);
    out.uv = mix(uv.xy, uv.zw, c);
    out.tint = tint;
    return out;
}

@fragment
fn fs_sprite(in: SpriteOut) -> @location(0) vec4<f32> {
    // The sheet is premultiplied, and so is the tint.
    return textureSample(sheet, sheet_sampler, in.uv) * in.tint;
}
