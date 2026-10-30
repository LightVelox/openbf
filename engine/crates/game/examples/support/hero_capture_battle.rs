//! Actual roster/hero-manager/input script for the offscreen hero capture.

use super::*;
use core::sim::{buttons, PadState};
use game::{
    roster::{ClassRoster, FactionRoster},
    walk::{ClassVisual, TeamTemplate},
};

fn templates(sim: &mut core::sim::Sim, rosters: &[FactionRoster; 2]) -> [TeamTemplate; 2] {
    let mut names = Vec::new();
    std::array::from_fn(|team| {
        sim.classes[team] = rosters[team]
            .classes
            .iter()
            .map(|c| c.register(sim, &mut names))
            .collect();
        let visuals = rosters[team]
            .classes
            .iter()
            .map(|c| {
                let mut v = ClassVisual::from_roster(c);
                v.held_hud = c.held.iter().map(|h| (h.slot, h.hud.clone())).collect();
                v.held_models = c
                    .held
                    .iter()
                    .map(|h| (h.slot, h.geometry.clone()))
                    .collect();
                v
            })
            .collect();
        TeamTemplate::from_roster(&rosters[team], &sim.classes[team][0], visuals)
    })
}

pub(super) fn tick(
    walk: &mut WalkSim,
    vis: &mut SoldierVisuals,
    fx: &mut game::fx::Fx,
    feed: &mut game::event_feed::EventFeed,
    pad: PadState,
) {
    walk.step_tick(&[pad]);
    fx.ingest(walk);
    fx.advance_frame(Some(vis), walk, &pad, core::TICK_DT);
    feed.tick(&walk.sim, walk.player);
    if walk.sim.events.iter().any(|e| {
        matches!(
            e,
            core::sim::Event::Ability { .. }
                | core::sim::Event::ForceHit { .. }
                | core::sim::Event::Hit { .. }
                | core::sim::Event::Kill { .. }
                | core::sim::Event::EnvDeath { .. }
        )
    }) {
        println!(
            "tick_trace {}",
            serde_json::json!({
                "tick":walk.sim.tick,"buttons":pad.buttons,"lightning":fx.force_bolts(),
                "burn":fx.force_damage_effects(),"events":walk.sim.events.iter().map(|e|format!("{e:?}")).collect::<Vec<_>>()
            })
        );
    }
}

