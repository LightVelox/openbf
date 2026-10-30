use core::collision::{CollisionWorld, HullFilter};
use core::projectile::{Bolt, Shooter, ShotFlight, VisualShot};
use core::sim::Event;
use game::bolt_vis::BoltVis;
use game::walk::WalkSim;
use glam::Vec3;

fn scene() -> (WalkSim, usize) {
    let mut walk = WalkSim::new(42, Vec3::ZERO, None);
    let class = walk.sim.unit(walk.player).unwrap().weapon_class;
    let look = core::laser::LaserLook {
        length: 100.0,
        glow_length: 100.0,
        blur_length: 100.0,
        ..core::laser::LaserLook::rifle()
    };
    walk.walker_shot_styles.insert(
        class,
        game::walker_shots::Style {
            tracer: Some(look),
            fire_sound: None,
            frequency: 1,
        },
    );
    walk.sim.events.clear();
    (walk, class)
}

fn bolt(walk: &WalkSim, class: usize, direction: Vec3) -> Bolt {
    let mut b = Bolt::fire(
        Vec3::ZERO,
        direction,
        100.0,
        1.0,
        Shooter {
            owner: walk.player,
            team: 0,
            weapon_class: class,
            is_player: true,
        },
        None,
        None,
        &CollisionWorld::default(),
        HullFilter::All,
    );
    b.end = direction * 100.0;
    b
}

fn trace(with_old_bolt: bool) -> (Vec3, Vec3) {
    let (mut walk, class) = scene();
    if with_old_bolt {
        let mut old = bolt(&walk, class, Vec3::X);
        old.advance(0.1);
        walk.sim.bolts.push(old);
    }
    walk.sim.bolts.push(bolt(&walk, class, Vec3::Z));
    walk.sim.events.push(Event::Shot {
        unit: walk.player,
        weapon_class: class as u32,
        origin: Vec3::ZERO,
        dir: Vec3::Z,
    });
    let mut vis = BoltVis::default();
    vis.track(&walk);
    let lines = vis.lines();
    assert_eq!(lines.len(), 1);
    lines[0]
}

#[test]
fn a_new_tracer_uses_its_own_endpoint_while_an_older_round_is_flying() {
    let control = trace(false);
    assert!(
        control.1.abs_diff_eq(Vec3::Z * 100.0, 0.001),
        "control {control:?}"
    );
    let actual = trace(true);
    assert!(
        actual.1.abs_diff_eq(Vec3::Z * 100.0, 0.001),
        "actual {actual:?}"
    );
}

#[path = "../../examples/support/walker_battle.rs"]
mod walker_battle;
use core::sim::{buttons, PadState};
use core::vehicle::VehicleHandle;

#[test]
#[ignore = "requires local BF2 Geonosis assets"]
fn a_secondary_atte_shot_uses_its_own_fire_point_after_the_primary_has_fired_more() {
    let root = game::assets::baked_root();
    let (mut w, _) = walker_battle::battle(&root).unwrap();
    let class = w
        .vehicle_classes
        .iter()
        .position(|c| c == "rep_walk_atte")
        .unwrap();
    let (i, v) = w
        .sim
        .vehicles
        .iter()
        .enumerate()
        .find(|(_, v)| v.alive && v.class as usize == class)
        .unwrap();
    let h = VehicleHandle {
        index: i as u32,
        gen: v.gen,
    };
    assert!(w.sim.seat_unit(w.player, h, 0));
    for _ in 0..60 {
        w.step_tick(&[PadState {
            buttons: buttons::FIRE,
            ..Default::default()
        }]);
    }
    w.step_tick(&[PadState::default()]);
    let secondary = w.sim.seat_setup[class].as_ref().unwrap().guns[0]
        .as_ref()
        .unwrap()
        .weapons[1]
        .class;
    let mut shot_seen = false;
    for _ in 0..60 {
        w.step_tick(&[PadState {
            buttons: buttons::SECONDARY_FIRE,
            ..Default::default()
        }]);
        if w.sim.events.iter().any(|e| matches!(e,Event::Shot{unit,weapon_class,..} if *unit==w.player && *weapon_class as usize==secondary)) {
            shot_seen=true;
            break;
        }
    }
    assert!(shot_seen);
    let mut vis = game::soldier_vis::SoldierVisuals::load_cpu(&root, "bf2", &w.teams);
    vis.set_walker_rigs(w.walker_rigs.clone());
    vis.update(&w, core::TICK_DT);
    let hash = w.sim.hash();
    let count = w.sim.vehicle(h).unwrap().guns[0].as_ref().unwrap().weapons[1].shots_fired;
    let expected = game::walker_shots::frame(&w, w.player, secondary, count.saturating_sub(1))
        .unwrap()
        .w_axis
        .truncate();
    let actual = vis.fire_points(&w)[w.player.index as usize].unwrap();
    let primary = w.sim.seat_setup[class].as_ref().unwrap().guns[0]
        .as_ref()
        .unwrap()
        .weapons[0]
        .class;
    let primary_count = w.sim.vehicle(h).unwrap().guns[0].as_ref().unwrap().weapons[0].shots_fired;
    let wrong = game::walker_shots::frame(&w, w.player, primary, primary_count.saturating_sub(1))
        .unwrap()
        .w_axis
        .truncate();
    assert!(expected.distance(wrong) > 0.1, "barrels must differ");
    assert_eq!(w.sim.hash(), hash);
    assert!(actual.abs_diff_eq(expected,0.001),"actual {actual:?}, expected secondary {expected:?}, primary {wrong:?}, counts primary={primary_count} secondary={count}");
}

