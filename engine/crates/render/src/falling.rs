//! Airborne leaf particles (LEAFF, `analysis/re-falling-leaves.md` [D]).
//! A visible patch fills its authored slots from a shared 100-leaf pool.
//! Flutter/tumble use the original two modes; terrain ends a fall. Three
//! seconds without a draw retires a leaf, rather than limiting its life.
//! [ours] A fixed cosmetic seed and a 100 ms step cap keep replay/camera
//! discontinuities bounded. Ground-leaf gusts are outside this module.
use crate::frustum::Frustum;
use crate::{
    Camera, DrawCmd, DrawLayer, LeafObject, LightPath, MeshData, MeshHandle, Renderer,
    TextureHandle,
};
use glam::{Mat4, Quat, Vec3};
use std::f32::consts::{FRAC_PI_4, TAU};

type Ground = Box<dyn Fn(f32, f32) -> f32 + Send + Sync>;
#[derive(Debug, Clone, Copy)]
pub struct LeafEmitter {
    pub count: u8,
    pub radius: f32,
    pub height: f32,
    pub scale: f32,
    pub distance: f32,
}
#[derive(Clone)]
struct FallingLeaf {
    owner: usize,
    pos: Vec3,
    birth: Quat,
    phase: f32,
    random: f32,
    tumble: bool,
    unseen: f32,
    ground: f32,
    rotation: Quat,
}
impl FallingLeaf {
    fn step(&mut self, dt: f32) {
        if dt == 0.0 {
            return;
        }
        let r = self.random;
        let (rotation, velocity) = if self.tumble {
            (
                self.birth * Quat::from_rotation_x((r + 2.0) * self.phase * TAU),
                self.birth * Vec3::Z * (r * 0.3 + 0.3) - Vec3::Y * (r + 1.0),
            )
        } else {
            let s = (self.phase * 2.0).sin();
            let mut q = self.birth * Quat::from_rotation_z(r * FRAC_PI_4 * s);
            if r < 0.15 {
                q *= Quat::from_rotation_y((r - 1.0) * self.phase * 10.0);
            }
            let down = Vec3::new(0.0, -0.5, 0.0);
            (q, down.lerp(q * Vec3::X * (-1.8 * s), (r * s).abs()))
        };
        self.rotation = rotation;
        self.pos += velocity * dt;
        self.phase = (self.phase + dt) % TAU;
        self.unseen += dt;
    }
}
#[derive(Default)]
pub(crate) struct FallingState {
    emitters: Vec<LeafEmitter>,
    texture: Option<TextureHandle>,
    ground: Option<Ground>,
    leaves: Vec<FallingLeaf>,
    mesh: Option<MeshHandle>,
    last: Option<f64>,
    rng: u32,
}
impl FallingState {
    fn unit(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let a = self.rng;
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f32::from_bits(((a >> 16 | self.rng & 0xffff_0000) >> 9) | 0x3f80_0000) - 1.0
    }
    fn update(&mut self, objects: &[LeafObject], camera: &Camera, aspect: f32, now: f64) {
        if !now.is_finite() {
            return;
        }
        let dt = self
            .last
            .map_or(0.0, |last| (now - last).clamp(0.0, 0.1) as f32);
        if self.last.is_some_and(|last| now < last) {
            self.leaves.clear();
        }
        self.last = Some(now);
        let view = Frustum::from_view_proj(&camera.view_proj(aspect));
        let visible =
            |p: Vec3, r: f32| view.intersects_aabb(p - Vec3::splat(r), p + Vec3::splat(r));
        for leaf in &mut self.leaves {
            leaf.step(dt);
            if visible(leaf.pos, 0.25) {
                leaf.unseen = 0.0;
            }
        }
        self.leaves
            .retain(|l| l.pos.y >= l.ground - 0.5 && l.unseen <= 3.0);
        let mut counts = vec![0usize; objects.len()];
        for leaf in &self.leaves {
            if let Some(count) = counts.get_mut(leaf.owner) {
                *count += 1;
            }
        }
        for (owner, object) in objects.iter().enumerate() {
            let Some(e) = self.emitters.get(object.class as usize).copied() else {
                continue;
            };
            let centre = object.matrix.w_axis.truncate();
            if e.count == 0
                || !e.radius.is_finite()
                || e.radius < 0.0
                || !e.height.is_finite()
                || e.height < 0.0
                || !e.scale.is_finite()
                || !e.distance.is_finite()
                || centre.distance(camera.position) > e.distance
                || !visible(centre, e.radius + e.height * e.scale.abs())
            {
                continue;
            }
            for _ in counts[owner]..usize::from(e.count.min(50)) {
                if self.leaves.len() == 100 {
                    return;
                }
                let pos = centre
                    + Vec3::new(
                        (self.unit() * 2.0 - 1.0) * e.radius,
                        (self.unit() * 2.0 - 1.0) * e.height * e.scale,
                        (self.unit() * 2.0 - 1.0) * e.radius,
                    );
                if !pos.is_finite() {
                    continue;
                }
                let Some(ground) = self.ground.as_ref().map(|g| g(pos.x, pos.z)) else {
                    continue;
                };
                if !ground.is_finite() || pos.y < ground - 0.5 {
                    continue;
                }
                let birth = Quat::from_rotation_y(self.unit() * TAU);
                let phase = self.unit() * TAU;
                let random = self.unit();
                let tumble = self.unit() < 0.2;
                self.leaves.push(FallingLeaf {
                    owner,
                    pos,
                    birth,
                    phase,
                    random,
                    tumble,
                    unseen: 0.0,
                    ground,
                    rotation: birth,
                });
            }
        }
    }
}
impl Renderer {
    /// Replace airborne emitters; class indices match `set_leaves`. Terrain
    /// sampling belongs to the caller; the renderer never owns sim state.
    pub fn set_leaf_fall(
        &mut self,
        emitters: Vec<LeafEmitter>,
        texture: Option<TextureHandle>,
        ground: Option<Ground>,
    ) {
        self.falling = FallingState {
            mesh: self.falling.mesh,
            emitters,
            texture,
            ground,
            rng: 0x6c65_6166,
            ..Default::default()
        };
    }
    pub(crate) fn falling_draws(&mut self, camera: &Camera) -> Vec<DrawCmd> {
        if self.falling.texture.is_none() || !self.falling.emitters.iter().any(|e| e.count > 0) {
            return Vec::new();
        }
        let (w, h) = match &self.target {
            crate::Target::Window { config, .. } => (config.width, config.height),
            crate::Target::Headless { width, height, .. } => (*width, *height),
        };
        self.falling.update(
            &self.leaves.objects,
            camera,
            w as f32 / h.max(1) as f32,
            self.water_time,
        );
        let mesh = match self.falling.mesh {
            Some(mesh) => mesh,
            None => {
                let mesh = self
                    .add_mesh(&MeshData {
                        positions: &[
                            [-0.17, 0.0, 0.17],
                            [0.17, 0.0, 0.17],
                            [0.17, 0.0, -0.17],
                            [-0.17, 0.0, -0.17],
                        ],
                        normals: &[[0.0, 1.0, 0.0]; 4],
                        uvs: &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                        colors: None,
                        indices: &[0, 1, 2, 0, 2, 3],
                    })
                    .expect("fixed leaf quad");
                self.falling.mesh = Some(mesh);
                mesh
            }
        };
        self.falling
            .leaves
            .iter()
            .map(|leaf| {
                let mut d = DrawCmd::textured(
                    mesh,
                    self.falling.texture.expect("leaf texture"),
                    Mat4::from_rotation_translation(leaf.rotation, leaf.pos),
                );
                d.layer = DrawLayer::Blend;
                d.light = LightPath::Unlit;
                let grey = leaf.random * 0.6 + 0.4;
                let linear = crate::lighting::decode(grey);
                d.color = [linear, linear, linear, 1.0];
                d
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> FallingLeaf {
        FallingLeaf {
            owner: 0,
            pos: Vec3::new(0.0, 10.0, -3.0),
            birth: Quat::from_rotation_y(0.09295412),
            phase: 3.6785447,
            random: 0.5500381,
            tumble: false,
            unseen: 0.0,
            ground: 0.0,
            rotation: Quat::IDENTITY,
        }
    }
    #[test]
    fn flutter_matches_original_near_probe_frame() {
        let mut p = sample();
        let before = p.pos;
        p.step(0.033364147);
        assert!((p.rotation * Vec3::X)
            .abs_diff_eq(Vec3::new(0.92474556, 0.37069345, -0.08620735), 0.00001));
        assert!(((p.pos - before) / 0.033364147)
            .abs_diff_eq(Vec3::new(-0.70750695, -0.5418512, 0.06595577), 0.00004));
    }
    #[test]
    fn repeated_view_at_same_time_keeps_the_pose() {
        let mut p = sample();
        p.step(0.03);
        let (pos, rotation, phase) = (p.pos, p.rotation, p.phase);
        p.step(0.0);
        assert_eq!((p.pos, p.rotation, p.phase), (pos, rotation, phase));
    }
    #[test]
    fn tumble_falls_and_advances_phase() {
        let mut p = sample();
        p.tumble = true;
        let before = p.pos;
        p.step(0.03);
        assert!((p.pos.y - before.y + (p.random + 1.0) * 0.03).abs() < 0.00001);
        assert!((p.phase - 3.7085447).abs() < 0.00001);
    }
    #[test]
    fn visible_lifetime_is_unbounded_and_ground_retires() {
        let mut state = FallingState {
            leaves: vec![sample()],
            ..Default::default()
        };
        for n in 0..50 {
            let camera = Camera {
                position: state.leaves[0].pos + Vec3::Z * 3.0,
                ..Default::default()
            };
            state.update(&[], &camera, 1.0, n as f64 * 0.1);
            assert_eq!(state.leaves.len(), 1);
            assert_eq!(state.leaves[0].unseen, 0.0);
        }
        state.leaves[0].ground = state.leaves[0].pos.y + 1.0;
        state.update(&[], &Camera::default(), 1.0, 5.0);
        assert!(state.leaves.is_empty());
    }
    #[test]
    fn tumble_matches_original_held_probe_frame() {
        let mut p = sample();
        p.birth = Quat::from_rotation_y(3.0303247);
        p.phase = 5.354345 - 0.033364147;
        p.random = 0.015179753;
        p.tumble = true;
        p.step(0.033364147);
        assert!((p.rotation * Vec3::Y)
            .abs_diff_eq(Vec3::new(-0.10941425, -0.1704129, 0.9792792), 0.0002));
    }
    #[test]
    fn pool_and_visibility_retirement_and_clock_reset() {
        let e = LeafEmitter {
            count: 50,
            radius: 0.0,
            height: 0.0,
            scale: 1.0,
            distance: 100.0,
        };
        let objects = vec![
            LeafObject {
                class: 0,
                matrix: Mat4::from_translation(Vec3::new(0.0, 10.0, -3.0)),
                color: [1.0; 4],
                distance: 100.0
            };
            3
        ];
        let mut s = FallingState {
            emitters: vec![e],
            ground: Some(Box::new(|_, _| -100.0)),
            ..Default::default()
        };
        let cam = Camera {
            position: Vec3::new(0.0, 10.0, 0.0),
            ..Default::default()
        };
        s.update(&objects, &cam, 1.0, 0.0);
        assert_eq!(s.leaves.len(), 100);
        // A visible particle can live beyond three seconds.
        for n in 1..41 {
            s.update(&objects, &cam, 1.0, n as f64 * 0.1);
        }
        assert!(s.leaves.iter().any(|l| l.unseen == 0.0));
        let away = Camera {
            yaw: std::f32::consts::PI,
            ..cam
        };
        for n in 41..82 {
            s.update(&objects, &away, 1.0, n as f64 * 0.1);
        }
        assert!(s.leaves.is_empty());
        s.update(&objects, &cam, 1.0, 0.0);
        assert_eq!(s.leaves.len(), 100);
        s.emitters[0].count = 0;
        s.leaves.clear();
        s.update(&objects, &cam, 1.0, 0.1);
        assert!(s.leaves.is_empty());
        s.emitters[0].count = 50;
        s.emitters[0].radius = f32::NAN;
        s.ground = Some(Box::new(|_, _| {
            panic!("invalid emitter reached terrain sampler")
        }));
        s.update(&objects, &cam, 1.0, 0.2);
        assert!(s.leaves.is_empty());
    }
}
