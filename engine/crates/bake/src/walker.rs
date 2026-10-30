//! A walker class's bake (card WALK, `analysis/re-walker.md` §5, §6): the
//! anim table's 11 fixed entries from its `AnimationName` bank — each
//! entry's first clip, looked up by the bare name (the bank's MINA CRC-32),
//! its frames, BF2's duration (N − 1)/30, the loop flag and track 0's root z
//! per frame (FUN_002a0c78) — and the model-space origin of every collision
//! primitive of its geometry at the skeleton's bind pose (the leg rays' foot
//! prims, FUN_002a2028, and the building spheres, §9).
//!
//! The integrator's feet are where the bind pose puts them (PROVISIONAL:
//! the PS2 reads the animated pose each tick; the port poses only the drawn
//! model, card WSKN).
//!
//! Card WSKN: the drawn walker — its skeleton and every clip the anim table
//! names, with the `_leftup` / `_rightup` layers FUN_002a0c78 loads beside
//! each (the AT-AT's), as a skinned GLB with no mesh ([`skin_glb`]; the
//! mesh is the model's own GLB with `JOINTS_0`), and each leg pair's two
//! layer bones ([`layer_bones`]).

use crate::skin_glb::{Clip, SkinnedModel};
use manifest::{ManifestPrimPoint, ManifestWalker, ManifestWalkerClip};
use mungefs::anim::AnimBank;
use mungefs::coll::{PrimKind, Primitives};
use mungefs::entity::EntityClass;
use mungefs::skel::Skeleton;

/// The anim table's entries (FUN_002663e0; the strings at 0x61f308..0x61f548):
/// each entry's first clip name, empty for entry 4, and the loop flag.
pub const ENTRIES: [(&str, bool); 11] = [
    ("idle", false),
    ("turnl", false),
    ("turnr", false),
    ("death01", false),
    ("", false),
    ("idle_to_leftfoot", false),
    ("idle_to_rightfoot", false),
    ("walk_leftfoot_rightfoot", true),
    ("walk_rightfoot_leftfoot", true),
    ("leftfoot_to_idle", false),
    ("rightfoot_to_idle", false),
];

/// The engines that build a walker (re-vehicles §2).
pub fn is_walker_base(base: &str) -> bool {
    ["walker", "commandwalker"]
        .iter()
        .any(|b| b.eq_ignore_ascii_case(base))
}

/// The anim table of `bank`: each entry's clip by its bare name, else the
/// bank-prefixed one.
pub fn clips(bank: &AnimBank<'_>) -> Vec<Option<ManifestWalkerClip>> {
    clips_of(bank, &ENTRIES)
}

pub fn clips_of(bank: &AnimBank<'_>, entries: &[(&str, bool)]) -> Vec<Option<ManifestWalkerClip>> {
    entries
        .iter()
        .map(|&(name, looping)| {
            if name.is_empty() {
                return None;
            }
            let k = bank
                .clip_index(name)
                .or_else(|| bank.clip_index(&format!("{}_{name}", bank.name)))?;
            let d = bank.decode_clip(k).ok()?;
            let root = d.tracks.first()?;
            let frames = d.frames;
            Some(ManifestWalkerClip {
                name: name.to_owned(),
                frames,
                duration: f32::from(frames.max(1) - 1) / 30.0,
                looping,
                root_z: root.translations.iter().map(|t| t[2]).collect(),
            })
        })
        .collect()
}

/// Each primitive's origin in model space: its translation through its
/// parent bone's bind matrix (a parent the skeleton lacks leaves it as
/// stored), with a bounding radius (0 for an undecoded kind).
pub fn prim_points(prims: &Primitives, skel: Option<&Skeleton>) -> Vec<ManifestPrimPoint> {
    let binds = skel.and_then(|s| s.bind_matrices().ok());
    prims
        .prims
        .iter()
        .map(|p| {
            let radius = match p.kind {
                PrimKind::Sphere { radius } => radius,
                PrimKind::Cylinder { radius, height } => radius.max(height * 0.5),
                PrimKind::Box { half } => {
                    (half[0] * half[0] + half[1] * half[1] + half[2] * half[2]).sqrt()
                }
                // A kind the decoder does not read (the one-man AT-ST's
                // two foot spheres): its origin still serves the leg rays.
                PrimKind::Unknown(_) => 0.0,
            };
            let bone = skel.and_then(|s| {
                s.bones
                    .iter()
                    .position(|b| b.name.eq_ignore_ascii_case(&p.parent))
            });
            let pos = match (bone, &binds) {
                (Some(i), Some(m)) => mungefs::skel::xform_point(&m[i], p.translation),
                _ => p.translation,
            };
            ManifestPrimPoint {
                name: p.name.clone(),
                bone: p.parent.clone(),
                pos,
                radius,
            }
        })
        .collect()
}

