//! Cosmetic chunk launches (`re-vehicle-death.md` §6, `re-droid-death.md` §2).
//! [ours] Sphere contacts against the static floor replace rigid-body contacts;
//! spin is cosmetic, chunks never damage units, and airborne pieces expire at 30 s.
use super::{FxAssets, Lcg};
use crate::{classdb::ClassDb, walk::WalkSim};
use core::sim::Event;
use glam::{Mat4, Quat, Vec3};
use std::collections::{HashMap, VecDeque};

#[derive(Clone)]
struct Spec {
    model: String,
    centre: Vec3,
    radius: f32,
    speed: f32,
    inherit: f32,
    up: f32,
    bounce: f32,
    life: f32,
}

struct Piece {
    spec: Spec,
    pos: Vec3,
    vel: Vec3,
    rot: Quat,
    spin: Vec3,
    age: f32,
    rest: f32,
    contacts: u8,
}

#[derive(Default)]
pub(super) struct Chunks {
    defs: HashMap<String, Vec<Spec>>,
    live: VecDeque<Piece>,
    rng: Option<Lcg>,
}

impl Chunks {
    pub(super) fn load(db: &ClassDb, names: &[String], dirs: &[std::path::PathBuf]) -> Self {
        let mut out = Self::default();
        let mut bounds = HashMap::new();
        for name in names {
            let mut specs = Vec::new();
            for props in chunk_properties(db.chain(name)) {
                let Some(model) = props.get(&crate::classdb::fnv1a_lowercase("ChunkGeometryName"))
                else {
                    continue;
                };
                let number = |key: &str| {
                    props
                        .get(&crate::classdb::fnv1a_lowercase(key))
                        .and_then(|v| v.parse::<f32>().ok())
                };
                let bound = bounds.entry(model.to_owned()).or_insert_with(|| {
                    let file = dirs
                        .iter()
                        .map(|d| d.join(format!("{model}.glb")))
                        .find(|p| pak::is_file(p))?;
                    let mesh = crate::assets::load_glb(&file).ok()?;
                    let (mut lo, mut hi) =
                        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
                    for &v in &mesh.positions {
                        lo = lo.min(Vec3::from(v));
                        hi = hi.max(Vec3::from(v));
                    }
                    (!mesh.positions.is_empty())
                        .then_some(((lo + hi) * 0.5, (hi - lo).length() * 0.5))
                });
                let Some((centre, radius)) = *bound else {
                    eprintln!("chunk model {model}: missing bake, no piece drawn");
                    continue;
                };
                specs.push(Spec {
                    model: model.to_owned(),
                    centre,
                    radius,
                    speed: number("ChunkSpeed").unwrap_or(-1.0),
                    inherit: number("ChunkVelocityFactor").unwrap_or(1.0),
                    up: number("ChunkUpFactor").unwrap_or(0.25),
                    bounce: number("ChunkBounciness").unwrap_or(0.6),
                    life: number("ChunkLifetime").filter(|v| *v >= 0.0).unwrap_or(8.0),
                });
            }
            if !specs.is_empty() {
                out.defs.insert(name.clone(), specs);
            }
        }
        out
    }

    pub(super) fn tick(&mut self, walk: &WalkSim) {
        let dt = core::TICK_DT;
        for p in &mut self.live {
            p.age += dt;
            if p.contacts >= 3 {
                p.rest += dt;
                continue;
            }
            p.vel.y -= 9.8 * dt;
            let before = p.pos;
            p.pos += p.vel * dt;
            p.rot = (Quat::from_scaled_axis(p.spin * dt) * p.rot).normalize();
            let floor = walk
                .sim
                .ground_point_under_n(p.pos, before.y + p.spec.radius)
                .0;
            if p.vel.y < 0.0 && p.pos.y - p.spec.radius <= floor {
                p.pos.y = floor + p.spec.radius;
                p.vel.y = -p.vel.y * p.spec.bounce;
                p.contacts += 1;
            }
        }
        self.live.retain(|p| p.age < 30.0 && p.rest < p.spec.life);
        for event in &walk.sim.events {
            match event {
                Event::DroidBurst {
                    unit,
                    team,
                    class,
                    pos: _,
                    scale,
                } => {
                    let name = &walk.teams[usize::from(*team)].class(*class).name;
                    let u = &walk.sim.units[unit.index as usize];
                    self.launch(
                        name,
                        u.soldier.pos,
                        u.soldier.vel,
                        Quat::from_rotation_y(u.soldier.yaw + std::f32::consts::PI),
                        1.0,
                        *scale,
                    );
                }
                Event::VehicleDestroyed {
                    vehicle,
                    class,
                    pos,
                    vel,
                    ..
                } => {
                    if let Some(name) = walk.vehicle_classes.get(*class as usize) {
                        let rot = walk
                            .sim
                            .vehicles
                            .get(vehicle.index as usize)
                            .map_or(Quat::IDENTITY, |v| v.rot);
                        let radius = walk
                            .sim
                            .seat_setup
                            .get(*class as usize)
                            .and_then(Option::as_ref)
                            .map_or(1.0, |s| s.radius);
                        self.launch(name, *pos, *vel, rot, radius, 1.0);
                    }
                }
                _ => {}
            }
        }
    }

