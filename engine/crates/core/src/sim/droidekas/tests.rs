use std::sync::Arc;

use glam::Vec3;

use crate::droideka::{DroidekaClass, Order, IDLE, IDLE_TO_ROLL, ROLLING, ROLL_TO_IDLE};
use crate::sim::tests::flat_ground;
use crate::sim::{buttons, Event, PadState, Sim};
use crate::unit::{Controller, UnitHandle, UnitSpec};
use crate::weapon::{ExplosionClass, HealthType};

fn class() -> Arc<DroidekaClass> {
    Arc::new(DroidekaClass::default())
}

fn deka(spec: UnitSpec) -> UnitSpec {
    UnitSpec {
        max_health: 550.0,
        health_type: HealthType::Droid,
        droideka: Some(class()),
        ..spec
    }
}

fn sim() -> Sim {
    let mut s = Sim::empty(11);
    s.heightfield = Some(flat_ground());
    s
}

fn pad(buttons: u32, ly: f32) -> PadState {
    PadState {
        buttons,
        ly: PadState::quantize(ly),
        ..Default::default()
    }
}

fn state(s: &Sim, h: UnitHandle) -> u8 {
    s.units[h.index as usize].droideka.as_ref().unwrap().state
}

fn shots(s: &Sim) -> usize {
    s.events
        .iter()
        .filter(|e| matches!(e, Event::Shot { .. }))
        .count()
}

fn cues(s: &Sim) -> Vec<crate::droideka::Cue> {
    s.events
        .iter()
        .filter_map(|e| match e {
            Event::Droideka { cue, .. } => Some(*cue),
            _ => None,
        })
        .collect()
}

#[test]
fn a_bot_droideka_folds_at_spawn_and_rolls() {
    let mut s = sim();
    let h = s.spawn(&deka(UnitSpec::trooper(1, Vec3::ZERO))).unwrap();
    s.step(&[]);
    assert_eq!(state(&s, h), IDLE_TO_ROLL);
    assert_eq!(cues(&s), vec![crate::droideka::Cue::Fold]);
    for _ in 0..20 {
        s.step(&[]);
    }
    assert_eq!(state(&s, h), ROLLING);
    assert!(s.units[h.index as usize].soldier.grounded);
}

#[test]
fn a_player_walks_fires_and_cannot_fire_while_folding() {
    let mut s = sim();
    let h = s.spawn(&deka(UnitSpec::player(0, Vec3::ZERO))).unwrap();
    let mut fired = 0;
    for _ in 0..60 {
        s.step(&[pad(buttons::FIRE, 1.0)]);
        fired += shots(&s);
    }
    assert!(fired > 5, "{fired}");
    let z = s.units[h.index as usize].soldier.pos.z;
    assert!(z < -3.0, "walked forward: {z}");
    s.step(&[pad(buttons::ROLL_STANCE, 0.0)]);
    assert_eq!(state(&s, h), IDLE_TO_ROLL);
    let mut fired = 0;
    for _ in 0..60 {
        s.step(&[pad(buttons::FIRE, 1.0)]);
        fired += shots(&s);
    }
    assert_eq!(state(&s, h), ROLLING);
    assert_eq!(fired, 0);
    let x0 = s.units[h.index as usize].soldier.pos;
    for _ in 0..30 {
        s.step(&[pad(0, 1.0)]);
    }
    let rolled = (s.units[h.index as usize].soldier.pos - x0).length();
    assert!(rolled > 6.0, "{rolled}");
    s.step(&[pad(buttons::ROLL_STANCE, 0.0)]);
    assert_eq!(state(&s, h), ROLL_TO_IDLE);
    for _ in 0..55 {
        s.step(&[pad(0, 0.0)]);
    }
    assert_eq!(state(&s, h), IDLE);
}

fn target(s: &mut Sim, shield: bool) -> UnitHandle {
    let h = s
        .spawn(&deka(UnitSpec::trooper(1, Vec3::new(0.0, 0.0, -20.0))))
        .unwrap();
    let d = s.units[h.index as usize].droideka.as_mut().unwrap();
    d.order = Order::Deploy;
    d.shield.on = shield;
    h
}

fn shoot(s: &mut Sim, ticks: usize) {
    for _ in 0..ticks {
        s.step(&[PadState {
            buttons: buttons::FIRE,
            ..Default::default()
        }]);
    }
}

#[test]
fn a_raised_shield_takes_the_bolts_and_the_body_takes_none() {
    let mut s = sim();
    s.spawn(&UnitSpec::player(0, Vec3::ZERO)).unwrap();
    let t = target(&mut s, true);
    shoot(&mut s, 60);
    let u = &s.units[t.index as usize];
    let d = u.droideka.as_ref().unwrap();
    assert!(d.shield.points < 3000.0, "{}", d.shield.points);
    assert_eq!(u.health.cur, 550.0);
    assert!(u.last_hit.is_some());
}

#[test]
fn a_lowered_shield_lets_the_bolts_reach_the_body() {
    let mut s = sim();
    s.spawn(&UnitSpec::player(0, Vec3::ZERO)).unwrap();
    let t = target(&mut s, false);
    shoot(&mut s, 60);
    let u = &s.units[t.index as usize];
    assert_eq!(u.droideka.as_ref().unwrap().shield.points, 3000.0);
    assert!(u.health.cur < 550.0, "{}", u.health.cur);
}

