// ── BF2's terrain light pass (`terrain_light.rs`, card FXBL) ────────────────
// Appended after `shader.wgsl`. `analysis/re-bolt.md` §5.3 [O]: per patch,
// each of its lights adds com_sfx_omniproj projected straight down from 5 m
// above the light, × the light's colour × f (already folded in by the
// caller), at vertex alpha 0x20 — Cs·As = T·c·f/4 — added in the GS's gamma
// bytes over the frame's copy, written as the linear difference.
struct TlParams {
    // x = lights, y = patches.
    counts: vec4<u32>,
    // Light k: [2k] = position, [2k + 1] = colour × f (1.0 = 255).
    lights: array<vec4<f32>, 64>,
    // Patch k: min x, min z, max x, max z.
    rects: array<vec4<f32>, 128>,
    // Patch k: up to three light indices, w = how many.
    sel: array<vec4<u32>, 128>,
    // Union of each consecutive eight rects. No reordering: overlaps still
    // take the first rectangle. CPU: terrain_light::PATCH_GROUP_SIZE.
    groups: array<vec4<f32>, 16>,
};
@group(1) @binding(0) var t_omni: texture_2d<f32>;
@group(1) @binding(1) var s_omni: sampler;
@group(1) @binding(2) var t_dest: texture_2d<f32>;
@group(1) @binding(3) var<uniform> tl: TlParams;

struct VsTlOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
};

// `vs_main`'s own clip expression, so the pass lands on the terrain's depth
// and the LessEqual test keeps it.
@vertex
fn vs_terrain_light(in: VsIn) -> VsTlOut {
    let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
    let world_pos = model * vec4<f32>(in.position, 1.0);
    var out: VsTlOut;
    out.clip_pos = globals.view_proj * world_pos;
    out.world_pos = world_pos.xyz;
    return out;
}

@fragment
fn fs_terrain_light(in: VsTlOut) -> @location(0) vec4<f32> {
    let p = in.world_pos;
    var found = -1;
    if tl.counts.z == 0u {
        // Original linear search, retained as a diagnostic reference.
        for (var i = 0u; i < tl.counts.y; i++) {
            let r = tl.rects[i];
            if p.x >= r.x && p.x < r.z && p.z >= r.y && p.z < r.w {
                found = i32(i);
                break;
            }
        }
    } else {
        for (var start = 0u; start < tl.counts.y; start += 8u) {
            let bound = tl.groups[start / 8u];
            if !(p.x >= bound.x && p.x < bound.z && p.z >= bound.y && p.z < bound.w) {
                continue;
            }
            for (var i = start; i < min(start + 8u, tl.counts.y); i++) {
                let r = tl.rects[i];
                if p.x >= r.x && p.x < r.z && p.z >= r.y && p.z < r.w {
                    found = i32(i);
                    break;
                }
            }
            if found >= 0 {
                break;
            }
        }
    }
    if found < 0 {
        discard;
    }
    let sel = tl.sel[found];
    var add = vec3<f32>(0.0);
    for (var k = 0u; k < sel.w; k++) {
        let li = sel[k];
        let l = tl.lights[2u * li].xyz;
        let c = tl.lights[2u * li + 1u].rgb;
        let zp = l.y + 5.0 - p.y;
        if zp <= 0.0 {
            continue;
        }
        let st = vec2<f32>(0.5 + 0.5 * (l.z - p.z) / zp, 0.5 + 0.5 * (p.x - l.x) / zp);
        let t = textureSampleLevel(t_omni, s_omni, st, 0.0);
        // MODULATE: T·c (the vertex byte's 1.0 = 0x80); As = At·0x20/0x80.
        add += t.rgb * c * (t.a * 0.25);
    }
    let dst = textureLoad(t_dest, vec2<i32>(in.clip_pos.xy), 0).rgb;
    let g = min(linear_to_srgb(dst) + add, vec3<f32>(1.0));
    return vec4<f32>(srgb_to_linear(g) - dst, 0.0);
}
