use glam::Vec3;

use super::{
    Body, Droideka, DroidekaClass, Input, Report, DEAD, DIR_BACK, DIR_FORWARD, DYING, EV_BACK,
    EV_DIE, EV_FORWARD, EV_STOP, EV_TURN_LEFT, EV_TURN_RIGHT, IDLE, LEG_UNITS, ROLLING, TURN_LEFT,
    TURN_RIGHT, WALK_BACK_HYSTERESIS, WALK_LEG_STOP, WALK_MOVE_DEADZONE_SQ,
};
use crate::fmath;
use crate::vehicle::gun::wrap;

impl Droideka {
    pub(super) fn walk(
        &mut self,
        dt: f32,
        input: &Input,
        c: &DroidekaClass,
        body: &mut Body,
        report: &mut Report,
    ) {
        let (mut fwd, mut strafe) = (input.fwd, input.strafe);
        let mut moving = fwd * fwd + strafe * strafe > WALK_MOVE_DEADZONE_SQ;
        let rate = c.max_turn_speed;
        self.turn_body(body, input.turn * dt * rate);
        let mut forward = false;
        if self.state != IDLE {
            if !matches!(self.state, TURN_LEFT | TURN_RIGHT) {
                let mut target = 0.0;
                if moving {
                    let (ls, lc) = fmath::sin_cos(self.legs / LEG_UNITS);
                    let thr = if lc * fwd - ls * strafe > 0.0 {
                        -WALK_BACK_HYSTERESIS
                    } else {
                        WALK_BACK_HYSTERESIS
                    };
                    forward = thr <= fwd;
                    if forward {
                        strafe = -strafe;
                    } else {
                        fwd = -fwd;
                    }
                    target = fmath::atan2(strafe, fwd);
                }
                let diff = wrap(target - self.legs);
                if diff.abs() > WALK_LEG_STOP {
                    forward = false;
                    moving = false;
                }
                let lim = c.max_turn_speed * dt;
                self.legs = wrap(self.legs + diff.clamp(-lim, lim));
            }
        } else {
            let mut target = 0.0;
            if moving {
                let (s, f) = if fwd >= 0.0 {
                    (-strafe, fwd)
                } else {
                    (strafe, -fwd)
                };
                target = fmath::atan2(s, f);
            }
            let diff = target - self.legs;
            if diff > c.turn_threshold {
                self.raise(EV_TURN_LEFT, c, body, report);
            } else if diff < -c.turn_threshold {
                self.raise(EV_TURN_RIGHT, c, body, report);
            } else if moving {
                forward = fwd >= 0.0;
                self.raise(if forward { EV_FORWARD } else { EV_BACK }, c, body, report);
            }
        }
        if dt <= 0.0 {
            return;
        }
        if moving {
            if forward && self.dir == DIR_BACK {
                self.dir = DIR_FORWARD;
            } else if !forward && self.dir == DIR_FORWARD {
                self.dir = DIR_BACK;
            }
        }
        let mut left = dt;
        let mut travel = 0.0;
        let s = usize::from(self.state);
        let rate = c.rate[s];
        let back = self.dir == DIR_BACK;
        let remain = if back {
            self.clip_t
        } else {
            c.duration[s] - self.clip_t
        };
        if rate * left >= remain {
            left -= remain / rate;
            self.clip_t += if back { -remain } else { remain };
            travel += self.advance(remain, back, c);
            if !self.alive {
                self.raise(EV_DIE, c, body, report);
            } else if moving {
                self.raise(if forward { EV_FORWARD } else { EV_BACK }, c, body, report);
            } else {
                self.raise(EV_STOP, c, body, report);
            }
            if self.state == ROLLING {
                return;
            }
        }
        let s = usize::from(self.state);
        let delta = left * c.rate[s];
        let back = self.dir == DIR_BACK;
        if !back || delta <= self.clip_t {
            self.clip_t += if back { -delta } else { delta };
        }
        travel += self.advance(delta, back, c);
        self.apply_walk(travel, dt, body, report);
    }

    fn advance(&mut self, delta: f32, back: bool, c: &DroidekaClass) -> f32 {
        let s = usize::from(self.state);
        match self.state {
            IDLE | DYING | DEAD | ROLLING => 0.0,
            TURN_LEFT | TURN_RIGHT => {
                self.legs = wrap(self.legs + delta * c.speed[s] * LEG_UNITS);
                0.0
            }
            _ => c.speed[s] * delta * if back { -1.0 } else { 1.0 },
        }
    }

    fn apply_walk(&mut self, travel: f32, dt: f32, body: &mut Body, report: &mut Report) {
        let heading = body.yaw + self.legs;
        let (sy, cy) = fmath::sin_cos(heading);
        let dir = Vec3::new(-sy, 0.0, -cy);
        body.pos += dir * travel;
        let v = if dt > 0.0 { travel / dt } else { 0.0 };
        body.vel.x = dir.x * v;
        body.vel.z = dir.z * v;
        report.walk_step = travel;
    }
}
