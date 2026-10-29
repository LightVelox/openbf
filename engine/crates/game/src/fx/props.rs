//! Static prop effects at their baked hardpoint frames, including repeat delays.
use super::{EffectDef, Frame, Lcg, Library, LiveEffect};
use glam::{Mat3, Mat4, Vec3};
use std::sync::Arc;

pub(super) struct Attached {
    def: Arc<EffectDef>,
    frame: Frame,
    delay: Option<[f32; 2]>,
    wait: f32,
    finished: bool,
    rng: Lcg,
    pub(super) live: Option<LiveEffect>,
}

impl Attached {
    fn new(def: Arc<EffectDef>, frame: Frame, delay: Option<[f32; 2]>, seed: u32) -> Self {
        let mut rng = Lcg(seed);
        let wait = delay
            .filter(|[lo, hi]| *lo >= 0.0 && *hi > 0.0)
            .map_or(0.0, |[_, hi]| rng.float() * hi);
        let live = (wait <= 0.0).then(|| LiveEffect::new(def.clone(), frame, &mut rng));
        Self {
            def,
            frame,
            delay,
            wait,
            finished: false,
            rng,
            live,
        }
    }
    pub(super) fn step(&mut self, dt: f32) {
        if let Some(live) = &mut self.live {
            live.advance(dt);
            if live.is_dead() {
                self.live = None;
                match self.delay {
                    Some([lo, hi]) => {
                        let lo = lo.trunc().max(0.0);
                        self.wait = lo + self.rng.float() * (hi - lo).max(0.0);
                    }
                    None => self.finished = true,
                }
            }
            return;
        }
        if self.finished {
            return;
        }
        self.wait -= dt;
        if self.wait <= 0.0 {
            self.live = Some(LiveEffect::new(self.def.clone(), self.frame, &mut self.rng));
        }
    }
}

pub(super) fn frame(matrix: Mat4, local: &manifest::ManifestAttachedEffect) -> Frame {
    let axes = Mat3::from_cols_array_2d(&local.axes);
    let full = matrix
        * Mat4::from_cols(
            axes.x_axis.extend(0.0),
            axes.y_axis.extend(0.0),
            axes.z_axis.extend(0.0),
            Vec3::from(local.pos).extend(1.0),
        );
    Frame {
        pos: full.w_axis.truncate(),
        rot: Mat3::from_mat4(full),
        vel: Vec3::ZERO,
    }
}

pub(super) fn load(scene: &crate::assets::Scene, library: &Library) -> Vec<Attached> {
    let mut out = Vec::new();
    let mut missing = std::collections::HashSet::new();
    let mut rng = Lcg(crate::classdb::fnv1a_lowercase(&scene.manifest.map));
    let paths = scene
        .manifest
        .foliage
        .as_ref()
        .map_or(&[][..], |f| &f.path_props[..]);
    for inst in scene.manifest.instances.iter().chain(paths) {
        let Some(class) = scene
            .manifest
            .classes
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(&inst.class))
        else {
            continue;
        };
        let matrix = crate::viewer::instance_matrix(inst.rotation, inst.translation);
        for effect in &class.effects {
            if let Some(def) = library.get(&effect.effect) {
                out.push(Attached::new(
                    def.clone(),
                    frame(matrix, effect),
                    effect.delay,
                    rng.step(),
                ));
            } else if missing.insert(effect.effect.to_ascii_lowercase()) {
                eprintln!("prop effect {}: not in the map's library", effect.effect);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hardpoint_and_instance_axes_both_place_the_effect() {
        let local = manifest::ManifestAttachedEffect {
            effect: "fixture".into(),
            hardpoint: "hp".into(),
            pos: [2.0, 0.0, 0.0],
            axes: Mat3::from_rotation_x(1.0).to_cols_array_2d(),
            delay: None,
        };
        let matrix = Mat4::from_rotation_translation(
            glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(10.0, 3.0, 5.0),
        );
        let f = frame(matrix, &local);
        assert!((f.pos - Vec3::new(10.0, 3.0, 3.0)).length() < 0.00001);
        assert!(
            (f.rot * Vec3::Y - matrix.transform_vector3(Mat3::from_rotation_x(1.0) * Vec3::Y))
                .length()
                < 0.00001
        );
    }
    #[test]
    fn finite_effects_repeat_only_when_the_hardpoint_authors_a_delay() {
        let root = crate::assets::baked_root();
        let lib = Library::load(&root, &root.join("bf2_mus1"), "bf2", &[]);
        let original = lib
            .effects
            .values()
            .flat_map(|d| &d.emitters)
            .next()
            .expect("real effect bake");
        let mut emitter = original.clone();
        emitter.max_particles = manifest::FxRange(1.0, 0.0);
        emitter.burst_count = manifest::FxRange(1.0, 0.0);
        emitter.start_delay = manifest::FxRange(0.0, 0.0);
        emitter.burst_delay = manifest::FxRange(0.0, 0.0);
        emitter.transformer.lifetime = 0.1;
        emitter.sibling = None;
        emitter.geometry.child = None;
        let def = Arc::new(EffectDef::new(
            1,
            None,
            vec![emitter],
            super::super::Era::Bf2,
            &mut Lcg(1),
        ));
        let mut once = Attached::new(def.clone(), Frame::at(Vec3::ZERO), None, 1);
        once.step(1.0);
        assert!(once.live.is_none());
        once.step(10.0);
        assert!(once.live.is_none());
        let mut repeat = Attached::new(def, Frame::at(Vec3::ZERO), Some([2.0, 2.0]), 1);
        assert!(repeat.wait >= 0.0 && repeat.wait < 2.0);
        repeat.step(2.0);
        assert!(repeat.live.is_some());
        repeat.step(1.0);
        assert!(repeat.live.is_none());
        repeat.step(1.99);
        assert!(repeat.live.is_none());
        repeat.step(0.02);
        assert!(repeat.live.is_some());
    }
}
