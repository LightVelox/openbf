//! BF2's secondary ordnance (`analysis/re-ordnance-kinds.md` §7): a weapon's
//! `SecondaryOrdnanceName` / `SecondaryOrdnancePeriod` (B2:324171-324181) is a
//! TRACER bolt spawned beside the primary round every `period`-th round - the
//! shotgun's one per pellet, the chaingun's one every 5th shot. The round is a
//! real [`Bolt`] of its own class (`MaxDamage 0`, its own speed, LifeSpan and
//! `LaserTexture`), so the drawn streak and the flight both come from the one
//! object; it flies the primary's ray (the PS2 spawns both from one packet, so
//! the port draws no extra LCG step) and does no damage.

use crate::collision::HullFilter;
use crate::projectile::{Bolt, Hit, Shooter};
use crate::weapon::Secondary;
use glam::Vec3;

use crate::sim::state::Sim;

impl Sim {
    /// Spawn the secondary round for round `round` (1-based) of the class at
    /// `stage`, on the primary's `origin` / `dir`. `false` when the class
    /// carries none, when the round is not its multiple, or when the class is
    /// not a tracer (a real second round is not modelled yet: the sonic
    /// blaster's sonic blast).
    ///
    /// The bolt is the tracer class's own: its Velocity and LifeSpan, its
    /// damage (0) and its laser look. The press ray's stop is dropped - the
    /// round is a drawn line, so it flies its whole reach even through a wall
    /// the primary stopped on - and [`Hit::None`] lets [`Sim::sweep_bolts`]
    /// meet soldiers and hulls on the way.
    pub(super) fn fire_secondary(
        &mut self,
        who: Shooter,
        stage: usize,
        round: u32,
        origin: Vec3,
        dir: Vec3,
    ) -> bool {
        let Some(sec) = self.weapon_classes[stage].secondary.as_ref() else {
            return false;
        };
        if !spawns_secondary(sec, round) {
            return false;
        }
        let ord = &sec.ordnance;
        let mut bolt = Bolt::fire(
            origin,
            dir,
            ord.velocity,
            ord.life_span,
            Shooter {
                weapon_class: sec.index,
                ..who
            },
            self.heightfield.as_ref(),
            self.water_plane(),
            &self.collision,
            HullFilter::Static,
        );
        bolt.hit = Hit::None;
        bolt.remaining = ord.life_span;
        bolt.end = origin + dir * (ord.velocity * ord.life_span);
        self.bolts.push(bolt);
        true
    }
}

