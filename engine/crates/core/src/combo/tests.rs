use super::*;
use manifest::combo::ComboValue::{Number, Text};

fn f(keyword: K, args: Vec<manifest::combo::ComboValue>, children: Vec<Field>) -> Field {
    Field {
        keyword,
        args,
        scoped: !children.is_empty(),
        children,
    }
}
fn text(s: &str) -> manifest::combo::ComboValue {
    Text(s.into())
}
fn state(name: &str, children: Vec<Field>) -> Field {
    f(K::State, vec![text(name)], children)
}
fn group(children: Vec<Field>) -> Field {
    f(K::If, vec![], children)
}
fn transition(name: &str, children: Vec<Field>) -> Field {
    f(K::Transition, vec![text(name)], children)
}
fn button(name: &str) -> Field {
    f(K::Button, vec![text(name), text("Press")], vec![])
}
fn immediate(name: &str, button_name: &str) -> Field {
    transition(
        name,
        vec![group(vec![
            f(K::Break, vec![], vec![]),
            button(button_name),
        ])],
    )
}
fn program() -> ComboDef {
    ComboDef {
        name_hash: 1,
        fields: vec![
            state(
                "IDLE",
                vec![
                    f(K::Duration, vec![Number(0.0)], vec![]),
                    immediate("DEFLECT", "FireSecondary"),
                    immediate("ATTACK1", "Fire"),
                ],
            ),
            state(
                "DEFLECT",
                vec![
                    f(K::Duration, vec![Number(0.0)], vec![]),
                    f(K::EnergyRestoreRate, vec![Number(-10.0)], vec![]),
                    f(
                        K::InputLock,
                        vec![Number(0.0), text("All"), text("!Thrust")],
                        vec![],
                    ),
                    f(
                        K::Deflect,
                        vec![],
                        vec![
                            f(K::DeflectAngle, vec![Number(-110.0), Number(110.0)], vec![]),
                            f(K::EnergyCost, vec![Number(2.5)], vec![]),
                        ],
                    ),
                    immediate("ATTACK1", "Fire"),
                ],
            ),
            state(
                "ATTACK1",
                vec![
                    f(K::Animation, vec![text("stand_attack1a")], vec![]),
                    f(K::Sound, vec![text("saber_swing")], vec![]),
                    f(K::InputLock, vec![text("All"), text("!Thrust")], vec![]),
                    f(
                        K::Attack,
                        vec![],
                        vec![
                            f(
                                K::DamageTime,
                                vec![Number(2.0), Number(9.0), text("Frames")],
                                vec![],
                            ),
                            f(K::Damage, vec![Number(300.0)], vec![]),
                        ],
                    ),
                    transition(
                        "ATTACK2",
                        vec![group(vec![
                            f(K::TimeStart, vec![Number(4.0), text("Frames")], vec![]),
                            f(K::TimeEnd, vec![Number(14.0), text("Frames")], vec![]),
                            button("Fire"),
                        ])],
                    ),
                    transition("RECOVER1", vec![]),
                ],
            ),
            state(
                "ATTACK2",
                vec![
                    f(K::Animation, vec![text("stand_attack1b")], vec![]),
                    transition("IDLE", vec![]),
                ],
            ),
            state(
                "RECOVER1",
                vec![
                    f(K::Animation, vec![text("stand_attack1a_end")], vec![]),
                    transition("IDLE", vec![]),
                ],
            ),
        ],
    }
}
fn input(buttons: u16) -> ComboInput {
    ComboInput {
        buttons,
        posture: STAND,
        energy: 100.0,
        ..ComboInput::default()
    }
}
const CLIPS: &[f32] = &[0.0, 0.0, 0.5, 0.5, 0.5];

#[test]
fn fire_does_not_take_the_first_deflect_transition() {
    let def = program();
    let mut r = ComboRuntime::default();
    r.step(&def, CLIPS, input(0), 0.0);
    let out = r.step(&def, CLIPS, input(FIRE), 1.0 / 60.0);
    assert_eq!(out.state_name, "ATTACK1");
    assert_eq!(out.animation, Some("stand_attack1a"));
    assert_eq!(out.sound, Some("saber_swing"));
    assert_eq!(out.input_lock & (THRUST | STRAFE), 0);
    assert_ne!(out.input_lock & JUMP, 0);
    let next = r.step(&def, CLIPS, input(0), 1.0 / 60.0);
    assert_eq!(next.sound, None);
}

