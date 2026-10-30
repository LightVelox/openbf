use super::{lerp_angle, lerp_look, mix};
use core::vehicle::{
    walker::{WalkerParams, MAX_PAIRS},
    Vehicle,
};

#[derive(Debug, Clone, PartialEq)]
pub struct WalkerPose {
    anim: u8,
    reversed: bool,
    time: f32,
    duration: f32,
    pitch: f32,
    h: (f32, f32),
    roll: (f32, f32),
    layers: [f32; MAX_PAIRS],
    aims: Vec<Option<(f32, f32)>>,
}

impl WalkerPose {
    pub(super) fn capture(v: &Vehicle, params: Option<&WalkerParams>) -> Self {
        Self {
            anim: v.walker.anim,
            reversed: v.walker.reversed,
            time: v.walker.clip_time,
            duration: params
                .and_then(|p| p.clips.get(v.walker.anim as usize))
                .and_then(Option::as_ref)
                .map_or(0.0, |c| c.duration),
            pitch: v.walker.pitch,
            h: v.walker.h,
            roll: v.walker.roll,
            layers: v.walker.layer_blend,
            aims: v.guns.iter().map(|g| g.as_ref().map(|g| g.aim)).collect(),
        }
    }

    pub(super) fn write(&self, v: &mut Vehicle) {
        v.walker.clip_time = self.time;
        v.walker.pitch = self.pitch;
        v.walker.h = self.h;
        v.walker.roll = self.roll;
        v.walker.layer_blend = self.layers;
        for (g, a) in v.guns.iter_mut().zip(&self.aims) {
            if let (Some(g), Some(a)) = (g.as_mut(), a) {
                g.aim = *a;
            }
        }
    }

    pub(super) fn blend(&mut self, prev: &Self, t: f32) {
        if self.anim == prev.anim && self.reversed == prev.reversed {
            self.time = clip_time(prev.time, self.time, self.duration, self.reversed, t);
        }
        self.pitch = mix(prev.pitch, self.pitch, t);
        self.h = pair(prev.h, self.h, t);
        self.roll = pair(prev.roll, self.roll, t);
        for (l, p) in self.layers.iter_mut().zip(prev.layers) {
            *l = mix(p, *l, t);
        }
        for (a, p) in self.aims.iter_mut().zip(&prev.aims) {
            if let (Some(a), Some(p)) = (a.as_mut(), p) {
                *a = lerp_look(*p, *a, t);
            }
        }
    }
}

fn pair(prev: (f32, f32), cur: (f32, f32), t: f32) -> (f32, f32) {
    if prev == cur || t == 1.0 {
        return cur;
    }
    if t == 0.0 {
        return prev;
    }
    let a = lerp_angle(
        core::fmath::atan2(prev.1, prev.0),
        core::fmath::atan2(cur.1, cur.0),
        t,
    );
    let (s, c) = core::fmath::sin_cos(a);
    (c, s)
}

fn clip_time(prev: f32, cur: f32, duration: f32, reversed: bool, t: f32) -> f32 {
    if t == 0.0 {
        return prev;
    }
    if t == 1.0 || prev == cur {
        return cur;
    }
    let wrap = duration > 0.0 && if reversed { cur > prev } else { cur < prev };
    if wrap {
        let end = if reversed {
            cur - duration
        } else {
            cur + duration
        };
        mix(prev, end, t).rem_euclid(duration)
    } else {
        mix(prev, cur, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_restart_takes_the_short_path_forward_and_backward() {
        assert!((clip_time(0.98, 0.02, 1.0, false, 0.25) - 0.99).abs() < 1e-6);
        assert!((clip_time(0.98, 0.02, 1.0, false, 0.75) - 0.01).abs() < 1e-6);
        assert!((clip_time(0.02, 0.98, 1.0, true, 0.25) - 0.01).abs() < 1e-6);
        assert!((clip_time(0.02, 0.98, 1.0, true, 0.75) - 0.99).abs() < 1e-6);
        assert_eq!(clip_time(0.2, 0.1, 1.0, true, 0.5), 0.15);
        assert_eq!(clip_time(0.4, 0.4, 1.0, false, 0.5), 0.4);
    }

    #[test]
    fn leg_and_roll_pairs_blend_across_the_angle_seam() {
        let a = 3.13f32;
        let b = -3.13f32;
        let p = pair((a.cos(), a.sin()), (b.cos(), b.sin()), 0.5);
        assert!((p.1.atan2(p.0).abs() - std::f32::consts::PI).abs() < 1e-5);
    }
}