impl Capture<'_> {
    pub(super) fn hero(&mut self, name: &str, db: &game::classdb::ClassDb) -> Result<()> {
        println!("hero_begin {name}");
        let side = name.split('_').next().unwrap();
        let enemy = if ["all", "rep"].contains(&side) {
            "cis"
        } else {
            "rep"
        };
        let (mut rosters, warnings) =
            FactionRoster::pair(&self.root.join("classes"), "bf2", [side, enemy], None)?;
        if !warnings.is_empty() {
            eprintln!("{name}: ordinary roster warnings: {warnings:?}");
            std::fs::write(
                self.out.join(format!("{name}_ordinary_warnings.json")),
                serde_json::to_vec_pretty(&warnings)?,
            )?;
        }
        rosters[0].classes.truncate(1);
        rosters[1].classes.truncate(1);
        rosters[0]
            .classes
            .push(ClassRoster::build(db, name, 0, Some(1))?);
        let mut sim = core::sim::Sim::empty(42);
        sim.ground_rules = core::soldier::GroundRules::Bf2;
        sim.ai_alive_cap = 0;
        self.scene.install_water(&mut sim);
        let teams = templates(&mut sim, &rosters);
        game::heroes::install(&mut sim, [Some(1), None]);
        let spawn = game::gallery::ground_view(&self.scene).1;
        let mut walk = WalkSim::with_teams(sim, self.scene.heightfield(), spawn, teams, None);
        self.scene.install_collision(&mut walk.sim);
        let layout = game::layout::MapLayout::from_manifest(&self.scene.manifest);
        walk.add_posts(&layout);
        let mut vis = SoldierVisuals::for_walk(&mut self.renderer, self.root, &self.scene, &walk);
        let mut fx = game::fx::Fx::for_walk(self.root, &self.scene.dir, "bf2", &walk);
        self.bolts = game::bolt_vis::BoltVis::new(self.root, "bf2");
        let mut feed = game::event_feed::EventFeed::for_walk(self.root, "bf2", &walk);
        // Explicit diagnostic intervention: score authority cache staged once.
        let manager = walk.sim.heroes.as_mut().unwrap();
        manager.points.resize(walk.sim.units.len(), 0);
        manager.points[walk.player.index as usize] = match manager.rules.unlock {
            core::sim::heroes::HeroUnlock::Points(n) => n,
            core::sim::heroes::HeroUnlock::Timer(_) => 0,
        };
        tick(&mut walk, &mut vis, &mut fx, &mut feed, PadState::default());
        if walk.sim.hero_offer(0).is_none() {
            return Err(anyhow!("{name}: manager did not offer hero"));
        }
        if matches!(name, "rep_hero_yoda" | "rep_hero_macewindu") {
            self.frame((name, "offer"), &walk, &mut vis, &mut fx, &feed, false)?;
        }
        tick(
            &mut walk,
            &mut vis,
            &mut fx,
            &mut feed,
            PadState {
                buttons: buttons::HERO_ACCEPT,
                ..Default::default()
            },
        );
        if !walk.sim.is_hero(walk.player) {
            return Err(anyhow!("{name}: offer was not accepted"));
        }
        for _ in 0..120 {
            tick(&mut walk, &mut vis, &mut fx, &mut feed, PadState::default());
        }
        self.frame((name, "idle"), &walk, &mut vis, &mut fx, &feed, true)?;
        if matches!(name, "imp_hero_darthvader" | "rep_hero_cloakedanakin") {
            self.preview(name, &walk, &vis)?;
        }
        let mut active = false;
        let mut active_ticks = 0;
        for i in 0..90 {
            tick(
                &mut walk,
                &mut vis,
                &mut fx,
                &mut feed,
                PadState {
                    buttons: if i == 0 { buttons::FIRE } else { 0 },
                    ..Default::default()
                },
            );
            let u = walk.sim.unit(walk.player).unwrap();
            let saber = walk.sim.weapon_classes[u.weapon_class].saber.is_some();
            let hit = u
                .weapon
                .melee
                .as_ref()
                .is_some_and(|m| !m.attacks.is_empty());
            active_ticks += usize::from(hit);
            if (saber && active_ticks >= 5) || (!saber && i == 0) {
                self.frame((name, "attack"), &walk, &mut vis, &mut fx, &feed, false)?;
                if matches!(
                    name,
                    "rep_hero_yoda" | "cis_hero_grievous" | "imp_hero_darthvader"
                ) {
                    self.frame(
                        (name, "attack_front"),
                        &walk,
                        &mut vis,
                        &mut fx,
                        &feed,
                        true,
                    )?;
                    self.frame((name, "attack_side"), &walk, &mut vis, &mut fx, &feed, true)?;
                }
                active = true;
                break;
            }
        }
        if !active {
            return Err(anyhow!("{name}: FIRE did not reach a visible action"));
        }
        for _ in 0..60 {
            tick(&mut walk, &mut vis, &mut fx, &mut feed, PadState::default());
        }
        if walk.sim.weapon_classes[walk.sim.unit(walk.player).unwrap().weapon_class]
            .saber
            .is_some()
        {
            let mut saved = false;
            for _ in 0..60 {
                tick(
                    &mut walk,
                    &mut vis,
                    &mut fx,
                    &mut feed,
                    PadState {
                        buttons: buttons::RELOAD,
                        ..Default::default()
                    },
                );
                if walk
                    .sim
                    .unit(walk.player)
                    .unwrap()
                    .weapon
                    .melee
                    .as_ref()
                    .is_some_and(|m| m.deflect.is_some())
                {
                    self.frame((name, "block"), &walk, &mut vis, &mut fx, &feed, true)?;
                    saved = true;
                    break;
                }
            }
            if !saved {
                return Err(anyhow!("{name}: block never reached authored deflect"));
            }
        }
        self.saber_contact(name, &mut walk, &mut vis, &mut fx, &mut feed)?;
        self.throw_and_catch(name, &mut walk, &mut vis, &mut fx, &mut feed)?;
        self.power_channels(name, &mut walk, &mut vis, &mut fx, &mut feed)?;
        // Model/cloth diagnostic of the real hero death transition.
        walk.sim
            .kill_environment(walk.player, core::sim::DeathCause::Suicide);
        for _ in 0..12 {
            tick(&mut walk, &mut vis, &mut fx, &mut feed, PadState::default());
        }
        self.frame((name, "death"), &walk, &mut vis, &mut fx, &feed, false)?;
        if matches!(
            name,
            "rep_hero_yoda" | "cis_hero_grievous" | "imp_hero_darthvader"
        ) {
            self.frame((name, "death_front"), &walk, &mut vis, &mut fx, &feed, true)?;
            self.frame((name, "death_side"), &walk, &mut vis, &mut fx, &feed, true)?;
            for (ticks, phase) in [(18, "death1p00"), (60, "death3p00"), (36, "death4p20")] {
                for _ in 0..ticks {
                    tick(&mut walk, &mut vis, &mut fx, &mut feed, PadState::default());
                }
                self.frame((name, phase), &walk, &mut vis, &mut fx, &feed, false)?;
                if phase == "death3p00" {
                    self.frame(
                        (name, "death3p00_front"),
                        &walk,
                        &mut vis,
                        &mut fx,
                        &feed,
                        true,
                    )?;
                    self.frame(
                        (name, "death3p00_side"),
                        &walk,
                        &mut vis,
                        &mut fx,
                        &feed,
                        true,
                    )?;
                }
            }
        }
        Ok(())
    }
}

