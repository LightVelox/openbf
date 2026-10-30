use super::*;

fn launch(walk: &mut WalkSim) -> (usize, String) {
    let i = walk.player.index as usize;
    let slot = walk.sim.units[i]
        .held
        .iter()
        .position(|w| walk.sim.weapon_classes[w.class].saber_throw.is_some())
        .unwrap();
    for _ in 0..walk.sim.units[i].held.len() + 2 {
        if walk.sim.units[i].cur1 == Some(slot as u8) && walk.sim.units[i].held[slot].state.active {
            break;
        }
        walk.step_tick(&[PadState {
            buttons: core::sim::buttons::SWITCH_SECONDARY,
            ..Default::default()
        }]);
        walk.step_tick(&[PadState::default()]);
    }
    assert_eq!(walk.sim.units[i].cur1, Some(slot as u8));
    for _ in 0..90 {
        walk.step_tick(&[PadState {
            buttons: core::sim::buttons::SECONDARY_FIRE,
            ..Default::default()
        }]);
        if walk.sim.units[i].held[slot].state.saber_flight.is_some() {
            return (
                slot,
                walk.teams[0]
                    .class(0)
                    .cries
                    .hero
                    .saber_flight
                    .clone()
                    .unwrap(),
            );
        }
    }
    panic!("normal secondary input did not launch the real saber");
}

#[test]
fn actual_flight_audio_survives_same_generation_death_but_ends_on_catch_or_despawn() {
    let Some((root, db)) = content() else { return };
    let mut walk = battle(&db, "rep_hero_obiwan");
    let (slot, sound) = launch(&mut walk);
    starts(&mut library(&root), &sound);
    let handle = walk.player;
    let i = handle.index as usize;
    walk.sim
        .kill_environment(handle, core::sim::DeathCause::Suicide);
    assert!(!walk.sim.units[i].alive);
    let flight = walk.sim.units[i].held[slot]
        .state
        .saber_flight
        .as_ref()
        .unwrap();
    let asks = game::event_sounds::hero_sample_asks(&walk);
    assert_eq!(
        asks,
        vec![(handle, sound.clone(), flight.pos)],
        "dead breathing/hum stop, independent flight remains"
    );
    let mut caught = false;
    for _ in 0..90 {
        walk.step_tick(&[PadState::default()]);
        assert_eq!(walk.sim.units[i].gen, handle.gen);
        let asks = game::event_sounds::hero_sample_asks(&walk);
        if let Some(flight) = &walk.sim.units[i].held[slot].state.saber_flight {
            assert_eq!(asks, vec![(handle, sound.clone(), flight.pos)]);
        } else {
            assert!(asks.is_empty());
            caught = true;
            break;
        }
    }
    assert!(
        caught,
        "same-generation dead owner still catches the flight"
    );
    let mut departed = battle(&db, "rep_hero_obiwan");
    let (slot, _) = launch(&mut departed);
    let i = departed.player.index as usize;
    assert!(departed.sim.vacate_player_slot(0).is_some());
    assert!(departed.sim.units[i].held[slot]
        .state
        .saber_flight
        .is_none());
    assert!(game::event_sounds::hero_sample_asks(&departed).is_empty());
}

#[test]
fn actual_same_tick_hero_spawn_and_death_preserve_event_message_and_real_spawn_voice() {
    let Some((root, db)) = content() else { return };
    let mut walk = battle(&db, "rep_hero_obiwan");
    let i = walk.player.index as usize;
    let class = walk.sim.classes[0][0].clone();
    walk.sim.classes[0].push(class);
    let visual = walk.teams[0].classes[0].clone();
    walk.teams[0].classes.push(visual);
    walk.sim.heroes = Some(Heroes::new([Some(1), None]));
    walk.sim
        .heroes
        .as_mut()
        .unwrap()
        .points
        .resize(walk.sim.units.len(), 0);
    walk.sim.heroes.as_mut().unwrap().points[i] = 10;
    walk.sim.add_cp(
        core::cp::CpDef {
            name: "fixture".into(),
            pos: Vec3::ZERO,
            capture: core::region::Region::sphere(Vec3::ZERO, 8.0),
            control: None,
            value_bleed: 1,
            value_atk: [1; 2],
            value_def: [1; 2],
            spawn_nodes: vec![(Vec3::ZERO, 0.0)],
            capture_time: 10.0,
            neutralize_time: 10.0,
        },
        Some(0),
    );
    walk.step_tick(&[PadState::default()]);
    assert!(walk.sim.hero_offer(0).is_some());
    walk.sim.kill_floor = Some(5.0);
    walk.step_tick(&[PadState {
        buttons: core::sim::buttons::HERO_ACCEPT,
        ..Default::default()
    }]);
    let born = walk
        .sim
        .events
        .iter()
        .find_map(|e| match e {
            Event::Respawn { unit } => Some(*unit),
            _ => None,
        })
        .expect("normal offer acceptance must spawn");
    assert!(walk
        .sim
        .events
        .iter()
        .any(|e| matches!(e, Event::EnvDeath { unit, .. } if *unit == born)));
    assert_eq!(
        (
            walk.sim.units[i].gen,
            walk.sim.units[i].spec.class,
            walk.sim.units[i].alive
        ),
        (born.gen, 1, false)
    );
    let mut commander = game::announce::Commander::new(
        core::announcer::BattleLines::default(),
        ["rep", "cis"],
        0,
        Vec::new(),
        Vec::new(),
    );
    commander.calls(&walk.sim, &walk.teams);
    let expected = walk.teams[0].class(1).cries.hero.spawned.as_ref().unwrap();
    let mut lib = library(&root);
    let line = commander
        .queue
        .step(0.0, |line| {
            let (path, gain, secs) = lib.voice_take(&line.name).expect("actual hero voice");
            assert!(audio::decode(&path).unwrap().frames() > 0 && gain > 0.0 && secs > 0.0);
            Some(secs)
        })
        .unwrap();
    let voiced = &line.name == expected;
    let mut feed = game::event_feed::EventFeed::default();
    feed.tick(&walk.sim, born);
    let messages = feed.take();
    let messaged = messages
        .iter()
        .any(|m| m.text.contains("is now playing as"));
    assert!(voiced && messaged,
        "birth identity after same-tick death: voice={voiced} message={messaged}; actual voice {:?}, expected {expected:?}, messages {messages:?}", line.name);
}
