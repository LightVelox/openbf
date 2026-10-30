use super::*;

#[test]
fn angular_endpoint_matches_the_saved_ps2_yoda_trail() {
    let mut h = History::default();
    h.append(
        f32::from_bits(0x3f111110),
        Vec3::new(
            f32::from_bits(0x41f4ffe8),
            f32::from_bits(0x4043ff19),
            f32::from_bits(0xc30236e4),
        ),
        Vec3::new(
            f32::from_bits(0xbf22bc23),
            f32::from_bits(0x3f0f433a),
            f32::from_bits(0xbf081f15),
        ),
        false,
    );
    h.append(
        f32::from_bits(0x3f199998),
        Vec3::new(
            f32::from_bits(0x41f6ec89),
            f32::from_bits(0x403761d2),
            f32::from_bits(0xc301fca2),
        ),
        Vec3::new(
            f32::from_bits(0xbca60c60),
            f32::from_bits(0x3e8c82dd),
            f32::from_bits(0xbf761db8),
        ),
        true,
    );
    let expected = Vec3::new(
        f32::from_bits(0x41aa2248),
        f32::from_bits(0xc14c6bf3),
        f32::from_bits(0xc0830ddc),
    );
    assert!(h
        .samples
        .back()
        .unwrap()
        .angular
        .abs_diff_eq(expected, 0.001));
}

#[test]
fn history_is_eight_frames_and_fades_after_the_native_120ms_lifetime() {
    let mut h = History::default();
    for k in 0..20 {
        let p = Vec3::new(k as f32 * 0.2, 0.0, 0.0);
        h.step(
            1.0 / 120.0,
            Some((p, p + Vec3::Y, 0.08, [80, 255, 7, 128])),
            true,
        );
        assert!(h.samples.len() <= 8);
    }
    assert_eq!(h.samples.len(), 8);
    assert!(
        h.sections().len() > 8,
        "native adaptive Hermite subdivisions"
    );
    h.step(0.05, None, false);
    assert!(
        !h.sections().is_empty(),
        "last swipe outlives a single fixed tick"
    );
    assert!(h.sections().iter().any(|s| s.3[3] > 0));
    h.step(LIFE, None, false);
    assert!(h.sections().is_empty());
}

#[test]
fn hermite_keeps_endpoint_tangents_and_does_not_reduce_to_a_linear_sweep() {
    let p = |u| hermite(Vec3::ZERO, Vec3::X, Vec3::ZERO, Vec3::ZERO, 1.0, u);
    assert_eq!(p(0.0), Vec3::ZERO);
    assert_eq!(p(1.0), Vec3::X);
    assert!(p(0.25).abs_diff_eq(Vec3::X * 0.15625, 1e-6));
    let tangent = (p(0.0001) - p(0.0)) / 0.0001;
    assert!(tangent.length() < 0.001);
}

#[test]
fn native_positive_alpha_conversion_truncates_the_fractional_byte() {
    let mut h = History {
        clock: 0.07,
        length: 1.0,
        color: [255; 4],
        ..Default::default()
    };
    h.append(0.0, Vec3::ZERO, Vec3::Y, false);
    h.append(0.02, Vec3::X, Vec3::Y, true);
    // age .05: ((.12-.05)/(.12-.02))²*255 = 124.95, not125.
    assert_eq!(h.sections().last().unwrap().3[3], 124);
}
