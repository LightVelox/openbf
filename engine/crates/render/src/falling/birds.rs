use super::*;

#[derive(Clone, Copy)]
pub struct ForestBirdType {
    pub size: f32,
    pub texture: TextureHandle,
}

struct Bird {
    origin: Vec3,
    pos: Vec3,
    velocity: Vec3,
    rotation: Quat,
    left: Quat,
    right: Quat,
    phase: f32,
    random: f32,
    age: f32,
    kind: usize,
}

impl Bird {
    fn step(&mut self, dt: f32) {
        let angle = (self.random * 8.0 + 8.0) * self.phase;
        let left = angle.sin() * FRAC_PI_4;
        let right = (angle - 0.5654867).sin() * 1.0995574;
        self.left = Quat::from_rotation_z(left) * Quat::from_rotation_y(FRAC_PI_4);
        self.right = Quat::from_rotation_z(right) * Quat::from_rotation_y(FRAC_PI_4);
        self.pos =
            self.origin + self.velocity * self.age - self.rotation * Vec3::Y * (right.sin() * 0.1);
        self.phase = (self.phase + dt) % TAU;
        self.age += dt;
    }

    fn draws(&self, kind: ForestBirdType) -> [ParticleDraw; 2] {
        let size = kind.size;
        let quad = |points: [Vec3; 4], uv: [[f32; 2]; 4]| {
            [0, 1, 2, 2, 1, 3].map(|i| (self.pos + self.rotation * points[i], uv[i]))
        };
        let body = quad(
            [
                Vec3::new(0.0, -size * 0.25, -size * 0.5),
                Vec3::new(0.0, -size * 0.25, size * 0.5),
                Vec3::new(0.0, size * 0.25, -size * 0.5),
                Vec3::new(0.0, size * 0.25, size * 0.5),
            ],
            [[0.0, 1.0], [1.0, 1.0], [0.0, 0.5], [1.0, 0.5]],
        );
        let mut wings = Vec::with_capacity(24);
        let root = Vec3::Y * (size * 0.325);
        let inner = self.left * Vec3::X * (size * 0.5);
        for (rotation, origin, u) in [(self.left, root, 0.0), (self.right, root + inner, 0.5)] {
            let span = rotation * Vec3::X * (size * 0.5);
            let chord = rotation * Vec3::Z * (size * 0.25);
            let points = [
                origin - chord,
                origin + chord,
                origin + span - chord,
                origin + span + chord,
            ];
            let uv = [[u, 0.5], [u, 0.0], [u + 0.5, 0.5], [u + 0.5, 0.0]];
            wings.extend(quad(points, uv));
            wings.extend(quad(points.map(|p| Vec3::new(-p.x, p.y, p.z)), uv));
        }
        [
            ParticleDraw {
                shape: ParticleShape::CutoutMesh {
                    tris: body.to_vec(),
                },
                key: self.pos,
                texture: Some(kind.texture),
                color: [204, 204, 204, 255],
                additive: false,
            },
            ParticleDraw {
                shape: ParticleShape::CutoutMesh { tris: wings },
                key: self.pos,
                texture: Some(kind.texture),
                color: [255; 4],
                additive: false,
            },
        ]
    }
}

#[derive(Default)]
pub(super) struct Birds {
    types: Vec<ForestBirdType>,
    live: Vec<Bird>,
    clock: f32,
    interval: f32,
}

impl Birds {
    pub(super) fn reset(&mut self) {
        self.live.clear();
        self.clock = 0.0;
    }

    pub(super) fn draws(&self) -> Vec<ParticleDraw> {
        self.live
            .iter()
            .flat_map(|bird| bird.draws(self.types[bird.kind]))
            .collect()
    }
}

impl FallingState {
    pub(super) fn step_birds(&mut self, objects: &[LeafObject], camera: &Camera, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        for bird in &mut self.birds.live {
            bird.step(dt);
        }
        self.birds.live.retain(|bird| bird.age <= 5.0);
        if self.birds.types.is_empty() {
            return;
        }
        self.birds.clock += dt;
        if self.birds.clock <= self.birds.interval {
            return;
        }
        let mut anchors: Vec<_> = objects
            .iter()
            .filter_map(|object| {
                let emitter = self.emitters.get(object.class as usize)?;
                let pos = object.matrix.w_axis.truncate();
                (emitter.birds > 0 && pos.distance(camera.position) <= emitter.distance)
                    .then_some((pos, emitter.birds, pos.distance_squared(camera.position)))
            })
            .collect();
        anchors.sort_by(|a, b| a.2.total_cmp(&b.2));
        anchors.truncate(5);
        if anchors.is_empty() {
            return;
        }
        self.birds.clock = 0.0;
        let index = (self.unit() * 5.0) as usize;
        let Some(&(origin, count, _)) = anchors.get(index) else {
            return;
        };
        for _ in 0..count {
            if self.birds.live.len() >= 15 {
                break;
            }
            let kind = ((self.unit() * self.birds.types.len() as f32) as usize)
                .min(self.birds.types.len() - 1);
            let phase = self.unit() * TAU;
            let rotation = Quat::from_rotation_y(self.unit() * TAU)
                * Quat::from_rotation_x((-0.25 - 0.25 * self.unit()) * std::f32::consts::FRAC_PI_2);
            let random = self.unit();
            let mut bird = Bird {
                origin,
                pos: origin,
                left: Quat::IDENTITY,
                right: Quat::IDENTITY,
                velocity: rotation * Vec3::Z * (random * 8.0 + 8.0),
                rotation,
                phase,
                random,
                age: 0.0,
                kind,
            };
            bird.step(dt);
            self.birds.live.push(bird);
        }
    }
}

