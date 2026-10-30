//! Headless GPU referee for the world shader's lighting paths
//! (`render::LightPath`, queue LITEB): the PS2's VU1 programs for a static
//! model (`analysis/re-model-lighting.md` §2/§3, `[O]`), each pinned to its
//! formula on one quad whose vertex colour stands in for the disc's COL0.
//!
//! The three light terms are put in three different channels — the sun in
//! red, `Light2` in green, the hemisphere ambient in blue — so one pixel says
//! which terms a path took.
//!
//! Ignored by default — needs a real adapter. Run them through
//! `scripts/gpu_tests.sh` (serialised).

use glam::{Mat4, Vec3};
use render::{light_sum, Camera, DirLight, DrawCmd, LightPath, Lighting, MeshData};

use crate::common;
use common::{capture_pixel, headless, upload_tex, N};

/// The vertex colour every quad below carries: 128/255 linear (a Unorm8 is
/// not sRGB-decoded), the COL0 the bake writes.
const C: u8 = 128;

/// sRGB encode of a linear value, as the target stores it.
fn srgb(l: f32) -> u8 {
    let l = l.clamp(0.0, 1.0);
    let s = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

fn close(got: [u8; 4], want: [f32; 3], what: &str) {
    for i in 0..3 {
        let w = srgb(want[i]);
        assert!(
            got[i].abs_diff(w) <= 1,
            "{what}: channel {i} got {got:?}, want {:?} (linear {want:?})",
            want.map(srgb)
        );
    }
}

#[test]
#[ignore = "requires a GPU adapter (run with --ignored on the NVIDIA box)"]
fn each_path_takes_the_vu1_terms_it_should() {
    let mut r = headless();
    // Every light hits the camera-facing quad (normal +z) head on; the
    // hemisphere at n.y = 0 is the mean of sky and ground.
    r.lighting = Lighting {
        sun_dir: Vec3::new(0.0, 0.0, -1.0),
        sun_color: [0.25, 0.0, 0.0],
        sun_static: true,
        light2: Some(DirLight {
            dir: Vec3::new(0.0, 0.0, -1.0),
            color: [0.0, 0.25, 0.0],
            is_static: false,
        }),
        ambient_sky: [0.0, 0.0, 0.5],
        ambient_ground: [0.0, 0.0, 0.0],
        ..Lighting::default()
    };
    let white = upload_tex(&mut r, 1, 1, &[255, 255, 255, 255], "white");
    let quad = r
        .add_mesh(&MeshData {
            positions: &[
                [-4.0, -4.0, -3.0],
                [4.0, -4.0, -3.0],
                [4.0, 4.0, -3.0],
                [-4.0, 4.0, -3.0],
            ],
            normals: &[[0.0, 0.0, 1.0]; 4],
            uvs: &[[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            colors: Some(&[[C, C, C, 255]; 4]),
            indices: &[0, 1, 2, 0, 2, 3],
        })
        .expect("quad");
    let mut shot = |light: LightPath| {
        let d = DrawCmd::textured(quad, white, Mat4::IDENTITY).with_light(light);
        r.render(&Camera::default(), &[d], [0.0, 0.0, 0.0, 1.0])
            .expect("frame");
        capture_pixel(&mut r, N / 2, N / 2)
    };
    let c = f32::from(C) / 255.0;

    // Lit (0x1bb/0x1e6 then 0x521): c × (ambient + sun + Light2) — every
    // light, the Static sun included. The hemisphere blends its two colours
    // in GS units (card LSUM): half the sky's display value.
    close(
        shot(LightPath::Lit),
        [c * 0.25, c * 0.25, light_sum(c, &[(0.5, 0.5)], 0.0)],
        "lit",
    );
    // Prelit (§3): no ambient, the Static sun skipped, Light2 kept, × c.
    close(shot(LightPath::Prelit), [0.0, c * 0.25, 0.0], "prelit");
    // Prelit, COL0 ADDED (0x537): c + the non-Static lights, in GS units.
    let add = light_sum(1.0, &[(c, 1.0), (0.25, 1.0)], 0.0);
    close(shot(LightPath::PrelitAdd), [c, add, c], "prelit_add");
    // Unlit (0x2a3 → 0x2da): c alone, no light at all.
    close(shot(LightPath::Unlit), [c; 3], "unlit");
    // Unlit, COL0 added to the unit material (0x2a3 → 0x2bf): past white.
    close(shot(LightPath::UnlitAdd), [1.0; 3], "unlit_add");
    // Terrain (0x2a3 → 0x2da on the heightfield): the byte is the GS's, so
    // 128 is unity and no light is taken.
    close(shot(LightPath::Terrain), [1.0; 3], "terrain");
}

/// The light sum is made in GS (display) units (card LSUM,
/// `analysis/re-kamino-decks.md` §2): one lit texel T under an ambient A and
/// a head-on sun S reads T × (A + S) on the screen — Kamino's blue channel,
/// ambient 100, sun 105 — not the sum of the linearised colours, which the
/// port made before and which is 41 levels darker here (87 against 128).
#[test]
#[ignore = "requires a GPU adapter (run with --ignored on the NVIDIA box)"]
fn the_lit_sum_is_made_in_display_units() {
    const T: u8 = 160;
    const A: u8 = 100;
    const S: u8 = 105;
    let lin = |b: u8| {
        let d = f32::from(b) / 255.0;
        if d <= 0.040_45 {
            d / 12.92
        } else {
            ((d + 0.055) / 1.055).powf(2.4)
        }
    };
    let mut r = headless();
    r.lighting = Lighting {
        sun_dir: Vec3::new(0.0, 0.0, -1.0),
        sun_color: [lin(S); 3],
        ambient_sky: [lin(A); 3],
        ambient_ground: [lin(A); 3],
        ..Lighting::default()
    };
    assert!(r.lighting.light2.is_none() && !r.lighting.sun_static);
    let tex = upload_tex(&mut r, 1, 1, &[T, T, T, 255], "grey");
    let quad = common::view_quad(&mut r, -3.0);
    r.render(
        &Camera::default(),
        &[DrawCmd::textured(quad, tex, Mat4::IDENTITY)],
        [0.0, 0.0, 0.0, 1.0],
    )
    .expect("frame");
    let got = capture_pixel(&mut r, N / 2, N / 2);
    // The shader's own arithmetic, by its CPU twin.
    let twin = light_sum(lin(T), &[(lin(A), 1.0), (lin(S), 1.0)], 0.0);
    close(got, [twin; 3], "lit, ambient + sun");
    // The PS2's product in display units: 160 × 205 / 255 = 128.6 (the
    // exact sRGB curve's toe leaves the port within 3 levels of it).
    let ps2 = f32::from(T) * (f32::from(A) + f32::from(S)) / 255.0;
    // The linear sum's value: 160 × encode(lin(100) + lin(105)).
    let linear = srgb(lin(T) * (lin(A) + lin(S)));
    for ch in got.iter().take(3) {
        assert!(
            (f32::from(*ch) - ps2).abs() <= 3.0,
            "got {got:?}, the PS2's product {ps2}"
        );
        assert!(
            ch.abs_diff(linear) >= 15,
            "got {got:?} is the linear sum's {linear}"
        );
    }
}

/// BF2's self-lit paths (card GLOW, `analysis/re-glow.md` §1): `Glow`
/// multiplies the unlit colour by 1 + As/128 in DISPLAY units, with As from
/// the texel's alpha and the vertex alpha (A = 64 → At = 32 → As = 63 at
/// Av = 255); `Unscaled` doubles it; both clamp at white and take no light.
#[test]
#[ignore = "requires a GPU adapter (run with --ignored on the NVIDIA box)"]
fn glow_and_unscaled_gain_in_display_units() {
    let mut r = headless();
    r.lighting = Lighting {
        sun_dir: Vec3::new(0.0, 0.0, -1.0),
        sun_color: [0.5, 0.5, 0.5],
        ..Lighting::default()
    };
    let tex = upload_tex(&mut r, 1, 1, &[255, 255, 255, 64], "glow");
    const V: u8 = 40;
    let quad = r
        .add_mesh(&MeshData {
            positions: &[
                [-4.0, -4.0, -3.0],
                [4.0, -4.0, -3.0],
                [4.0, 4.0, -3.0],
                [-4.0, 4.0, -3.0],
            ],
            normals: &[[0.0, 0.0, 1.0]; 4],
            uvs: &[[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            colors: Some(&[[V, V, V, 255]; 4]),
            indices: &[0, 1, 2, 0, 2, 3],
        })
        .expect("quad");
    let shot = |r: &mut render::Renderer, light: LightPath| {
        // A red instance tint: `Glow` drops it, `Unscaled` keeps it.
        let mut d = DrawCmd::textured(quad, tex, Mat4::IDENTITY).with_light(light);
        d.color = [1.0, 0.5, 0.5, 1.0];
        r.render(&Camera::default(), &[d], [0.0, 0.0, 0.0, 1.0])
            .expect("frame");
        capture_pixel(r, N / 2, N / 2)
    };
    let v = f32::from(V) / 255.0;
    let disp = f32::from(srgb(v)) / 255.0;
    let lin = |d: f32| {
        let d = d.min(1.0);
        if d <= 0.040_45 {
            d / 12.92
        } else {
            ((d + 0.055) / 1.055).powf(2.4)
        }
    };
    let g = lin(disp * (1.0 + 63.0 / 128.0));
    close(shot(&mut r, LightPath::Glow), [g; 3], "glow");
    let u = |t: f32| lin(f32::from(srgb(v * t)) / 255.0 * 2.0);
    close(
        shot(&mut r, LightPath::Unscaled),
        [u(1.0), u(0.5), u(0.5)],
        "unscaled",
    );
    // A map's tune (`Lighting::glow_gain`) scales both gains in display units.
    r.lighting.glow_gain = 0.8;
    let g = lin(disp * (1.0 + 63.0 / 128.0) * 0.8);
    close(shot(&mut r, LightPath::Glow), [g; 3], "glow x0.8");
    let u = |t: f32| lin(f32::from(srgb(v * t)) / 255.0 * 2.0 * 0.8);
    close(
        shot(&mut r, LightPath::Unscaled),
        [u(1.0), u(0.5), u(0.5)],
        "unscaled x0.8",
    );
}
