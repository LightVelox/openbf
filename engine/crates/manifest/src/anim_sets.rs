//! `side_<game>/soldiers/<model>.sets.ron`: the PS2's soldier animation sets
//! over the clips of the soldier's GLB (card ANIMS; `analysis/
//! re-soldier-anim-layers.md` §2, §6). `bake::anim_sets` resolves and writes
//! it, `game::animator` plays it. The name tables are the two executables'
//! own, in their order: a slot's index is its place in them.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The file of `model` under a side's bake dir.
pub fn path(side_dir: &std::path::Path, model: &str) -> std::path::PathBuf {
    side_dir.join("soldiers").join(format!("{model}.sets.ron"))
}

/// Original DummyRoot translations, before the GLB bake removes model travel.
pub type ClipMotions = BTreeMap<String, Vec<[f32; 3]>>;

pub fn motion_path(side_dir: &std::path::Path, model: &str) -> std::path::PathBuf {
    side_dir
        .join("soldiers")
        .join(format!("{model}.motion.ron"))
}

/// One clip a set names.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SetClip {
    /// The disc's clip name = the animation's name in the soldier's GLB.
    pub name: String,
    pub frames: u32,
    /// BF1: the clip's anim record loops, and its blend seconds (§2.1). BF2
    /// keeps both per animation, not per clip ([`Bf2Animation`]): `None`.
    pub looping: Option<bool>,
    pub blend: Option<f32>,
}

/// What `soldiers/<model>.sets.ron` holds (see the module doc). Slot values
/// index `clips`; `None` is a slot the PS2 leaves empty.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct SoldierSets {
    /// "bf1" or "bf2".
    pub game: String,
    /// The bones the UPPER player poses and the bones the LOWER player poses,
    /// by name (BF2: `DummyRoot` is in neither, §6.4).
    pub upper_bones: Vec<String>,
    pub lower_bones: Vec<String>,
    /// Every clip a slot below names.
    pub clips: Vec<SetClip>,
    /// The root speed (m/s; x sideways, z forward) of each clip the PS2
    /// measures one for, by clip name (`bake::anim_sets::root_speed`).
    pub root_speed: BTreeMap<String, [f32; 2]>,
    pub bf1: Option<Bf1Sets>,
    /// One per skeleton family a class on this model has.
    pub bf2: Vec<Bf2Sets>,
    /// BF2's per-animation facts, pair by pair ([`bf2::PAIRS`]).
    pub bf2_animations: Vec<Bf2Animation>,
}

/// BF1's tables as indices into [`SoldierSets::clips`]; the dimensions are
/// `bake::anim_sets::bf1::Tables`'.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Bf1Sets {
    pub weapons: Vec<String>,
    pub base: Vec<Vec<Option<u16>>>,
    pub special: Vec<Option<u16>>,
    pub w: Vec<Vec<Vec<Option<u16>>>>,
    pub alert: Vec<Vec<Option<u16>>>,
    pub u: Vec<Vec<Vec<Option<u16>>>>,
    pub s: Vec<Vec<Option<u16>>>,
}

/// BF2's sets of one skeleton family: weapon animation class -> 268 slots
/// (`2 * pair + layer`, layer 0 upper, 1 lower; pairs as [`bf2::base_name`]).
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Bf2Sets {
    pub family: String,
    pub sets: BTreeMap<String, Vec<Option<u16>>>,
}

/// One layer of a BF2 animation, from the disc's `sanm` chunk
/// (`mungefs::sanm`).
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Bf2Layer {
    pub looping: bool,
    /// "None", "Head", "Torso" or "FullBody".
    pub aim: String,
    pub sync: Option<String>,
    pub blend_in: f32,
    pub blend_out: f32,
    pub blend_to: Vec<(String, f32)>,
}

/// A BF2 animation's two layers; `None` where the chunk has no scope for it
/// (the PS2 then holds 0 / 0, no loop).
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Bf2Animation {
    pub name: String,
    pub upper: Option<Bf2Layer>,
    pub lower: Option<Bf2Layer>,
}

/// BF1's tables' axes (`FUN_0022d828`, §2.1).
pub mod bf1 {
    pub const STANCES: [&str; 3] = ["stand", "crouch", "prone"];
    /// The 13 movement actions (`PTR_s_idle_emote_00517b70`).
    pub const ACTIONS: [&str; 13] = [
        "idle_emote",
        "idle_checkweapon",
        "idle_lookaround",
        "turnleft",
        "turnright",
        "walkforward",
        "walkbackward",
        "runforward",
        "runbackward",
        "walkright",
        "walkleft",
        "runright",
        "runleft",
    ];
    /// The nine weapon sets (`PTR_s_rifle_00517bb8`), by the name a weapon
    /// class's `AnimationBank` gives. The disc's sabre weapons name
    /// `lightsabre`; the clips are `human_sabre_*`.
    pub const WEAPONS: [&str; 9] = [
        "rifle",
        "bazooka",
        "pistol",
        "grenade",
        "tool",
        "sabre",
        "sbdroid",
        "bdroid",
        "lascannon",
    ];
    /// The six upper-body actions (`PTR_DAT_00517be0`).
    pub const UPPER: [&str; 6] = ["idle", "shoot", "shoot2", "shoot3", "charge", "reload"];
    /// The 31 special animations (`PTR_DAT_00517af0`; 0x1e repeats 0x1b).
    pub const SPECIAL: [&str; 31] = [
        "jump",
        "jump_up",
        "landsoft",
        "landhard",
        "fall",
        "thrown_flail",
        "thrown_flyingfront",
        "thrown_flyingback",
        "thrown_flyingleft",
        "thrown_flyingright",
        "thrown_tumblefront",
        "thrown_tumbleback",
        "thrown_bouncefrontsoft",
        "thrown_bouncebacksoft",
        "thrown_landfrontsoft",
        "thrown_landbacksoft",
        "stand_getupfront",
        "stand_getupback",
        "thrown_restfrontsoft",
        "thrown_restbacksoft",
        "diveforward",
        "jetpack_hover",
        "stand_getdown_prone",
        "prone_getup_stand",
        "crouch_getdown_prone",
        "prone_getup_crouch",
        "stand_death_forward",
        "stand_death_backward",
        "stand_death_left",
        "stand_death_right",
        "stand_death_backward",
    ];

