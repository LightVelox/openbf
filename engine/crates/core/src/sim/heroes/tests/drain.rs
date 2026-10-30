use super::*;
use crate::sim::{DeathCause, DropTable, ItemClass};
use crate::soldier::GroundRules;
use crate::weapon::DamageDesc;

fn active_hero() -> (Sim, UnitHandle) {
    let (mut sim, player) = scene();
    sim.heightfield = Some(crate::sim::tests::flat_ground());
    sim.ground_rules = GroundRules::Bf2;
    sim.units[player.index as usize].soldier.pos = Vec3::ZERO;
    earn(&mut sim, player, 10);
    sim.step(&[PadState::default()]);
    sim.step(&[PadState {
        buttons: buttons::HERO_ACCEPT,
        ..Default::default()
    }]);
    let hero = sim.players[0].unwrap();
    assert!(sim.is_hero(hero));
    assert_eq!(sim.unit(hero).unwrap().health.cur, 1200.0);
    assert_eq!(
        sim.heroes.as_ref().unwrap().teams[0].owner,
        Some(hero.index)
    );
    (sim, hero)
}

fn idle_enemy(sim: &mut Sim) -> UnitHandle {
    sim.spawn(&UnitSpec {
        controller: Controller::Idle,
        ..UnitSpec::trooper(1, Vec3::X * 50.0)
    })
    .unwrap()
}

fn hurt(sim: &mut Sim, victim: UnitHandle, attacker: UnitHandle, amount: f32) {
    let damage = sim.unit_mut(victim).unwrap().health.take_damage(
        &DamageDesc {
            damage: amount,
            scales: [1.0; 7],
            shield_scale: 1.0,
        },
        1.0,
        false,
        1.0,
    );
    sim.finish_unit_damage(victim, Some(attacker), damage, 0.0, Vec3::ZERO);
}

#[test]
fn hero_exhaustion_runs_the_normal_death_once_and_releases_the_reservation() {
    let (mut sim, hero) = active_hero();
    let item = sim.add_item_class(ItemClass::basic("health", 100.0, 0));
    sim.unit_mut(hero).unwrap().spec.drops = DropTable::from_pairs(&[(item, 1.0)]);
    sim.unit_mut(hero).unwrap().health.cur = 0.1;
    let tickets = sim.battle.teams[0].tickets();
    let now = sim.tick;
    sim.heroes.as_mut().unwrap().teams[0].kill_streak = 19;
    sim.step(&[PadState::default()]);
    let dead = &sim.units[hero.index as usize];
    assert!(!dead.alive);
    assert!(dead.health.cur <= 0.0);
    assert_eq!(dead.deaths, 1);
    assert!(dead.body, "normal on-foot death keeps its body");
    assert!(!dead.drop_due, "the dead-state block made the drop");
    assert_eq!(dead.soldier.stance, crate::soldier::Stance::Dead);
    assert!(dead.respawn_at.is_some_and(|tick| tick > now));
    assert!(dead.spawn_slot.is_some());
    assert_eq!(sim.pickups.len(), 1);
    assert_eq!(sim.battle.teams[0].tickets(), tickets - 1);
    assert_eq!(sim.battle.teams[0].alive_players, 0);
    let team = &sim.heroes.as_ref().unwrap().teams[0];
    assert_eq!(team.owner, None);
    assert_eq!(team.kill_streak, 0);
    assert_eq!(team.ready_at, now + 90 * u64::from(TICK_HZ));
    assert!(sim.events.iter().any(|e| matches!(
        e,
        Event::EnvDeath { unit, cause: DeathCause::HeroExhausted } if *unit == hero
    )));
    assert!(!sim.events.iter().any(|e| matches!(e, Event::Kill { .. })));
    let health = dead.health.cur;
    sim.step(&[PadState::default()]);
    assert_eq!(sim.units[hero.index as usize].health.cur, health);
    assert_eq!(sim.units[hero.index as usize].deaths, 1);
    assert_eq!(sim.battle.teams[0].tickets(), tickets - 1);
    assert_eq!(sim.pickups.len(), 1);
    assert!(!sim.events.iter().any(|e| matches!(
        e,
        Event::EnvDeath { .. } | Event::Kill { .. } | Event::ItemDropped { .. }
    )));
}

#[test]
fn hero_exhaustion_preserves_a_live_or_dead_attackers_existing_credit() {
    for attacker_alive in [true, false] {
        let (mut sim, hero) = active_hero();
        let attacker = idle_enemy(&mut sim);
        sim.unit_mut(hero).unwrap().health.cur = 2.125;
        hurt(&mut sim, hero, attacker, 2.0);
        assert_eq!(sim.unit(hero).unwrap().health.cur, 0.125);
        if !attacker_alive {
            sim.kill_environment(attacker, DeathCause::Suicide);
        }
        let attribution = sim.units[hero.index as usize].last_hit;
        assert_eq!(attribution.map(|hit| hit.0), Some(attacker));
        sim.step(&[PadState::default()]);
        assert!(!sim.units[hero.index as usize].alive);
        assert_eq!(sim.units[hero.index as usize].last_hit, attribution);
        assert_eq!(sim.units[attacker.index as usize].kills, 1);
        assert_eq!(sim.nav_stats.unit_kills, 1);
        assert_eq!(
            sim.events
                .iter()
                .filter(|e| matches!(e,
                    Event::Kill { shooter, victim } if *shooter == attacker && *victim == hero
                ))
                .count(),
            1
        );
        assert!(!sim
            .events
            .iter()
            .any(|e| matches!(e, Event::EnvDeath { .. })));
    }
}