#[test]
fn another_shooters_nearby_parallel_impact_cannot_take_the_new_tracer() {
    let (mut w, class) = scene();
    w.sim.events = vec![
        Event::Shot {
            unit: w.player,
            weapon_class: class as u32,
            origin: Vec3::ZERO,
            dir: Vec3::Z,
        },
        Event::Impact {
            pos: Vec3::new(0.2, 0.0, 10.0),
            dir: Vec3::Z,
            weapon_class: class as u32,
            surface: core::impact::Surface::Building,
            vehicle: None,
        },
    ];
    let mut vis = BoltVis::default();
    vis.track(&w);
    assert!(vis.lines()[0].1.abs_diff_eq(Vec3::Z * 100.0, 0.001));
}

#[test]
fn two_identical_shots_cannot_both_take_one_impact() {
    let (mut w, class) = scene();
    w.sim.weapon_classes[class].ordnance.velocity = 1000.0;
    for _ in 0..2 {
        w.sim.events.push(Event::Shot {
            unit: w.player,
            weapon_class: class as u32,
            origin: Vec3::ZERO,
            dir: Vec3::Z,
        });
    }
    w.sim.events.push(Event::Impact {
        pos: Vec3::Z * 10.0,
        dir: Vec3::Z,
        weapon_class: class as u32,
        surface: core::impact::Surface::Building,
        vehicle: None,
    });
    let mut vis = BoltVis::default();
    vis.track(&w);
    let lines = vis.lines();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].1.abs_diff_eq(Vec3::Z * 10.0, 0.001));
    assert!(lines[1].1.abs_diff_eq(Vec3::Z * 100.0, 0.001));
}

#[test]
fn almost_parallel_or_neighboring_shots_keep_their_own_impact_endpoints() {
    for (origins, directions) in [
        (
            [Vec3::ZERO; 2],
            [Vec3::Z, (Vec3::Z + Vec3::X * 0.000005).normalize()],
        ),
        ([Vec3::ZERO, Vec3::X * 0.02], [Vec3::Z; 2]),
    ] {
        let (mut w, class) = scene();
        w.sim.weapon_classes[class].ordnance.velocity = 1000.0;
        let ends = [
            origins[0] + directions[0] * 10.0,
            origins[1] + directions[1] * 5.0,
        ];
        for k in 0..2 {
            w.sim.events.push(Event::Shot {
                unit: w.player,
                weapon_class: class as u32,
                origin: origins[k],
                dir: directions[k],
            });
        }
        for k in 0..2 {
            w.sim.events.push(Event::Impact {
                pos: ends[k],
                dir: directions[k],
                weapon_class: class as u32,
                surface: core::impact::Surface::Building,
                vehicle: None,
            });
        }
        let mut vis = BoltVis::default();
        vis.track(&w);
        let lines = vis.lines();
        assert_eq!(lines.len(), 2);
        for (actual, expected) in lines.iter().zip(ends) {
            assert!(
                actual.1.abs_diff_eq(expected, 1e-6),
                "shot's own endpoint {:?} != {expected:?}",
                actual.1
            );
        }
    }
}

