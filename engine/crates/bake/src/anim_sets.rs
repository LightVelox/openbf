//! The PS2's soldier animation sets, resolved from the disc (card ANIMS,
//! `analysis/re-soldier-anim-layers.md` §2, §5, §6 `[O]`): which clip each
//! slot of a weapon's set plays on the upper and on the lower body, after the
//! loaders' own fallbacks.
//!
//! * [`bf1`] — BF1's one set object (`FUN_0022d828`): six tables over nine
//!   weapon sets picked by the weapon class's `AnimationBank`, with the
//!   fallback table `DAT_00535598`. 979 slots; equal to PS2 memory.
//! * [`bf2`] — BF2's sets, one per (skeleton family, weapon animation class)
//!   the level's classes register (`FUN_00282f78`): the first 134 of a set's
//!   151 {upper, lower} pairs (the last 17 are the melee combos), filled along
//!   the two parent chains.
//!
//! Both are ports of the resolvers that were checked against save states
//! (`engine/artifacts/lead_tmp/anim/sets_bf1.py`, `sets_bf2.py`; the BF2 one
//! over 176 battle states, `analysis/re-soldier-anim-layers.md`); the keys in
//! `tests/data/anim_sets_bf{1,2}.json` are what those scripts print (BF2:
//! three of the states), and `tests/suite/anim_sets.rs` holds the port to
//! them slot by slot. The rules are
//! not re-derived here: a change to one must be made against PS2 memory.
//!
//! [`SoldierSets`] is what the side bake writes beside each soldier
//! (`soldiers/<model>.sets.ron`): its sets as clip names into the soldier's
//! GLB, the two bone masks, and each clip's loop flag, blend times and root
//! speed; the type lives in `manifest::anim_sets`, and `game::animator` plays it.

use mungefs::anim::{crc32_lower, AnimBank};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A clip a name resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipRef {
    pub name: String,
    pub frames: u16,
    pub tracks: u16,
    pub bank: String,
}

/// The clips of a list of banks, searched in the list's order: the first bank
/// that holds a name answers (BF1 `FUN_0022f888`).
#[derive(Debug, Clone, Default)]
pub struct Inventory {
    /// crc -> (frames, tracks, bank index).
    clips: HashMap<u32, (u16, u16, usize)>,
    banks: Vec<String>,
}

impl Inventory {
    /// `banks` in search order.
    pub fn new<'a, 'b: 'a>(banks: impl IntoIterator<Item = &'a AnimBank<'b>>) -> Self {
        let mut inv = Self::default();
        for b in banks {
            inv.add(b);
        }
        inv
    }

    /// Append a bank to the search order.
    pub fn add(&mut self, b: &AnimBank<'_>) {
        let k = self.banks.len();
        self.banks.push(b.name.clone());
        for c in &b.clips {
            self.clips
                .entry(c.name_crc)
                .or_insert((c.frames, c.bones, k));
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.clips.contains_key(&crc32_lower(name))
    }

    pub fn find(&self, name: &str) -> Option<ClipRef> {
        let &(frames, tracks, k) = self.clips.get(&crc32_lower(name))?;
        Some(ClipRef {
            name: name.to_string(),
            frames,
            tracks,
            bank: self.banks[k].clone(),
        })
    }

    pub fn len(&self) -> usize {
        self.clips.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }
}

/// BF1's animation set (`DAT_00517ae8`), §2.
pub mod bf1;

/// BF2's sets (`DAT_005fdd48`), §6.
pub mod bf2;

pub use manifest::anim_sets::{Bf1Sets, Bf2Animation, Bf2Layer, Bf2Sets, SetClip, SoldierSets};

/// BF1's upper-body bones (`FUN_0022f650`, §2.3); every other bone is lower.
pub const BF1_UPPER_BONES: [&str; 13] = [
    "bone_a_spine",
    "bone_b_spine",
    "bone_ribcage",
    "bone_neck",
    "bone_head",
    "bone_l_clavicle",
    "bone_l_upperarm",
    "bone_l_forearm",
    "bone_l_hand",
    "bone_r_clavicle",
    "bone_r_upperarm",
    "bone_r_forearm",
    "bone_r_hand",
];

/// BF2's lower-body bones (`FUN_00282018`, the list at 0x620ac8, §6.4); the
/// upper body is every other bone but bone 0.
pub const BF2_LOWER_BONES: [&str; 10] = [
    "bone_root",
    "bone_pelvis",
    "bone_l_thigh",
    "bone_l_calf",
    "bone_l_foot",
    "bone_l_toe",
    "bone_r_thigh",
    "bone_r_calf",
    "bone_r_foot",
    "bone_r_toe",
];