#[test]
fn hero_exhaustion_does_not_credit_a_replacement_generation() {
    let (mut sim, hero) = active_hero();
    let attacker = idle_enemy(&mut sim);
    sim.unit_mut(hero).unwrap().health.cur = 2.125;
    hurt(&mut sim, hero, attacker, 2.0);
    sim.kill_environment(attacker, DeathCause::Suicide);
    sim.respawn_slot_with_class(attacker.index as usize, Some(0));
    let replacement = sim.handle(attacker.index as usize);
    assert_ne!(replacement.gen, attacker.gen);
    assert!(sim.unit(replacement).is_some());
    sim.step(&[PadState::default()]);
    assert_eq!(sim.unit(replacement).unwrap().kills, 0);
    assert!(sim.events.iter().any(|e| matches!(
        e,
        Event::EnvDeath { unit, cause: DeathCause::HeroExhausted } if *unit == hero
    )));
    assert!(!sim.events.iter().any(|e| matches!(e, Event::Kill { .. })));
}

#[test]
fn hero_drain_bypasses_the_damage_cheat_and_shield_without_faking_a_hit() {
    let (mut sim, hero) = active_hero();
    sim.cheats.invincible = true;
    let health = &mut sim.unit_mut(hero).unwrap().health;
    health.cur = 0.1;
    health.shield = 100.0;
    health.shield_max = 100.0;
    sim.step(&[PadState::default()]);
    let dead = &sim.units[hero.index as usize];
    assert!(!dead.alive, "offline exhaustion directly sets health");
    assert_eq!(dead.health.shield, 100.0);
    assert!(dead.last_hit.is_none());
    assert!(sim.events.iter().any(|e| matches!(
        e,
        Event::EnvDeath {
            cause: DeathCause::HeroExhausted,
            ..
        }
    )));
    assert!(!sim.events.iter().any(|e| matches!(e, Event::Hit { .. })));
}

#[test]
fn mounted_hero_drains_its_body_and_unseats_on_exhaustion_without_vehicle_damage() {
    let (mut sim, hero) = active_hero();
    let vehicle = crate::sim::hero_bodies::seat_fixture(&mut sim, hero, false);
    let vehicle_health = sim.vehicle(vehicle).unwrap().health.cur;
    sim.unit_mut(hero).unwrap().health.cur = 2.0;
    sim.step(&[PadState::default()]);
    assert!(sim.unit(hero).unwrap().mount.is_some());
    assert_eq!(
        sim.unit(hero).unwrap().health.cur,
        2.0 - HeroHealthRules::default().time_drain_per_second * crate::TICK_DT
    );
    assert_eq!(sim.vehicle(vehicle).unwrap().health.cur, vehicle_health);
    sim.unit_mut(hero).unwrap().health.cur = 0.1;
    sim.step(&[PadState::default()]);
    assert!(!sim.units[hero.index as usize].alive);
    assert!(sim.units[hero.index as usize].mount.is_none());
    assert!(sim.vehicle(vehicle).unwrap().seats[0].occupant.is_none());
    assert_eq!(sim.vehicle(vehicle).unwrap().health.cur, vehicle_health);
}

#[test]
fn hero_damage_kill_healing_clamp_and_time_drain_share_the_same_health_pool() {
    let (mut sim, hero) = active_hero();
    let enemy = idle_enemy(&mut sim);
    sim.unit_mut(hero).unwrap().health.cur = 100.0;
    hurt(&mut sim, hero, enemy, 20.0);
    assert_eq!(sim.unit(hero).unwrap().health.cur, 80.0);
    let lethal = sim.unit(enemy).unwrap().health.cur;
    hurt(&mut sim, enemy, hero, lethal);
    assert_eq!(sim.unit(hero).unwrap().health.cur, 230.0);
    sim.step(&[PadState::default()]);
    assert_eq!(
        sim.unit(hero).unwrap().health.cur,
        230.0 - HeroHealthRules::default().time_drain_per_second * crate::TICK_DT
    );
    assert_eq!(sim.heroes.as_ref().unwrap().teams[0].kill_streak, 1);
    let enemy = idle_enemy(&mut sim);
    sim.unit_mut(hero).unwrap().health.cur = 1190.0;
    let lethal = sim.unit(enemy).unwrap().health.cur;
    hurt(&mut sim, enemy, hero, lethal);
    assert_eq!(sim.unit(hero).unwrap().health.cur, 1200.0);
    sim.step(&[PadState::default()]);
    assert_eq!(
        sim.unit(hero).unwrap().health.cur,
        1200.0 - HeroHealthRules::default().time_drain_per_second * crate::TICK_DT
    );
}

