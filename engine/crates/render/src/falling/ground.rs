use super::*;

#[derive(Clone, Copy, Debug)]
pub struct LeafActor {
    pub id: u64,
    pub pos: Vec3,
    pub velocity: Vec3,
    pub radius: f32,
    pub height: f32,
}

impl FallingLeaf {
    pub(super) fn kick(&mut self, velocity: Vec3) {
        self.impulse += velocity * 0.9 + Vec3::Y * (velocity.length() * 0.4);
    }

    pub(super) fn step_impulse(&mut self, dt: f32) {
        if (self.impulse + self.drift).length() <= 0.1 {
            return;
        }
        let turning = self.impulse + self.drift * (16.0 * (1.0 - self.phase / TAU));
        let axis = (turning * TAU).cross(Vec3::Y);
        if axis.length() > 0.0001 {
            self.rotation = Quat::from_scaled_axis(axis) * self.rotation;
        }
        self.pos += (self.impulse + self.drift) * dt;
        self.impulse *= (1.0 - 2.0 * dt).max(0.0);
    }

    pub(super) fn stir(&mut self, actors: &[LeafActor]) {
        let mut contacts = Vec::new();
        for actor in actors {
            let radius = actor.radius + 0.17;
            let delta = self.pos - actor.pos;
            if !actor.pos.is_finite()
                || !actor.velocity.is_finite()
                || !radius.is_finite()
                || radius <= 0.0
                || !actor.height.is_finite()
                || delta.y < -0.25
                || delta.y > actor.height + 0.25
                || delta.x * delta.x + delta.z * delta.z > radius * radius
            {
                continue;
            }
            let velocity = Vec3::new(actor.velocity.x, 0.0, actor.velocity.z);
            if velocity.length_squared() <= 0.01 {
                continue;
            }
            contacts.push(actor.id);
            if !self.contacts.contains(&actor.id) {
                self.kick(velocity.normalize() * 2.0);
            }
        }
        self.contacts = contacts;
    }
}

impl FallingState {
    pub(super) fn spawn_gust(&mut self, objects: &[LeafObject], camera: &Camera) -> bool {
        let wind = self.wind.normalize_or_zero();
        let upwind = -camera.forward() + wind;
        let far = camera.position - upwind * 30.0;
        let min = camera.position.min(far);
        let max = camera.position.max(far);
        let forest = objects.iter().any(|object| {
            let Some(emitter) = self.emitters.get(object.class as usize) else {
                return false;
            };
            let centre = object.matrix.w_axis.truncate();
            emitter.count > 0
                && centre.x + emitter.radius >= min.x
                && centre.x - emitter.radius <= max.x
                && centre.z + emitter.radius >= min.z
                && centre.z - emitter.radius <= max.z
        });
        if !forest || self.ground.is_none() {
            return false;
        }
        for _ in 0..20 {
            if self.leaves.len() == 100 {
                break;
            }
            let mut pos = camera.position + camera.forward() * (15.0 + self.unit() * 30.0)
                - wind * (15.0 + self.unit() * 30.0);
            let height = self.ground.as_ref().unwrap()(pos.x, pos.z);
            if !height.is_finite() {
                continue;
            }
            pos.y = height + 1.0 + self.unit();
            let birth = Quat::from_rotation_y(self.unit() * TAU);
            let phase = self.unit() * TAU;
            let random = self.unit();
            let mut leaf = FallingLeaf {
                owner: usize::MAX,
                pos,
                birth,
                phase,
                random,
                tumble: false,
                unseen: 0.0,
                ground: height,
                rotation: birth,
                grounded: true,
                impulse: Vec3::ZERO,
                drift: wind * 5.0,
                contacts: Vec::new(),
            };
            leaf.kick(wind * 5.0);
            self.leaves.push(leaf);
        }
        true
    }
}

impl Renderer {
    pub fn set_leaf_time(&mut self, time: Option<f64>) {
        self.falling.clock = time.filter(|time| time.is_finite());
    }

    pub fn set_leaf_actors(&mut self, actors: Vec<LeafActor>) {
        self.falling.actors = actors;
    }

