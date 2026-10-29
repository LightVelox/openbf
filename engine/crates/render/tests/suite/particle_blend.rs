use glam::Vec3;
use render::{Camera, ParticleDraw, ParticleShape};

use crate::common::{capture_pixel, headless, upload_tex, N};

fn linear(display: u8) -> f64 {
    let c = f64::from(display) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn shot(texel: [u8; 4], color: [u8; 4], additive: bool, clear: u8) -> [u8; 4] {
    let mut r = headless();
    let tex = upload_tex(&mut r, 1, 1, &texel, "particle");
    r.set_particles(vec![ParticleDraw {
        shape: ParticleShape::Sprite {
            pos: Vec3::new(0.0, 0.0, -3.0),
            half: 20.0,
            angle: 0.0,
            uv: [0.0, 0.0, 1.0, 1.0],
        },
        key: Vec3::new(0.0, 0.0, -3.0),
        color,
        texture: Some(tex),
        additive,
    }]);
    let c = linear(clear);
    r.render(&Camera::default(), &[], [c, c, c, 1.0])
        .expect("render");
    capture_pixel(&mut r, N / 2, N / 2)
}

fn close(px: [u8; 4], want: u8, what: &str) {
    for ch in &px[..3] {
        assert!(ch.abs_diff(want) <= 2, "{what}: {px:?}, want {want}");
    }
}

#[test]
#[ignore = "requires a GPU adapter (run with --ignored on the NVIDIA box)"]
fn a_normal_particle_blends_in_display_bytes() {
    close(
        shot([0, 0, 0, 255], [255, 255, 255, 0], false, 200),
        200,
        "alpha 0 byte",
    );
    close(
        shot([0, 0, 0, 255], [255, 255, 255, 255], false, 200),
        0,
        "opaque black",
    );
    close(
        shot([0, 0, 0, 255], [255, 255, 255, 128], false, 200),
        100,
        "half alpha: 200 · (1 − 0.5)",
    );
    close(
        shot([0, 0, 0, 128], [255, 255, 255, 255], false, 200),
        100,
        "half texel alpha",
    );
    close(
        shot([100, 100, 100, 255], [255, 255, 255, 255], false, 200),
        100,
        "opaque texel at full colour",
    );
    close(
        shot([200, 200, 200, 255], [128, 128, 128, 255], false, 0),
        100,
        "colour 128 halves the texel",
    );
}

#[test]
#[ignore = "requires a GPU adapter (run with --ignored on the NVIDIA box)"]
fn an_additive_particle_adds_in_display_bytes() {
    close(
        shot([255, 255, 255, 255], [255, 255, 255, 128], true, 60),
        188,
        "60 + 255 · 0.5",
    );
    close(
        shot([100, 100, 100, 255], [255, 255, 255, 255], true, 200),
        255,
        "clamped",
    );
}
