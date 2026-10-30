use super::look::far_cull;
use super::scene::spec_cull;
use super::upload::upload_cpu_texture;
use super::vehicles::{
    find_model, pose_class, upload_parts, ClassVis, ModelSource, TexSource, VehicleSides,
};
use super::walker_pose::WalkerRig;
use crate::assets::{load_glb_joints, load_skinned_glb, Scene};
use anyhow::Result;
use core::droideka::{Droideka, DroidekaClass, ROLLING};
use core::vehicle::walker::WalkerState;
use glam::{Mat4, Vec3};
use render::{DrawCmd, MeshHandle, ParticleDraw, ParticleShape, Renderer, TextureHandle};
use std::collections::HashMap;
use std::path::Path;

#[derive(Default)]
pub struct DroidekaVisuals {
    models: Vec<ClassVis>,
    by_class: [Vec<Option<usize>>; 2],
    posed: HashMap<usize, ((u32, usize), Vec<MeshHandle>)>,
    cull: Option<f32>,
    spec_cull: Option<f32>,
    bubble_texture: Option<TextureHandle>,
    bubble_color: [u8; 4],
    bubbles: HashMap<usize, (u32, bool, f32)>,
}

pub const BUBBLE_GROW: f32 = 0.15;
pub const BUBBLE_SHRINK: f32 = 0.15;
pub const BUBBLE_INSIDE_ALPHA: u8 = 128;
pub const BUBBLE_SEGMENTS: usize = 16;
pub const BUBBLE_RINGS: usize = 8;

pub fn bubble_tris(centre: Vec3, radius: f32, eye: Vec3) -> Vec<(Vec3, [f32; 2])> {
    let axis = (eye - centre).try_normalize().unwrap_or(Vec3::Z);
    let (side, up) = axis.any_orthonormal_pair();
    let point = |ring: usize, seg: usize| {
        let v = ring as f32 / BUBBLE_RINGS as f32;
        let u = seg as f32 / BUBBLE_SEGMENTS as f32;
        let (st, ct) = (v * std::f32::consts::FRAC_PI_2).sin_cos();
        let (sp, cp) = (u * std::f32::consts::TAU).sin_cos();
        (
            centre + (axis * ct + side * (st * cp) + up * (st * sp)) * radius,
            [u, v],
        )
    };
    let mut out = Vec::with_capacity(BUBBLE_RINGS * BUBBLE_SEGMENTS * 6);
    for ring in 0..BUBBLE_RINGS {
        for seg in 0..BUBBLE_SEGMENTS {
            let (a, b) = (point(ring, seg), point(ring, seg + 1));
            let (c, d) = (point(ring + 1, seg), point(ring + 1, seg + 1));
            out.extend([a, c, b, b, c, d]);
        }
    }
    out
}

pub fn bubble_color(game: &str) -> [u8; 4] {
    if game.eq_ignore_ascii_case("bf2") {
        [96, 160, 192, 255]
    } else {
        [96, 160, 192, 192]
    }
}

pub fn bubble_scale(on: bool, t: f32) -> Option<f32> {
    if on {
        let s = (t / BUBBLE_GROW).min(1.0);
        Some(if s < 0.94 {
            s * 1.127_659_6
        } else {
            (1.0 - s) * 0.999_999 + 1.0
        })
    } else if t < BUBBLE_SHRINK {
        Some(1.0 - t / BUBBLE_SHRINK)
    } else {
        None
    }
}

pub fn spin_angle(d: &Droideka) -> f32 {
    2.0 * core::fmath::atan2(d.spin[1], d.spin[0])
}

pub fn roll_time(d: &Droideka, c: &DroidekaClass) -> f32 {
    let turn = spin_angle(d) / std::f32::consts::TAU;
    (turn - turn.floor()) * c.duration[usize::from(ROLLING)]
}

pub fn walker_state(d: &Droideka, c: &DroidekaClass) -> WalkerState {
    WalkerState {
        anim: d.state,
        clip_time: if d.state == ROLLING {
            roll_time(d, c)
        } else {
            d.clip_t
        },
        h: (1.0, 0.0),
        pitch: 0.0,
        roll: (1.0, 0.0),
        ..WalkerState::default()
    }
}

pub fn model_matrix(pos: Vec3, yaw: f32, d: &Droideka, c: &DroidekaClass) -> Mat4 {
    let base = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw + std::f32::consts::PI);
    if d.state != ROLLING {
        return base;
    }
    let centre = Vec3::new(0.0, c.ball_radius, 0.0);
    base * Mat4::from_translation(centre)
        * Mat4::from_rotation_z(d.lean.clamp(-1.0, 1.0).asin())
        * Mat4::from_translation(-centre)
}