/// (upper, lower) bone names of a skeleton, `bones` in skeleton order.
pub fn masks(bones: &[String], bf2: bool) -> (Vec<String>, Vec<String>) {
    let is = |list: &[&str], b: &str| list.iter().any(|n| n.eq_ignore_ascii_case(b));
    let (mut upper, mut lower) = (Vec::new(), Vec::new());
    for (i, b) in bones.iter().enumerate() {
        let up = if bf2 {
            if i == 0 {
                continue;
            }
            !is(&BF2_LOWER_BONES, b)
        } else {
            is(&BF1_UPPER_BONES, b)
        };
        if up { &mut upper } else { &mut lower }.push(b.clone());
    }
    (upper, lower)
}

/// The sets of one soldier before its clips are decoded: [`SoldierSets`]
/// without root speeds, the bank each clip is in and the clips the PS2
/// measures a root speed for.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub sets: SoldierSets,
    /// Per `sets.clips` entry.
    pub banks: Vec<String>,
    pub measured: BTreeSet<String>,
}

impl Plan {
    fn clip(&mut self, c: &ClipRef) -> u16 {
        if let Some(i) = self.sets.clips.iter().position(|k| k.name == c.name) {
            return i as u16;
        }
        self.sets.clips.push(SetClip {
            name: c.name.clone(),
            frames: u32::from(c.frames),
            looping: None,
            blend: None,
        });
        self.banks.push(c.bank.clone());
        (self.sets.clips.len() - 1) as u16
    }
}

/// BF1: the six tables, the same for every soldier.
pub fn plan_bf1(inv: &Inventory, bones: &[String]) -> Plan {
    let t = bf1::build(inv);
    let mut p = Plan::default();
    p.sets.game = "bf1".into();
    (p.sets.upper_bones, p.sets.lower_bones) = masks(bones, false);
    let row = |p: &mut Plan, r: &[Option<ClipRef>]| -> Vec<Option<u16>> {
        r.iter().map(|c| c.as_ref().map(|c| p.clip(c))).collect()
    };
    let base: Vec<_> = t.base.iter().map(|r| row(&mut p, r)).collect();
    let special = row(&mut p, &t.special);
    let w: Vec<Vec<_>> =
        t.w.iter()
            .map(|st| st.iter().map(|r| row(&mut p, r)).collect())
            .collect();
    let alert: Vec<_> = t.alert.iter().map(|r| row(&mut p, r)).collect();
    let u: Vec<Vec<_>> =
        t.u.iter()
            .map(|st| st.iter().map(|r| row(&mut p, r)).collect())
            .collect();
    let s: Vec<_> = t.s.iter().map(|r| row(&mut p, r)).collect();
    // A record's loop flag and blend time are those of the name it was first
    // looked up under: a slot filled by a fallback points at that record.
    let mut facts: HashMap<String, (bool, f32)> = HashMap::new();
    for wn in bf1::WEAPONS {
        for (si, sn) in bf1::STANCES.iter().enumerate() {
            for (a, an) in bf1::ACTIONS.iter().enumerate() {
                let n = format!("human_{wn}_{sn}_{an}");
                facts.insert(
                    format!("{n}_alert"),
                    (bf1::move_loops(a), bf1::move_blend(si)),
                );
                facts.insert(n, (bf1::move_loops(a), bf1::move_blend(si)));
            }
            for (k, un) in bf1::UPPER.iter().enumerate() {
                facts.insert(
                    format!("human_{wn}_{sn}_{un}"),
                    (false, bf1::upper_blend(k)),
                );
            }
        }
        for (i, n) in bf1::SPECIAL.iter().enumerate() {
            facts.insert(format!("human_{wn}_{n}"), bf1::special_blend(i));
        }
    }
    for c in &mut p.sets.clips {
        if let Some(&(l, b)) = facts.get(&c.name) {
            (c.looping, c.blend) = (Some(l), Some(b));
        }
    }
    // `FUN_0022d4f0` measures the base and the alert movement records, a > 4.
    for r in t.base.iter().chain(&t.alert) {
        p.measured
            .extend(r[5..].iter().flatten().map(|c| c.name.clone()));
    }
    p.sets.bf1 = Some(Bf1Sets {
        weapons: bf1::WEAPONS.iter().map(|w| (*w).to_string()).collect(),
        base,
        special,
        w,
        alert,
        u,
        s,
    });
    p
}

