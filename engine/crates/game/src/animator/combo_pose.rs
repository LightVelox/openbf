//! Combo states use their own authored clip clocks, shared with blade damage.

use super::*;

impl Animator {
    pub fn status_pose(
        &mut self,
        clips: &[Clip],
        skel: &Skeleton,
        family: &str,
        seconds: f32,
    ) -> bool {
        let own = format!("{family}_choking");
        let Some(clip) = clips.iter().find(|c| {
            [own.as_str(), "human_choking"].iter().any(|name| {
                c.name.eq_ignore_ascii_case(name)
                    || c.name.eq_ignore_ascii_case(&format!("{name}_full"))
            })
        }) else {
            return false;
        };
        self.kept = Some(clip.sample(skel, seconds * 30.0, false));
        self.combo_override = true;
        self.combo_aim = false;
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn combo_pose(
        &mut self,
        rig: &Rig,
        clips: &[Clip],
        skel: &Skeleton,
        class: &ClassAnim,
        weapon: &str,
        animation: &str,
        seconds: f32,
        looping: bool,
    ) -> bool {
        let bank = class.bank_of(weapon);
        let bank = bank.split_once('_').map_or(bank, |(_, b)| b);
        let upper = crate::hero_blades::clip_for(clips, &class.family, bank, animation, "upper");
        let lower = crate::hero_blades::clip_for(clips, &class.family, bank, animation, "lower");
        let Some(base) = lower.or(upper) else {
            return false;
        };
        let mut pose = base.sample(skel, seconds * 30.0, looping);
        if let Some(upper) = upper {
            let sampled = upper.sample(skel, seconds * 30.0, looping);
            for (i, mask) in rig.upper.iter().enumerate() {
                if *mask {
                    pose.0[i] = sampled.0[i];
                }
            }
        }
        self.kept = Some(pose);
        self.combo_override = true;
        self.combo_aim =
            rig.game == Game::Bf2 && class.custom.upper(animation).aim != aim::AimKind::None;
        true
    }
}
