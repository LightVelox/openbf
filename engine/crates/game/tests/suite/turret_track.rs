//! Playtest 2026-10-29, BF2 Geonosis: "when trying to walk around the
//! [cannon] when there is an enemy in it, its rotational movement is
//! extremely jumpy and jittery. Like it jumps back and forth while I'm
//! trying to walk around it."
//!
//! A bot gunner in the Geonosian Turret (`tur_bldg_geoturret`) with a human
//! 5 m away, standing still for 2 s and then walking round it at 4 m/s. The
//! drawn gun is the aimer node, which looks along `GunState::aim`
//! (`game::viewer::turret_pose`, card VTURD). The aimer measured the
//! barrel's line from the muzzle, which swings with the aim; the cannon's
//! muzzle sits 2.5 m ahead of its pivot, so from 5 m each step asked for a
//! direction past the target and the aimer flipped about 4 deg each tick
//! (+5.4 / -2.3 deg while he walked, +-3.9 deg while he stood). Measured
//! from the line's point nearest the pivot (the same line), it settles and
//! follows him one way round.

use super::turret_damage::{spawn, turrets, world};
use core::sim::PadState;
use core::unit::Controller;
use glam::Vec3;

fn wrap_deg(d: f32) -> f32 {
    (d + 180.0).rem_euclid(360.0) - 180.0
}

#[test]
fn geonosis_cannon_follows_a_circling_human_without_jitter() {
    let Some(mut w) = world("bf2_geo1", ["rep", "cis"]) else {
        return;
    };
    let near = Vec3::new(142.28, -63.11, -0.63);
    let vh = turrets(&w, "tur_bldg_geoturret")
        .into_iter()
        .find(|&h| w.sim.vehicle(h).unwrap().pos.distance(near) < 1.0)
        .expect("the playtest's cannon");
    let v = w.sim.vehicle(vh).unwrap().clone();
    let gunner = spawn(&mut w, "cis_inf_rifleman", 1, v.pos, Controller::Ai);
    assert!(w.sim.seat_unit(gunner, vh, 0));
    for k in 0..4 {
        let far = v.pos + Vec3::new(400.0 + 3.0 * k as f32, 300.0, 0.0);
        spawn(&mut w, "cis_inf_rifleman", 1, far, Controller::Ai);
    }
    let r = 5.0;
    let at = |a: f32, w: &super::turret_damage::World| {
        let mut p = v.pos + Vec3::new(a.sin(), 0.0, a.cos()) * r;
        p.y = w.sim.spawn_ground(p);
        p
    };
    let p0 = at(0.0, &w);
    let me = spawn(
        &mut w,
        "rep_inf_ep2_jettrooper",
        0,
        p0,
        Controller::Player { slot: 0 },
    );
    w.sim.cheats.invincible = true;
    let (mut a, mut prev) = (0.0f32, None::<f32>);
    let (mut still_worst, mut backward, mut moved) = (0.0f32, 0, 0.0f32);
    // The bug flipped the gun about 4 degrees each tick; the bot's own aim
    // error moves it once a think (every 5 ticks), up to about 1.5 degrees.
    let (mut flips, mut last_d) = (0, 0.0f32);
    for k in 0..700 {
        // He spawns untargeted for a second (`dfocus::UNTARGETED`): the
        // gunner takes him at tick 30 and settles before the standing window.
        if k >= 150 {
            a += 4.0 / r * core::TICK_DT;
        }
        let p = at(a, &w);
        if let Some(u) = w.sim.unit_mut(me) {
            u.soldier.pos = p;
            u.soldier.vel = Vec3::ZERO;
        }
        w.sim.step(&[PadState::default()]);
        let aim = w.sim.vehicle(vh).unwrap().guns[0]
            .as_ref()
            .map(|g| g.aim.0.to_degrees());
        if let (Some(y), Some(q)) = (aim, prev) {
            let d = wrap_deg(y - q);
            if d.abs() > 0.5 && last_d.abs() > 0.5 && d.signum() != last_d.signum() {
                flips += 1;
            }
            last_d = d;
            if (120..150).contains(&k) {
                still_worst = still_worst.max(d.abs());
            }
            if k >= 160 {
                moved += d;
                if d < -0.5 {
                    backward += 1;
                }
            }
        }
        prev = aim;
    }
    eprintln!(
        "geo1 cannon aimer: standing, worst step {still_worst:.2} deg; walking, {backward} \
         backward steps, turned {moved:.0} deg"
    );
    assert!(moved > 600.0, "the gun followed him round: {moved}");
    assert!(
        still_worst < 2.0,
        "the aimer jitters on a standing man: {still_worst}"
    );
    assert_eq!(flips, 0, "the aimer flips back and forth tick to tick");
    assert_eq!(
        backward, 0,
        "the aimer jumps back and forth as he walks round"
    );
}
