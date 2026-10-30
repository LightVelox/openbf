use super::*;
use crate::sim::{ClassRefusal, SpawnStage};
use crate::unit::SpawnPost;

fn tap(sim: &mut Sim, button: u32) {
    sim.step(&[PadState {
        buttons: button,
        ..Default::default()
    }]);
    sim.step(&[PadState::default()]);
}

fn screen_scene() -> (Sim, UnitHandle) {
    let (mut sim, p) = scene();
    sim.heightfield = Some(crate::sim::tests::flat_ground());
    sim.units[p.index as usize].soldier.pos = Vec3::ZERO;
    sim.spawn_screens = true;
    sim.add_cp(
        crate::sim::tests::sphere_cp("far", Vec3::X * 200.0, 8.0, true),
        Some(0),
    );
    (sim, p)
}

#[test]
fn hero_offer_opens_normal_map_then_spawns_at_the_confirmed_post() {
    let (mut sim, p) = screen_scene();
    earn(&mut sim, p, 10);
    sim.step(&[PadState::default()]);
    let tickets = sim.battle.teams[0].tickets();
    sim.step(&[PadState {
        buttons: buttons::HERO_ACCEPT,
        ..Default::default()
    }]);
    assert_eq!(sim.spawn_stage[0], SpawnStage::Map);
    let off = &sim.units[p.index as usize];
    assert!(!off.alive && !off.body && !off.drop_due);
    assert_eq!(off.deaths, 0);
    assert_eq!(off.respawn_at, None);
    assert_eq!(sim.battle.teams[0].alive_players, 0);
    assert_eq!(sim.battle.teams[0].tickets(), tickets);
    assert!(!sim
        .events
        .iter()
        .any(|e| matches!(e, Event::Kill { .. } | Event::EnvDeath { .. })));
    assert_eq!(sim.spawn_class_indices(0, 0), vec![0, 1]);
    for _ in 0..120 {
        sim.step(&[PadState::default()]);
    }
    assert!(!sim.units[p.index as usize].alive);
    tap(&mut sim, buttons::SPAWN_POST_RIGHT);
    assert_eq!(sim.spawn_post_selected(0), Some(SpawnPost::Post(2)));
    tap(&mut sim, buttons::SPAWN_ACCEPT);
    assert_eq!(sim.spawn_stage[0], SpawnStage::Class);
    assert!(!sim.units[p.index as usize].alive);
    // Browsing an ordinary row does not spend the hero reservation.
    tap(&mut sim, buttons::SPAWN_CLASS_NEXT);
    assert_eq!(sim.spawn_choice[0], 0);
    for _ in 0..25 {
        sim.step(&[PadState::default()]);
    }
    assert_eq!(sim.heroes.as_ref().unwrap().teams[0].owner, Some(p.index));
    tap(&mut sim, buttons::SPAWN_CLASS_PREV);
    assert_eq!(sim.spawn_choice[0], 1);
    tap(&mut sim, buttons::SPAWN_ACCEPT);
    let hero = sim.players[0].unwrap();
    assert_ne!(hero.gen, p.gen);
    assert!(sim.is_hero(hero));
    assert_eq!(sim.spawn_stage[0], SpawnStage::Accepted);
    let hero = sim.unit(hero).unwrap();
    assert!((hero.soldier.pos.x - 200.0).abs() <= 2.1);
    assert_eq!(
        (hero.health.cur, hero.deaths),
        (
            1200.0
                - crate::sim::heroes::HeroHealthRules::default().time_drain_per_second
                    * crate::TICK_DT,
            0
        )
    );
    assert_eq!(sim.battle.teams[0].tickets(), tickets);
    assert_eq!(sim.battle.teams[0].alive_players, 1);
}