#[test]
fn enabled_ai_hero_has_the_same_time_drain_as_a_human_hero() {
    let (mut sim, _) = scene();
    sim.heightfield = Some(crate::sim::tests::flat_ground());
    sim.ground_rules = GroundRules::Bf2;
    let bot = sim
        .spawn(&crate::sim::tests::ai(1, Vec3::X * 100.0, 0.0))
        .unwrap();
    earn(&mut sim, bot, 20);
    sim.heroes.as_mut().unwrap().rules.ai_candidates = true;
    sim.step(&[PadState::default()]);
    let hero = sim.handle(bot.index as usize);
    assert!(sim.is_hero(hero));
    assert_eq!(sim.unit(hero).unwrap().health.cur, 1500.0);
    sim.unit_mut(hero).unwrap().health.cur = 300.0;
    sim.step(&[PadState::default()]);
    assert_eq!(sim.unit(hero).unwrap().health.cur.to_bits(), 0x4395_eaab);
}

#[test]
fn hero_drain_stops_for_finished_rounds_and_off_field_spawn_members() {
    let (mut finished, hero) = active_hero();
    finished.outcome = Some(0);
    finished.step(&[PadState::default()]);
    assert_eq!(finished.unit(hero).unwrap().health.cur, 1200.0);

    let (mut off_field, hero) = active_hero();
    assert!(off_field.open_spawn_screens(0, false));
    assert!(!off_field.units[hero.index as usize].alive);
    let health = off_field.units[hero.index as usize].health.cur;
    for _ in 0..TICK_HZ {
        off_field.step(&[PadState::default()]);
    }
    assert_eq!(off_field.units[hero.index as usize].health.cur, health);

    let (mut vacated, hero) = active_hero();
    vacated.vacate_player_slot(0).unwrap();
    let health = vacated.units[hero.index as usize].health.cur;
    vacated.step(&[]);
    assert_eq!(vacated.units[hero.index as usize].health.cur, health);
    assert!(!vacated
        .events
        .iter()
        .any(|e| matches!(e, Event::EnvDeath { .. } | Event::Kill { .. })));
}

#[test]
fn conquest_hero_health_drains_five_per_second_half_the_native_rate() {
    // FUN_0034cea8, 0x34d15c..178: HP -= game_delta * 10.0; ours halves it.
    assert_eq!(crate::sim::heroes::PS2_TIME_DRAIN, 10.0);
    assert_eq!(HeroHealthRules::default().time_drain_per_second, 5.0);
    for rate in [0.0, 7.5, 10.0] {
        let mut rules = HeroRules::default();
        rules.health.time_drain_per_second = rate;
        assert!(Heroes::with_rules([Some(1), None], rules).is_ok());
    }
    for rate in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut rules = HeroRules::default();
        rules.health.time_drain_per_second = rate;
        assert!(rules.validate().is_err(), "invalid rate {rate}");
    }
}

#[test]
fn conquest_hero_health_drains_each_tick_with_ordinary_and_zero_rate_controls() {
    let world = || {
        let (mut sim, hero) = active_hero();
        sim.unit_mut(hero).unwrap().health.cur = 300.0;
        let ordinary = sim
            .spawn(&UnitSpec {
                controller: Controller::Idle,
                ..UnitSpec::trooper(1, Vec3::X * 50.0)
            })
            .unwrap();
        (sim, hero, ordinary)
    };
    let (mut sim, hero, ordinary) = world();
    let ordinary_health = sim.unit(ordinary).unwrap().health.cur;
    let (mut no_manager, no_manager_hero, no_manager_ordinary) = world();
    no_manager.heroes = None;
    let (mut zero_rate, zero_rate_hero, zero_rate_ordinary) = world();
    zero_rate
        .heroes
        .as_mut()
        .unwrap()
        .rules
        .health
        .time_drain_per_second = 0.0;

    sim.step(&[PadState::default()]);
    assert_eq!(sim.unit(hero).unwrap().health.cur.to_bits(), 0x4395_eaab);
    for _ in 1..TICK_HZ {
        sim.step(&[PadState::default()]);
    }
    assert!((sim.unit(hero).unwrap().health.cur - 295.0).abs() < 0.002);
    assert_eq!(sim.unit(ordinary).unwrap().health.cur, ordinary_health);
    assert!(sim.events.iter().all(|e| !matches!(e, Event::Hit { .. })));
    for (control, hero, ordinary) in [
        (&mut no_manager, no_manager_hero, no_manager_ordinary),
        (&mut zero_rate, zero_rate_hero, zero_rate_ordinary),
    ] {
        for _ in 0..TICK_HZ {
            control.step(&[PadState::default()]);
        }
        assert_eq!(control.unit(hero).unwrap().health.cur, 300.0);
        assert_eq!(control.unit(ordinary).unwrap().health.cur, ordinary_health);
    }
}
