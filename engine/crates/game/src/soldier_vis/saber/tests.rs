use super::*;
use glam::Vec3;

#[test]
fn saber_draw_preserves_native_white_and_taper_without_a_second_halving() {
    let segment = core::hero_weapon::BladeSegment {
        start: Vec3::ZERO,
        end: Vec3::Y * 1.2,
    };
    let d = blade_particle(segment, 0.08, None);
    assert_eq!(d.color, [255; 4]);
    assert!(d.additive);
    let ParticleShape::CappedRibbon {
        head,
        head_half,
        tail,
        tail_half,
    } = d.shape
    else {
        panic!("native capped ribbon required");
    };
    assert_eq!(head, segment.end);
    assert!((head_half - 0.064).abs() < 1e-6);
    assert!((tail_half - 0.08).abs() < 1e-6);
    assert!(tail.abs_diff_eq(Vec3::Y * -0.048, 1e-6));
}

#[test]
fn native_saber_glow_has_its_own_alpha_and_wider_ribbon() {
    let segment = core::hero_weapon::BladeSegment {
        start: glam::Vec3::ZERO,
        end: glam::Vec3::Y,
    };
    let rng = Cell::new(0);
    let d = glow_particle(segment, 0.08, None, &rng);
    assert_eq!(d.color, [255, 255, 255, 51]);
    assert!(d.additive);
    let ParticleShape::CappedRibbon {
        head_half,
        tail,
        tail_half,
        ..
    } = d.shape
    else {
        panic!("native glow ribbon required");
    };
    assert_eq!(tail, segment.start, "glow does not extend behind the root");
    assert!((0.12..0.20).contains(&tail_half));
    assert!((head_half - tail_half * 0.8).abs() < 1e-6);
    assert_eq!(rng.get(), 1196435762, "two native LCG advances per glow");
}

mod flight;
mod visibility;