#[test]
fn first_tick_fire_is_not_dropped_when_the_combo_initializes() {
    let def = program();
    let mut r = ComboRuntime::default();
    assert_eq!(
        r.step(&def, CLIPS, input(FIRE), 1.0 / 60.0).state_name,
        "ATTACK1"
    );
}

#[test]
fn destination_animation_motion_is_sampled_on_the_transition_tick() {
    let mut def = program();
    def.fields[2]
        .children
        .push(f(K::AnimatedMove, vec![], vec![]));
    let mut runtime = ComboRuntime::default();
    let out = runtime.step_with_animation(&def, CLIPS, input(FIRE), 1.0 / 30.0, |name, time| {
        assert_eq!(name, Some("stand_attack1a"));
        assert_eq!(time, 1.0 / 30.0);
        [1.25, -2.5]
    });
    assert_eq!(out.state_name, "ATTACK1");
    // Packed50 defaults use the loader's EE multiply/repack/decode chain.
    assert_eq!(
        out.movement.unwrap().velocity.map(f32::to_bits),
        [0x3f9e_b851, 0xc01e_b851]
    );
}

#[test]
fn native_posture_mask_interrupts_actions_without_forcing_stances() {
    assert_eq!((ROLL, JET), (0x20, 0x40));
    let mut def = program();
    def.fields[2]
        .children
        .push(f(K::Posture, vec![text("Stand")], vec![]));
    for (posture, expected) in [
        (STAND, "ATTACK1"),
        (POSTURE_CROUCH, "ATTACK1"),
        (PRONE, "ATTACK1"),
        (POSTURE_SPRINT, "ATTACK1"),
        (POSTURE_JUMP, "IDLE"),
        (ROLL, "IDLE"),
        (JET, "IDLE"),
        (TUMBLE, "IDLE"),
    ] {
        let mut runtime = ComboRuntime::default();
        let out = runtime.step(
            &def,
            CLIPS,
            ComboInput {
                posture,
                ..input(FIRE)
            },
            0.0,
        );
        assert_eq!(out.state_name, expected, "posture {posture:#x}");
    }
    // Native mask0x50 permits the other air posture to finish its action.
    def.fields[2]
        .children
        .last_mut()
        .unwrap()
        .args
        .push(text("Jump"));
    let mut runtime = ComboRuntime::default();
    assert_eq!(
        runtime
            .step(
                &def,
                CLIPS,
                ComboInput {
                    posture: JET,
                    ..input(FIRE)
                },
                0.0
            )
            .state_name,
        "ATTACK1"
    );
    let mut runtime = ComboRuntime::default();
    assert_eq!(
        runtime
            .step(
                &def,
                CLIPS,
                ComboInput {
                    posture: ROLL,
                    ..input(FIRE)
                },
                0.0
            )
            .state_name,
        "IDLE"
    );
}

#[test]
fn attack_window_uses_the_disc_frames_and_damage() {
    let def = program();
    let mut r = ComboRuntime::default();
    r.step(&def, CLIPS, input(0), 0.0);
    r.step(&def, CLIPS, input(FIRE), 0.0);
    assert!(r.step(&def, CLIPS, input(0), 1.0 / 30.0).attacks.is_empty());
    let out = r.step(&def, CLIPS, input(0), 1.0 / 30.0);
    assert_eq!(out.attacks.len(), 1);
    assert_eq!(out.attacks[0].damage, Some(300.0));
    assert_eq!(out.attacks[0].edge, 0);
    assert!(r.step(&def, CLIPS, input(0), 0.3).attacks.len() == 1);
    assert!(r.step(&def, CLIPS, input(0), 0.01).attacks.is_empty());
}

