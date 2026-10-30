//! Long-range bot sniping (2026-10-29, the maintainer after BF1 Yavin 4:
//! Arena): "bots never tried sniping me from the other side, so I was
//! essentially safe standing out in the open". He sniped 66-95 m post to
//! post across the arena; the bots' sight stopped at 49 m.
//!
//! A scout trooper bot on a Snipe hint by its own west post (cp4) and the
//! player standing still in the open at his own east post (cp5), about 76 m
//! across the arena floor: at Hard the bot fires its rifle at him, from its
//! spot; at Easy (the maintainer: "Easy would be what it was before") it
//! never sees him.
//!
//! Real classes and the real map from the bake; skips without them.

use core::sim::{Event, PadState};
use core::unit::Controller;
use glam::Vec3;

/// Snipe hint `hintnode2` by cp4 (the bot's side's post), and cp5's centre
/// (the player's side's post), from the bake's manifest.
const HINT: Vec3 = Vec3::new(-29.929, -51.342, 17.567);
const CP5: Vec3 = Vec3::new(45.393, -52.925, 15.080);

/// One rifle shot of the bot: the distance to the player, on its spot, at
/// the player.
struct Shot {
    d: f32,
    on: bool,
    at_player: bool,
}

/// The scene at tier `d`: the bot's first three rifle shots in 40 s.
/// `None` without the bake.
fn scene(d: core::skill::Difficulty) -> Option<Vec<Shot>> {
    let root = game::assets::baked_root();
    let map = root.join("bf1_yav2");
    let Ok(mut scene) = game::assets::load_scene(&map) else {
        eprintln!("SKIP: no bf1_yav2 bake");
        return None;
    };
    scene.apply_mode_layer("con");
    let Ok(db) = game::roster::load_classdb(&root.join("classes"), "bf1", &["all", "imp"]) else {
        eprintln!("SKIP: no baked classes");
        return None;
    };
    let mut sim = core::sim::Sim::empty(42);
    sim.difficulty = d;
    sim.ground_rules = core::soldier::GroundRules::for_game("bf1");
    sim.omni_radius = core::sim::omni_radius_for("bf1");
    let mut names = Vec::new();
    let mut team = |sim: &mut core::sim::Sim, class: &str, side: &str| {
        let roster = game::roster::ClassRoster::build(&db, class, 1, None).unwrap();
        let uc = roster.register(sim, &mut names);
        let visual = game::walk::ClassVisual::from_roster(&roster);
        let faction = game::roster::FactionRoster {
            faction: side.into(),
            classes: vec![roster.clone()],
        };
        game::walk::TeamTemplate::from_roster(&faction, &uc, vec![visual])
    };
    let t0 = team(&mut sim, "all_inf_soldier", "all");
    let t1 = team(&mut sim, "imp_inf_scout_trooper", "imp");
    let nav = game::layout::nav_graph(&scene.manifest, &scene.dir);
    let mut w =
        game::walk::WalkSim::with_teams(sim, scene.heightfield(), CP5, [t0, t1.clone()], nav);
    scene.install_collision(&mut w.sim);
    w.add_posts(&game::layout::MapLayout::from_manifest(&scene.manifest));
    let me = w.player.index as usize;
    assert_eq!(
        w.sim.ai_tier(1).scope_sight,
        d.tier().scope_sight,
        "the bot's tier"
    );
    let bot = w
        .sim
        .spawn(&t1.spec(1, HINT, 0.0, Controller::Ai, 0))
        .expect("the sniper");
    let rifle = w.sim.unit(bot).expect("alive").weapon_class;
    assert!(
        w.sim.weapon_classes[rifle].ai_distance_table(),
        "a scoped rifle in hand"
    );
    // The player 6 m in front of his post, toward the arena (the post's
    // own building stands over its centre).
    let stand = {
        let q = CP5 - Vec3::X * 6.0;
        Vec3::new(q.x, w.sim.ground_under(q + Vec3::Y * 10.0, false), q.z)
    };
    w.sim.units[me].soldier.pos = stand;
    let mut shots = Vec::new();
    for t in 0..40 * core::TICK_HZ as u64 {
        {
            let p = &mut w.sim.units[me];
            p.health.cur = p.health.max;
            p.soldier.pos.x = stand.x;
            p.soldier.pos.z = stand.z;
            p.soldier.vel = Vec3::ZERO;
        }
        w.step_tick(&[PadState::default()]);
        let Some(u) = w.sim.unit(bot) else {
            panic!("the sniper died at tick {t}");
        };
        let fired = w.sim.events.iter().any(|e| {
            matches!(*e, Event::Shot { unit, weapon_class, .. }
                if unit == bot && weapon_class as usize == rifle)
        });
        if !fired {
            continue;
        }
        let d = (w.sim.units[me].soldier.pos - u.soldier.pos).length();
        let on = u.ai.snipe.cur.is_some_and(|c| {
            c.arrived
                && Vec3::new(c.spot.x - u.soldier.pos.x, 0.0, c.spot.z - u.soldier.pos.z).length()
                    <= 1.5
        });
        println!(
            "tick {t}: shot at {d:.1} m, on its spot {on}, at {}",
            u.soldier.pos
        );
        shots.push(Shot {
            d,
            on,
            at_player: u.ai.target.and_then(|f| f.unit()) == Some(w.player),
        });
        if shots.len() == 3 {
            break;
        }
    }
    Some(shots)
}

#[test]
fn at_hard_a_sniper_bot_shoots_a_player_standing_across_the_arena() {
    let Some(shots) = scene(core::skill::Difficulty::Hard) else {
        return;
    };
    assert_eq!(shots.len(), 3, "rifle shots at the player in 40 s");
    for s in shots {
        assert!(s.at_player, "at the player");
        assert!(s.d > 60.0, "from across the arena: {} m", s.d);
        assert!(s.on, "from its spot");
    }
}

#[test]
fn at_easy_the_sniper_bot_does_not_see_him() {
    let Some(shots) = scene(core::skill::Difficulty::Easy) else {
        return;
    };
    assert!(shots.is_empty(), "{} rifle shots", shots.len());
}
