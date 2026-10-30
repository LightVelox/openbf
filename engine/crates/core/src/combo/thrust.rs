//! Retained-input rotation in FUN_0033f600, with its literal 45-degree caps.

use glam::Vec3;

// FUN_0033f600 materializes bits 0x3f350481 at 33f6b4..33f6ec.
// The native decimal cap differs from FRAC_1_SQRT_2 and must stay exact.
#[allow(clippy::approx_constant)]
pub(crate) const RETAINED_CAP: f32 = 0.7071;

pub(crate) fn orient(
    forward: Vec3,
    retained: [f32; 2],
    current_forward: f32,
    inverse_input: bool,
) -> Vec3 {
    let [mut z, mut x] = retained;
    if x * x + z * z < 0.09 {
        return forward;
    }
    if !inverse_input {
        x = -x;
    }
    if current_forward < 0.0 {
        x = -x;
        z = -z;
    }
    if z <= x {
        x = RETAINED_CAP;
        z = RETAINED_CAP;
    } else if z <= -x {
        z = RETAINED_CAP;
        x = -RETAINED_CAP;
    } else {
        let inverse = 1.0 / (x * x + z * z).sqrt();
        x *= inverse;
        z *= inverse;
    }
    Vec3::new(
        forward.x * z + forward.z * x,
        forward.y,
        forward.z * z - forward.x * x,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use manifest::combo::{ComboDef, ComboField as F, ComboKeyword as K, ComboValue as V};
    fn field(keyword: K, args: Vec<V>, children: Vec<F>) -> F {
        F {
            keyword,
            args,
            scoped: !children.is_empty(),
            children,
        }
    }
    fn text(value: &str) -> V {
        V::Text(value.into())
    }

    #[test]
    fn physical_native_axes_rotate_the_cached_pose_and_keep_the_literal_cap() {
        let forward = Vec3::NEG_Z;
        assert_eq!(orient(forward, [0.0, 0.0], 0.0, false), forward);
        assert_eq!(
            orient(forward, [0.0, 1.0], 0.0, false),
            Vec3::new(RETAINED_CAP, 0.0, -RETAINED_CAP)
        );
        assert_eq!(
            orient(forward, [0.0, -1.0], 0.0, false),
            Vec3::new(-RETAINED_CAP, 0.0, -RETAINED_CAP)
        );
        assert_eq!(
            orient(forward, [-1.0, 1.0], -1.0, false),
            Vec3::new(-RETAINED_CAP, 0.0, -RETAINED_CAP)
        );
        assert_eq!(orient(forward, [0.1, 0.1], 0.0, false), forward);
    }

    #[test]
    fn initial_uses_the_previous_states_retained_pair_and_hashes_it_before_use() {
        use crate::hero_weapon::MeleeState;
        use manifest::combo::ComboKeyword as K;
        let combo = ComboDef {
            name_hash: 0,
            fields: vec![
                field(K::State, vec![text("IDLE")], vec![]),
                field(
                    K::State,
                    vec![text("ATTACK")],
                    vec![field(K::AlignedToThrust, vec![text("Initial")], vec![])],
                ),
            ],
        };
        let mut state = MeleeState::default();
        state.runtime.state = 0;
        state.remember_thrust(&combo, 1.0, 0.0);
        state.runtime.state = 1;
        state.remember_thrust(&combo, 0.0, 1.0);
        assert_eq!(state.retained_thrust, [1.0, 0.0]);
        assert_eq!(state.current_thrust_forward, 0.0);
        let hash = |state: &MeleeState| {
            let mut bytes = Vec::new();
            state.hash_into(&mut |b| bytes.extend_from_slice(b));
            bytes
        };
        let before = hash(&state);
        for i in 0..3 {
            let mut changed = state.clone();
            if i < 2 {
                changed.retained_thrust[i] += 0.5
            } else {
                changed.current_thrust_forward = -1.0
            }
            assert_ne!(hash(&changed), before);
        }
        state.runtime.state = 0;
        state.remember_thrust(&combo, -0.5, 0.5);
        assert_eq!(state.retained_thrust, [-0.5, 0.5]);
    }

    #[test]
    fn aligned_motion_rotates_input_then_scales_axes_then_rotates_the_target() {
        use crate::combo::{ComboInput, ComboRuntime};
        use manifest::combo::{ComboDef, ComboKeyword as K, ComboValue as V};
        for aligned in [false, true] {
            let mut fields = vec![field(
                K::AnimatedMove,
                vec![],
                vec![
                    field(K::VelocityX, vec![V::Number(0.0)], vec![]),
                    field(K::VelocityZ, vec![V::Number(4.0)], vec![]),
                    field(K::VelocityFromStrafe, vec![V::Number(2.0)], vec![]),
                    field(K::VelocityFromThrust, vec![V::Number(6.0)], vec![]),
                ],
            )];
            if aligned {
                fields.push(field(K::AlignedToThrust, vec![V::Number(1.0)], vec![]));
            }
            let def = ComboDef {
                name_hash: 0,
                fields: vec![field(K::State, vec![text("IDLE")], fields)],
            };
            let mut runtime = ComboRuntime::default();
            let out = runtime.step(
                &def,
                &[1.0],
                ComboInput {
                    strafe: -1.0,
                    retained_thrust: [0.0, 1.0],
                    ..Default::default()
                },
                1.0 / 30.0,
            );
            let velocity = out.movement.unwrap().velocity;
            if aligned {
                // Independent scalar f32 calculation of the native two-stage law.
                assert_eq!(velocity.map(f32::to_bits), [0xc0da_819f, 0x409a_81ef]);
            } else {
                assert_eq!(
                    velocity.map(f32::to_bits),
                    [0xc000_0000, 0x407f_ffff],
                    "the existing artist-X strafe conversion is preserved"
                );
            }
        }
    }
}