    pub fn set_leaf_wind(&mut self, wind: Vec3) {
        if wind.is_finite() {
            self.falling.wind = wind;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gusts_share_the_pool_and_require_an_authored_emitter() {
        let emitter = LeafEmitter {
            count: 6,
            birds: 0,
            radius: 5.0,
            height: 5.0,
            scale: 1.0,
            distance: 100.0,
        };
        let objects = [LeafObject {
            class: 0,
            matrix: glam::Mat4::from_translation(Vec3::new(-30.0, 0.0, -30.0)),
            color: [1.0; 4],
            distance: 100.0,
        }];
        let mut state = FallingState {
            emitters: vec![emitter],
            wind: Vec3::X * 5.0,
            ground: Some(Box::new(|_, _| 0.0)),
            rng: 7,
            ..Default::default()
        };
        assert!(state.spawn_gust(&objects, &Camera::default()));
        assert_eq!(state.leaves.len(), 20);
        assert!(state
            .leaves
            .iter()
            .all(|leaf| leaf.grounded && leaf.pos.y >= 1.0 && leaf.pos.y < 2.0));
        for _ in 0..5 {
            state.spawn_gust(&objects, &Camera::default());
        }
        assert_eq!(state.leaves.len(), 100);
        state.emitters[0].count = 0;
        state.leaves.clear();
        let rng = state.rng;
        assert!(!state.spawn_gust(&objects, &Camera::default()));
        assert_eq!(state.rng, rng);
        assert!(state.leaves.is_empty());
    }

    #[test]
    fn actor_contact_kicks_once_then_rearms_on_separation() {
        let mut leaf = super::super::tests::sample();
        leaf.pos = Vec3::Y * 0.1;
        leaf.grounded = true;
        let actor = LeafActor {
            id: 9,
            pos: Vec3::ZERO,
            velocity: Vec3::Z * 2.0,
            radius: 0.5,
            height: 1.8,
        };
        leaf.stir(&[actor]);
        assert!(leaf.impulse.abs_diff_eq(Vec3::new(0.0, 0.8, 1.8), 1e-6));
        let first = leaf.impulse;
        leaf.stir(&[actor]);
        assert_eq!(leaf.impulse, first);
        leaf.stir(&[]);
        leaf.stir(&[actor]);
        assert!(leaf.impulse.abs_diff_eq(first * 2.0, 1e-6));
    }

    #[test]
    fn soldier_and_vehicle_footprints_only_move_decorations() {
        let mut state = FallingState {
            emitters: vec![LeafEmitter {
                count: 1,
                birds: 0,
                radius: 0.0,
                height: 0.0,
                scale: 1.0,
                distance: 100.0,
            }],
            ground: Some(Box::new(|x, _| x * 0.1)),
            ..Default::default()
        };
        let mut leaf = super::super::tests::sample();
        leaf.pos = Vec3::new(0.0, 0.1, -3.0);
        leaf.grounded = true;
        leaf.owner = usize::MAX;
        state.leaves.push(leaf.clone());
        let camera = Camera {
            position: Vec3::new(0.0, 2.0, 0.0),
            pitch: -0.4,
            ..Default::default()
        };
        for (id, radius, height) in [(1, 0.5, 1.8), (1 << 63, 3.0, 6.0)] {
            state.leaves[0] = leaf.clone();
            state.actors = vec![LeafActor {
                id,
                pos: Vec3::new(0.0, 0.0, -3.0),
                velocity: Vec3::X * 3.0,
                radius,
                height,
            }];
            state.last = Some(0.0);
            state.update(&[], &camera, 1.0, 0.033333333);
            assert!(state.leaves[0].pos.x > 0.05);
            assert!((state.leaves[0].pos.y - state.leaves[0].pos.x * 0.1 - 0.1).abs() < 1e-6);
            let at = state.leaves[0].pos;
            let impulse = state.leaves[0].impulse;
            state.update(&[], &camera, 1.0, 0.033333333);
            assert_eq!(state.leaves[0].pos, at);
            assert_eq!(state.leaves[0].impulse, impulse);
            assert_eq!(state.actors[0].pos, Vec3::new(0.0, 0.0, -3.0));
            assert_eq!(state.actors[0].velocity, Vec3::X * 3.0);
        }
    }

    #[test]
    fn ground_impulse_matches_the_live_ps2_step() {
        #[derive(serde::Deserialize)]
        struct Frame {
            matrix: [[f32; 4]; 4],
            phase: f32,
            impulse: [f32; 4],
            drift: [f32; 4],
            floor: f32,
        }
        let root = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../artifacts/lead_tmp/leaff/full-reference"
        ));
        let (Ok(before), Ok(after)) = (
            std::fs::read(root.join("ground-before.json")),
            std::fs::read(root.join("ground-after.json")),
        ) else {
            eprintln!("SKIP: local PS2 ground-leaf samples are absent");
            return;
        };
        let before: Frame = serde_json::from_slice(&before).unwrap();
        let after: Frame = serde_json::from_slice(&after).unwrap();
        let mut leaf = super::super::tests::sample();
        leaf.pos = Vec3::from_slice(&before.matrix[3]);
        leaf.rotation = Quat::from_mat4(&glam::Mat4::from_cols_array_2d(&before.matrix));
        leaf.phase = before.phase;
        leaf.grounded = true;
        leaf.impulse = Vec3::from_slice(&before.impulse);
        leaf.drift = Vec3::from_slice(&before.drift);
        let dt = after.phase - before.phase;
        assert!(dt > 0.0 && dt < 0.05);
        leaf.step(dt);
        leaf.pos.y = after.floor + 0.1;
        assert!(leaf
            .pos
            .abs_diff_eq(Vec3::from_slice(&after.matrix[3]), 0.00004));
        assert!(leaf
            .impulse
            .abs_diff_eq(Vec3::from_slice(&after.impulse), 0.00001));
        let axes = glam::Mat3::from_quat(leaf.rotation);
        for index in 0..3 {
            assert!(axes
                .col(index)
                .abs_diff_eq(Vec3::from_slice(&after.matrix[index]), 0.0003));
        }
    }
}