/// Is `sec` a registered tracer and is `round` its multiple? The whole spawn
/// rule in one place.
fn spawns_secondary(sec: &Secondary, round: u32) -> bool {
    sec.index != usize::MAX && sec.ordnance.is_tracer() && sec.on_round(round)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::state::Event;
    use crate::sim::PadState;
    use crate::unit::{UnitHandle, UnitSpec};
    use crate::weapon::{OrdnanceClass, Secondary, ShotSpec, WeaponClass};

    /// A test gun: the BF1 rifle's bolt as the primary, a 0-damage 200 m/s
    /// 0.05 s tracer as its secondary (the disc tracers' shape).
    fn gun(period: u32, damage: f32) -> WeaponClass {
        let mut w = WeaponClass::bf1_rifle();
        w.name = format!("test_gun_{period}_{damage}");
        w.secondary = Some(Secondary {
            name: format!("test_tracer_{period}_{damage}"),
            ordnance: OrdnanceClass {
                velocity: 200.0,
                life_span: 0.05,
                max_damage: damage,
                ..OrdnanceClass::default()
            },
            period,
            index: usize::MAX,
        });
        w
    }

    fn scene(period: u32, damage: f32) -> (Sim, UnitHandle, usize, usize) {
        let mut s = Sim::empty(9);
        let gun = s.add_weapon_class(gun(period, damage));
        let spec = UnitSpec {
            weapon_class: gun,
            ..UnitSpec::player(0, Vec3::ZERO)
        };
        let h = s.spawn(&spec).unwrap();
        let tracer = s.weapon_classes[gun].secondary.as_ref().unwrap().index;
        (s, h, gun, tracer)
    }

    fn shot() -> ShotSpec {
        ShotSpec {
            tilt: (0.0, 0.0),
            charge: 1.0,
            stage: 0,
            pitch_spread: 0.0,
            yaw_spread: 0.0,
        }
    }

    #[test]
    fn period_one_rides_a_tracer_on_every_round() {
        let (mut s, h, gun, tracer) = scene(1, 0.0);
        // Registration: the tracer owns a class entry with its own data, and
        // the weapon's `Secondary` carries its index.
        assert_eq!(s.weapon_classes[tracer].name, "test_tracer_1_0");
        assert!(s.weapon_classes[tracer].ordnance.is_tracer());
        assert_eq!(
            s.weapon_classes[gun].secondary.as_ref().unwrap().index,
            tracer
        );
        s.soldier_shots(h.index as usize, None, &[shot()]);
        assert_eq!(
            s.bolts.iter().map(|b| b.weapon_class).collect::<Vec<_>>(),
            vec![gun, tracer]
        );
        s.soldier_shots(h.index as usize, None, &[shot()]);
        assert_eq!(
            s.bolts.iter().map(|b| b.weapon_class).collect::<Vec<_>>(),
            vec![gun, tracer, gun, tracer]
        );
        assert_eq!(s.unit(h).unwrap().weapon.secondary_rounds, 2);
    }

    #[test]
    fn period_five_rides_only_the_fifth_round() {
        let (mut s, h, gun, tracer) = scene(5, 0.0);
        // One burst of five rounds: the 5th is the tracer's (the note's
        // "every 5th shot"; the first four carry none).
        s.soldier_shots(h.index as usize, None, &[shot(); 5]);
        assert_eq!(
            s.bolts.iter().map(|b| b.weapon_class).collect::<Vec<_>>(),
            vec![gun, gun, gun, gun, gun, tracer]
        );
        assert_eq!(s.unit(h).unwrap().weapon.secondary_rounds, 5);
        // The next burst's first round is the 6th of the weapon: no tracer.
        s.soldier_shots(h.index as usize, None, &[shot()]);
        assert_eq!(s.bolts.last().map(|b| b.weapon_class), Some(gun));
    }

    #[test]
    fn the_tracer_does_no_damage_and_dies_at_its_life_end() {
        let (mut s, h, _, tracer) = scene(1, 0.0);
        s.soldier_shots(h.index as usize, None, &[shot()]);
        assert_eq!(s.bolts[1].weapon_class, tracer);
        assert_eq!(
            s.weapon_classes[tracer]
                .ordnance
                .damage_desc(0.0, false)
                .damage,
            0.0
        );
        let mut expired = false;
        // Its LifeSpan is 0.05 s: two ticks at 30 Hz (the primary rifle's is
        // 1.5 s and still in the air).
        for _ in 0..4 {
            s.step(&[PadState::default()]);
            expired |= s
                .events
                .iter()
                .any(|e| matches!(e, Event::BoltExpired { .. }));
        }
        assert!(expired, "the tracer must end its flight in the air");
        assert!(s.bolts.iter().all(|b| b.weapon_class != tracer));
        assert!(s.bolts.iter().any(|b| b.weapon_class != tracer));
    }

    #[test]
    fn no_secondary_class_spawns_nothing_extra() {
        let mut s = Sim::new(11);
        s.soldier_shots(0, None, &[shot()]);
        assert_eq!(s.bolts.len(), 1);
        assert_eq!(s.bolts[0].weapon_class, 0);
        assert_eq!(s.units[0].weapon.secondary_rounds, 0);
        assert_eq!(s.weapon_classes.len(), 1);
    }

    #[test]
    fn a_real_second_round_is_not_spawned_as_a_tracer() {
        // The probe droid's sonic blaster names a 100-damage round; the gate
        // is `is_tracer`, so it must not double that round.
        let (mut s, h, gun, _) = scene(1, 100.0);
        assert_eq!(
            s.weapon_classes[gun].secondary.as_ref().unwrap().index,
            usize::MAX
        );
        s.soldier_shots(h.index as usize, None, &[shot()]);
        assert_eq!(
            s.bolts.iter().map(|b| b.weapon_class).collect::<Vec<_>>(),
            vec![gun]
        );
    }
}