#[test]
fn a_blast_spends_the_shield_before_the_health_and_does_not_throw_it() {
    let mut s = sim();
    let t = target(&mut s, true);
    s.units[t.index as usize]
        .droideka
        .as_mut()
        .unwrap()
        .shield
        .points = 40.0;
    let e = ExplosionClass {
        damage: 100.0,
        damage_outer: 4.0,
        push: 20.0,
        ..Default::default()
    };
    let at = s.units[t.index as usize].soldier.pos + Vec3::Y;
    s.explode(&e, at, None, Some(0));
    let u = &s.units[t.index as usize];
    assert_eq!(u.droideka.as_ref().unwrap().shield.points, 0.0);
    assert!(
        u.health.cur < 550.0 && u.health.cur > 450.0,
        "{}",
        u.health.cur
    );
    assert!(!u.soldier.stance.knocked());
}

#[test]
fn a_dead_droideka_bursts_with_no_corpse_and_respawns_whole() {
    let mut s = sim();
    let t = target(&mut s, true);
    let i = t.index as usize;
    s.units[i].health.cur = -10.0;
    s.unit_killed(t, None);
    s.step(&[]);
    assert!(s
        .events
        .iter()
        .any(|e| matches!(e, Event::DroidBurst { unit, .. } if *unit == t)));
    assert!(!s.units[i].body && s.units[i].burst);
    let fresh = s.units[i].spec.build(t.gen + 1, &s.weapon_classes);
    let d = fresh.droideka.as_ref().unwrap();
    assert_eq!(d.state, IDLE);
    assert_eq!(d.shield.points, 3000.0);
    assert!(!d.shield.on);
}

#[test]
fn deep_water_wears_the_droideka_down() {
    let mut s = sim();
    s.water_level = Some(2.0);
    let h = s.spawn(&deka(UnitSpec::player(0, Vec3::ZERO))).unwrap();
    for _ in 0..45 {
        s.step(&[pad(0, 0.0)]);
    }
    let hp = s.units[h.index as usize].health.cur;
    assert!((550.0 - 10.0 * 16.0..550.0).contains(&hp), "{hp}");
}

#[test]
fn droideka_runs_replay_to_the_same_hash() {
    let run = |turn: f32| {
        let mut s = sim();
        s.spawn(&deka(UnitSpec::player(0, Vec3::ZERO))).unwrap();
        s.spawn(&deka(UnitSpec {
            controller: Controller::Idle,
            ..UnitSpec::trooper(1, Vec3::new(5.0, 0.0, -10.0))
        }))
        .unwrap();
        for i in 0..240u32 {
            let mut p = pad(if i % 40 == 0 { buttons::ROLL_STANCE } else { 0 }, 0.8);
            p.rx = PadState::quantize(turn);
            if i % 55 == 0 {
                p.buttons |= buttons::SECONDARY_FIRE | buttons::JUMP;
            }
            s.step(&[p]);
        }
        s.hash()
    };
    assert_eq!(run(0.3), run(0.3));
    assert_ne!(run(0.3), run(-0.3));
}

fn bot_and_foe(dist: f32) -> (Sim, UnitHandle) {
    let mut s = sim();
    s.spawn(&UnitSpec::player(0, Vec3::new(0.0, 0.0, -dist)))
        .unwrap();
    for _ in 0..crate::ai::dfocus::UNTARGETED {
        s.step(&[PadState::default()]);
    }
    let h = s
        .spawn(&deka(UnitSpec {
            controller: Controller::Ai,
            ..UnitSpec::trooper(1, Vec3::ZERO)
        }))
        .unwrap();
    (s, h)
}

#[test]
fn a_bot_droideka_stays_rolled_with_a_far_enemy_in_sight() {
    let (mut s, h) = bot_and_foe(30.0);
    for _ in 0..90 {
        s.step(&[PadState::default()]);
    }
    let u = &s.units[h.index as usize];
    println!("{:?} state {}", u.ai.behaviour, state(&s, h));
    assert_ne!(u.ai.behaviour, crate::ai::Behaviour::Fight);
    assert_eq!(state(&s, h), ROLLING);
}

#[test]
fn a_bot_droideka_deploys_raises_its_shield_and_fires_at_a_near_enemy() {
    let (mut s, h) = bot_and_foe(8.0);
    let mut fired = 0;
    let mut shield = false;
    let mut states = Vec::new();
    for _ in 0..150 {
        s.step(&[PadState::default()]);
        s.units[0].health.cur = s.units[0].health.max;
        let d = s.units[h.index as usize].droideka.as_ref().unwrap();
        shield |= d.shield.on;
        if states.last() != Some(&d.state) {
            states.push(d.state);
        }
        fired += s
            .events
            .iter()
            .filter(|e| matches!(e, Event::Shot { unit, .. } if *unit == h))
            .count();
    }
    let u = &s.units[h.index as usize];
    println!(
        "{:?} states {states:?} fired {fired} shield {shield}",
        u.ai.behaviour
    );
    assert_eq!(u.ai.behaviour, crate::ai::Behaviour::Fight);
    assert_eq!(states, [IDLE], "it stays deployed");
    assert!(shield);
    assert!(fired > 0);
}

#[test]
fn a_bot_droideka_rolls_again_once_its_fight_is_over() {
    let (mut s, h) = bot_and_foe(8.0);
    let mut fought = false;
    for _ in 0..300 {
        s.step(&[PadState::default()]);
        fought |= s.units[h.index as usize].ai.behaviour == crate::ai::Behaviour::Fight;
    }
    assert!(fought);
    assert!(!s.units[0].alive || s.units[0].health.cur <= 0.0);
    assert!(
        matches!(state(&s, h), IDLE_TO_ROLL | ROLLING),
        "{}",
        state(&s, h)
    );
}
