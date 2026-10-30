use super::*;
use crate::sim::tests::{flat_ground, hits};
use crate::unit::{UnitHandle, UnitSpec};
use crate::weapon::{BeamClass, OrdnanceClass, WeaponClass};

fn award_rifle() -> BeamClass {
    BeamClass {
        range: 500.0,
        life: 0.15,
        pierce: true,
        impact: None,
    }
}

fn scene() -> (Sim, usize, UnitHandle) {
    let mut s = Sim::empty(5);
    s.heightfield = Some(flat_ground());
    let class = s.add_weapon_class(WeaponClass {
        ordnance: OrdnanceClass {
            max_damage: 30.0,
            beam: Some(award_rifle()),
            ..OrdnanceClass::default()
        },
        ..WeaponClass::default()
    });
    let shooter = s.spawn(&UnitSpec::player(0, Vec3::ZERO)).unwrap();
    (s, class, shooter)
}

fn shooter(owner: UnitHandle, class: usize) -> Shooter {
    Shooter {
        owner,
        team: 0,
        weapon_class: class,
        is_player: true,
    }
}

fn open(s: &mut Sim, class: usize, owner: UnitHandle, follow: bool) {
    let round = Bolt::fire(
        Vec3::new(0.0, 1.4, 0.0),
        Vec3::X,
        150.0,
        0.15,
        shooter(owner, class),
        None,
        None,
        &s.collision,
        HullFilter::All,
    );
    s.launch_round(round, follow);
}

fn run(s: &mut Sim, ticks: usize) {
    for _ in 0..ticks {
        s.fly_bolts(crate::TICK_DT);
    }
}

#[test]
fn a_beam_spreads_its_damage_over_its_life() {
    let (mut s, class, owner) = scene();
    let target = s
        .spawn(&UnitSpec::trooper(1, Vec3::new(20.0, 0.0, 0.0)))
        .unwrap();
    open(&mut s, class, owner, false);
    run(&mut s, 10);
    let got = hits(&s);
    assert_eq!(got.len(), 5);
    for (victim, damage) in got {
        assert_eq!(victim, target.index);
        assert!((damage - 30.0 * crate::TICK_DT / 0.15).abs() < 1e-3);
    }
    assert!(s.beams.is_empty());
    assert!(s.bolts.is_empty());
}

#[test]
fn a_beam_stops_on_the_first_soldier() {
    let (mut s, class, owner) = scene();
    let near = s
        .spawn(&UnitSpec::trooper(1, Vec3::new(10.0, 0.0, 0.0)))
        .unwrap();
    s.spawn(&UnitSpec::trooper(1, Vec3::new(20.0, 0.0, 0.0)))
        .unwrap();
    open(&mut s, class, owner, false);
    run(&mut s, 1);
    assert_eq!(
        hits(&s).iter().map(|h| h.0).collect::<Vec<_>>(),
        [near.index]
    );
    let len = s.beams[0].len;
    assert!(len > 9.0 && len < 10.5, "{len}");
}

#[test]
fn a_beam_that_meets_nothing_reaches_its_range() {
    let (mut s, class, owner) = scene();
    s.heightfield = None;
    open(&mut s, class, owner, false);
    run(&mut s, 1);
    assert!((s.beams[0].len - 500.0).abs() < 1e-2);
    assert!((s.beams[0].life - (0.15 - crate::TICK_DT)).abs() < 1e-6);
    assert!(hits(&s).is_empty());
}

#[test]
fn a_held_beam_follows_its_shooters_fire_point() {
    let (mut s, class, _) = scene();
    let bot = s
        .spawn(&UnitSpec::trooper(0, Vec3::new(0.0, 0.0, 5.0)))
        .unwrap();
    open(&mut s, class, bot, true);
    run(&mut s, 1);
    s.units[bot.index as usize].soldier.pos = Vec3::new(3.0, 0.0, 5.0);
    run(&mut s, 1);
    let u = &s.units[bot.index as usize];
    assert_eq!(s.beams[0].origin, u.muzzle());
    assert_eq!(s.beams[0].dir, u.aim_frame().0);
}

#[test]
fn a_beam_hashes_only_while_it_lives() {
    let (mut s, class, owner) = scene();
    let before = s.hash();
    open(&mut s, class, owner, false);
    assert_ne!(s.hash(), before);
    s.beams.clear();
    assert_eq!(s.hash(), before);
}