#[test]
fn damage_distance_is_the_ps2_length_alias_with_last_write_winning() {
    let mut def = program();
    let attack = def.fields[2]
        .children
        .iter_mut()
        .find(|f| f.keyword == K::Attack)
        .unwrap();
    attack
        .children
        .push(f(K::DamageLength, vec![Number(2.0)], vec![]));
    attack
        .children
        .push(f(K::DamageDistance, vec![Number(-1.5)], vec![]));
    let mut runtime = ComboRuntime::default();
    runtime.step(&def, CLIPS, input(FIRE), 0.0);
    let out = runtime.step(&def, CLIPS, input(0), 0.1);
    assert_eq!(out.attacks[0].length, Some(1.5));
    assert!(out.attacks[0].length_from_edge);
    let attack = def.fields[2]
        .children
        .iter_mut()
        .find(|f| f.keyword == K::Attack)
        .unwrap();
    attack
        .children
        .push(f(K::DamageLength, vec![Number(0.75)], vec![]));
    let mut runtime = ComboRuntime::default();
    runtime.step(&def, CLIPS, input(FIRE), 0.0);
    let out = runtime.step(&def, CLIPS, input(0), 0.1);
    assert_eq!(out.attacks[0].length, Some(0.75));
    assert!(!out.attacks[0].length_from_edge);
}

#[test]
fn block_angle_energy_and_rate_are_distinct_outputs() {
    let def = program();
    let mut r = ComboRuntime::default();
    r.step(&def, CLIPS, input(0), 0.0);
    let out = r.step(&def, CLIPS, input(FIRE_SECONDARY), 1.0 / 60.0);
    assert_eq!(out.state_name, "DEFLECT");
    assert_eq!(out.energy_restore_rate, Some(-10.0));
    let d = out.deflect.unwrap();
    assert!(d.covers(110.0));
    assert!(d.covers(-110.0));
    assert!(!d.covers(110.1));
    assert!(d.covers(0.0));
    assert_eq!(d.energy_cost, 2.5);
    assert_eq!(out.energy_cost, 0.0);
}

#[test]
fn an_early_press_does_not_latch_beyond_its_authored_time_end() {
    let def = program();
    let mut r = ComboRuntime::default();
    r.step(&def, CLIPS, input(0), 0.0);
    r.step(&def, CLIPS, input(FIRE), 0.0);
    r.step(&def, CLIPS, input(0), 0.15);
    assert_eq!(r.step(&def, CLIPS, input(FIRE), 0.05).state_name, "ATTACK1");
    let out = r.step(&def, CLIPS, input(0), 0.31);
    assert_eq!(out.state_name, "RECOVER1");
    assert!(out.entered);
}

#[test]
fn out_of_window_press_does_not_chain() {
    let def = program();
    let mut r = ComboRuntime::default();
    r.step(&def, CLIPS, input(0), 0.0);
    r.step(&def, CLIPS, input(FIRE), 0.0);
    r.step(&def, CLIPS, input(0), 0.01);
    r.step(&def, CLIPS, input(FIRE), 0.01);
    assert_eq!(r.step(&def, CLIPS, input(0), 0.49).state_name, "RECOVER1");
}

#[test]
fn transition_cost_refuses_only_depletion_and_is_reported_once() {
    let mut def = program();
    def.fields[0].children[1]
        .children
        .insert(0, f(K::EnergyCost, vec![Number(25.0)], vec![]));
    let mut r = ComboRuntime::default();
    r.step(&def, CLIPS, input(0), 0.0);
    let low = ComboInput {
        energy: 24.0,
        energy_depleted: true,
        ..input(FIRE_SECONDARY)
    };
    assert_eq!(r.step(&def, CLIPS, low, 0.01).state_name, "IDLE");
    let out = r.step(
        &def,
        CLIPS,
        ComboInput {
            energy: 1.0,
            ..input(FIRE_SECONDARY)
        },
        0.01,
    );
    assert_eq!(out.state_name, "DEFLECT");
    assert_eq!(out.energy_cost, 25.0);
    assert_eq!(r.step(&def, CLIPS, input(0), 0.01).energy_cost, 0.0);
}

#[test]
fn reload_remap_and_limited_input_lock_work() {
    let mut def = program();
    def.fields
        .push(f(K::RemapReloadButton, vec![text("FireSecondary")], vec![]));
    let mut r = ComboRuntime::default();
    r.step(&def, CLIPS, input(0), 0.0);
    assert_eq!(
        r.step(&def, CLIPS, input(RELOAD), 0.01).state_name,
        "DEFLECT"
    );
    let idle = def
        .fields
        .iter_mut()
        .find(|s| s.text(0) == Some("IDLE"))
        .unwrap();
    idle.children.push(f(
        K::InputLock,
        vec![Number(3.0), text("Frames"), text("Jump")],
        vec![],
    ));
    let mut r = ComboRuntime::default();
    assert_eq!(r.step(&def, CLIPS, input(0), 0.0).input_lock, JUMP);
    assert_eq!(r.step(&def, CLIPS, input(0), 0.11).input_lock, 0);
}

