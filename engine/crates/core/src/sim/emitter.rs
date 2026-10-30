//! Emitter shots (card UORDE, `analysis/re-ordnance-kinds.md` §8): one
//! instant cone-and-chain query per weapon shot — no bolt. BF1's arc caster
//! and BF2's arc casters, incinerator and flamethrowers all arrive here; the
//! query law itself lives in [`crate::emitter`].

use super::{soldier_blast_sphere, Event, Sim};
use crate::emitter::{query, Candidate};
use crate::unit::{Controller, UnitHandle};
use crate::vehicle::VehicleHandle;
use crate::weapon::{DamageDesc, ExplosionClass};
use glam::Vec3;

/// A body the query can touch, with the handle that applies its damage.
#[derive(Clone, Copy)]
enum Victim {
    Unit(UnitHandle),
    Vehicle(VehicleHandle),
}

impl Sim {
    /// Fire weapon class `class`'s emitter for unit `i` at `charge`. The
    /// shooter's weapon path calls this instead of spawning a
    /// [`crate::projectile::Bolt`].
    pub(super) fn fire_emitter(&mut self, i: usize, class: usize, charge: f32) {
        let Some(em) = self.weapon_classes[class].emitter.clone() else {
            return;
        };
        let ord = self.weapon_classes[class].ordnance.clone();
        let blend = ord.charged(charge);
        let floor = ord.min_damage.unwrap_or(0.0);
        let unit = self.handle(i);
        let team = self.units[i].team;
        let origin = self.units[i].muzzle();
        let (aim, _, _) = self.units[i].aim_frame();
        // The pull itself is a shot like any other: the fire sound, the
        // muzzle flash and the trigger animation read this, and the arc
        // renderer pairs it with the `Emitter` links below.
        self.events.push(Event::Shot {
            unit,
            weapon_class: class as u32,
            origin,
            dir: aim,
        });

        // The gather: alive soldiers (riders included; BF1's mask has no
        // vehicles) then alive hulls (BF2). The body point is 1 m above an
        // entity's origin, the PS2's "body + 1 m".
        let mut targets = Vec::new();
        let mut victims = Vec::new();
        for (h, u) in self.alive_units() {
            if h.index as usize == i {
                continue;
            }
            targets.push(Candidate {
                pos: u.soldier.pos + Vec3::Y,
                team: Some(u.team),
                vehicle: false,
            });
            victims.push(Victim::Unit(h));
        }
        for (h, v) in self.alive_vehicles() {
            let (centre, _) = self.vehicle_blast_sphere(v);
            targets.push(Candidate {
                pos: centre + Vec3::Y,
                team: v.team,
                vehicle: true,
            });
            victims.push(Victim::Vehicle(h));
        }

        let shot = query(
            &em,
            blend.damage,
            floor,
            origin,
            aim,
            team,
            &targets,
            |from, to| crate::perception::los(self, from, to),
        );

        // BF1 forces every health-type and shield scale to 1.0; BF2 uses the
        // class's own.
        let (scales, shield_scale) = if em.bf2 {
            (blend.scales, blend.shield_scale)
        } else {
            ([1.0; 7], 1.0)
        };
        let ff = self.friendly_fire_scale;
        for hit in &shot.hits {
            let body = targets[hit.index];
            let victim = victims[hit.index];
            let desc = DamageDesc {
                damage: hit.damage,
                scales,
                shield_scale,
            };
            match victim {
                Victim::Unit(h) => {
                    let Some(friendly) = self.unit(h).map(|u| u.team == team) else {
                        continue;
                    };
                    let mult = self.buff_scale(Some(unit), h);
                    let Some(v) = self.unit_mut(h) else { continue };
                    let damage = v.health.take_damage(&desc, mult, friendly, ff);
                    if self
                        .units
                        .get(unit.index as usize)
                        .is_some_and(|o| o.gen == unit.gen && o.controller == Controller::Ai)
                    {
                        self.nav_stats.ai_hits += 1;
                    }
                    self.nav_stats.bolt_hits += 1;
                    if friendly && unit != h {
                        self.nav_stats.friend_hits += 1;
                    }
                    self.finish_unit_damage(h, Some(unit), damage, 0.0, aim);
                }
                Victim::Vehicle(h) => {
                    self.damage_vehicle_by(h, &desc, 1.0, Some(unit), Some(team), Some(aim));
                }
            }
            // BF2's `Explosion` goes off at each body hit (the Imperial arc
            // caster's chassis shock).
            if em.bf2 {
                if let Some(e) = &ord.explosion {
                    self.explode(e, body.pos, Some(unit), Some(team));
                }
            }
            // The shove (`Push`; 0 = none, and then no LCG draw either).
            if em.push != 0.0 {
                if let Victim::Unit(h) = victim {
                    if let Some(u) = self.units.get(h.index as usize) {
                        let (_, w) = soldier_blast_sphere(u.soldier.stance);
                        let e = ExplosionClass {
                            push: em.push,
                            push_outer: f32::INFINITY,
                            ..Default::default()
                        };
                        let pos = body.pos;
                        self.kick_and_push(h, &e, hit.from, pos, w);
                    }
                }
            }
            self.events.push(Event::Emitter {
                unit,
                weapon_class: class as u32,
                from: hit.from,
                to: body.pos,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::{Body, Shape};
    use crate::sim::tests::flat_ground;
    use crate::sim::Event;
    use crate::unit::UnitSpec;
    use crate::vehicle::{SeatKind, VehicleKind, VehicleSpec};
    use crate::weapon::{EmitterOrdnance, HealthType, OrdnanceClass, ShotSpec, WeaponClass, DEG};
    use glam::Mat4;

    fn bf1() -> EmitterOrdnance {
        EmitterOrdnance {
            cone_length: 30.0,
            cone_angle: 50.0 * DEG,
            max_targets: 4,
            jump_deduction: 75.0,
            damage_threshold: 75.0,
            max_jump_distance: 20.0,
            first_radius: 0.6,
            push: 0.0,
            lightning: Some("arccaster_lightning".into()),
            affect_friends: true,
            affect_enemies: true,
            hit_all: false,
            radius: 0.0,
            bf2: false,
        }
    }

    fn bf2() -> EmitterOrdnance {
        EmitterOrdnance {
            cone_length: 30.0,
            cone_angle: 50.0 * DEG,
            max_targets: 5,
            jump_deduction: 5.0,
            damage_threshold: 300.0,
            max_jump_distance: 20.0,
            first_radius: 1.2,
            push: 3.0,
            lightning: Some("com_sfx_lightning".into()),
            affect_friends: true,
            affect_enemies: true,
            hit_all: false,
            radius: 0.0,
            bf2: true,
        }
    }

    /// `com_weap_inf_flamethrower_ord`: Damage 5, MaxDamage 15, hit-all.
    fn flame() -> EmitterOrdnance {
        EmitterOrdnance {
            cone_length: 10.0,
            cone_angle: 45.0 * DEG,
            max_targets: 10,
            jump_deduction: 0.0,
            damage_threshold: 0.0,
            max_jump_distance: 0.0,
            first_radius: 2.0,
            push: 0.0,
            lightning: Some("com_sfx_flamethrower".into()),
            affect_friends: true,
            affect_enemies: true,
            hit_all: true,
            radius: 0.0,
            bf2: true,
        }
    }

    fn sim_with(em: EmitterOrdnance, ord: OrdnanceClass) -> (Sim, usize) {
        let mut s = Sim::empty(7);
        s.heightfield = Some(flat_ground());
        let class = s.add_weapon_class(WeaponClass {
            emitter: Some(em),
            ordnance: ord,
            ..Default::default()
        });
        (s, class)
    }

    fn arc_ord() -> OrdnanceClass {
        OrdnanceClass {
            max_damage: 250.0,
            min_damage: Some(100.0),
            scales: [1.0; 7],
            shield_scale: 1.0,
            ..Default::default()
        }
    }

    /// A player at `at` aiming +X (yaw -π/2).
    fn shooter(s: &mut Sim, at: Vec3) -> UnitHandle {
        let h = s.spawn(&UnitSpec::player(0, at)).unwrap();
        s.units[h.index as usize].soldier.yaw = -std::f32::consts::FRAC_PI_2;
        h
    }

    fn line(s: &mut Sim, n: usize) -> Vec<UnitHandle> {
        (0..n)
            .map(|k| {
                s.spawn(&UnitSpec::trooper(
                    1,
                    Vec3::new(6.0 + 3.0 * k as f32, 0.0, 0.0),
                ))
                .unwrap()
            })
            .collect()
    }

    fn hits(s: &Sim) -> Vec<(u32, f32)> {
        s.events
            .iter()
            .filter_map(|e| match e {
                Event::Hit { victim, damage, .. } => Some((victim.index, *damage)),
                _ => None,
            })
            .collect()
    }

    /// The BF1 clone pilot's arc caster: 1 target under charge 1/3, 2 under
    /// 5/6, 3 above.
    #[test]
    fn bf1_arc_caster_takes_one_two_or_three_by_charge() {
        for (charge, want) in [(0.3, 1), (0.5, 2), (0.9, 3)] {
            let (mut s, class) = sim_with(bf1(), arc_ord());
            let unit = shooter(&mut s, Vec3::ZERO);
            line(&mut s, 3);
            s.fire_emitter(unit.index as usize, class, charge);
            let n = s
                .events
                .iter()
                .filter(|e| matches!(e, Event::Emitter { .. }))
                .count();
            assert_eq!(n, want, "charge {charge}");
        }
    }

    #[test]
    fn bf1_full_charge_deals_250_175_100_down_a_line() {
        let (mut s, class) = sim_with(bf1(), arc_ord());
        let unit = shooter(&mut s, Vec3::ZERO);
        let at = line(&mut s, 3);
        s.fire_emitter(unit.index as usize, class, 1.0);
        let got = hits(&s);
        let want = [
            (at[0].index, 250.0),
            (at[1].index, 175.0),
            (at[2].index, 100.0),
        ];
        assert_eq!(got, want);
        assert_eq!(s.unit(at[0]).unwrap().health.cur, 50.0);
        assert_eq!(s.unit(at[1]).unwrap().health.cur, 125.0);
        assert_eq!(s.unit(at[2]).unwrap().health.cur, 200.0);
    }

    /// A wall between the first and second bodies skips the second link; the
    /// chain still reaches a third body it can see.
    #[test]
    fn a_wall_skips_one_link_without_ending_the_chain() {
        let (mut s, class) = sim_with(bf1(), arc_ord());
        let unit = shooter(&mut s, Vec3::ZERO);
        let at = line(&mut s, 2);
        let far = s
            .spawn(&UnitSpec::trooper(1, Vec3::new(15.0, 0.0, 3.5)))
            .unwrap();
        s.collision.push_body(Body::new(
            Mat4::from_translation(Vec3::new(8.7, 1.0, 0.0)),
            Shape::Box {
                half: Vec3::new(0.2, 2.0, 0.6),
            },
        ));
        s.fire_emitter(unit.index as usize, class, 1.0);
        assert_eq!(
            hits(&s),
            vec![(at[0].index, 250.0), (far.index, 175.0)],
            "the middle body is behind the wall"
        );
        assert_eq!(s.unit(at[1]).unwrap().health.cur, 300.0);
    }

    /// BF1 has no team test: an ally in the cone is zapped too.
    #[test]
    fn bf1_zaps_an_ally() {
        let (mut s, class) = sim_with(bf1(), arc_ord());
        let unit = shooter(&mut s, Vec3::ZERO);
        let ally = s
            .spawn(&UnitSpec::trooper(0, Vec3::new(6.0, 0.0, 0.0)))
            .unwrap();
        s.fire_emitter(unit.index as usize, class, 1.0);
        assert_eq!(hits(&s), vec![(ally.index, 250.0)]);
        assert_eq!(s.unit(ally).unwrap().health.cur, 50.0);
    }

    #[test]
    fn bf2_hit_all_splits_between_two_bodies() {
        let (mut s, class) = sim_with(
            flame(),
            OrdnanceClass {
                max_damage: 15.0,
                min_damage: Some(5.0),
                ..Default::default()
            },
        );
        let unit = shooter(&mut s, Vec3::ZERO);
        let a = s
            .spawn(&UnitSpec::trooper(1, Vec3::new(3.0, 0.0, 0.0)))
            .unwrap();
        let b = s
            .spawn(&UnitSpec::trooper(1, Vec3::new(4.0, 0.0, 0.0)))
            .unwrap();
        s.fire_emitter(unit.index as usize, class, 1.0);
        assert_eq!(hits(&s), vec![(a.index, 7.5), (b.index, 7.5)]);
        assert_eq!(s.unit(a).unwrap().health.cur, 292.5);
        assert_eq!(s.unit(b).unwrap().health.cur, 292.5);
    }

    #[test]
    fn bf2_affect_enemies_false_leaves_the_enemy_alone() {
        let mut em = bf2();
        em.affect_enemies = false;
        let (mut s, class) = sim_with(
            em,
            OrdnanceClass {
                max_damage: 300.0,
                min_damage: Some(300.0),
                ..Default::default()
            },
        );
        let unit = shooter(&mut s, Vec3::ZERO);
        let enemy = s
            .spawn(&UnitSpec::trooper(1, Vec3::new(6.0, 0.0, 0.0)))
            .unwrap();
        let ally = s
            .spawn(&UnitSpec::trooper(0, Vec3::new(9.0, 0.0, 0.0)))
            .unwrap();
        s.fire_emitter(unit.index as usize, class, 1.0);
        assert_eq!(hits(&s), vec![(ally.index, 300.0)]);
        assert_eq!(s.unit(enemy).unwrap().health.cur, 300.0);
    }

    #[test]
    fn bf2_hits_a_vehicle_and_bf1_does_not() {
        let v = |pos| VehicleSpec {
            kind: VehicleKind::Hover,
            class: 0,
            home_team: None,
            neutral_when_empty: true,
            spawn_pad: None,
            pos,
            yaw: 0.0,
            max_health: 500.0,
            health_type: HealthType::Vehicle,
            seats: vec![SeatKind::Pilot],
        };
        let (mut s, class) = sim_with(bf1(), arc_ord());
        let unit = shooter(&mut s, Vec3::ZERO);
        let h = s.spawn_vehicle(&v(Vec3::new(6.0, 0.0, 0.0)));
        s.fire_emitter(unit.index as usize, class, 1.0);
        assert_eq!(s.vehicle(h).unwrap().health.cur, 500.0);

        let (mut s, class) = sim_with(
            bf2(),
            OrdnanceClass {
                max_damage: 300.0,
                min_damage: Some(300.0),
                ..Default::default()
            },
        );
        let unit = shooter(&mut s, Vec3::ZERO);
        let h = s.spawn_vehicle(&v(Vec3::new(6.0, 0.0, 0.0)));
        s.fire_emitter(unit.index as usize, class, 1.0);
        assert_eq!(s.vehicle(h).unwrap().health.cur, 200.0);
    }

    /// A zero-damage cone (the Force push / pull) stays on the bolt path:
    /// the instant query would early-out and drop its blast.
    #[test]
    fn a_zero_damage_emitter_keeps_its_bolt() {
        let (mut s, class) = sim_with(
            flame(),
            OrdnanceClass {
                max_damage: 0.0,
                ..Default::default()
            },
        );
        let unit = shooter(&mut s, Vec3::ZERO);
        s.units[unit.index as usize].weapon_class = class;
        s.soldier_shots(
            unit.index as usize,
            None,
            &[ShotSpec {
                tilt: (0.0, 0.0),
                charge: 1.0,
                stage: 0,
                pitch_spread: 0.0,
                yaw_spread: 0.0,
            }],
        );
        assert_eq!(s.bolts.len(), 1);
        assert!(!s.events.iter().any(|e| matches!(e, Event::Emitter { .. })));
    }

    /// The weapon path routes an emitter class here, not to a bolt.
    #[test]
    fn an_emitter_shot_goes_through_soldier_shots_without_a_bolt() {
        let (mut s, class) = sim_with(bf1(), arc_ord());
        let unit = shooter(&mut s, Vec3::ZERO);
        s.units[unit.index as usize].weapon_class = class;
        let target = s
            .spawn(&UnitSpec::trooper(1, Vec3::new(6.0, 0.0, 0.0)))
            .unwrap();
        s.soldier_shots(
            unit.index as usize,
            None,
            &[ShotSpec {
                tilt: (0.0, 0.0),
                charge: 1.0,
                stage: 0,
                pitch_spread: 0.0,
                yaw_spread: 0.0,
            }],
        );
        assert!(s.bolts.is_empty(), "an emitter spawns no bolt");
        assert_eq!(s.unit(target).unwrap().health.cur, 50.0);
        assert!(s.events.iter().any(|e| matches!(e, Event::Emitter { .. })));
        assert_eq!(
            s.events
                .iter()
                .filter(|e| matches!(e, Event::Shot { .. }))
                .count(),
            1,
            "the pull is one shot event (the fire sound and the animation)"
        );
    }
}