#[test]
fn hero_carousel_and_confirmation_share_current_member_ownership() {
    let (mut sim, p) = screen_scene();
    sim.classes[0].push(class(450.0));
    assert!(sim.open_spawn_screens(0, false));
    sim.spawn_stage[0] = SpawnStage::Class;
    assert_eq!(sim.spawn_class_indices(0, 0), vec![0, 2]);
    // Held carousel steps wrap across the absent middle hero row.
    for _ in 0..100 {
        sim.step(&[PadState {
            buttons: buttons::SPAWN_CLASS_NEXT,
            ..Default::default()
        }]);
        assert_ne!(sim.spawn_choice[0], 1);
    }
    // A stale external choice cannot bypass confirmation.
    sim.spawn_choice[0] = 1;
    assert_eq!(
        sim.class_refusal(0, 0, 1),
        Some(ClassRefusal::HeroUnavailable)
    );
    tap(&mut sim, buttons::SPAWN_ACCEPT);
    assert!(sim.unit(sim.players[0].unwrap()).is_some());
    assert_ne!(sim.units[p.index as usize].spec.class, 1);
    let p2 = sim
        .spawn(&UnitSpec::player_in_slot(1, 0, Vec3::ZERO))
        .unwrap();
    earn(&mut sim, p, 10);
    sim.tick = sim.heroes.as_ref().unwrap().teams[0].ready_at + 1;
    sim.step(&[PadState::default(); 2]);
    sim.step(&[
        PadState {
            buttons: buttons::HERO_ACCEPT,
            ..Default::default()
        },
        PadState::default(),
    ]);
    assert_eq!(sim.spawn_class_indices(0, 0), vec![0, 1, 2]);
    assert_eq!(sim.spawn_class_indices(1, 0), vec![0, 2]);
    assert_eq!(
        sim.class_refusal(1, 0, 1),
        Some(ClassRefusal::HeroUnavailable)
    );
    assert_eq!(sim.players[1], Some(p2));
}

#[test]
fn hero_death_and_stale_automatic_choice_cannot_spawn_an_unowned_hero() {
    let (mut sim, p) = scene();
    sim.heightfield = Some(crate::sim::tests::flat_ground());
    sim.units[p.index as usize].soldier.pos = Vec3::ZERO;
    earn(&mut sim, p, 10);
    sim.step(&[PadState::default()]);
    tap(&mut sim, buttons::HERO_ACCEPT);
    let hero = sim.players[0].unwrap();
    assert!(sim.is_hero(hero));
    sim.kill_environment(hero, super::super::super::DeathCause::Suicide);
    sim.step(&[PadState::default()]);
    assert_eq!(sim.heroes.as_ref().unwrap().teams[0].owner, None);
    assert_eq!(sim.spawn_class_indices(0, 0), vec![0]);
    // Simulate an old saved/menu choice after ownership has expired.
    sim.spawn_choice[0] = 1;
    sim.units[hero.index as usize].respawn_at = Some(sim.tick);
    sim.step(&[PadState::default()]);
    let ordinary = sim.players[0].unwrap();
    assert!(sim.unit(ordinary).is_some());
    assert!(!sim.is_hero(ordinary));
    assert_eq!(
        (sim.spawn_choice[0], sim.unit(ordinary).unwrap().spec.class),
        (0, 0)
    );
}

#[test]
fn a_table_without_heroes_keeps_its_old_out_of_range_carousel_clamp() {
    for (button, expected) in [
        (buttons::SPAWN_CLASS_NEXT, 0),
        (buttons::SPAWN_CLASS_PREV, 1),
    ] {
        let (mut sim, _) = screen_scene();
        sim.classes[0].push(class(450.0));
        sim.heroes = None;
        assert!(sim.open_spawn_screens(0, false));
        sim.spawn_screens = false;
        sim.spawn_choice[0] = u8::MAX;
        assert_eq!(sim.spawn_class_indices(0, 0), vec![0, 1, 2]);
        tap(&mut sim, button);
        assert_eq!(sim.spawn_choice[0], expected);
    }
}

#[test]
fn a_vacated_human_cannot_become_an_uncontrolled_hero_candidate() {
    let (mut sim, player) = screen_scene();
    sim.spawn_screens = false;
    earn(&mut sim, player, 10);
    assert!(sim.vacate_player_slot(0).is_some());
    assert_eq!(sim.players[0], None);
    sim.step(&[]);
    assert!(sim.heroes.as_ref().unwrap().teams[0].owner.is_none());
    assert!(sim.heroes.as_ref().unwrap().teams[0].offer.is_none());
    assert!(
        sim.units.iter().all(|u| !u.alive),
        "no-fill vacate must stay off field"
    );
}