fn bytes(r: &ComboRuntime) -> Vec<u8> {
    let mut out = Vec::new();
    r.hash_into(&mut |b| out.extend_from_slice(b));
    out
}

#[test]
fn every_kept_runtime_field_changes_the_hash() {
    let base = ComboRuntime {
        state: 2,
        time: 0.5,
        state_serial: 7,
        chain_serial: 9,
        clock: 8.0,
        start: 7.5,
        history: input::ButtonHistory {
            held: 2,
            events: vec![input::ButtonEvent {
                time: 7.8,
                button: 1,
                flags: 2,
            }],
        },
        retro: Some(Retro {
            state: 1,
            duration: 1.0,
        }),
        sound_played: true,
        move_stopped: true,
        forced: false,
    };
    macro_rules! change {
        ($field:ident, $value:expr) => {{
            let mut r = base.clone();
            r.$field = $value;
            assert_ne!(bytes(&r), bytes(&base), stringify!($field));
        }};
    }
    change!(state, 3);
    change!(time, 0.6);
    change!(state_serial, 8);
    change!(chain_serial, 10);
    change!(clock, 9.0);
    change!(start, 8.0);
    change!(sound_played, false);
    change!(move_stopped, false);
    change!(retro, None);
    for which in 0..5 {
        let mut r = base.clone();
        match which {
            0 => r.history.held ^= 4,
            1 => r.history.events[0].time += 1.0,
            2 => r.history.events[0].button += 1,
            3 => r.history.events[0].flags ^= 1,
            _ => r.history.events.clear(),
        }
        assert_ne!(bytes(&r), bytes(&base));
    }
    for which in 0..2 {
        let mut r = base.clone();
        let old = r.retro.as_mut().unwrap();
        if which == 0 {
            old.state += 1;
        } else {
            old.duration += 1.0;
        }
        assert_ne!(bytes(&r), bytes(&base));
    }
}

#[test]
fn ps2_packed_dash_speed_saturates_and_strings_are_not_numeric() {
    let mut def = program();
    def.fields[0].children.push(f(
        K::AnimatedMove,
        vec![],
        vec![
            f(K::VelocityX, vec![Number(-150.0)], vec![]),
            f(K::VelocityZ, vec![Number(150.0)], vec![]),
        ],
    ));
    let mut r = ComboRuntime::default();
    let out = r.step(&def, CLIPS, input(0), 0.0);
    assert_eq!(out.movement.unwrap().velocity, [-40.96, 40.94]);
    let bad = f(K::TimeStart, vec![text("-12"), text("Frames")], vec![]);
    assert_eq!(time(&bad, 0, 1.0, 1.0), 0.0);
    let good = f(K::TimeStart, vec![Number(-12.0), text("Frames")], vec![]);
    assert_eq!(time(&good, 0, 1.0, 1.0).to_bits(), 0xbecc_cccc);
}

include!("tests/packed.rs");

#[test]
fn same_animation_preserves_time_and_rebases_existing_contact_cache() {
    let mut def = program();
    def.fields[4]
        .children
        .iter_mut()
        .find(|f| f.keyword == K::Animation)
        .unwrap()
        .args[0] = text("stand_attack1a");
    def.fields[2]
        .children
        .push(f(K::Duration, vec![Number(0.5)], vec![]));
    let clips = &[0.0, 0.0, 1.0, 0.5, 1.0];
    let mut runtime = ComboRuntime::default();
    runtime.step(&def, clips, input(FIRE), 0.1);
    let old_time = runtime.time;
    let out = runtime.step(&def, clips, input(0), 0.41);
    assert_eq!(out.state_name, "RECOVER1");
    assert!((out.state_time - (old_time + 0.41)).abs() < 1e-6);
    assert!((out.cache_shift + out.state_time).abs() < 1e-6);
}

#[test]
fn clip_lengths_are_resolved_into_native_hundredths() {
    let def = program();
    assert_eq!(
        duration(def.state("ATTACK1").unwrap(), 25.0 / 30.0).to_bits(),
        0x3f54_7ae0
    );
    assert_eq!(duration(def.state("IDLE").unwrap(), 1.0), 0.0);
}

include!("tests/reference.rs");
