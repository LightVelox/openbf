//! The weapon callback supplies desired movement before ordinary integration.

use super::*;

fn motion(forward: f32, strafe: f32, weight: f32) -> Option<MotionTarget> {
    Some(MotionTarget {
        forward,
        strafe,
        weight,
    })
}

fn bytes(soldier: &Soldier) -> Vec<u8> {
    let mut bytes = Vec::new();
    soldier.hash_into(&mut |part| bytes.extend_from_slice(part));
    bytes
}

#[test]
fn motion_target_obeys_ground_acceleration_instead_of_setting_velocity() {
    let mut s = Soldier::new(Vec3::ZERO);
    let c = SoldierConstants {
        acceleration: 6.0,
        motion_target: motion(20.0, -20.0, 1.0),
        ..Default::default()
    };
    tick(&mut s, &PadState::default(), &c);
    assert!((s.vel.z + 0.2).abs() < 1e-6);
    assert!((s.vel.x + 0.2).abs() < 1e-6);
    assert!((s.pos.z + 0.2 * DT).abs() < 1e-6);
    assert!((s.pos.x + 0.2 * DT).abs() < 1e-6);
}

#[test]
fn motion_target_blends_local_desired_ground_speed_before_the_mover() {
    let mut s = Soldier::new(Vec3::ZERO);
    let c = SoldierConstants {
        acceleration: 600.0,
        motion_target: motion(1.0, -3.0, 0.5),
        ..Default::default()
    };
    let pad = PadState {
        ly: i16::MAX,
        lx: i16::MAX,
        ..Default::default()
    };
    tick(&mut s, &pad, &c);
    assert!((s.vel.z + 5.0).abs() < 1e-6);
    assert!((s.vel.x - 2.0).abs() < 1e-6);
}

#[test]
fn motion_target_is_air_acceleration_and_preserves_normal_drag_and_gravity() {
    let mut s = Soldier::new(Vec3::Y * 100.0);
    s.stance = Stance::Fall;
    s.time_in_stance = 1.0;
    s.grounded = false;
    s.air_time = 1.0;
    s.vel = Vec3::new(3.0, 8.0, 4.0);
    let c = SoldierConstants {
        motion_target: motion(12.0, -6.0, 0.5),
        ..Default::default()
    };
    let initial = s.vel;
    let drag = fmath::exp(-AIR_DRAG * initial.length() * DT);
    s.step(DT, &full_fwd(), &c, GroundRules::Bf2);
    let expected = initial * drag + Vec3::new(-3.0, -GRAVITY, -6.0) * DT;
    assert!(
        (s.vel - expected).length() < 1e-6,
        "{} != {}",
        s.vel,
        expected
    );
    assert!((s.pos - (Vec3::Y * 100.0 + expected * DT)).length() < 1e-6);
}

#[test]
fn motion_target_zero_weight_and_unconsumed_stances_keep_the_same_state_hash() {
    for (rules, stance, weight) in [
        (GroundRules::Bf2, Stance::Stand, 0.0),
        (GroundRules::Bf1, Stance::Stand, 1.0),
        (GroundRules::Bf2, Stance::Roll, 1.0),
        (GroundRules::Bf2, Stance::Jet, 1.0),
        (GroundRules::Bf2, Stance::Hover, 1.0),
    ] {
        let mut a = Soldier::new(Vec3::Y * 10.0);
        a.stance = stance;
        a.fuel = 1.0;
        a.time_in_stance = 0.1;
        a.grounded = !matches!(stance, Stance::Jet | Stance::Hover);
        let mut b = a.clone();
        let c = SoldierConstants::default();
        let changed = SoldierConstants {
            motion_target: motion(50.0, -50.0, weight),
            ..c
        };
        a.step(DT, &full_fwd(), &c, rules);
        b.step(DT, &full_fwd(), &changed, rules);
        assert_eq!(bytes(&a), bytes(&b), "{rules:?} {stance:?} weight{weight}");
    }
}