impl Renderer {
    pub fn set_forest_birds(&mut self, types: Vec<ForestBirdType>, interval: f32) {
        self.falling.birds = Birds {
            interval: if interval.is_finite() && interval > 0.0 {
                interval
            } else {
                30.0
            },
            types: types
                .into_iter()
                .filter(|kind| kind.size.is_finite() && kind.size > 0.0)
                .take(3)
                .collect(),
            ..Default::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_birds_match_the_live_ps2_step() {
        #[derive(serde::Deserialize)]
        struct Frame {
            age: f32,
            phase: f32,
            random: f32,
            origin: [f32; 4],
            pos: [f32; 4],
            velocity: [f32; 4],
            rotation: [[f32; 4]; 3],
            #[serde(default)]
            left: [[f32; 4]; 3],
            #[serde(default)]
            right: [[f32; 4]; 3],
        }
        let root = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../artifacts/lead_tmp/leaff/full-reference"
        ));
        let (Ok(before), Ok(after)) = (
            std::fs::read(root.join("birds-before.json")),
            std::fs::read(root.join("birds-after.json")),
        ) else {
            eprintln!("SKIP: local PS2 bird samples are absent");
            return;
        };
        let before: Vec<Frame> = serde_json::from_slice(&before).unwrap();
        let after: Vec<Frame> = serde_json::from_slice(&after).unwrap();
        assert_eq!(before.len(), 3);
        let matrix = |axes: [[f32; 4]; 3]| {
            glam::Mat3::from_cols(
                Vec3::from_slice(&axes[0]),
                Vec3::from_slice(&axes[1]),
                Vec3::from_slice(&axes[2]),
            )
        };
        for (before, after) in before.iter().zip(after) {
            let mut bird = Bird {
                origin: Vec3::from_slice(&before.origin),
                pos: Vec3::from_slice(&before.pos),
                velocity: Vec3::from_slice(&before.velocity),
                rotation: Quat::from_mat3(&matrix(before.rotation)),
                phase: before.phase,
                random: before.random,
                age: before.age,
                left: Quat::IDENTITY,
                right: Quat::IDENTITY,
                kind: 0,
            };
            bird.step(after.age - before.age);
            assert!(bird.pos.abs_diff_eq(Vec3::from_slice(&after.pos), 0.00004));
            for (actual, expected) in [(bird.left, after.left), (bird.right, after.right)] {
                let actual = glam::Mat3::from_quat(actual);
                let expected = matrix(expected);
                for i in 0..3 {
                    assert!(
                        actual.col(i).abs_diff_eq(expected.col(i), 0.0003),
                        "axis {i}: {:?} != {:?}",
                        actual.col(i),
                        expected.col(i)
                    );
                }
            }
        }
    }

    #[test]
    fn anchors_obey_authored_counts_timer_lifetime_and_budget() {
        let mut state = FallingState {
            emitters: vec![LeafEmitter {
                count: 6,
                birds: 3,
                radius: 5.0,
                height: 5.0,
                scale: 1.0,
                distance: 100.0,
            }],
            birds: Birds {
                interval: 30.0,
                types: vec![ForestBirdType {
                    size: 1.0,
                    texture: TextureHandle(0),
                }],
                ..Default::default()
            },
            rng: 7,
            ..Default::default()
        };
        let objects: Vec<_> = (0..5)
            .map(|i| LeafObject {
                class: 0,
                matrix: glam::Mat4::from_translation(Vec3::new(i as f32, 0.0, -5.0)),
                color: [1.0; 4],
                distance: 100.0,
            })
            .collect();
        state.step_birds(&objects, &Camera::default(), 30.0);
        assert!(state.birds.live.is_empty());
        for _ in 0..6 {
            state.birds.clock = 30.0;
            state.step_birds(&objects, &Camera::default(), 0.01);
        }
        assert_eq!(state.birds.live.len(), 15);
        assert!(state.birds.live.iter().all(|b| b.velocity.y > 0.0
            && b.velocity.length() >= 8.0
            && b.velocity.length() < 16.0));
        for bird in &mut state.birds.live {
            bird.age = 5.0;
        }
        state.step_birds(&objects, &Camera::default(), 0.01);
        assert!(state.birds.live.is_empty());
        state.emitters[0].birds = 0;
        state.birds.clock = 30.0;
        let rng = state.rng;
        state.step_birds(&objects, &Camera::default(), 0.01);
        assert!(state.birds.live.is_empty());
        assert_eq!(state.rng, rng);
    }
}
