use super::*;
use crate::sim::saber::deflect::selection::{direction_mask, reflected_origin};
use glam::Mat4;

#[test]
fn block_buckets_use_motion_with_backwards_axis_reversed_and_ordered_fallbacks() {
    let facing = Vec3::NEG_Z;
    assert_eq!(direction_mask(63, Vec3::Z, facing, Vec3::ZERO), 1);
    assert_eq!(direction_mask(63, Vec3::X, facing, Vec3::ZERO), 16);
    assert_eq!(direction_mask(63, Vec3::NEG_X, facing, Vec3::ZERO), 4);
    let diagonal = Vec3::new(1.0, 0.0, -1.0);
    assert_eq!(direction_mask(63, Vec3::Z, facing, diagonal), 8);
    assert_eq!(direction_mask(63, Vec3::Z, facing, -diagonal), 8);
    assert_eq!(direction_mask(8, Vec3::NEG_X, facing, Vec3::ZERO) & 8, 8);
    assert_eq!(direction_mask(63, Vec3::Z, facing, diagonal * 0.1), 1);
}

#[test]
fn ordinary_reflection_consumes_four_lcg_steps_and_beam_uses_its_address_stream() {
    for beam in [false, true] {
        let (mut sim, defender, owner) = scene();
        sim.weapon_classes[1].ordnance.kind = if beam {
            crate::weapon::OrdKind::Beam
        } else {
            crate::weapon::OrdKind::Other
        };
        sim.unit_mut(defender)
            .unwrap()
            .weapon
            .melee
            .as_mut()
            .unwrap()
            .deflect_precision = 2.0;
        let gun_rng = sim.lcg.state();
        let mut expected = sim.melee_lcg.clone();
        if !beam {
            expected.unit();
            expected.unit();
        }
        let bolt = impact(&sim, defender, owner, Vec3::Z);
        assert!(sim.deflect_bolt(defender, &bolt));
        assert_eq!(
            sim.lcg.state(),
            gun_rng,
            "native d870 is a different stream"
        );
        assert_eq!(sim.melee_lcg.state(), expected.state());
        let precision = sim
            .unit(defender)
            .unwrap()
            .weapon
            .melee
            .as_ref()
            .unwrap()
            .deflect_precision;
        assert_eq!(precision, if beam { 1.0 - 2.0 * TICK_DT } else { 0.0 });
    }
}

#[test]
fn the_melee_stream_is_hashed_before_its_first_draw_and_retained_after_death() {
    let mut ordinary = Sim::empty(123);
    let before = ordinary.hash();
    ordinary.melee_lcg.step();
    assert_eq!(ordinary.hash(), before, "no-saber legacy control");
    ordinary.weapon_classes.push(saber(program(vec![])));
    let h = ordinary
        .spawn(&UnitSpec {
            weapon_class: 1,
            ..UnitSpec::trooper(0, Vec3::ZERO)
        })
        .unwrap();
    assert!(ordinary.melee_lcg_active);
    let hash = ordinary.hash();
    ordinary.melee_lcg.step();
    assert_ne!(
        ordinary.hash(),
        hash,
        "unused activated RNG is future state"
    );
    ordinary.unit_mut(h).unwrap().alive = false;
    let hash = ordinary.hash();
    ordinary.melee_lcg.step();
    assert_ne!(ordinary.hash(), hash, "retirement cannot erase future RNG");
}

fn block_pose(sim: &mut Sim) {
    let class = sim.weapon_classes[0].saber.as_mut().unwrap();
    class.combo.fields[1].children.push(field(
        K::Deflect,
        vec![],
        vec![field(
            K::DeflectAnimation,
            vec![text("block"), text("Forward")],
            vec![],
        )],
    ));
    let mut clip = class.clips[0].clone();
    clip.name = "block".into();
    clip.weapon_frames = vec![Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)); 4];
    class.clips.push(clip);
    class.deflect_edges = vec![BladeSegment {
        start: Vec3::ZERO,
        end: Vec3::Z * 2.0,
    }];
}

#[test]
fn the_selected_block_pose_places_the_reflected_origin_on_the_cached_primary_edge() {
    let (mut sim, defender, owner) = scene();
    block_pose(&mut sim);
    sim.unit_mut(defender)
        .unwrap()
        .weapon
        .melee
        .as_mut()
        .unwrap()
        .runtime
        .state = 1;
    let mut bolt = impact(&sim, defender, owner, Vec3::Z);
    bolt.pos = Vec3::new(-1.0, 2.0, -3.0);
    assert!(sim.deflect_bolt(defender, &bolt));
    assert_eq!(sim.bolts.last().unwrap().pos, Vec3::new(-1.0, 2.0, -4.0));
    let state = sim.unit(defender).unwrap().weapon.melee.as_ref().unwrap();
    assert_eq!(state.pose(), Some(("block", 0.0, false)));
    let class = sim.weapon_classes[0].saber.as_ref().unwrap();
    assert_eq!(
        reflected_origin(class, None, Mat4::IDENTITY, bolt.pos),
        bolt.pos
    );
    assert_eq!(
        class.deflect_attachment("block"),
        Some(Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)))
    );
}