impl Capture<'_> {
    /// A target on the blade: frames before, at and after the first contact,
    /// with the authored HitEffect live. A saber without contact is an error.
    fn saber_contact(
        &mut self,
        name: &str,
        walk: &mut WalkSim,
        vis: &mut SoldierVisuals,
        fx: &mut game::fx::Fx,
        feed: &mut game::event_feed::EventFeed,
    ) -> Result<()> {
        let u = walk.sim.unit(walk.player).unwrap().clone();
        if walk.sim.weapon_classes[u.weapon_class].saber.is_none() {
            return Ok(());
        }
        let pos = u.soldier.pos
            + Mat4::from_rotation_y(u.soldier.yaw).transform_vector3(Vec3::new(0.0, 0.0, -1.2));
        let spec = walk.teams[1].spec(
            1,
            pos,
            u.soldier.yaw + std::f32::consts::PI,
            core::unit::Controller::Idle,
            0,
        );
        walk.sim.spawn(&spec).context("contact target")?;
        for i in 0..90 {
            let before = fx.live().len();
            tick(
                walk,
                vis,
                fx,
                feed,
                PadState {
                    buttons: if i % 20 == 0 { buttons::FIRE } else { 0 },
                    ..Default::default()
                },
            );
            let contact = walk
                .sim
                .events
                .iter()
                .any(|e| matches!(e, core::sim::Event::SaberContact { .. }));
            if contact {
                println!(
                    "contact tick {} live {before} -> {}",
                    walk.sim.tick,
                    fx.live().len()
                );
                self.frame((name, "contact0"), walk, vis, fx, feed, false)?;
                for n in 1..4 {
                    tick(walk, vis, fx, feed, PadState::default());
                    tick(walk, vis, fx, feed, PadState::default());
                    self.frame((name, &format!("contact{n}")), walk, vis, fx, feed, false)?;
                }
                // The authored DeflectEffect, staged: the event is written here
                // and the sim's own block is what the unit tests prove.
                let wc = walk.sim.unit(walk.player).unwrap().weapon_class as u32;
                walk.sim.events = vec![core::sim::Event::SaberContact {
                    pos: pos + Vec3::Y * 1.2,
                    weapon_class: wc,
                    deflect: true,
                }];
                fx.ingest(walk);
                for n in 0..3 {
                    tick(walk, vis, fx, feed, PadState::default());
                    tick(walk, vis, fx, feed, PadState::default());
                    self.frame((name, &format!("deflect{n}")), walk, vis, fx, feed, false)?;
                }
                return Ok(());
            }
        }
        Err(anyhow!("{name}: the swing never touched its target"))
    }

    /// A class with a melee throw: frames of the throw clip and of the catch
    /// state, through the real secondary fire.
    fn throw_and_catch(
        &mut self,
        name: &str,
        walk: &mut WalkSim,
        vis: &mut SoldierVisuals,
        fx: &mut game::fx::Fx,
        feed: &mut game::event_feed::EventFeed,
    ) -> Result<()> {
        let unit = |w: &WalkSim| w.sim.unit(w.player).cloned();
        let Some(u) = unit(walk) else { return Ok(()) };
        let classes = &walk.sim.weapon_classes;
        if !u
            .held
            .iter()
            .any(|h| classes[h.class].saber_throw.is_some())
        {
            return Ok(());
        }
        for _ in 0..u.held.len() {
            let current = unit(walk).unwrap();
            let selected = current
                .cur1
                .and_then(|k| current.held.get(usize::from(k)))
                .map(|h| h.class);
            if selected.is_some_and(|c| walk.sim.weapon_classes[c].saber_throw.is_some()) {
                break;
            }
            tick(
                walk,
                vis,
                fx,
                feed,
                PadState {
                    buttons: buttons::SWITCH_SECONDARY,
                    ..Default::default()
                },
            );
            tick(walk, vis, fx, feed, PadState::default());
        }
        for _ in 0..120 {
            tick(walk, vis, fx, feed, PadState::default());
        }
        // A target down the throw's line: the flight stops on him and returns.
        for k in 0..walk.sim.units.len() {
            if walk.sim.units[k].team == 1 {
                walk.sim.units[k].soldier.pos.y += 400.0;
            }
        }
        let u2 = unit(walk).unwrap();
        let ahead = u2.soldier.pos
            + Mat4::from_rotation_y(u2.soldier.yaw).transform_vector3(Vec3::new(0.0, 0.0, -15.0));
        let spec = walk.teams[1].spec(
            1,
            ahead,
            u2.soldier.yaw + std::f32::consts::PI,
            core::unit::Controller::Idle,
            0,
        );
        walk.sim.spawn(&spec).context("throw target")?;
        let mut thrown = false;
        for i in 0..12 {
            let press = if i < 2 { buttons::SECONDARY_FIRE } else { 0 };
            tick(
                walk,
                vis,
                fx,
                feed,
                PadState {
                    buttons: press,
                    ..Default::default()
                },
            );
            if i == 7 {
                self.frame((name, "throw"), walk, vis, fx, feed, true)?;
                self.frame((name, "throw_side"), walk, vis, fx, feed, true)?;
                thrown = true;
            }
        }
        if !thrown {
            return Err(anyhow!("{name}: no throw frame"));
        }
        let mut shot_contact = false;
        for n in 0..150 {
            let contact_frame = n > 0 && !shot_contact && {
                let hit = walk
                    .sim
                    .events
                    .iter()
                    .any(|e| matches!(e, core::sim::Event::SaberThrowHit { .. }));
                hit
            };
            if contact_frame {
                shot_contact = true;
                self.frame((name, "throw_contact"), walk, vis, fx, feed, false)?;
            }
            tick(walk, vis, fx, feed, PadState::default());
            let caught = unit(walk)
                .and_then(|u| u.weapon.melee)
                .is_some_and(|m| m.animation.as_deref() == Some("stand_catch"));
            if caught {
                for _ in 0..6 {
                    tick(walk, vis, fx, feed, PadState::default());
                }
                self.frame((name, "catch"), walk, vis, fx, feed, true)?;
                return Ok(());
            }
        }
        Err(anyhow!("{name}: the catch state never played"))
    }
}