/// The PS2 never offers the hero to a human without a soldier: the pick is
/// dropped and tried again next update (FUN_003ef9d0 B2:449210-449214). He
/// is offered once he is back on the field.
#[test]
fn a_dead_human_is_offered_the_hero_only_once_back_on_the_field() {
    let (mut sim, player) = screen_scene();
    sim.kill_environment(player, crate::sim::DeathCause::Suicide);
    earn(&mut sim, player, 10);
    for _ in 0..5 {
        sim.step(&[PadState::default()]);
    }
    let team = &sim.heroes.as_ref().unwrap().teams[0];
    assert_eq!((team.owner, team.offer), (None, None));
    assert!(team.declined.is_empty(), "not a decline");
    // Back on the field: the offer comes.
    sim.spawn_stage[0] = SpawnStage::Class;
    tap(&mut sim, buttons::SPAWN_ACCEPT);
    let p = sim.players[0].unwrap();
    assert!(sim.unit(p).unwrap().alive);
    sim.step(&[PadState::default()]);
    assert!(sim.hero_offer(0).is_some());
}

fn screen_accept(sim: &mut Sim) {
    tap(sim, buttons::SPAWN_POST_RIGHT);
    tap(sim, buttons::SPAWN_ACCEPT);
    assert_eq!(sim.spawn_stage[0], SpawnStage::Class);
}

/// The maintainer's flow (playtest 2026-10-30): the offer counts down from
/// 30 s; Yes takes the body away with no death or ticket and opens the
/// command post map; the class screen opens on the hero, but any class can
/// be chosen and the hero is kept for later (the PS2's owner survives a
/// life in another class: FUN_0034e1f0 B2:345046-345058); the next class
/// screen opens on the hero again; the hero's own death ends the ownership
/// and starts the respawn timer.
#[test]
fn hero_yes_goes_map_then_class_and_another_class_keeps_the_hero() {
    let (mut sim, p) = screen_scene();
    earn(&mut sim, p, 10);
    sim.step(&[PadState::default()]);
    let (class, seconds) = sim.hero_offer(0).unwrap();
    assert_eq!(class, 1);
    assert!((29.9..=30.0).contains(&seconds), "{seconds}");
    let tickets = sim.battle.teams[0].tickets();
    sim.step(&[PadState {
        buttons: buttons::HERO_ACCEPT,
        ..Default::default()
    }]);
    let off = &sim.units[p.index as usize];
    assert!(!off.alive && !off.body, "the body is gone at once");
    assert_eq!(off.deaths, 0);
    assert_eq!(sim.battle.teams[0].tickets(), tickets);
    assert!(sim
        .events
        .iter()
        .any(|e| matches!(e, Event::Despawn { unit } if *unit == p)));
    assert_eq!(sim.spawn_stage[0], SpawnStage::Map, "the post map first");
    sim.step(&[PadState::default()]);
    screen_accept(&mut sim);
    assert_eq!(sim.spawn_choice[0], 1, "the class screen opens on the hero");
    // Another class: the hero is saved for later.
    tap(&mut sim, buttons::SPAWN_CLASS_NEXT);
    assert_eq!(sim.spawn_choice[0], 0);
    tap(&mut sim, buttons::SPAWN_ACCEPT);
    let trooper = sim.players[0].unwrap();
    assert!(sim.unit(trooper).unwrap().alive && !sim.is_hero(trooper));
    for _ in 0..90 {
        sim.step(&[PadState::default()]);
    }
    assert_eq!(sim.heroes.as_ref().unwrap().teams[0].owner, Some(p.index));
    assert_eq!(sim.hero_offer(0), None, "no new offer while he owns it");
    // He dies as the trooper: still the owner, the class screen opens on
    // the hero.
    sim.kill_environment(trooper, crate::sim::DeathCause::Suicide);
    sim.step(&[PadState::default()]);
    assert_eq!(sim.heroes.as_ref().unwrap().teams[0].owner, Some(p.index));
    sim.open_spawn_map(0, 0);
    screen_accept(&mut sim);
    assert_eq!(sim.spawn_choice[0], 1);
    tap(&mut sim, buttons::SPAWN_ACCEPT);
    let hero = sim.players[0].unwrap();
    assert!(sim.is_hero(hero));
    sim.step(&[PadState::default()]);
    // The hero's death ends it and starts the 90 s timer.
    sim.kill_environment(hero, crate::sim::DeathCause::Suicide);
    sim.step(&[PadState::default()]);
    let team = &sim.heroes.as_ref().unwrap().teams[0];
    assert_eq!(team.owner, None);
    assert_eq!(team.ready_at, sim.tick - 1 + 90 * u64::from(crate::TICK_HZ));
    assert!(!sim.spawn_class_available(0, 0, 1));
}

