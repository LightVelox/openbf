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

#[test]
fn the_bubble_is_a_shell_facing_the_eye() {
    let centre = Vec3::new(1.0, 2.0, 3.0);
    let eye = centre + Vec3::new(0.0, 0.0, 10.0);
    let tris = bubble_tris(centre, 1.25, eye);
    assert_eq!(tris.len(), BUBBLE_RINGS * BUBBLE_SEGMENTS * 6);
    for (p, _) in &tris {
        assert!(((*p - centre).length() - 1.25).abs() < 1e-4);
        assert!((*p - centre).z >= -1e-4, "{p}");
    }
}

#[test]
fn the_bubble_colour_is_the_disc_config_per_game() {
    assert_eq!(bubble_color("bf1"), [96, 160, 192, 192]);
    assert_eq!(bubble_color("BF2"), [96, 160, 192, 255]);
}