#[test]
fn recorded_identical_shots_keep_their_own_first_flight_endpoints() {
    for tracer in [false, true] {
        let (mut w, class) = scene();
        w.sim.tick = 1;
        w.sim.weapon_classes[class].ordnance.velocity = 1000.0;
        if !tracer {
            w.walker_shot_styles.clear();
        }
        for (event, distance) in [10.0, 5.0].into_iter().enumerate() {
            w.sim.events.push(Event::Shot {
                unit: w.player,
                weapon_class: class as u32,
                origin: Vec3::ZERO,
                dir: Vec3::Z,
            });
            w.sim.shot_flights.push(ShotFlight {
                shot: VisualShot { tick: 0, event },
                end: Vec3::Z * distance,
            });
        }
        for distance in [10.0, 5.0] {
            w.sim.events.push(Event::Impact {
                pos: Vec3::Z * distance,
                dir: Vec3::Z,
                weapon_class: class as u32,
                surface: core::impact::Surface::Building,
                vehicle: None,
            });
        }
        let hash = w.sim.hash();
        let mut vis = BoltVis::default();
        vis.track(&w);
        assert_eq!(w.sim.hash(), hash);
        let lines = vis.lines();
        assert_eq!(lines.len(), 2);
        for (actual, distance) in lines.iter().zip([10.0, 5.0]) {
            assert!(
                actual.1.abs_diff_eq(Vec3::Z * distance, 0.001),
                "recorded first-flight endpoint was reassigned"
            );
        }
    }
}

#[test]
#[ignore = "requires local BF2 Geonosis assets"]
fn simultaneous_near_parallel_atte_rounds_keep_distinct_muzzles_between_ticks() {
    for (difference, first_hits, later_hit) in [
        (0.0, false, false),
        (0.001, false, false),
        (0.000005, true, false),
        (0.0, true, false),
        (0.0, false, true),
    ] {
        simultaneous_atte_rounds(difference, first_hits, later_hit);
    }
}