/// BF2: every set `reg` holds for `families`, with the chunk's facts.
pub fn plan_bf2(
    inv: &Inventory,
    reg: &bf2::Registry,
    families: &[usize],
    sanm: &mungefs::sanm::SoldierAnimations,
    bones: &[String],
) -> Plan {
    use mungefs::sanm::Layer;
    let built = bf2::build_with_combos(inv, reg);
    let mut p = Plan::default();
    p.sets.game = "bf2".into();
    (p.sets.upper_bones, p.sets.lower_bones) = masks(bones, true);
    for &f in families {
        let mut sets = BTreeMap::new();
        // In class order: the clip list's order is the order slots name them,
        // and `built` is a hash map.
        let mut classes: Vec<usize> = built.keys().filter(|k| k.0 == f).map(|k| k.1).collect();
        classes.sort_unstable();
        for b in classes {
            let slots = &built[&(f, b)];
            let row: Vec<Option<u16>> = slots
                .iter()
                .enumerate()
                .map(|(w, n)| {
                    let c = inv.find(n.as_deref()?)?;
                    // The loader measures each lower or full clip
                    // (`FUN_00282900`).
                    if w % 2 == 1 {
                        p.measured.insert(c.name.clone());
                    }
                    Some(p.clip(&c))
                })
                .collect();
            sets.insert(reg.classes[b].name.clone(), row);
        }
        p.sets.bf2.push(Bf2Sets {
            family: reg.families[f].name.clone(),
            sets,
        });
    }
    let layer = |base: &str, l: Layer| {
        sanm.get(base, l).map(|a| Bf2Layer {
            looping: a.looping,
            aim: format!("{:?}", a.aim),
            sync: a.sync.clone(),
            blend_in: a.blend_in,
            blend_out: a.blend_out,
            blend_to: a.blend_to.clone(),
        })
    };
    p.sets.bf2_animations = (0..bf2::PAIRS)
        .map(|i| {
            let name = bf2::base_name(i);
            Bf2Animation {
                upper: layer(&name, Layer::Upper),
                lower: layer(&name, Layer::Lower),
                name,
            }
        })
        .collect();
    p
}

/// A clip's root speed as the PS2 measures it (BF1 `FUN_0022d4f0`, BF2
/// `FUN_00282900`; §2.1, §6.3): the clip is played to half its length and
/// the `DummyRoot` translation divided by that time; a zero or wrong-signed
/// result takes a default — 0.9 walk, 3.5 run (BF2 sprint 11.5), + for
/// forward and left, - for backward and right. `[x, z]` m/s; `None` for a
/// name that is no walk, run or sprint.
pub fn root_speed(name: &str, clip: &mungefs::anim::DecodedClip, bf2: bool) -> Option<[f32; 2]> {
    let (default, rest) = if let Some(i) = name.find("walk") {
        (0.9, &name[i + 4..])
    } else if let Some(i) = name.find("run") {
        (3.5, &name[i + 3..])
    } else if bf2 && name.contains("sprint") {
        (11.5, "forward")
    } else {
        return None;
    };
    // (axis: 0 x, 1 z; sign)
    let (axis, sign) = match rest {
        r if r.starts_with("forward") => (1, 1.0),
        r if r.starts_with("backward") => (1, -1.0),
        r if r.starts_with("left") => (0, 1.0),
        r if r.starts_with("right") => (0, -1.0),
        _ => return None,
    };
    let frames = f32::from(clip.frames);
    let secs = 0.5 * if bf2 { frames - 1.0 } else { frames } / 30.0;
    let mut v = [0.0f32; 2];
    let root = crc32_lower("DummyRoot");
    if let Some(t) = clip.tracks.iter().find(|t| t.bone_crc == root) {
        // The position at the half, not the distance from frame 0
        // (`FUN_0022d4f0` B1 144119-144144): a clip that starts off the
        // origin and walks back to it measures 0 and takes the default
        // (BF1's prone walkbackward, -0.9).
        if !t.translations.is_empty() && secs > 0.0 {
            let at = 0.5 * (frames - 1.0);
            let (i, f) = (at.floor() as usize, at.fract());
            let k0 = t.translations[i.min(t.translations.len() - 1)];
            let k1 = t.translations[(i + 1).min(t.translations.len() - 1)];
            let half = |c: usize| k0[c] + (k1[c] - k0[c]) * f;
            v = [half(0) / secs, half(2) / secs];
        }
    }
    if v[axis] * sign <= 0.0 {
        v[axis] = default * sign;
    }
    Some(v)
}
