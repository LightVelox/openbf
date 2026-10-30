//! Native block pose lifecycle and the cached 0.1-second attachment pose.

use super::*;
use manifest::combo::ComboKeyword as K;

impl SaberClass {
    pub fn deflect_animations(&self, state: usize) -> Vec<(&str, u8)> {
        self.combo
            .states()
            .nth(state)
            .and_then(|s| s.find(K::Deflect))
            .into_iter()
            .flat_map(|d| d.all(K::DeflectAnimation))
            .filter_map(|a| {
                let mask = a
                    .args
                    .iter()
                    .skip(1)
                    .filter_map(|a| match a {
                        manifest::combo::ComboValue::Text(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .fold(0, |m, s| {
                        m | match s.to_ascii_lowercase().as_str() {
                            "forward" => 1,
                            "right" => 2,
                            "backright" => 4,
                            "left" => 8,
                            "backleft" => 16,
                            "back" => 32,
                            _ => 0,
                        }
                    });
                Some((a.text(0)?, mask))
            })
            .collect()
    }

    /// FUN_003448d8 caches the first Deflect list, sampling its attachment
    /// at 0.1 seconds. Every PS2 hero entry uses the primary attachment.
    pub fn deflect_attachment(&self, animation: &str) -> Option<Mat4> {
        let first = self
            .combo
            .states()
            .position(|s| s.find(K::Deflect).is_some())?;
        self.deflect_animations(first)
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case(animation))
            .then_some(())?;
        let clip = self
            .clips
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(animation))?;
        clip.weapon_frames.get(3).copied()
    }
}

impl MeleeState {
    /// FUN_0033c9e0 changes the weapon aim, leaving the owner frame alone.
    /// A Deflect state skips this alignment even when bit20 is also set.
    pub fn weapon_forward(&self, owner_aim: Vec3) -> Vec3 {
        if self.aligned_to_thrust && self.deflect.is_none() {
            crate::combo::thrust::orient(
                owner_aim,
                self.retained_thrust,
                self.current_thrust_forward,
                false,
            )
        } else {
            owner_aim
        }
    }

    pub fn blade_frame(&self, position: Vec3, yaw: f32, pitch: f32) -> Mat4 {
        let (s, c) = fmath::sin_cos(yaw);
        let (sp, cp) = fmath::sin_cos(if self.aligned_to_thrust { pitch } else { 0.0 });
        let owner_aim = Vec3::new(-s * cp, sp, -c * cp);
        let forward = if self.aligned_to_thrust {
            self.weapon_forward(owner_aim)
        } else {
            owner_aim
        };
        let right = Vec3::Y.cross(forward).normalize_or_zero();
        Mat4::from_cols(
            right.extend(0.0),
            forward.cross(right).extend(0.0),
            forward.extend(0.0),
            position.extend(1.0),
        )
    }

    pub fn remember_thrust(
        &mut self,
        combo: &manifest::combo::ComboDef,
        forward: f32,
        strafe: f32,
    ) {
        let initial = combo.states().nth(self.runtime.state)
            .and_then(|s| s.find(K::AlignedToThrust))
            .is_some_and(|a| a.args.iter().any(|arg| matches!(arg, manifest::combo::ComboValue::Text(s) if s.eq_ignore_ascii_case("Initial"))));
        self.current_thrust_forward = forward;
        if !initial {
            self.retained_thrust = [forward, strafe];
        }
    }

    /// State callbacks 0x341e28/60/70 initialise, clear and advance precision.
    pub fn advance_deflect(&mut self, entered: bool, active: bool, dt: f32) {
        if entered {
            self.block_animation = None;
            self.block_animation_time = 0.0;
            self.deflect_precision = if active { 1.0 } else { 0.0 };
        }
        if active {
            self.deflect_precision += dt;
            self.block_animation_time += dt;
            if self.deflect_precision > 1.0 {
                self.block_animation = None;
                self.block_animation_time = 0.0;
            }
        }
    }

    pub fn select_block_animation(&mut self, animation: Option<&str>) {
        if self.block_animation.as_deref() != animation {
            self.block_animation = animation.map(str::to_owned);
            self.block_animation_time = 0.0;
        }
    }

    pub fn pose(&self) -> Option<(&str, f32, bool)> {
        if let Some(animation) = self.block_animation.as_deref() {
            Some((animation, self.block_animation_time, false))
        } else {
            self.animation
                .as_deref()
                .map(|a| (a, self.runtime.time, self.animation_loop))
        }
    }
}