fn simultaneous_atte_rounds(difference: f32, first_hits: bool, later_hit: bool) {
    let (mut w, _) = walker_battle::battle(&game::assets::baked_root()).unwrap();
    let class = w
        .vehicle_classes
        .iter()
        .position(|c| c == "rep_walk_atte")
        .unwrap();
    let (i, v) = w
        .sim
        .vehicles
        .iter()
        .enumerate()
        .find(|(_, v)| v.alive && v.class as usize == class)
        .unwrap();
    let h = VehicleHandle {
        index: i as u32,
        gen: v.gen,
    };
    assert!(w.sim.seat_unit(w.player, h, 0));
    // Use the actual two-barrel secondary salvo, with two controlled close directions.
    let gun = w.sim.seat_setup[class].as_ref().unwrap().guns[0]
        .as_ref()
        .unwrap()
        .weapons[1]
        .class;
    // Boarding activates the seat gun on its first update.
    for _ in 0..60 {
        w.step_tick(&[PadState {
            buttons: buttons::SECONDARY_FIRE,
            ..Default::default()
        }]);
        if w.sim.events.iter().any(|e| {
            matches!(e, Event::Shot {unit, weapon_class, ..}
            if *unit == w.player && *weapon_class as usize == gun)
        }) {
            break;
        }
    }
    assert_eq!(w.sim.weapon_classes[gun].salvo_count, 2);
    let count = w.sim.vehicle(h).unwrap().guns[0].as_ref().unwrap().weapons[1].shots_fired;
    assert_eq!(
        count, 2,
        "the actual secondary fires both barrels in one tick"
    );
    let origin = w.sim.seat_look(w.player).unwrap().muzzle;
    let directions = [Vec3::Z, (Vec3::Z + Vec3::X * difference).normalize()];
    // Exercise persistent bolt tracking as well as the real salvo and hardpoints.
    // Identical rounds must keep separate starts on the following tick.
    w.walker_shot_styles.remove(&gun);
    w.sim.weapon_classes[gun].ordnance.laser = Some(core::laser::LaserLook::rifle());
    w.sim.bolts.clear();
    w.sim.events.clear();
    w.sim.shot_flights.clear();
    for direction in directions {
        let mut b = bolt(&w, gun, direction);
        let shot = VisualShot {
            tick: w.sim.tick.saturating_sub(1),
            event: w.sim.events.len(),
        };
        b.visual_shot = Some(shot);
        b.pos = origin;
        b.end = origin + direction * 100.0;
        b.advance(core::TICK_DT);
        w.sim.shot_flights.push(ShotFlight { shot, end: b.end });
        w.sim.bolts.push(b);
        w.sim.events.push(Event::Shot {
            unit: w.player,
            weapon_class: gun as u32,
            origin,
            dir: direction,
        });
    }
    let expected: Vec<_> = (0..2)
        .map(|k| {
            game::walker_shots::frame(&w, w.player, gun, k)
                .unwrap()
                .w_axis
                .truncate()
        })
        .collect();
    let impact = origin + directions[0] * 2.0;
    let survivor_end = w.sim.bolts[1].end;
    if first_hits {
        w.sim.bolts.remove(0);
        w.sim.shot_flights[0].end = impact;
        w.sim.events.push(Event::Impact {
            pos: impact,
            dir: directions[0],
            weapon_class: gun as u32,
            surface: core::impact::Surface::Building,
            vehicle: None,
        });
        let look = core::laser::LaserLook {
            length: 1000.0,
            ..core::laser::LaserLook::rifle()
        };
        w.walker_shot_styles.insert(
            gun,
            game::walker_shots::Style {
                tracer: Some(look),
                fire_sound: None,
                frequency: 1,
            },
        );
    }
    assert!(expected[0].distance(expected[1]) > 0.1);
    let mut visual =
        game::soldier_vis::SoldierVisuals::load_cpu(&game::assets::baked_root(), "bf2", &w.teams);
    visual.set_walker_rigs(w.walker_rigs.clone());
    visual.update(&w, core::TICK_DT);
    let mut vis = BoltVis::default();
    vis.set_fire_points(visual.fire_points(&w));
    let hash = w.sim.hash();
    vis.track(&w);
    assert_eq!(w.sim.hash(), hash);
    for ((start, _), expected) in vis.lines().iter().zip(&expected) {
        assert!(
            start.abs_diff_eq(*expected, 0.001),
            "distinct salvo muzzle {start:?} != {expected:?}"
        );
    }
    assert_eq!(vis.lines().len(), 2);
    if first_hits {
        // Only the almost-parallel second round survived. Each tracer still
        // owns its barrel and its own live/removed endpoint.
        assert!(
            vis.lines()[0].1.abs_diff_eq(impact, 0.001),
            "removed first tracer used surviving second endpoint"
        );
        assert!(vis.lines()[1].1.abs_diff_eq(survivor_end, 0.001));
        return;
    }
    w.sim.events.clear();
    w.sim.shot_flights.clear();
    w.sim.tick += 1;
    for b in &mut w.sim.bolts {
        b.advance(core::TICK_DT);
    }
    if later_hit {
        let first = w.sim.bolts.remove(0);
        w.sim.events.push(Event::Impact {
            pos: first.pos,
            dir: directions[0],
            weapon_class: gun as u32,
            surface: core::impact::Surface::Building,
            vehicle: None,
        });
    }
    vis.track(&w);
    assert_eq!(vis.lines().len(), 2);
    if later_hit {
        assert!(
            vis.lines()[0].0.abs_diff_eq(expected[1], 0.001),
            "surviving second round took the first barrel's continuation"
        );
        assert!(vis.lines()[1].0.abs_diff_eq(expected[0], 0.001));
        return;
    }
    for ((start, _), expected) in vis.lines().iter().zip(&expected) {
        assert!(
            start.abs_diff_eq(*expected, 0.001),
            "continued {start:?} != {expected:?}"
        );
    }
}
