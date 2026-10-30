//! [ours] Pitch the gun bones behind the class's authored muzzle hardpoints.
//! The arm and leg animation stays intact. PROVISIONAL: rotate the hardpoint's
//! parent subtree; native per-aimer drawing is untraced
//! (`analysis/re-droideka-unit.md` §3).

use crate::skin::Skeleton;
use crate::viewer::walker_pose::WalkerRig;
use core::droideka::{Droideka, DroidekaClass};
use glam::{Mat4, Quat, Vec3};

#[derive(Default)]
pub(super) struct GunAim {
    guns: Vec<Gun>,
}

struct Gun {
    muzzle: usize,
    pivot: usize,
    bones: Vec<usize>,
}

impl GunAim {
    pub fn new(skeleton: &Skeleton, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut guns: Vec<Gun> = Vec::new();
        for name in names {
            let Some(muzzle) = skeleton.index_of(name.as_ref()) else {
                continue;
            };
            let Some(pivot) = skeleton.parents[muzzle] else {
                continue;
            };
            if guns.iter().any(|g| g.pivot == pivot) {
                continue;
            }
            let bones = (0..skeleton.len())
                .filter(|&n| {
                    let mut parent = Some(n);
                    while let Some(p) = parent {
                        if p == pivot {
                            return true;
                        }
                        parent = skeleton.parents[p];
                    }
                    false
                })
                .collect();
            guns.push(Gun {
                muzzle,
                pivot,
                bones,
            });
        }
        Self { guns }
    }

    pub fn palette(
        &self,
        rig: &WalkerRig,
        d: &Droideka,
        c: &DroidekaClass,
        pitch: f32,
    ) -> Vec<Mat4> {
        let mut palette = rig.palette(&super::walker_state(d, c), 1.0);
        if !d.in_transform() && d.alive {
            self.pose(
                &rig.skeleton,
                &mut palette,
                pitch.clamp(c.pitch_limits.0, c.pitch_limits.1),
            );
        }
        palette
    }

    fn pose(&self, skeleton: &Skeleton, palette: &mut [Mat4], pitch: f32) {
        let (s, c) = pitch.sin_cos();
        let direction = Vec3::new(0.0, s, c);
        let nodes: Vec<_> = palette
            .iter()
            .zip(&skeleton.global_bind)
            .map(|(p, b)| *p * *b)
            .collect();
        for gun in &self.guns {
            let Some(forward) = nodes[gun.muzzle].z_axis.truncate().try_normalize() else {
                continue;
            };
            let pivot = nodes[gun.pivot].w_axis.truncate();
            let rotation = Quat::from_rotation_arc(forward, direction);
            let turn = Mat4::from_translation(pivot)
                * Mat4::from_quat(rotation)
                * Mat4::from_translation(-pivot);
            for &n in &gun.bones {
                palette[n] = turn * nodes[n] * skeleton.inverse_bind[n];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::droideka::{Order, IDLE, IDLE_TO_ROLL, ROLLING, ROLL_TO_IDLE};

    #[test]
    #[ignore = "requires newly baked local BF1 and BF2 CIS droideka skins"]
    fn authored_gun_nodes_pitch_the_real_weighted_guns_without_moving_the_body() {
        let root = crate::assets::baked_root();
        for game in ["bf1", "bf2"] {
            let sides =
                crate::vehicle_pads::side_vehicle_files(&root.join(format!("{game}_geo1")), game);
            let (dir, manifest, record) = sides
                .iter()
                .find_map(|(dir, vm)| vm.droidekas.first().map(|d| (dir, vm, d)))
                .expect("rebake the CIS side with droideka metadata");
            let skin = record.skin.as_ref().expect("a skinned droideka");
            let asset =
                crate::assets::load_skinned_glb(&dir.join("walkers").join(format!("{skin}.glb")))
                    .unwrap();
            // The animation GLB transports the rig with a placeholder triangle;
            // production skins this separate LOD0 mesh and its JOINTS_0 stream.
            let geometry = manifest
                .geometries
                .iter()
                .find(|g| g.name.eq_ignore_ascii_case(&record.geometry))
                .unwrap();
            let model = geometry
                .lods
                .iter()
                .find(|(lod, _)| lod.eq_ignore_ascii_case("LOD0"))
                .map_or(geometry.model.as_str(), |(_, m)| m.as_str());
            let (mesh, joints) =
                crate::assets::load_glb_joints(&dir.join("models").join(format!("{model}.glb")))
                    .unwrap();
            assert_eq!(
                mesh.positions.len(),
                joints.len(),
                "{game}: the rendered vertex/joint streams match"
            );
            let classes =
                crate::roster::load_classdb(&root.join("classes"), game, &["cis"]).unwrap();
            let c = crate::droideka::droideka_class(&classes, &classes, &record.class);
            let aim = GunAim::new(
                &asset.skeleton,
                classes.props_all(&record.class, "AimerNodeName"),
            );
            assert_eq!(aim.guns.len(), 2, "{game}: both authored aimers resolve");
            let rig = WalkerRig::new(
                asset.skeleton,
                asset.clips,
                &crate::droideka::as_walker(record),
            );
            let mut d = Droideka::new(&c, Order::Deploy);
            d.state = IDLE;
            d.clip_t = 0.37;
            let base = rig.palette(&super::super::walker_state(&d, &c), 1.0);
            let affected = |n| aim.guns.iter().any(|g| g.bones.contains(&n));
            for pitch in [-0.35f32, 0.0, 0.35, 1.0] {
                let palette = aim.palette(&rig, &d, &c, pitch);
                let pitch = pitch.clamp(c.pitch_limits.0, c.pitch_limits.1);
                let desired = Vec3::new(0.0, pitch.sin(), pitch.cos());
                for gun in &aim.guns {
                    let muzzle = palette[gun.muzzle] * rig.skeleton.global_bind[gun.muzzle];
                    assert!(
                        muzzle
                            .z_axis
                            .truncate()
                            .normalize()
                            .abs_diff_eq(desired, 1e-4),
                        "{game}: pitch {pitch}"
                    );
                }
                for (n, (before, after)) in base.iter().zip(&palette).enumerate() {
                    if !affected(n) {
                        assert_eq!(
                            before, after,
                            "{game}: body/leg {} moved",
                            rig.skeleton.names[n]
                        );
                    }
                }
                let mut moved_vertices = 0;
                let mut fixed_vertices = 0;
                for (p, joints) in mesh.positions.iter().zip(&joints) {
                    let n = usize::from(joints[0]);
                    let p = Vec3::from_array(*p);
                    let before = base[n].transform_point3(p);
                    let after = palette[n].transform_point3(p);
                    if affected(n) {
                        moved_vertices += usize::from(before.distance(after) > 0.01);
                    } else {
                        assert_eq!(before, after, "{game}: a body/leg vertex moved");
                        fixed_vertices += 1;
                    }
                }
                assert!(fixed_vertices > 100, "{game}: body control has geometry");
                if pitch.abs() > 0.1 {
                    assert!(
                        moved_vertices > 20,
                        "{game}: pitch {pitch} moved no gun geometry"
                    );
                }
                eprintln!("{game}: pitch {pitch:.6}, two authored guns; moved gun vertices {moved_vertices}, fixed body/leg vertices {fixed_vertices}");
            }
            for state in [IDLE_TO_ROLL, ROLLING, ROLL_TO_IDLE] {
                d.state = state;
                assert_eq!(
                    aim.palette(&rig, &d, &c, 0.35),
                    rig.palette(&super::super::walker_state(&d, &c), 1.0),
                    "{game}: transform state {state} changed"
                );
            }
        }
    }
}
