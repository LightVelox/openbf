use super::turret_pose::{aimer_node, barrel_node, turret_node};
use crate::skin::Skeleton;
use crate::vehicle_class::{SeatKind, SeatSpec};
use core::vehicle::Vehicle;
use glam::{Mat4, Vec3};

#[derive(Clone)]
enum Writer {
    Turret {
        seat: usize,
        pitch_factor: f32,
    },
    Aimer {
        seat: usize,
        yaw: [f32; 2],
        pitch: [f32; 2],
    },
    Barrel,
}

#[derive(Default)]
pub struct WalkerAim {
    writers: Vec<Option<Writer>>,
    seats: Vec<SeatSpec>,
    turret: Vec<Option<usize>>,
}

impl WalkerAim {
    pub fn new(skel: &Skeleton, seats: Vec<SeatSpec>) -> Self {
        let mut writers = vec![None; skel.len()];
        let turret: Vec<_> = seats
            .iter()
            .map(|s| {
                matches!(s.kind, SeatKind::Gunner(_))
                    .then(|| s.turret_node.as_deref().and_then(|n| skel.index_of(n)))
                    .flatten()
            })
            .collect();
        for (seat, spec) in seats.iter().enumerate() {
            if let Some(n) = turret[seat] {
                writers[n] = Some(Writer::Turret {
                    seat,
                    pitch_factor: spec.pitch_turn_factor.unwrap_or(1.0),
                });
            }
        }
        for (seat, spec) in seats.iter().enumerate().rev() {
            for a in &spec.aimers {
                if let Some(n) = skel.index_of(&a.node) {
                    writers[n] = Some(Writer::Aimer {
                        seat,
                        yaw: a.yaw.map(f32::to_radians),
                        pitch: a.pitch.map(f32::to_radians),
                    });
                }
                for (name, _) in &a.barrels {
                    if let Some(n) = skel.index_of(name) {
                        writers[n] = Some(Writer::Barrel);
                    }
                }
            }
        }
        Self {
            writers,
            seats,
            turret,
        }
    }

    pub fn pose(&self, skel: &Skeleton, walk: &[Mat4], v: &Vehicle) -> Vec<Mat4> {
        let mut now = vec![None; skel.len()];
        for n in 0..skel.len() {
            self.resolve(n, skel, walk, v, &mut now);
        }
        now.into_iter().map(Option::unwrap).collect()
    }

    fn resolve(
        &self,
        n: usize,
        skel: &Skeleton,
        walk: &[Mat4],
        v: &Vehicle,
        now: &mut [Option<Mat4>],
    ) -> Mat4 {
        if let Some(m) = now[n] {
            return m;
        }
        let parent = skel.parents[n];
        let p = parent.map_or(Mat4::IDENTITY, |p| self.resolve(p, skel, walk, v, now));
        let old_p = parent.map_or(Mat4::IDENTITY, |p| walk[p]);
        let inherited = p != old_p;
        let carried = p * skel.local_bind[n];
        let mut m = if inherited {
            p * old_p.inverse() * walk[n]
        } else {
            walk[n]
        };
        match self.writers.get(n).and_then(Option::as_ref) {
            Some(Writer::Turret { seat, pitch_factor }) => {
                if let Some(g) = v
                    .guns
                    .get(*seat)
                    .and_then(Option::as_ref)
                    .filter(|g| g.look != (0.0, 0.0) || g.aim != (0.0, 0.0))
                {
                    let look = limited_look(&self.seats[*seat], g.look);
                    m = turret_node(&carried, look.0, -look.1 * pitch_factor);
                }
            }
            Some(Writer::Aimer { seat, yaw, pitch }) => {
                if let Some(g) = v
                    .guns
                    .get(*seat)
                    .and_then(Option::as_ref)
                    .filter(|g| g.look != (0.0, 0.0) || g.aim != (0.0, 0.0))
                {
                    let spec = &self.seats[*seat];
                    let pivot = self.turret[*seat]
                        .or_else(|| spec.aimers.first().and_then(|a| skel.index_of(&a.node)))
                        .map_or(Vec3::ZERO, |k| skel.global_bind[k].w_axis.truncate());
                    let eye = pivot + Vec3::from(spec.eye_point_offset.unwrap_or([0.0; 3]));
                    let target = eye
                        + core::vehicle::gun::dir(g.aim.0, g.aim.1) * core::vehicle::gun::AIM_RAY;
                    let frame = self.turret[*seat].map_or(carried, |t| {
                        let tp = skel.parents[t]
                            .map_or(Mat4::IDENTITY, |k| self.resolve(k, skel, walk, v, now));
                        let look = limited_look(spec, g.look);
                        turret_node(&(tp * skel.local_bind[t]), look.0, -look.1)
                    });
                    let d = frame
                        .inverse()
                        .transform_vector3(target - carried.w_axis.truncate())
                        .normalize_or_zero();
                    let y = clamp_yaw(d.x.atan2(d.z), *yaw);
                    let x = d.y.clamp(-1.0, 1.0).asin().clamp(pitch[0], pitch[1]);
                    let dir = frame
                        .transform_vector3(core::vehicle::gun::dir(y, x))
                        .normalize_or_zero();
                    m = aimer_node(&p, &skel.local_bind[n], dir);
                }
            }
            Some(Writer::Barrel) if inherited => {
                m = barrel_node(&p, skel.local_bind[n].w_axis.truncate(), 0.0);
            }
            _ => {}
        }
        now[n] = Some(m);
        m
    }
}

fn clamp_yaw(mut angle: f32, limits: [f32; 2]) -> f32 {
    if limits[1] > std::f32::consts::PI && angle < 0.0 {
        angle += std::f32::consts::TAU;
    }
    angle.clamp(limits[0], limits[1])
}

fn limited_look(spec: &SeatSpec, look: (f32, f32)) -> (f32, f32) {
    let yaw = spec
        .yaw_limits
        .filter(|l| l[1] - l[0] < 360.0)
        .map_or(look.0, |l| clamp_yaw(look.0, l.map(f32::to_radians)));
    let pitch = spec
        .pitch_limits
        .filter(|l| l[1] - l[0] < 360.0)
        .map_or(look.1, |l| {
            look.1.clamp(-l[1].to_radians(), -l[0].to_radians())
        });
    (yaw, pitch)
}