    fn launch(
        &mut self,
        name: &str,
        pos: Vec3,
        velocity: Vec3,
        rot: Quat,
        radius: f32,
        scale: f32,
    ) {
        let Some(specs) = self.defs.get(name) else {
            return;
        };
        let centre = specs.iter().map(|s| s.centre).sum::<Vec3>() / specs.len() as f32;
        let rng = self.rng.get_or_insert(Lcg(0xdeb1_5eed));
        for spec in specs {
            let local = spec.centre - centre;
            let d = rot * local;
            let dir = (d + Vec3::Y * (spec.up * d.length()))
                .try_normalize()
                .unwrap_or(Vec3::Y);
            let vel = launch_velocity(
                velocity,
                dir,
                local.length_squared(),
                radius,
                spec.speed,
                spec.inherit,
                scale,
                rng,
            );
            // [ours] Bounded independent visual pool, oldest piece evicted first.
            if self.live.len() == 80 {
                self.live.pop_front();
            }
            self.live.push_back(Piece {
                spec: spec.clone(),
                pos: pos + rot * spec.centre,
                vel,
                rot,
                spin: Vec3::new(rng.float() - 0.5, rng.float() - 0.5, rng.float() - 0.5) * 8.0,
                age: 0.0,
                rest: 0.0,
                contacts: 0,
            });
        }
    }

    pub(super) fn draws(&self, assets: &mut dyn FxAssets, out: &mut Vec<render::ParticleDraw>) {
        for p in &self.live {
            if let Some(model) = assets.model(&p.spec.model.to_ascii_lowercase()) {
                let m = Mat4::from_rotation_translation(p.rot, p.pos)
                    * Mat4::from_translation(-p.spec.centre);
                model.draws(m, [255; 4], p.pos, false, out);
            }
        }
    }

    pub(super) fn models(&self) -> impl Iterator<Item = &str> {
        self.defs.values().flatten().map(|s| s.model.as_str())
    }

    pub(super) fn len(&self) -> usize {
        self.live.len()
    }

    pub(super) fn clear(&mut self) {
        self.live.clear();
    }
}

fn chunk_properties(chain: Vec<&crate::classdb::ClassDef>) -> Vec<HashMap<u32, String>> {
    use crate::classdb::fnv1a_lowercase as hash;
    let keys: Vec<_> = [
        "ChunkGeometryName",
        "ChunkSpeed",
        "ChunkVelocityFactor",
        "ChunkUpFactor",
        "ChunkBounciness",
        "ChunkLifetime",
    ]
    .map(hash)
    .into();
    let mut slots: std::collections::BTreeMap<String, HashMap<u32, String>> = Default::default();
    let mut defaults = HashMap::new();
    for def in chain.into_iter().rev() {
        let mut slot = None;
        let mut implicit = 0;
        for (key, value) in &def.props {
            if *key == hash("CHUNKSECTION") {
                slot = Some(value.to_ascii_lowercase());
            } else if keys.contains(key) {
                if *key == hash("ChunkGeometryName") && slot.is_none() {
                    slot = Some(format!("implicit{implicit}"));
                    implicit += 1;
                }
                if let Some(slot) = &slot {
                    slots
                        .entry(slot.clone())
                        .or_insert_with(|| defaults.clone())
                        .insert(*key, value.clone());
                } else {
                    defaults.insert(*key, value.clone());
                }
            }
        }
    }
    slots.into_values().collect()
}