    /// The weapon set an `AnimationBank` value names (`lightsabre` is the
    /// `sabre` set); `None` for a name that is no set.
    pub fn weapon_set(animation_bank: &str) -> Option<usize> {
        let n = animation_bank.trim().to_ascii_lowercase();
        let n = if n == "lightsabre" { "sabre" } else { &n };
        WEAPONS.iter().position(|w| *w == n)
    }
}

/// BF2's pair order (§6.1).
pub mod bf2 {
    /// The 38 simple animation ids (ELF table 0x5fde48).
    pub const SIMPLE: [&str; 38] = [
        "sprint",
        "jump",
        "jump_forward",
        "jump_backward",
        "jump_left",
        "jump_right",
        "landsoft",
        "landhard",
        "fall",
        "thrown_flail",
        "thrown_flyingfront",
        "thrown_flyingback",
        "thrown_flyingleft",
        "thrown_flyingright",
        "thrown_tumblefront",
        "thrown_tumbleback",
        "thrown_bouncefrontsoft",
        "thrown_bouncebacksoft",
        "thrown_landfrontsoft",
        "thrown_landbacksoft",
        "stand_getupfront",
        "stand_getupback",
        "thrown_restfrontsoft",
        "thrown_restbacksoft",
        "diveforward",
        "jetpack_hover",
        "stand_deadhero",
        "prone_getup_stand",
        "crouch_getdown_prone",
        "prone_getup_crouch",
        "stand_death_forward",
        "stand_death_backward",
        "stand_death_left",
        "stand_death_right",
        "stand_hitfront",
        "stand_hitback",
        "stand_hitleft",
        "stand_hitright",
    ];
    /// The 13 moves of a block (0x5fdee0) — not BF1's order.
    pub const MOVES: [&str; 13] = [
        "idle_emote",
        "idle_checkweapon",
        "idle_lookaround",
        "turnleft",
        "turnright",
        "walkforward",
        "runforward",
        "walkbackward",
        "runbackward",
        "walkright",
        "runright",
        "walkleft",
        "runleft",
    ];
    /// The six movement blocks (0x5fdf30).
    pub const BLOCKS: [&str; 6] = [
        "stand",
        "crouch",
        "prone",
        "standalert",
        "crouchalert",
        "pronealert",
    ];
    /// The six weapon actions (0x5fdf18).
    pub const ACTIONS: [&str; 6] = [
        "shoot",
        "shoot2",
        "shoot_secondary",
        "shoot_secondary2",
        "charge",
        "reload",
    ];
    pub const STANCES: [&str; 3] = ["stand", "crouch", "prone"];
    /// Pairs the resolver fills: 38 simple, 6 x 13 movement, 3 x 6 actions.
    /// (A set holds 151; the last 17 belong to the melee combos.)
    pub const PAIRS: usize = 134;
    /// The first movement pair.
    pub const MOVE0: usize = 38;
    /// The first weapon-action pair.
    pub const ACT0: usize = 116;

    /// The animation of pair `p` (`<block>_<move>`, the two crouch blocks'
    /// third move being `idle_takeknee`).
    pub fn base_name(p: usize) -> String {
        if p < MOVE0 {
            return SIMPLE[p].to_string();
        }
        if p < ACT0 {
            let (b, m) = ((p - MOVE0) / 13, (p - MOVE0) % 13);
            if m == 2 && (b == 1 || b == 4) {
                return format!("{}_idle_takeknee", BLOCKS[b]);
            }
            return format!("{}_{}", BLOCKS[b], MOVES[m]);
        }
        let (s, k) = ((p - ACT0) / 6, (p - ACT0) % 6);
        format!("{}_{}", STANCES[s], ACTIONS[k])
    }

    /// The pair of move `m` ([`MOVES`]) in block `b` ([`BLOCKS`]).
    pub fn move_pair(b: usize, m: usize) -> usize {
        MOVE0 + 13 * b + m
    }

    /// The pair of weapon action `k` ([`ACTIONS`]) in stance `s`.
    pub fn action_pair(s: usize, k: usize) -> usize {
        ACT0 + 6 * s + k
    }

    /// The pair of a simple animation, by name.
    pub fn simple_pair(name: &str) -> Option<usize> {
        SIMPLE.iter().position(|n| *n == name)
    }
}
