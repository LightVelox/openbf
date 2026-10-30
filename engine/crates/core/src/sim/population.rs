//! Optional combatant limits, including reserved human slots.
use super::{Event, Sim};

impl Sim {
    /// Change the combatant target without spending tickets or removing humans.
    pub fn set_combatants(&mut self, counts: [u32; 2]) {
        self.team_combatants = Some(counts);
        for team in 0..2u8 {
            let alive = self.battle.teams[team as usize].alive_ai;
            let surplus = alive.saturating_sub(self.ai_limit(team));
            self.battle.teams[team as usize].alive_ai -= surplus;
            let start = self.events.len();
            self.shed_ai(team, surplus);
            // A reserved human slot can move to the other team later.
            // Suppress replenishment at the limit, but retain these roster slots.
            for event in &self.events[start..] {
                if let Event::Despawn { unit } = event {
                    self.units[unit.index as usize].respawn_at =
                        Some(self.tick + 5 * u64::from(crate::TICK_HZ));
                }
            }
        }
    }

    /// A seated human reserves one combatant slot even while awaiting respawn.
    pub fn ai_limit(&self, team: u8) -> u32 {
        if team > 1 {
            return self.ai_alive_cap;
        }
        self.team_combatants.map_or(self.ai_alive_cap, |counts| {
            let humans = self
                .players
                .iter()
                .flatten()
                .filter(|h| {
                    self.units
                        .get(h.index as usize)
                        .is_some_and(|u| u.gen == h.gen && u.team == team)
                })
                .count() as u32;
            counts[usize::from(team.min(1))].saturating_sub(humans)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::tests::flat_ground;
    use crate::unit::{Controller, UnitSpec};
    use glam::Vec3;

    #[test]
    fn combatant_limits_reserve_dead_humans_and_shed_without_spending_tickets() {
        let mut s = Sim::empty(9);
        s.heightfield = Some(flat_ground());
        let human = s.spawn(&UnitSpec::player(0, Vec3::ZERO)).unwrap();
        let mut bot = UnitSpec::trooper(0, Vec3::X * 10.0);
        bot.controller = Controller::Ai;
        let first = s.spawn(&bot).unwrap();
        bot.pos.x = 20.0;
        s.spawn(&bot).unwrap();
        let tickets = s.battle.teams[0].tickets();
        s.set_combatants([2, 3]);
        assert_eq!((s.ai_limit(0), s.ai_limit(1)), (1, 3));
        assert!(s.unit(human).is_some());
        assert!(
            s.unit(first).is_none(),
            "surplus bot removed through normal despawn path"
        );
        assert_eq!(s.battle.teams[0].alive_ai, 1);
        assert!(
            s.units[first.index as usize].respawn_at.is_some(),
            "a moved human slot can be refilled later"
        );
        assert_eq!(s.battle.teams[0].tickets(), tickets);
        s.kill_environment(human, crate::sim::DeathCause::Suicide);
        assert_eq!(s.ai_limit(0), 1, "dead human still reserves its slot");
        s.set_combatants([4, 3]);
        let new = s.spawn_roster_slot(&bot).unwrap();
        assert!(new.index > first.index);
        assert_eq!(s.units[first.index as usize].gen, first.gen);
        assert!(s.units[first.index as usize].respawn_at.is_some());

        assert_eq!(
            s.ai_limit(2),
            s.ai_alive_cap,
            "natives keep their own policy"
        );
    }
}
