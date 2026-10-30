use super::emitter::Frame;
use super::jet::bone_frame;
use super::trail::along;
use super::Fx;
use core::sim::{Event, Sim};
use core::unit::{Unit, UnitHandle};
use glam::{Mat4, Vec3};

impl Fx {
    pub(super) fn flame_shots(&mut self, sim: &Sim) {
        for e in &sim.events {
            let Event::Shot {
                unit,
                weapon_class,
                origin,
                dir,
            } = e
            else {
                continue;
            };
            let name = sim
                .weapon_classes
                .get(*weapon_class as usize)
                .and_then(|w| w.emitter.as_ref())
                .and_then(|em| em.effect.as_deref());
            let Some(name) = name else { continue };
            let vel = sim.unit(*unit).map_or(Vec3::ZERO, |u| u.soldier.vel);
            let frame = Frame {
                rot: along(*dir),
                pos: *origin,
                vel,
            };
            self.spawn(name, frame);
        }
    }

    pub fn set_burn_frames(&mut self, frames: Vec<Vec<Option<Mat4>>>) {
        for e in &mut self.live {
            let Some((h, _, slot)) = e.burn else { continue };
            if let Some(Some(m)) = frames.get(h.index as usize).and_then(|f| f.get(slot)) {
                e.frame = bone_frame(*m, e.frame.vel);
            }
        }
        self.burn_frames = frames;
    }

    fn burn_frame(&self, h: UnitHandle, slot: usize, u: &Unit) -> Frame {
        match self
            .burn_frames
            .get(h.index as usize)
            .and_then(|f| f.get(slot))
        {
            Some(Some(m)) => bone_frame(*m, u.soldier.vel),
            _ => Frame {
                vel: u.soldier.vel,
                ..Frame::at(u.soldier.pos + Vec3::Y)
            },
        }
    }

    pub(super) fn follow_burns(&mut self, sim: &Sim) {
        let mut live = std::mem::take(&mut self.live);
        for e in &mut live {
            let Some((h, class, slot)) = e.burn else {
                continue;
            };
            let keep = sim.unit(h).filter(|u| {
                u.alive
                    && sim
                        .burn_effects(u)
                        .is_some_and(|(c, fx)| c == class && slot < fx.len())
            });
            match keep {
                Some(u) => e.frame = self.burn_frame(h, slot, u),
                None => {
                    e.stop();
                    e.burn = None;
                }
            }
        }
        self.live = live;
        for (h, u) in sim.alive_units() {
            let Some((class, effects)) = sim.burn_effects(u) else {
                continue;
            };
            for (slot, (_, name)) in effects.iter().enumerate() {
                if self.live.iter().any(|e| e.burn == Some((h, class, slot))) {
                    continue;
                }
                let frame = self.burn_frame(h, slot, u);
                if self.spawn(name, frame) {
                    if let Some(e) = self.live.last_mut() {
                        e.burn = Some((h, class, slot));
                    }
                }
            }
        }
    }

    #[cfg(test)]
    pub fn burns(&self) -> Vec<(UnitHandle, usize)> {
        self.live
            .iter()
            .filter_map(|e| e.burn.map(|(h, _, slot)| (h, slot)))
            .collect()
    }
}

#[cfg(test)]
mod tests;