impl DroidekaVisuals {
    pub fn load(
        r: &mut Renderer,
        scene: &Scene,
        assets_dir: &Path,
        walk: &crate::walk::WalkSim,
    ) -> Self {
        let game = scene.manifest.source_game.to_ascii_lowercase();
        let cull = far_cull(&scene.manifest.lighting).map(|c| c.world);
        let mut out = Self {
            cull,
            spec_cull: spec_cull(scene, cull),
            bubble_color: bubble_color(&game),
            ..Self::default()
        };
        let bakes = crate::droideka::bakes(&scene.dir, &game);
        if bakes.is_empty() {
            return out;
        }
        out.bubble_texture = upload_cpu_texture(
            r,
            &crate::assets::CpuTexture {
                width: 1,
                height: 1,
                rgba: vec![255; 4],
                extra_mips: Vec::new(),
                detail: None,
            },
            "droideka_shield",
        )
        .map_err(|e| eprintln!("droideka shield: {e:#}"))
        .ok();
        let sides = VehicleSides::load(assets_dir, &game);
        let mut loaded: HashMap<String, Option<usize>> = HashMap::new();
        let mut textures: HashMap<(Option<TexSource>, String), Option<TextureHandle>> =
            HashMap::new();
        for (t, team) in walk.teams.iter().enumerate() {
            out.by_class[t] = team
                .classes
                .iter()
                .map(|cv| {
                    let key = cv.name.to_ascii_lowercase();
                    if let Some(&k) = loaded.get(&key) {
                        return k;
                    }
                    let k = bakes.get(&key).and_then(|b| {
                        match upload(r, scene, &sides, &mut textures, &b.record) {
                            Ok(m) => {
                                out.models.push(m);
                                Some(out.models.len() - 1)
                            }
                            Err(e) => {
                                eprintln!("droideka {key}: {e:#}");
                                None
                            }
                        }
                    });
                    loaded.insert(key, k);
                    k
                })
                .collect();
        }
        out
    }

    pub fn draws(&mut self, r: &mut Renderer, sim: &core::sim::Sim) -> Vec<DrawCmd> {
        let mut out = Vec::new();
        for (slot, u) in sim.units.iter().enumerate() {
            let (Some(d), Some(c)) = (u.droideka.as_deref(), u.droideka_class()) else {
                continue;
            };
            if !u.alive || u.burst {
                continue;
            }
            let Some(k) = self
                .by_class
                .get(usize::from(u.team).min(1))
                .and_then(|t| t.get(usize::from(u.spec.class)))
                .copied()
                .flatten()
            else {
                continue;
            };
            let model = &self.models[k];
            let Some(rig) = model.rig() else {
                continue;
            };
            let palette = rig.palette(&walker_state(d, c), 1.0);
            let meshes = match pose_class(r, &mut self.posed, slot, (u.gen, k), model, &palette) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("droideka {slot}: {e:#}");
                    continue;
                }
            };
            let m = model_matrix(u.soldier.pos, u.soldier.yaw, d, c);
            model.draws(
                &mut out,
                Some(&meshes),
                m,
                (self.cull, self.spec_cull),
                None,
            );
        }
        out
    }
}

impl DroidekaVisuals {
    pub fn bubbles(&mut self, sim: &core::sim::Sim, eye: Vec3) -> Vec<ParticleDraw> {
        let now = sim.tick as f32 * core::TICK_DT;
        let mut out = Vec::new();
        for (slot, u) in sim.units.iter().enumerate() {
            let (Some(d), Some(c)) = (u.droideka.as_deref(), u.droideka_class()) else {
                self.bubbles.remove(&slot);
                continue;
            };
            let on = u.alive && d.shield.on;
            let settled = (
                u.gen,
                on,
                now - if on { BUBBLE_GROW } else { BUBBLE_SHRINK },
            );
            let e = self.bubbles.entry(slot).or_insert(settled);
            if e.0 != u.gen {
                *e = settled;
            } else if e.1 != on {
                *e = (u.gen, on, now);
            }
            let Some(f) = bubble_scale(on, now - e.2) else {
                continue;
            };
            if u.burst {
                continue;
            }
            let pos = u.droideka_point(c.shield.sphere.center);
            let radius = c.shield.sphere.radius;
            let mut color = self.bubble_color;
            if pos.distance_squared(eye) < radius * radius {
                color[3] = BUBBLE_INSIDE_ALPHA;
            }
            out.push(ParticleDraw {
                shape: ParticleShape::Mesh {
                    tris: bubble_tris(pos, radius * f, eye),
                },
                key: pos,
                color,
                texture: self.bubble_texture,
                additive: true,
            });
        }
        out
    }
}

fn upload(
    r: &mut Renderer,
    scene: &Scene,
    sides: &VehicleSides,
    textures: &mut HashMap<(Option<TexSource>, String), Option<TextureHandle>>,
    record: &manifest::ManifestDroideka,
) -> Result<ClassVis> {
    let Some(ModelSource::Side(i, model)) = find_model(&scene.manifest, sides, &record.geometry)
    else {
        anyhow::bail!("no side model for {}", record.geometry);
    };
    let dir = sides.dir(i);
    let (cpu, joints) = load_glb_joints(&dir.join("models").join(format!("{model}.glb")))?;
    if joints.is_empty() {
        anyhow::bail!("{model}: no JOINTS_0");
    }
    let skin_name = record.skin.as_deref().unwrap_or(&model);
    let a = load_skinned_glb(&dir.join("walkers").join(format!("{skin_name}.glb")))?;
    let rig = WalkerRig::new(a.skeleton, a.clips, &crate::droideka::as_walker(record));
    upload_parts(r, scene, sides, textures, &cpu, Some(i), Some(rig), joints)
}

#[cfg(test)]
mod tests;