/// The walker record of one class, from its bank and its geometry's prims;
/// `skin` names the skin GLB [`skin_glb`] writes for it.
pub fn bake(
    e: &EntityClass,
    bank_name: &str,
    banks: &[AnimBank<'_>],
    prims: Option<&Primitives>,
    skel: Option<&Skeleton>,
    skin: Option<String>,
) -> ManifestWalker {
    let bank = banks
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case(bank_name));
    ManifestWalker {
        bank: bank_name.to_owned(),
        clips: bank.map_or_else(|| vec![None; ENTRIES.len()], clips),
        prims: prims.map_or_else(Vec::new, |p| prim_points(p, skel)),
        skin,
        layer_bones: layer_bones(e),
    }
}

/// The two props naming a leg pair's layer bones (the setter FUN_00263f68
/// masks each one's subtree, FUN_002668c0, into the current pair's class
/// entry +0x28 / +0x30; FUN_002a1468 gives both masks to the pair player's
/// layers 1 and 2 — `_leftup` and `_rightup`). Their names are not known;
/// on every shipped walker they repeat LegBoneLeft / LegBoneRight but the
/// AT-TE's, which names its shoulders.
const PROP_LAYER_BONE_LEFT: u32 = 0x580F_4D75;
const PROP_LAYER_BONE_RIGHT: u32 = 0x9EB9_FC8E;

/// Each `WalkerLegPair`'s layer bones, in the class's pair order.
pub fn layer_bones(e: &EntityClass) -> Vec<[String; 2]> {
    let mut out: Vec<[String; 2]> = Vec::new();
    let pair = mungefs::fnv1a_lowercase("WalkerLegPair");
    for p in &e.props {
        match p.key_hash {
            k if k == pair => out.push(Default::default()),
            PROP_LAYER_BONE_LEFT => {
                if let Some(last) = out.last_mut() {
                    last[0] = p.value.clone();
                }
            }
            PROP_LAYER_BONE_RIGHT => {
                if let Some(last) = out.last_mut() {
                    last[1] = p.value.clone();
                }
            }
            _ => {}
        }
    }
    out
}

/// The layers FUN_002a0c78 loads beside an entry's first clip.
pub const LAYERS: [&str; 3] = ["", "_leftup", "_rightup"];

/// The skin GLB of a walker model: `skel`'s bones and, from `bank`, every
/// entry's clip and layers (by the bare name, else the bank-prefixed one;
/// named in the GLB by the bare name + layer suffix), DummyRoot's
/// translation held at rest — its z is the root motion the integrator moves
/// the walker by (§4.9). A clip that names one bone twice keeps the LATER
/// track (the AT-ST's bone_root: a still copy, then the one that moves;
/// PROVISIONAL — which one the PS2's sampler takes is not read).
pub fn skin_glb(skel: &Skeleton, bank: Option<&AnimBank<'_>>) -> anyhow::Result<Vec<u8>> {
    let mut clips = Vec::new();
    if let Some(bank) = bank {
        for (name, _) in ENTRIES.iter().filter(|(n, _)| !n.is_empty()) {
            for layer in LAYERS {
                let full = format!("{name}{layer}");
                let Some(k) = bank
                    .clip_index(&full)
                    .or_else(|| bank.clip_index(&format!("{}_{full}", bank.name)))
                else {
                    continue;
                };
                let Ok(d) = bank.decode_clip(k) else { continue };
                let (tracks, _) = crate::soldier::clip_to_tracks(&d, skel, None);
                let mut kept: Vec<crate::skin_glb::ClipTrack> = Vec::new();
                for t in tracks {
                    kept.retain(|k| k.bone != t.bone);
                    kept.push(t);
                }
                clips.push(Clip {
                    name: full,
                    fps: 30.0,
                    frames: u32::from(d.frames),
                    tracks: kept,
                });
            }
        }
    }
    let model = SkinnedModel {
        name: skel.name.clone(),
        bones: crate::soldier::bones_of(skel),
        segments: Vec::new(),
    };
    crate::skin_glb::skinned_to_glb(&model, &clips)
}
