use super::*;

#[test]
fn aligned_attack_keeps_the_actor_yaw_and_camera_while_aiming_the_weapon_right() {
    let (mut sim, hero, _) = scene();
    sim.weapon_classes[0] = saber(program(vec![
        field(K::Animation, vec![text("swing")], vec![]),
        field(K::AlignedToThrust, vec![V::Number(1.0)], vec![]),
        field(
            K::AnimatedMove,
            vec![],
            vec![field(K::VelocityZ, vec![V::Number(4.0)], vec![])],
        ),
    ]));
    sim.unit_mut(hero).unwrap().weapon.melee = None;
    sim.unit_mut(hero).unwrap().soldier.pitch = 0.0;
    let before = sim.unit(hero).unwrap().soldier.yaw;
    sim.saber_inputs(&mut [PadState {
        lx: i16::MAX,
        buttons: buttons::FIRE,
        ..Default::default()
    }]);
    let u = sim.unit(hero).unwrap();
    let state = u.weapon.melee.as_ref().unwrap();
    assert!(state.aligned_to_thrust);
    assert_eq!(
        u.soldier.yaw.to_bits(),
        before.to_bits(),
        "native owner frame never rotates toward the right input"
    );
    assert_eq!(u.soldier.pitch, 0.0);
    assert_eq!(
        state.retained_thrust,
        [0.0, 1.0],
        "physical D is raw native +right"
    );
    assert_eq!(
        state.weapon_forward(u.aim_frame().0),
        Vec3::new(
            f32::from_bits(0x3f35_0481),
            0.0,
            -f32::from_bits(0x3f35_0481)
        )
    );
    let frame = state.blade_frame(u.soldier.pos, u.soldier.yaw, u.soldier.pitch);
    assert_eq!(
        frame.z_axis.truncate(),
        state.weapon_forward(u.aim_frame().0)
    );
    let mut blocker = state.clone();
    blocker.deflect = Some(combo::Deflect {
        min_angle: -110.0,
        max_angle: 110.0,
        energy_cost: 0.0,
    });
    assert_eq!(
        blocker.weapon_forward(u.aim_frame().0),
        Vec3::NEG_Z,
        "33c9e0 skips Deflect states"
    );
}

#[test]
fn native_right_attack_rows_prove_aligned_state_does_not_turn_the_actor() {
    let Some(dir) = std::env::var_os("OPENBF_COMBO_REFERENCE_DIR") else {
        eprintln!("SKIP: original BF2 motion directory not supplied");
        return;
    };
    let text =
        std::fs::read_to_string(std::path::Path::new(&dir).join("hero_motion_attackright.jsonl"))
            .unwrap();
    let mut aligned = 0;
    let mut right = 0;
    let mut rows = 0;
    for line in text.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        if row.get("setup").is_some() {
            continue;
        }
        rows += 1;
        let flags = u32::from_str_radix(
            row["state_flags"]
                .as_str()
                .unwrap()
                .trim_start_matches("0x"),
            16,
        )
        .unwrap();
        if flags & (1 << 20) == 0 {
            continue;
        }
        aligned += 1;
        let frame = row["owner_frame"].as_array().unwrap();
        assert_eq!(frame[8].as_f64().unwrap(), 0.0);
        assert_eq!(frame[10].as_f64().unwrap(), -1.0);
        if row["retained_axes"][1].as_f64().unwrap() == 1.0 {
            right += 1
        }
    }
    assert_eq!(rows, 61);
    assert!(
        aligned >= 5 && right >= 5,
        "nonempty native aligned/right controls: {aligned}/{right}"
    );
}
