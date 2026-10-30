#[test]
fn implicit_from_anim_defaults_follow_the_complete_native_velocity_pack_chain() {
    // Maul jumpattack_end's corrected half-track metadata packs to +40cm/s.
    // Native 30a650: 40 * .01 => 0x3ecccccc; 30a704..30a730:
    // factor 50 * .02 => 0x3f7fffff, product => 0x3ecccccb,
    // *50 => 0x419ffffe, truncate19; runtime19*.02 => 0x3ec28f5b.
    // This fixed instruction-chain oracle precedes the production correction.
    for explicit in [false, true] {
        let movement = if explicit {
            vec![
                f(K::VelocityX, vec![Number(1.0), text("FromAnim")], vec![]),
                f(K::VelocityZ, vec![Number(1.0), text("FromAnim")], vec![]),
            ]
        } else {
            vec![]
        };
        let def = ComboDef {
            name_hash: 1,
            fields: vec![state(
                "IDLE",
                vec![
                    f(K::Animation, vec![text("jumpattack_end")], vec![]),
                    f(K::AnimatedMove, vec![], movement),
                ],
            )],
        };
        for (raw, expected) in [
            ([0.0, f32::from_bits(0x3ecc_cccc)], [0, 0x3ec2_8f5b]),
            ([-f32::from_bits(0x3ecc_cccc), 0.0], [0xbec2_8f5b, 0]),
            // Exact2 discriminates factor50*.02 from division50/50:
            // the native product packs99, whereas factor1 would pack100.
            ([2.0, 0.0], [0x3ffd_70a3, 0]),
        ] {
            let mut runtime = ComboRuntime::default();
            let out = runtime.step_with_animation(&def, &[1.0], input(0), 1.0 / 30.0, |name, _| {
                assert_eq!(name, Some("jumpattack_end"));
                raw
            });
            assert_eq!(out.movement.unwrap().velocity.map(f32::to_bits), expected);
            assert!(out.movement_active);
        }
    }
}

#[test]
fn animated_velocity_literals_use_native_multiplication_at_both_pack_boundaries() {
    // Parser309178 packs authored values with *50/CVT; runtime33c4d0
    // decodes with *.02. A division shortcut changes boundary0.02 and
    // several decoded values even without FromAnim.
    for (authored, bits) in [
        (0.02, 0),
        (4.0, 0x407f_ffff),
        (150.0, 0x4223_c28f),
        (-150.0, 0xc223_d70a),
    ] {
        let def = ComboDef {
            name_hash: 1,
            fields: vec![state(
                "IDLE",
                vec![f(
                    K::AnimatedMove,
                    vec![],
                    vec![f(K::VelocityZ, vec![Number(authored)], vec![])],
                )],
            )],
        };
        let mut runtime = ComboRuntime::default();
        let out = runtime.step(&def, &[1.0], input(0), 0.0);
        assert_eq!(out.movement.unwrap().velocity[1].to_bits(), bits);
    }
}

#[test]
fn native_attack_cache_identity_is_packed_and_uses_absolute_window_end() {
    let mut def = program();
    let a = def
        .fields
        .iter_mut()
        .find(|s| s.text(0) == Some("ATTACK1"))
        .unwrap();
    let hit = a
        .children
        .iter_mut()
        .find(|f| f.keyword == K::Attack)
        .unwrap();
    hit.children
        .push(f(K::AttackId, vec![text("same_combo_attack")], vec![]));
    let mut runtime = ComboRuntime::default();
    runtime.step(&def, &[0.0, 0.0, 1.0, 0.2], input(0), 1.0);
    let out = runtime.step(&def, &[0.0, 0.0, 1.0, 0.2], input(FIRE), 0.1);
    // Move into the actual authored 2..9 frame damage interval.
    let _ = out;
    let out = runtime.step(&def, &[0.0, 0.0, 1.0, 0.2], input(0), 0.1);
    let hit = out.attacks.first().unwrap();
    // Native OR0x20 byte folding also changes underscores; packed low20 bits.
    assert_eq!(hit.id, 0x1d94f);
    assert_eq!(hit.record, 2u64 << 32);
    assert!(
        hit.cache_end > 1.0,
        "window end must be absolute, not reset to state time"
    );
}

#[test]
fn gravity_is_independent_of_animated_move_and_last_authored_impulse_wins() {
    let gravity = f(K::Gravity, vec![Number(0.3)], vec![]);
    let target = f(
        K::GravityVelocityTarget,
        vec![Number(-2.5), text("Impulse"), text("ZeroGravity")],
        vec![],
    );
    for (fields, impulse) in [
        (vec![gravity.clone(), target.clone()], true),
        (vec![target.clone(), gravity.clone()], false),
    ] {
        let def = ComboDef {
            name_hash: 1,
            fields: vec![state("IDLE", fields)],
        };
        let mut runtime = ComboRuntime::default();
        let out = runtime.step(&def, &[1.0], input(0), 0.01);
        assert!(
            out.movement.is_none(),
            "state gravity does not create XZ movement"
        );
        let g = out.gravity.unwrap();
        assert_eq!(g.impulse, impulse);
        assert_eq!(g.factor, 0.3);
        assert_eq!(g.target, Some(-2.5));
        assert!(g.zero_at_target);
    }
    let def = ComboDef {
        name_hash: 1,
        fields: vec![state(
            "IDLE",
            vec![f(K::GravityVelocityTarget, vec![Number(-1000.0)], vec![])],
        )],
    };
    let mut runtime = ComboRuntime::default();
    assert_eq!(
        runtime
            .step(&def, &[1.0], input(0), 0.01)
            .gravity
            .unwrap()
            .target,
        Some(-204.7)
    );
}
