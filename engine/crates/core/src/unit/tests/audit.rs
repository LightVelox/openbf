//! Regressions for behavior-relevant aim history in the simulation hash.

use super::*;

#[test]
fn bot_aim_history_hashes_each_nonzero_component() {
    let history = |from| {
        let b = AimBlock {
            from,
            ..AimBlock::default()
        };
        let mut out = Vec::new();
        b.hash_history_into(&mut |x| out.extend_from_slice(x));
        out
    };
    assert!(
        history(Vec3::ZERO).is_empty(),
        "default history has no extension"
    );
    let mut seen = Vec::new();
    for from in [Vec3::X, Vec3::Y, Vec3::Z] {
        let bytes = history(from);
        assert!(bytes.starts_with(b"AIM_FROM"));
        assert!(!seen.contains(&bytes));
        seen.push(bytes);
    }
}

#[test]
fn human_aim_history_changes_neither_hash_nor_continuation() {
    for player in [true, false] {
        let mut observations = Vec::new();
        for from in [Vec3::X, -Vec3::X] {
            let mut sim = crate::sim::Sim::empty(31);
            sim.ground_rules = crate::soldier::GroundRules::Bf2;
            sim.heightfield = Some(crate::heightfield::Heightfield {
                size: 2,
                unit: 4000.0,
                offset_x: 2000.0,
                offset_z: 2000.0,
                heights: vec![0.0; 4],
            });
            let spec = if player {
                UnitSpec::player(0, Vec3::ZERO)
            } else {
                UnitSpec {
                    controller: Controller::Ai,
                    ..UnitSpec::trooper(0, Vec3::ZERO)
                }
            };
            let human = sim.spawn(&spec).unwrap();
            if !player {
                sim.unit_mut(human).unwrap().proxy_human =
                    Some(crate::ai::proxy::Strength::Good.profile());
            }
            sim.spawn(&UnitSpec {
                controller: Controller::Ai,
                ..UnitSpec::trooper(1, Vec3::new(0.0, 0.0, -15.0))
            })
            .unwrap();
            sim.tick = 1;
            sim.unit_mut(human).unwrap().aim.from = from;
            let hash = sim.hash();
            sim.step(&[crate::sim::PadState::default()]);
            observations.push((hash, sim.hash()));
        }
        assert_eq!(
            observations[0].0, observations[1].0,
            "human history is not read: player={player}"
        );
        assert_eq!(
            observations[0].1, observations[1].1,
            "same actual continuation: player={player}"
        );
    }
}

#[test]
fn audit_aim_history_changes_the_hash_and_next_input() {
    let mut sim = crate::sim::Sim::empty(31);
    sim.heightfield = Some(crate::heightfield::Heightfield {
        size: 2,
        unit: 4000.0,
        offset_x: 2000.0,
        offset_z: 2000.0,
        heights: vec![0.0; 4],
    });
    sim.ground_rules = crate::soldier::GroundRules::Bf2;
    let bot = sim
        .spawn(&UnitSpec {
            controller: Controller::Ai,
            ..UnitSpec::trooper(0, Vec3::ZERO)
        })
        .unwrap();
    let enemy = sim
        .spawn(&UnitSpec::trooper(1, Vec3::new(0.0, 0.0, -15.0)))
        .unwrap();
    sim.tick = 1;
    sim.unit_mut(bot).unwrap().aim.dir = Vec3::Y;
    sim.unit_mut(enemy).unwrap().aim.err = 1.5;
    let mut observations = Vec::new();
    for from in [Vec3::X, -Vec3::X] {
        sim.unit_mut(bot).unwrap().aim.from = from;
        let hash = sim.hash();
        let mut state = crate::ai::AiState::default();
        let (pad, _) = crate::ai::think(&sim, bot, &mut state);
        observations.push((hash, pad));
    }
    assert_ne!(
        observations[0].1, observations[1].1,
        "history changes the next aim input"
    );
    assert_ne!(
        observations[0].0, observations[1].0,
        "behavioral history must enter the hash"
    );
}
