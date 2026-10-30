use super::*;
use crate::{event_feed::EventFeed, walk::WalkSim};
use core::sim::{heroes::Heroes, SpawnStage};
use glam::Vec3;
use ui::spawn_draw::SpawnPage;

#[test]
fn hero_class_page_and_lineup_use_only_the_current_owners_visible_rows() {
    let mut walk = WalkSim::new(3, Vec3::ZERO, None);
    let p = walk.player;
    let u = &walk.sim.units[p.index as usize];
    let class = fixture_class(u);
    walk.sim.classes[0] = vec![class.clone(), class.clone(), class];
    let mut visual = walk.teams[0].classes[0].clone();
    visual.label = "Hero".into();
    walk.teams[0].classes.push(visual.clone());
    visual.label = "Ordinary second".into();
    walk.teams[0].classes.push(visual);
    walk.sim.heroes = Some(Heroes::new([Some(1), None]));
    walk.sim.spawn_screens = true;
    walk.sim.spawn_stage[0] = SpawnStage::Class;
    walk.sim.spawn_choice[0] = 2;
    let feed = EventFeed::default();
    match page(&walk, &feed, 0).unwrap() {
        SpawnPage::Class {
            title,
            class,
            classes,
            refused,
            ..
        } => {
            assert_eq!(title, "Ordinary second");
            assert_eq!((class, classes, refused), (1, vec![0, 2], 0));
        }
        p => panic!("{p:?}"),
    }
    // Even a stale hero highlight is displayed as an ordinary row.
    walk.sim.spawn_choice[0] = 1;
    match page(&walk, &feed, 0).unwrap() {
        SpawnPage::Class { class, classes, .. } => {
            assert_eq!((class, classes), (0, vec![0, 2]))
        }
        p => panic!("{p:?}"),
    }
    let state = &mut walk.sim.heroes.as_mut().unwrap().teams[0];
    state.unlocked = true;
    state.owner = Some(p.index);
    match page(&walk, &feed, 0).unwrap() {
        SpawnPage::Class {
            title,
            class,
            classes,
            ..
        } => {
            assert_eq!(title, "Hero");
            assert_eq!((class, classes), (1, vec![0, 1, 2]));
        }
        p => panic!("{p:?}"),
    }
}

fn fixture_class(u: &core::unit::Unit) -> core::unit::UnitClass {
    core::unit::UnitClass {
        consts: u.spec.consts,
        max_health: u.health.max,
        health_type: u.spec.health_type,
        weapon_class: u.weapon_class,
        total_clips: 6,
        weapon_slot: u.weapon_slot,
        slots: u.spec.slots.clone(),
        jet: false,
        capture_posts: true,
        drops: Default::default(),
        chunk: None,
        droideka: None,
        min: 1,
        max: None,
        unlock_points: 0,
    }
}

#[test]
fn a_second_players_dead_class_page_uses_that_slots_own_team_and_reservation() {
    let mut walk = WalkSim::new(3, Vec3::ZERO, None);
    let second = walk
        .sim
        .spawn(&core::unit::UnitSpec::player_in_slot(1, 1, Vec3::X))
        .unwrap();
    let class = fixture_class(&walk.sim.units[second.index as usize]);
    for team in 0..2 {
        walk.sim.classes[team] = vec![class.clone(), class.clone()];
        let ordinary = walk.teams[team].classes[0].clone();
        walk.teams[team].classes = vec![ordinary.clone(), ordinary];
        walk.teams[team].classes[0].label = format!("Team{team} ordinary");
        walk.teams[team].classes[1].label = format!("Team{team} hero");
    }
    walk.sim.heroes = Some(Heroes::new([Some(1), Some(1)]));
    let state = &mut walk.sim.heroes.as_mut().unwrap().teams[1];
    state.unlocked = true;
    state.owner = Some(second.index);
    assert!(walk.sim.open_spawn_screens(1, false));
    walk.sim.spawn_stage[1] = SpawnStage::Class;
    walk.sim.spawn_choice[1] = 1;
    assert!(!walk.sim.units[second.index as usize].alive);
    let feed = EventFeed::default();
    match page(&walk, &feed, 1).unwrap() {
        SpawnPage::Class {
            team,
            title,
            class,
            classes,
            ..
        } => {
            assert_eq!(
                (team, title.as_str(), class, classes),
                (1, "Team1 hero", 1, vec![0, 1])
            );
        }
        p => panic!("{p:?}"),
    }
    // Dead actors remain valid spawn-screen members; stale or absent handles do not.
    walk.sim.units[second.index as usize].gen += 1;
    assert_eq!(page(&walk, &feed, 1), None);
    walk.sim.players[1] = None;
    assert_eq!(page(&walk, &feed, 1), None);
}
