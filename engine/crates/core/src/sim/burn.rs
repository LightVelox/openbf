use super::Sim;
use crate::unit::UnitHandle;
use crate::weapon::DamageDesc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Burn {
    pub timer: f32,
    pub rate: f32,
    pub by: UnitHandle,
    pub weapon_class: usize,
}

impl Burn {
    pub fn hash_into(&self, eat: &mut impl FnMut(&[u8])) {
        eat(&[0xb7]);
        eat(&self.timer.to_bits().to_le_bytes());
        eat(&self.rate.to_bits().to_le_bytes());
        eat(&self.by.index.to_le_bytes());
        eat(&self.by.gen.to_le_bytes());
        eat(&(self.weapon_class as u64).to_le_bytes());
    }
}

impl Sim {
    pub fn burn_effects(&self, u: &crate::unit::Unit) -> Option<(usize, &[(String, String)])> {
        let burn = u.burn?;
        let em = self
            .weapon_classes
            .get(burn.weapon_class)?
            .emitter
            .as_ref()?;
        Some((burn.weapon_class, em.burn.effects.as_slice()))
    }

    pub(super) fn ignite(&mut self, victim: UnitHandle, by: UnitHandle, class: usize) {
        let Some(em) = self.weapon_classes[class].emitter.as_ref() else {
            return;
        };
        let (time, rate) = (em.burn.time, em.burn.rate);
        if !em.bf2 || time <= 0.0 {
            return;
        }
        let Some(u) = self.unit_mut(victim) else {
            return;
        };
        if u.droideka.is_some() {
            return;
        }
        let timer = u.burn.map_or(0.0, |b| b.timer).max(time);
        u.burn = Some(Burn {
            timer,
            rate,
            by,
            weapon_class: class,
        });
    }

    pub(super) fn tick_burns(&mut self, dt: f32) {
        for i in 0..self.units.len() {
            let Some(mut burn) = self.units[i].burn else {
                continue;
            };
            if !self.units[i].alive {
                self.units[i].burn = None;
                continue;
            }
            burn.timer = (burn.timer - dt).max(0.0);
            if burn.timer <= 0.0 {
                self.units[i].burn = None;
                continue;
            }
            self.units[i].burn = Some(burn);
            if burn.rate == 0.0 {
                continue;
            }
            let victim = self.handle(i);
            let attacker = self
                .units
                .get(burn.by.index as usize)
                .filter(|u| u.gen == burn.by.gen)
                .map(|u| u.team);
            let friendly = attacker == Some(self.units[i].team) && burn.by != victim;
            let scale = self.buff_scale(Some(burn.by), victim);
            let invincible = self.units[i].is_player && self.cheats.invincible;
            let (ff, now) = (self.friendly_fire_scale, self.tick);
            let u = &mut self.units[i];
            if !invincible {
                u.cloak.strength = 0.0;
                u.health.take_damage(
                    &DamageDesc {
                        damage: burn.rate * dt,
                        scales: [1.0; 7],
                        shield_scale: 1.0,
                    },
                    scale,
                    friendly,
                    ff,
                );
            }
            u.last_hit = Some((burn.by, now));
            if !u.health.alive() {
                self.unit_killed(victim, Some(burn.by));
                self.units[i].burn = None;
            }
        }
    }
}

#[cfg(test)]
mod tests;