#[test]
fn cached_origin_uses_the_native_transpose_even_for_a_nonunit_basis() {
    let (mut sim, _, _) = scene();
    block_pose(&mut sim);
    let class = sim.weapon_classes[0].saber.as_ref().unwrap();
    let basis = Mat4::from_scale(Vec3::new(1.0, 1.0, 2.0));
    let impact = Vec3::new(1.0, 2.0, 7.0);
    // Cache translation becomes (1,2,6). Native dot gives local z=2,
    // selecting the end of the two-unit edge, hence world z=10.
    assert_eq!(
        reflected_origin(class, Some("block"), basis, impact),
        Vec3::new(1.0, 2.0, 10.0)
    );
    // A general inverse gives local z=.5 and instead produces world z=8.5.
    let world = basis * class.deflect_attachment("block").unwrap();
    assert_eq!(world.inverse().transform_point3(impact).z, 0.5);
}

#[test]
fn block_precision_entry_update_exit_and_pose_future_fields_are_distinct() {
    let mut state = MeleeState::default();
    state.advance_deflect(true, true, TICK_DT);
    assert_eq!(state.deflect_precision, 1.0 + TICK_DT);
    state.select_block_animation(Some("block"));
    state.deflect_precision = 0.0;
    state.advance_deflect(false, true, TICK_DT);
    assert_eq!(state.pose(), Some(("block", TICK_DT, false)));
    let bytes = |s: &MeleeState| {
        let mut b = Vec::new();
        s.hash_into(&mut |v| b.extend_from_slice(v));
        b
    };
    let before = bytes(&state);
    for changed in 0..3 {
        let mut next = state.clone();
        match changed {
            0 => next.block_animation = Some("other".into()),
            1 => next.block_animation_time += 0.1,
            _ => next.previous_pitch = 0.5,
        }
        assert_ne!(before, bytes(&next));
    }
    state.advance_deflect(false, true, 1.0);
    assert!(state.block_animation.is_none());
    state.advance_deflect(true, false, TICK_DT);
    assert_eq!(state.deflect_precision, 0.0);
}

#[test]
fn off_axis_hand_contact_blocks_by_the_attack_basis_and_does_not_set_attacker_precision() {
    for defender_yaw in [0.0, std::f32::consts::PI] {
        let (mut sim, defender, attacker) = scene();
        let class = sim.weapon_classes[0].saber.as_mut().unwrap();
        for frame in &mut class.clips[0].frames {
            frame[0].start.x = -1.0;
            frame[0].end.x = -1.0;
        }
        let victim = sim.unit_mut(defender).unwrap();
        victim.soldier.pos = Vec3::new(1.0, 0.0, -0.5);
        victim.soldier.yaw = defender_yaw;
        let deflect = victim
            .weapon
            .melee
            .as_mut()
            .unwrap()
            .deflect
            .as_mut()
            .unwrap();
        deflect.min_angle = -30.0;
        deflect.max_angle = 30.0;
        let a = sim.unit_mut(attacker).unwrap();
        a.weapon_class = 0;
        a.soldier.pos = Vec3::ZERO;
        a.soldier.yaw = 0.0;
        a.weapon.melee = Some(MeleeState {
            animation: Some("swing".into()),
            previous_position: Vec3::ZERO,
            attacks: vec![combo::AttackWindow {
                index: 0,
                id: 0,
                record: 1,
                cache_end: 1.0,
                edge: 0,
                damage: Some(100.0),
                length: None,
                length_from_edge: false,
                width: None,
                width_from_edge: false,
                push: 0.0,
                play_explosion: false,
            }],
            ..Default::default()
        });
        let before = sim.unit(defender).unwrap().health.cur;
        sim.saber_hits();
        let u = sim.unit(defender).unwrap();
        assert_eq!(
            u.health.cur,
            if defender_yaw == 0.0 {
                before - 100.0
            } else {
                before
            }
        );
        assert_eq!(
            u.soldier.energy,
            if defender_yaw == 0.0 { 100.0 } else { 97.5 }
        );
        assert_eq!(
            sim.unit(attacker)
                .unwrap()
                .weapon
                .melee
                .as_ref()
                .unwrap()
                .deflect_precision,
            0.0
        );
    }
}
