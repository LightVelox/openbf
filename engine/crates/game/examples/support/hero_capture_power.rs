//! Select every authored alternate through normal inputs and capture the
//! actual action after its warmup, including channel-zero flamethrowers.

use super::*;
use battle::tick;
use core::sim::{buttons, Event, PadState};

fn targets(walk: &mut WalkSim) -> Result<()> {
    let origin = walk.sim.unit(walk.player).unwrap().soldier.pos;
    let yaw = walk.sim.unit(walk.player).unwrap().soldier.yaw;
    for (dx, dz) in [(0.0, -2.5), (1.2, -4.0), (-1.2, -4.0)] {
        let pos = origin + Mat4::from_rotation_y(yaw).transform_vector3(Vec3::new(dx, 0.0, dz));
        let spec = walk.teams[1].spec(
            1,
            pos,
            yaw + std::f32::consts::PI,
            core::unit::Controller::Idle,
            0,
        );
        walk.sim
            .spawn(&spec)
            .context("diagnostic Force target spawn")?;
    }
    Ok(())
}

impl Capture<'_> {
    pub(super) fn power_channels(
        &mut self,
        name: &str,
        walk: &mut WalkSim,
        vis: &mut SoldierVisuals,
        fx: &mut game::fx::Fx,
        feed: &mut game::event_feed::EventFeed,
    ) -> Result<()> {
        for channel in [0, 1] {
            let count = walk
                .sim
                .unit(walk.player)
                .unwrap()
                .held
                .iter()
                .filter(|h| h.channel == channel)
                .count();
            for p in 0..count {
                for _ in 0..90 {
                    tick(walk, vis, fx, feed, PadState::default());
                }
                if walk.sim.unit(walk.player).is_none() {
                    return Ok(());
                }
                if p > 0 || channel == 0 {
                    tick(
                        walk,
                        vis,
                        fx,
                        feed,
                        PadState {
                            buttons: if channel == 0 {
                                buttons::SWITCH_WEAPON
                            } else {
                                buttons::SWITCH_SECONDARY
                            },
                            ..Default::default()
                        },
                    );
                    tick(walk, vis, fx, feed, PadState::default());
                }
                targets(walk)?;
                let base = if channel == 0 {
                    format!("primary{}", p + 1)
                } else {
                    format!("secondary{p}")
                };
                let mut saved_action = false;
                let mut saved_fx = false;
                for i in 0..120 {
                    tick(
                        walk,
                        vis,
                        fx,
                        feed,
                        PadState {
                            buttons: if i < 60 {
                                if channel == 0 {
                                    buttons::FIRE
                                } else {
                                    buttons::SECONDARY_FIRE
                                }
                            } else {
                                0
                            },
                            ..Default::default()
                        },
                    );
                    let u = &walk.sim.units[walk.player.index as usize];
                    let glow = fx.force_bolts() > 0 || fx.force_damage_effects() > 0;
                    let fired = walk.sim.events.iter().any(|e| matches!(e,
                        Event::Ability { unit, .. } | Event::Shot { unit, .. } | Event::Throw { unit, .. }
                        if *unit == walk.player));
                    let flying = std::iter::once(&u.weapon)
                        .chain(u.held.iter().map(|h| &h.state))
                        .any(|s| s.saber_flight.is_some());
                    let choking = std::iter::once(&u.weapon)
                        .chain(u.held.iter().map(|h| &h.state))
                        .filter_map(|s| s.force_channel.as_ref())
                        .any(|c| c.target.is_some());
                    if (!saved_action && (fired || flying || choking)) || (glow && !saved_fx) {
                        let phase = if saved_action {
                            format!("{base}_fx")
                        } else {
                            base.clone()
                        };
                        self.frame((name, &phase), walk, vis, fx, feed, false)?;
                        saved_action = true;
                        saved_fx |= glow;
                    }
                    if !walk.sim.units[walk.player.index as usize].alive {
                        return Ok(());
                    }
                }
                if !saved_action {
                    self.frame(
                        (name, &format!("{base}_no_action")),
                        walk,
                        vis,
                        fx,
                        feed,
                        false,
                    )?;
                }
            }
        }
        Ok(())
    }
}
