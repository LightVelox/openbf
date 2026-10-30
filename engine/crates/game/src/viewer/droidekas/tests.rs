use super::*;
use core::droideka::{Order, IDLE, LEFT_TO_RIGHT};

fn deka() -> (Droideka, DroidekaClass) {
    let c = DroidekaClass::default();
    (Droideka::new(&c, Order::Roll), c)
}

#[test]
fn a_walking_droideka_plays_its_state_clip_at_its_clip_time() {
    let (mut d, c) = deka();
    d.state = LEFT_TO_RIGHT;
    d.clip_t = 0.25;
    let w = walker_state(&d, &c);
    assert_eq!(w.anim, LEFT_TO_RIGHT);
    assert_eq!(w.clip_time, 0.25);
    assert_eq!(w.h, (1.0, 0.0));
}

#[test]
fn the_roll_clip_runs_one_loop_per_turn_of_the_spin() {
    let (mut d, c) = deka();
    d.state = ROLLING;
    let dur = c.duration[usize::from(ROLLING)];
    for (turn, want) in [(0.0f32, 0.0), (0.25, 0.25), (0.5, 0.5), (1.25, 0.25)] {
        let half = turn * std::f32::consts::PI;
        d.spin = [half.cos(), half.sin()];
        let t = roll_time(&d, &c);
        assert!((t - want * dur).abs() < 1e-3, "{turn}: {t}");
    }
    let half = -0.25 * std::f32::consts::PI;
    d.spin = [half.cos(), half.sin()];
    assert!((roll_time(&d, &c) - 0.75 * dur).abs() < 1e-3);
}

#[test]
fn only_the_ball_leans_and_about_its_own_centre() {
    let (mut d, c) = deka();
    let pos = Vec3::new(3.0, 1.0, -2.0);
    d.state = IDLE;
    d.lean = 0.5;
    let walk = model_matrix(pos, 0.3, &d, &c);
    assert!(walk.abs_diff_eq(
        Mat4::from_translation(pos) * Mat4::from_rotation_y(0.3 + std::f32::consts::PI),
        1e-6
    ));
    d.state = ROLLING;
    let ball = model_matrix(pos, 0.0, &d, &c);
    let centre = ball.transform_point3(Vec3::new(0.0, c.ball_radius, 0.0));
    assert!(centre.abs_diff_eq(pos + Vec3::Y * c.ball_radius, 1e-5));
    let top = ball.transform_point3(Vec3::new(0.0, 2.0 * c.ball_radius, 0.0));
    assert!(
        top.x > pos.x,
        "turning right, the top leans right (+X at yaw 0): {top}"
    );
}

#[test]
fn the_bubble_grows_past_full_then_settles_and_shrinks_away() {
    assert_eq!(bubble_scale(true, 0.0), Some(0.0));
    let peak = bubble_scale(true, 0.94 * BUBBLE_GROW).unwrap();
    assert!((peak - 1.06).abs() < 1e-3, "{peak}");
    assert_eq!(bubble_scale(true, BUBBLE_GROW), Some(1.0));
    assert_eq!(bubble_scale(true, 5.0), Some(1.0));
    assert_eq!(bubble_scale(false, 0.0), Some(1.0));
    let half = bubble_scale(false, 0.5 * BUBBLE_SHRINK).unwrap();
    assert!((half - 0.5).abs() < 1e-5, "{half}");
    assert_eq!(bubble_scale(false, BUBBLE_SHRINK), None);
}

fn bubble_camera() -> Camera {
    crate::viewer::look_at(Vec3::new(0.0, 1.0, 6.0), Vec3::new(0.0, 1.0, 0.0))
}

/// B2 `FUN_0046ce30`: 15 degree segments, 22.5 degree rings, r on the sphere.
#[test]
fn the_bubble_is_the_discs_sphere_turned_to_the_camera() {
    let cam = bubble_camera();
    let centre = Vec3::new(0.0, 1.0, 0.0);
    for (inside, facing) in [(false, 1.0), (true, -1.0)] {
        let tris: Vec<_> = bubble_indices()
            .map(|i| bubble_grid(centre, 1.25, &cam, inside)[i])
            .collect();
        assert_eq!(tris.len(), 24 * 4 * 6);
        for v in &tris {
            assert!(((v.pos - centre).length() - 1.25).abs() < 1e-3);
            assert!((v.normal.length() - 1.0).abs() < 1e-3);
            assert!(
                (v.pos - centre).dot(-cam.forward()) * facing >= -1e-3,
                "inside {inside}: {:?}",
                v.pos
            );
        }
    }
}

/// A close chase camera looking past the bubble: no ring folds over the far
/// side, and the rim's full ring lies on the sphere's true silhouette.
#[test]
fn a_close_camera_sees_the_rim_on_the_silhouette() {
    let centre = Vec3::new(0.0, 1.0, 0.0);
    let cam = crate::viewer::look_at(Vec3::new(0.0, 2.4, 2.6), Vec3::new(0.0, 2.2, -6.0));
    let grid = bubble_grid(centre, 1.25, &cam, false);
    let to_eye = |v: &BubbleVertex| (cam.position - v.pos).normalize();
    for v in &grid {
        assert!(v.normal.dot(to_eye(v)) >= -1e-3, "{:?}", v.pos);
    }
    for v in grid.iter().filter(|v| rim(v.theta) > 0.999) {
        assert!(v.normal.dot(to_eye(v)).abs() < 1e-3, "{:?}", v.pos);
    }
}

/// The rim's vertex alpha at the disc's rings: sin^6, so the disc's centre is
/// clear and its edge is full.
#[test]
fn the_bubble_rim_is_clear_at_the_centre_and_full_at_the_edge() {
    let at = |ring: f32| rim(ring * std::f32::consts::PI / 8.0);
    assert_eq!(at(0.0), 0.0);
    assert!((at(1.0) - 0.003).abs() < 1e-3);
    assert!((at(2.0) - 0.125).abs() < 1e-3);
    assert!((at(3.0) - 0.62).abs() < 0.01);
    assert!((at(4.0) - 1.0).abs() < 1e-6);
}

/// The look reads no charge: full and nearly spent draw the same.
#[test]
fn the_bubble_does_not_depend_on_the_charge() {
    let mut s = core::sim::Sim::empty(1);
    let class = std::sync::Arc::new(DroidekaClass::default());
    let spec = core::unit::UnitSpec {
        droideka: Some(class),
        ..core::unit::UnitSpec::trooper(1, Vec3::ZERO)
    };
    let h = s.spawn(&spec).unwrap();
    let d = s.units[h.index as usize].droideka.as_mut().unwrap();
    d.shield.on = true;
    let mut vis = DroidekaVisuals {
        bubble_color: bubble_color("bf2"),
        ..DroidekaVisuals::default()
    };
    let cam = bubble_camera();
    let full = vis.bubbles(&s, &cam, Vec3::NEG_Y);
    assert_eq!(full.len(), 1);
    assert!(matches!(full[0].shape, ParticleShape::Shaded { .. }));
    s.units[h.index as usize]
        .droideka
        .as_mut()
        .unwrap()
        .shield
        .points = 1.0;
    assert_eq!(vis.bubbles(&s, &cam, Vec3::NEG_Y), full);
}

#[test]
fn the_bubble_colour_is_the_disc_config_per_game() {
    assert_eq!(bubble_color("bf1"), [96, 160, 192, 192]);
    assert_eq!(bubble_color("BF2"), [96, 160, 192, 255]);
}