/// No or the timer running out dismisses the offer: nothing changes for the
/// player, and the hero waits the respawn timer (the only human declined).
#[test]
fn hero_no_and_timeout_dismiss_the_offer_and_keep_the_soldier() {
    for timeout in [false, true] {
        let (mut sim, p) = screen_scene();
        earn(&mut sim, p, 10);
        sim.step(&[PadState::default()]);
        let expires = sim.heroes.as_ref().unwrap().teams[0].offer.unwrap().expires;
        if timeout {
            while sim.tick <= expires {
                sim.step(&[PadState::default()]);
            }
            sim.step(&[PadState::default()]);
        } else {
            sim.step(&[PadState {
                buttons: buttons::HERO_DECLINE,
                ..Default::default()
            }]);
        }
        assert_eq!(sim.hero_offer(0), None, "timeout {timeout}");
        let team = &sim.heroes.as_ref().unwrap().teams[0];
        assert_eq!(team.owner, None);
        assert!(team.ready_at > sim.tick);
        let me = sim.unit(sim.players[0].unwrap()).unwrap();
        assert!(me.alive && me.spec.class == 0 && sim.players[0] == Some(p));
    }
}

/// B2:448990-449000: the offered member loses his soldier while the offer
/// is up - he is the owner, and the hero is chosen on his spawn screen.
#[test]
fn hero_offered_player_who_dies_during_the_offer_owns_the_hero() {
    let (mut sim, p) = screen_scene();
    earn(&mut sim, p, 10);
    sim.step(&[PadState::default()]);
    assert!(sim.hero_offer(0).is_some());
    sim.kill_environment(p, crate::sim::DeathCause::Suicide);
    sim.step(&[PadState::default()]);
    assert_eq!(sim.heroes.as_ref().unwrap().teams[0].owner, Some(p.index));
    assert_eq!(sim.spawn_choice[0], 1);
}

/// Screens off (a headless run): an accepted owner whose respawn is refused
/// (his side has no post) stays the owner and comes back as the hero once
/// a post is his again.
#[test]
fn hero_refused_respawn_keeps_the_hero() {
    let (mut sim, p) = scene();
    earn(&mut sim, p, 10);
    sim.step(&[PadState::default()]);
    let post = sim.cps.remove(0);
    sim.step(&[PadState {
        buttons: buttons::HERO_ACCEPT,
        ..Default::default()
    }]);
    assert!(!sim.units[p.index as usize].alive, "refused: no post");
    for _ in 0..30 {
        sim.step(&[PadState::default()]);
    }
    assert_eq!(sim.heroes.as_ref().unwrap().teams[0].owner, Some(p.index));
    sim.cps.insert(0, post);
    for _ in 0..30 * 20 {
        sim.step(&[PadState::default()]);
        if sim.players[0].is_some_and(|h| sim.unit(h).is_some()) {
            break;
        }
    }
    let hero = sim.players[0].unwrap();
    assert!(sim.unit(hero).is_some_and(|u| u.alive), "back on the field");
    assert!(sim.is_hero(hero));
}