#[allow(clippy::too_many_arguments)]
fn launch_velocity(
    parent: Vec3,
    dir: Vec3,
    distance2: f32,
    radius: f32,
    speed: f32,
    inherit: f32,
    scale: f32,
    rng: &mut Lcg,
) -> Vec3 {
    let mut v = parent * inherit;
    if speed >= 0.0 {
        v += dir * speed;
    } else {
        if distance2 > 1.0 {
            v += dir * ((15.0 + 15.0 * rng.float()) / radius.max(0.001));
        }
        if v.length() < 5.0 {
            v = dir * (4.0 + 2.0 * rng.float());
            if v.y < 2.0 {
                v.y = 2.0 + rng.float();
            }
        }
    }
    v * scale
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_launch_inherits_velocity_and_droid_scale() {
        let v = launch_velocity(
            Vec3::X * 10.0,
            Vec3::Y,
            2.0,
            2.0,
            6.0,
            0.5,
            0.1,
            &mut Lcg(1),
        );
        assert!((v - Vec3::new(0.5, 0.6, 0.0)).length() < 0.00001);
        let v = launch_velocity(Vec3::ZERO, Vec3::X, 0.5, 1.0, -1.0, 1.0, 1.0, &mut Lcg(1));
        assert!((4.0..6.0).contains(&v.x));
        assert!((2.0..3.0).contains(&v.y));
    }

    use crate::classdb::{fnv1a_lowercase as hash, ClassDef};
    #[test]
    fn chunk_sections_inherit_by_slot_without_sharing_rates() {
        let def = |props: &[(&str, &str)]| ClassDef {
            name: "fixture".into(),
            kind: "entc".into(),
            base: "hover".into(),
            props: props.iter().map(|(k, v)| (hash(k), (*v).into())).collect(),
        };
        let base = def(&[
            ("CHUNKSECTION", "CHUNK1"),
            ("ChunkGeometryName", "a"),
            ("ChunkSpeed", "2"),
            ("CHUNKSECTION", "CHUNK2"),
            ("ChunkGeometryName", "b"),
            ("ChunkSpeed", "9"),
        ]);
        let child = def(&[("CHUNKSECTION", "CHUNK1"), ("ChunkSpeed", "6")]);
        let slots = chunk_properties(vec![&child, &base]);
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0][&hash("ChunkGeometryName")], "a");
        assert_eq!(slots[0][&hash("ChunkSpeed")], "6");
        assert_eq!(slots[1][&hash("ChunkSpeed")], "9");
    }

    fn chunks() -> Chunks {
        let spec = Spec {
            model: "fixture".into(),
            centre: Vec3::ZERO,
            radius: 0.2,
            speed: 6.0,
            inherit: 0.5,
            up: 0.25,
            bounce: 0.6,
            life: 3.0,
        };
        Chunks {
            defs: HashMap::from([("fixture".into(), vec![spec])]),
            ..Default::default()
        }
    }
    #[test]
    fn vehicle_event_launches_then_expires_without_touching_the_sim() {
        let mut c = chunks();
        let mut walk = WalkSim::new(1, Vec3::ZERO, None);
        walk.vehicle_classes = vec!["fixture".into()];
        walk.sim.events.push(Event::VehicleDestroyed {
            vehicle: core::vehicle::VehicleHandle { index: 0, gen: 0 },
            class: 0,
            pos: Vec3::Y * 10.0,
            vel: Vec3::X * 10.0,
            killer: None,
        });
        let hash = walk.sim.hash();
        c.tick(&walk);
        assert_eq!(c.len(), 1);
        assert_eq!(walk.sim.hash(), hash);
        assert!((c.live[0].vel - Vec3::new(5.0, 6.0, 0.0)).length() < 0.00001);
        walk.sim.events.clear();
        for _ in 0..1000 {
            c.tick(&walk);
        }
        assert_eq!(c.len(), 0, "airborne pieces are bounded too");
    }
    #[test]
    fn floor_contacts_bounce_settle_and_expire_after_rest() {
        let mut c = chunks();
        let mut walk = WalkSim::new(1, Vec3::ZERO, None);
        walk.sim.heightfield = Some(core::heightfield::Heightfield {
            size: 2,
            unit: 100.0,
            offset_x: 50.0,
            offset_z: 50.0,
            heights: vec![0.0; 4],
        });
        c.launch(
            "fixture",
            Vec3::Y * 0.21,
            Vec3::ZERO,
            Quat::IDENTITY,
            1.0,
            1.0,
        );
        c.live[0].vel = Vec3::NEG_Y * 10.0;
        c.tick(&walk);
        assert_eq!(c.live[0].contacts, 1);
        assert!(c.live[0].vel.y > 6.0);
        assert!((c.live[0].pos.y - 0.2).abs() < 0.00001);
        for _ in 0..300 {
            if c.live[0].contacts >= 3 {
                break;
            }
            c.tick(&walk);
        }
        assert_eq!(c.live[0].contacts, 3);
        let pos = c.live[0].pos;
        for _ in 0..60 {
            c.tick(&walk);
        }
        assert_eq!(c.live[0].pos, pos);
        for _ in 0..40 {
            c.tick(&walk);
        }
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn visual_pool_is_bounded_and_clear_removes_every_piece() {
        let mut c = chunks();
        for _ in 0..100 {
            c.launch("fixture", Vec3::ZERO, Vec3::ZERO, Quat::IDENTITY, 1.0, 1.0);
        }
        assert_eq!(c.len(), 80);
        c.clear();
        assert_eq!(c.len(), 0);
    }
    #[test]
    #[ignore = "requires the targeted real droid chunk overlay"]
    fn real_bf2_class_resolves_five_droid_pieces() {
        let root = crate::assets::baked_root();
        let db =
            crate::roster::load_classdb(&root.join("classes"), "bf2", &["cis", "rep"]).unwrap();
        let c = Chunks::load(
            &db,
            &["cis_inf_rifleman".into()],
            &[root.join("side_bf2/weapons")],
        );
        assert_eq!(c.defs["cis_inf_rifleman"].len(), 5);
    }
}
