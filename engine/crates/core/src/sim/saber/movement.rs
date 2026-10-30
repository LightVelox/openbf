//! Native gravity/target latch before the soldier's ordinary integration.

use super::*;
use crate::combo::Gravity;

/// Before the ordinary 1g step, the weapon supplies its difference or an
/// impulse (B2 240bf8 calls 33c4d0 before the stance mover). Reaching a target resets gravity without
/// clamping velocity; its latch lasts until the next authored state.
fn gravity_velocity(mut velocity: f32, gravity: Gravity, done: &mut bool) -> f32 {
    if gravity.impulse {
        if !*done {
            *done = true;
            if let Some(target) = gravity.target {
                velocity = target;
            }
        }
        return velocity;
    }
    let mut factor = gravity.factor;
    if factor == 1.0 {
        return velocity;
    }
    if let Some(target) = gravity.target {
        if *done || ((factor <= 0.0) != (velocity <= target)) {
            *done = true;
            factor = if gravity.zero_at_target { 0.0 } else { 1.0 };
        }
    }
    velocity - (factor - 1.0) * crate::soldier::GRAVITY * TICK_DT
}

impl Sim {
    /// The stance mover applies drag/gravity and moves from this velocity.
    pub(in crate::sim) fn saber_gravity(&mut self, i: usize) {
        let u = &mut self.units[i];
        if !u.alive || u.mount.is_some() || u.soldier.no_fire() || u.choked.is_some() {
            return;
        }
        let Some(state) = u.weapon.melee.as_mut() else {
            return;
        };
        if let Some(gravity) = state.gravity {
            u.soldier.vel.y = gravity_velocity(u.soldier.vel.y, gravity, &mut state.gravity_done);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gravity(factor: f32, target: Option<f32>) -> Gravity {
        Gravity {
            factor,
            target,
            impulse: false,
            zero_at_target: false,
        }
    }

    #[test]
    fn gravity_target_latches_without_clamping_and_can_reset_to_zero() {
        let mut done = false;
        let g = gravity(2.0, Some(-20.0));
        assert_eq!(gravity_velocity(-5.0, g, &mut done), -5.6);
        assert!(!done);
        assert_eq!(gravity_velocity(-20.1, g, &mut done), -20.1);
        assert!(done);
        assert_eq!(gravity_velocity(5.0, g, &mut done), 5.0);
        let mut done = false;
        let g = Gravity {
            zero_at_target: true,
            ..g
        };
        assert_eq!(gravity_velocity(-20.0, g, &mut done), -19.4);
        assert!(done);
        assert_eq!(gravity_velocity(5.0, g, &mut done), 5.6);
    }

    #[test]
    fn gravity_impulse_is_once_per_state_and_upward_condition_is_strict() {
        let mut done = false;
        let impulse = Gravity {
            impulse: true,
            zero_at_target: true,
            ..gravity(0.3, Some(-2.5))
        };
        assert_eq!(gravity_velocity(-10.0, impulse, &mut done), -2.5);
        assert_eq!(gravity_velocity(-7.0, impulse, &mut done), -7.0);
        let mut done = false;
        let up = gravity(-1.0, Some(3.0));
        assert_eq!(gravity_velocity(3.0, up, &mut done), 4.2);
        assert!(!done);
        assert_eq!(gravity_velocity(3.1, up, &mut done), 3.1);
        assert!(done);
    }

    #[test]
    fn gravity_runs_without_animated_move_and_keeps_ordinary_horizontal_velocity() {
        let mut sim = Sim::empty(42);
        let h = sim
            .spawn(&crate::unit::UnitSpec::player(0, Vec3::ZERO))
            .unwrap();
        let u = sim.unit_mut(h).unwrap();
        u.soldier.vel = Vec3::new(2.0, -5.0, 3.0);
        u.weapon.melee = Some(crate::hero_weapon::MeleeState {
            gravity: Some(gravity(2.0, None)),
            ..Default::default()
        });
        let before = sim.unit(h).unwrap().soldier.pos;
        sim.saber_gravity(h.index as usize);
        let u = sim.unit(h).unwrap();
        assert_eq!(u.soldier.vel, Vec3::new(2.0, -5.6, 3.0));
        assert_eq!(u.soldier.pos, before);
        assert_eq!((u.soldier.pos.x, u.soldier.pos.z), (before.x, before.z));
    }

    #[test]
    fn the_production_step_applies_a_combo_impulse_before_ordinary_air_integration() {
        use crate::sim::saber::tests::{field, program, saber, text};
        use manifest::combo::{ComboKeyword as K, ComboValue as V};
        let mut sim = Sim::empty(93);
        sim.heightfield = Some(crate::sim::tests::flat_ground());
        sim.ground_rules = crate::soldier::GroundRules::Bf2;
        sim.weapon_classes[0] = saber(program(vec![
            field(K::Animation, vec![text("swing")], vec![]),
            field(K::Gravity, vec![V::Number(0.3)], vec![]),
            field(
                K::GravityVelocityTarget,
                vec![V::Number(-2.5), text("Impulse"), text("ZeroGravity")],
                vec![],
            ),
        ]));
        let h = sim
            .spawn(&crate::unit::UnitSpec::player(0, Vec3::Y * 10.0))
            .unwrap();
        let u = sim.unit_mut(h).unwrap();
        // Spawn resolves its floor; put the existing live actor in clear air.
        u.soldier.pos.y = 10.0;
        u.soldier.stance = Stance::Fall;
        u.soldier.grounded = false;
        u.soldier.time_in_stance = 1.0;
        u.soldier.vel = Vec3::Y * -10.0;
        assert!(u.soldier.pos.y > 5.0 && !u.soldier.grounded);
        let constants = u.consts;
        let mut expected = u.soldier.clone();
        expected.vel.y = -2.5;
        let pad = PadState {
            buttons: buttons::FIRE,
            ..Default::default()
        };
        expected.step(TICK_DT, &pad, &constants, crate::soldier::GroundRules::Bf2);
        sim.step(&[pad]);
        let u = sim.unit(h).unwrap();
        assert_eq!(u.soldier.vel.y.to_bits(), expected.vel.y.to_bits());
        assert_eq!(u.soldier.pos.y.to_bits(), expected.pos.y.to_bits());
        assert!(
            u.soldier.vel.y < -2.5,
            "ordinary gravity still follows the impulse"
        );
    }

    #[test]
    fn the_motion_blend_weight_is_future_state_and_changes_the_hash() {
        let mut a = crate::hero_weapon::MeleeState {
            movement_weight: 0.2,
            ..Default::default()
        };
        let mut first = Vec::new();
        a.hash_into(&mut |bytes| first.extend_from_slice(bytes));
        a.movement_weight = 0.3;
        let mut second = Vec::new();
        a.hash_into(&mut |bytes| second.extend_from_slice(bytes));
        assert_ne!(first, second);
    }
}
