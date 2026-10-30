use super::state::Sim;
use crate::collision::HullFilter;
use crate::projectile::{Bolt, Shooter};
use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    pub shooter: Shooter,
    pub hulls: HullFilter,
    pub charge: f32,
    pub origin: Vec3,
    pub dir: Vec3,
    pub life: f32,
    pub len: f32,
    pub follow: bool,
}

impl Beam {
    pub fn hash_into(&self, eat: &mut impl FnMut(&[u8])) {
        eat(&[0xbe, u8::from(self.follow)]);
        for v in [
            self.origin.x,
            self.origin.y,
            self.origin.z,
            self.dir.x,
            self.dir.y,
            self.dir.z,
            self.life,
            self.len,
            self.charge,
        ] {
            eat(&v.to_bits().to_le_bytes());
        }
        eat(&self.shooter.owner.index.to_le_bytes());
        eat(&self.shooter.owner.gen.to_le_bytes());
        eat(&(self.shooter.weapon_class as u64).to_le_bytes());
    }
}

impl Sim {
    pub(super) fn launch_round(&mut self, round: Bolt, follow: bool) {
        let Some(class) = &self.weapon_classes[round.weapon_class].ordnance.beam else {
            self.bolts.push(round);
            return;
        };
        self.beams.push(Beam {
            shooter: round.shooter(),
            hulls: round.hulls,
            charge: round.charge,
            origin: round.pos,
            dir: round.vel.normalize_or_zero(),
            life: class.life,
            len: 0.0,
            follow,
        });
    }

    pub(super) fn fly_bolts(&mut self, dt: f32) {
        let beams = self.cast_beams(dt);
        self.sweep_bolts(dt);
        self.settle_beams(beams, dt);
        let mut done = Vec::new();
        for (i, b) in self.bolts.iter_mut().enumerate() {
            if b.advance(dt) {
                done.push(i);
            }
            if let Some(shot) = b.visual_shot.filter(|shot| shot.tick == self.tick) {
                self.shot_flights
                    .push(crate::projectile::ShotFlight { shot, end: b.end });
            }
        }
        for &i in &done {
            let b = self.bolts[i].clone();
            self.resolve_bolt(&b);
        }
        self.bolts.retain(|b| b.remaining > 0.0);
    }

    fn cast_beams(&mut self, dt: f32) -> usize {
        let first = self.bolts.len();
        for k in 0..self.beams.len() {
            let beam = self.beams[k];
            let (origin, dir) = if beam.follow {
                self.beam_frame(beam)
            } else {
                (beam.origin, beam.dir)
            };
            let ord = &self.weapon_classes[beam.shooter.weapon_class].ordnance;
            let range = ord.beam.as_ref().map_or(0.0, |c| c.range);
            let mut bolt = Bolt::fire(
                origin,
                dir,
                range / dt,
                dt,
                beam.shooter,
                self.heightfield.as_ref(),
                self.water_plane(),
                &self.collision,
                beam.hulls,
            );
            bolt.charge = beam.charge;
            self.beams[k].origin = origin;
            self.beams[k].dir = dir;
            self.bolts.push(bolt);
        }
        first
    }

    fn settle_beams(&mut self, first: usize, dt: f32) {
        for (beam, b) in self.beams.iter_mut().zip(&self.bolts[first..]) {
            beam.len = (b.end - beam.origin).length();
            beam.life -= dt;
        }
        self.beams.retain(|b| b.life > 0.0);
    }

    fn beam_frame(&self, beam: Beam) -> (Vec3, Vec3) {
        let h = beam.shooter.owner;
        if self.unit(h).is_none_or(|u| u.mount.is_some()) {
            return (beam.origin, beam.dir);
        }
        let (origin, (fwd, _, _)) = self.shot_frame(h.index as usize);
        (origin, fwd)
    }
}

#[cfg(test)]
mod tests;
