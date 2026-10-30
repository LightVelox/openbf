//! The leaf patches (card LEAF, `analysis/re-leafpatch.md` [O]): every
//! `leafpatch` class's properties ([`leaf_class`], §3) and the objects props
//! hang on their model's hardpoints ([`attach`], §2). The sprite template the
//! PS2 builds from a class is the runtime's (`game::viewer::leaf`).
//!
//! **The attach rule** (B1 `FUN_001fb6e8`, B2 `FUN_0022f6e8`; §2.1–§2.2):
//! `AttachOdf` sets ONE current class, a global that no other class resets;
//! each `AttachToHardpoint "name [dist]"` appends (current class, the model's
//! hardpoint matrix) — only a hardpoint the model's skeleton has (by the
//! CRC-32 of its lower-case name), up to 30 (BF1) / 16 (BF2) a class;
//! `AttachEffect` clears the current class. So hardpoints a class lists
//! before its own `AttachOdf` take the class current from the previous
//! `entc` (BF2 DAG1's `barrier_1` and `grandtree_3` get their leaves that
//! way). Replayed over the map's `entc` chunks in walk order, this predicts
//! every live attached leaf object in 9 PCSX2 states (§2.3).
//!
//! Only attachments of a `leafpatch` class are written; a current class that
//! is not one (a light, a godray) or that the LVL does not ship attaches
//! nothing here and does not count toward the cap (BF1 and BF2 YAV1's
//! `yav_prop_leaftree` mixes godrays in and still makes its 9 leaf objects
//! in RAM, §2.3).

use manifest::{LeafLayout, ManifestAttach, ManifestClass, ManifestLeafClass};

use crate::hardpoints::Skeletons;

/// `AttachOdf` (FNV of the property name).
pub const ATTACH_ODF: u32 = 0xa9d0_d48b;
/// `AttachToHardpoint`.
pub const ATTACH_TO_HARDPOINT: u32 = 0x3be7_b80a;
/// `AttachEffect`: clears the current class (its hardpoints go to another
/// list).
pub const ATTACH_EFFECT: u32 = 0x6a6c_7e0d;

/// An `entc` BASE that makes a leaf patch class.
pub fn is_leafpatch(base: &str) -> bool {
    base.eq_ignore_ascii_case("leafpatch")
}

/// Whether a class property takes part in the attach replay.
pub fn is_attach_prop(hash: u32) -> bool {
    matches!(hash, ATTACH_ODF | ATTACH_TO_HARDPOINT | ATTACH_EFFECT)
}

/// `%i`: an optionally signed integer prefix (`None` when there is none —
/// the setter then leaves the field alone).
fn int(v: &str) -> Option<i64> {
    let t = v.trim_start();
    let end = t
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map_or(t.len(), |(i, _)| i);
    t[..end].parse().ok()
}

/// A `leafpatch` class's manifest row from its ODF properties (`(fnv hash,
/// value)` in PROP order, a later one overriding an earlier as the setter
/// B1 `FUN_001b27b8` 57475–57643 does), defaults from the constructor
/// `FUN_001b21e0` 57277–57341. A hash the setter does not read is ignored.
pub fn leaf_class(name: &str, props: &[(u32, String)]) -> ManifestLeafClass {
    let mut g = ManifestLeafClass {
        name: name.to_owned(),
        texture: "grass".into(),
        falling_count: 0,
        bird_count: 0,
        particles: 50,
        min_size: 10.0,
        max_size: 15.0,
        radius: 5.0,
        height: 5.0,
        scale: 1.0,
        offset: [0.0; 3],
        taper: 0.0,
        seed: None,
        layout: LeafLayout::Ball,
        vine_run: [2, 4],
        vine_step: 0.6,
        darkness_min: 1.0,
        darkness_max: 1.0,
        alpha: 0.5,
        max_distance: 100.0,
        num_parts: 4,
        wind: [1.2, 0.11],
    };
    let mut vine = [1.0f32, 0.0];
    for (h, v) in props {
        let f: Vec<Option<f32>> = v.split_whitespace().map(|t| t.parse().ok()).collect();
        let fl = |k: usize| f.get(k).copied().flatten();
        let one = |d: f32| fl(0).unwrap_or(d);
        match *h {
            0x3b19_48f3 => g.bird_count = int(v).map_or(0, |n| n.clamp(0, 5) as u8),
            0xfd7f_ae90 => {
                g.falling_count = int(v).map_or(g.falling_count, |n| n.clamp(0, 50) as u8)
            }
            0xd133_f7da => g.particles = int(v).map_or(g.particles, |n| n.max(0) as u32),
            0xeaeb_cf6e => g.min_size = one(g.min_size),
            0x80ba_1308 => g.max_size = one(g.max_size),
            0x0dba_4cb3 => g.radius = one(g.radius),
            0xd5bd_bb42 => g.height = one(g.height),
            0xcaaa_3778 => g.scale = one(g.scale),
            0x14c8_d3ca => g.offset = [0, 1, 2].map(|k| fl(k).unwrap_or(0.0)),
            0xeaaa_a7a9 => g.taper = one(g.taper),
            // `%i` into a u32: the disc's 5232344514 is 937377218 in RAM.
            0x5045_bcac => g.seed = int(v).map(|n| n as u32).or(g.seed),
            0x35a4_393b => {
                g.layout = LeafLayout::Box([0, 1, 2].map(|k| fl(k).unwrap_or(0.0)));
            }
            0xcdbc_383d => {
                vine = [fl(0).unwrap_or(vine[0]), fl(1).unwrap_or(vine[1])];
                g.layout = LeafLayout::Vine(vine);
            }
            0xa8c0_d41d => {
                let mut it = v.split_whitespace().map(int);
                g.vine_run = [
                    it.next().flatten().map_or(g.vine_run[0], |n| n as i32),
                    it.next().flatten().map_or(g.vine_run[1], |n| n as i32),
                ];
            }
            0x4532_8dc6 => g.vine_step = one(g.vine_step),
            0x13fa_c25e => g.darkness_min = one(g.darkness_min),
            0x25e5_c3fc => g.darkness_max = one(g.darkness_max),
            0x5d8b_6dab => g.alpha = one(g.alpha),
            0xe691_5cf6 => g.max_distance = one(g.max_distance),
            0x3c64_68f4 => g.texture = v.trim().to_owned(),
            0x5ac0_2207 => g.num_parts = int(v).map_or(g.num_parts, |n| n as i32),
            0xd4cb_d35b => g.wind[0] = one(g.wind[0]),
            0x7cbb_796c => g.wind[1] = one(g.wind[1]),
            _ => {}
        }
    }
    g
}

/// Replay the attach properties of every class (`entcs`: `(name, props)` in
/// the walk's order, only [`is_attach_prop`] properties needed) and fill
/// [`ManifestClass::attach`] of each class in `classes` whose model
/// `skels` holds. `cap` = 30 (BF1) or 16 (BF2).
pub fn attach(
    entcs: &[(String, Vec<(u32, String)>)],
    leaf: &[ManifestLeafClass],
    cap: usize,
    skels: &Skeletons,
    classes: &mut [ManifestClass],
) {
    let leaf_name = |n: &str| {
        leaf.iter()
            .find(|l| l.name.eq_ignore_ascii_case(n.trim()))
            .map(|l| l.name.clone())
    };
    let mut current: Option<String> = None;
    for (name, props) in entcs {
        let class = classes
            .iter_mut()
            .find(|k| k.name.eq_ignore_ascii_case(name));
        let skel = class
            .as_ref()
            .and_then(|k| k.geometry.as_deref())
            .and_then(|g| skels.find(g));
        let binds = skel.and_then(|s| s.bind_matrices().ok());
        let mut out: Vec<ManifestAttach> = Vec::new();
        for (h, v) in props {
            match *h {
                ATTACH_ODF => current = leaf_name(v),
                ATTACH_EFFECT => current = None,
                ATTACH_TO_HARDPOINT => {
                    let (Some(cur), Some(s), Some(m)) = (&current, skel, &binds) else {
                        continue;
                    };
                    // `"%s %f"`: the hardpoint, then an optional MaxDistance
                    // override no leaf ODF uses.
                    let Some(hp) = v.split_whitespace().next() else {
                        continue;
                    };
                    if out.len() >= cap {
                        continue;
                    }
                    if let Some(i) = s.index_of(hp) {
                        let r = m[i];
                        out.push(ManifestAttach {
                            class: cur.clone(),
                            hardpoint: hp.to_owned(),
                            up: [r[1][0], r[1][1], r[1][2]],
                            pos: [r[3][0], r[3][1], r[3][2]],
                        });
                    }
                }
                _ => {}
            }
        }
        if let Some(k) = class {
            if !out.is_empty() {
                k.attach = out;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(p: &[(u32, &str)]) -> Vec<(u32, String)> {
        p.iter().map(|&(h, v)| (h, v.to_owned())).collect()
    }

    #[test]
    fn falling_count_is_optional_and_bounded() {
        assert_eq!(leaf_class("empty", &[]).falling_count, 0);
        for (value, count) in [("6", 6), ("90", 50), ("-1", 0), ("bad", 0)] {
            assert_eq!(
                leaf_class("leaf", &props(&[(0xfd7f_ae90, value)])).falling_count,
                count
            );
        }
    }

    #[test]
    fn bird_counts_stay_within_the_authored_budget() {
        for (input, expected) in [("-1", 0), ("0", 0), ("3", 3), ("99", 5)] {
            let leaf = leaf_class("fixture", &[(0x3b19_48f3, input.into())]);
            assert_eq!(leaf.bird_count, expected, "{input}");
        }
    }

    #[test]
    fn falling_leaf_map_census_matches_both_discs() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../extracted"));
        for (game, expected) in [
            ("bf1", vec!["YAV1", "YAV2", "YAV2_H"]),
            ("bf2", vec!["DAG1", "KAS2", "YAV1"]),
        ] {
            let dir = root.join(game).join("lvl/DATA/_LVL_PS2");
            let Ok(dirs) = std::fs::read_dir(&dir) else {
                eprintln!("SKIP: {}", dir.display());
                continue;
            };
            let mut found = Vec::new();
            for entry in dirs.flatten().filter(|e| e.path().is_dir()) {
                if ![
                    "BES", "COR", "DAG", "DEA", "END", "FEL", "GEO", "HOT", "KAM", "KAS", "MUS",
                    "MYG", "NAB", "POL", "RHN", "TAN", "TAT", "UTA", "YAV",
                ]
                .contains(&entry.file_name().to_str().unwrap_or(""))
                {
                    continue;
                }
                for entry in std::fs::read_dir(entry.path()).unwrap().flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) != Some("LVL") {
                        continue;
                    }
                    let buf = std::fs::read(&path).unwrap();
                    let chunks = mungefs::content_chunks(mungefs::open(&buf).unwrap()).unwrap();
                    let mut emits = false;
                    for c in chunks.iter().filter(|c| &c.tag == b"entc") {
                        let e = mungefs::entity::EntityClass::parse(c).unwrap();
                        if !is_leafpatch(&e.base) {
                            continue;
                        }
                        let props = e
                            .props
                            .iter()
                            .map(|p| (p.key_hash, p.value.clone()))
                            .collect::<Vec<_>>();
                        let class = leaf_class(&e.name, &props);
                        if class.falling_count != 0 {
                            assert_eq!(class.falling_count, 6, "{} {}", path.display(), e.name);
                            assert_eq!(class.bird_count, 3, "{} {}", path.display(), e.name);
                            emits = true;
                        }
                    }
                    if emits {
                        found.push(path.file_stem().unwrap().to_str().unwrap().to_owned());
                    }
                }
            }
            found.sort();
            assert_eq!(found, expected, "{game}");
        }
    }

    /// BF1 NAB1's class as the disc spells it (re-leafpatch §1); a seed past
    /// 2³² wraps; the vine and box set their layouts.
    #[test]
    fn a_class_reads_its_properties() {
        let g = leaf_class(
            "nab_prop_leafpatch",
            &props(&[
                (0xd133_f7da, "60"),
                (0xeaeb_cf6e, "5.7"),
                (0x80ba_1308, "6.0"),
                (0x0dba_4cb3, "9.5"),
                (0xd5bd_bb42, "8"),
                (0x5045_bcac, "5232344514"),
                (0x3c64_68f4, "nab_leaf"),
                (0x5ac0_2207, "1"),
                (0xe691_5cf6, "120"),
            ]),
        );
        assert_eq!((g.particles, g.min_size, g.max_size), (60, 5.7, 6.0));
        assert_eq!((g.radius, g.height, g.scale), (9.5, 8.0, 1.0));
        assert_eq!(g.seed, Some(937_377_218));
        assert_eq!((g.texture.as_str(), g.num_parts), ("nab_leaf", 1));
        assert_eq!(g.layout, LeafLayout::Ball);
        let v = leaf_class("v", &props(&[(0xcdbc_383d, "0 0.9"), (0xa8c0_d41d, "3 5")]));
        assert_eq!(v.layout, LeafLayout::Vine([0.0, 0.9]));
        assert_eq!((v.vine_run, v.seed, v.alpha), ([3, 5], None, 0.5));
        let b = leaf_class("b", &props(&[(0x35a4_393b, "1 2 3")]));
        assert_eq!(b.layout, LeafLayout::Box([1.0, 2.0, 3.0]));
    }

    /// Data-gated: every leaf object the PS2 creates from BF1 NAB1 / BF2
    /// DAG1 / BF2 KAS2 props, counted per class as re-leafpatch §2.3 found
    /// them live (NAB1 106 trees × 5; DAG1 197 on world props, two classes
    /// by the carried-over current class; KAS2 4 × 12 of 19 listed).
    #[test]
    fn the_replay_attaches_what_the_ps2_creates() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../extracted/");
        for (lvl, game, want) in [
            (
                "bf1/lvl/DATA/_LVL_PS2/NAB/NAB1.LVL",
                "bf1",
                vec![("nab_prop_trees2", 5)],
            ),
            (
                "bf2/lvl/DATA/_LVL_PS2/DAG/DAG1.LVL",
                "bf2",
                vec![
                    ("dag1_prop_grandtree", 3),
                    ("dag1_prop_barrier_1", 6),
                    ("dag1_prop_grandtree_3", 1),
                    ("dag1_prop_grandtree_2", 0),
                    ("dag1_prop_treegroup_1", 9),
                    ("dag1_prop_treegroup_2", 12),
                ],
            ),
            (
                "bf2/lvl/DATA/_LVL_PS2/KAS/KAS2.LVL",
                "bf2",
                vec![("kas_prop_treegroup", 12)],
            ),
        ] {
            let Ok(buf) = std::fs::read(format!("{root}{lvl}")) else {
                eprintln!("SKIP: {lvl} not present");
                continue;
            };
            let inner = mungefs::open(&buf).unwrap();
            let chunks = mungefs::content_chunks(inner).unwrap();
            let (mut classes, mut entcs, mut leaf) = (Vec::new(), Vec::new(), Vec::new());
            for c in &chunks {
                if &c.tag != b"entc" {
                    continue;
                }
                let e = mungefs::entity::EntityClass::parse(c).unwrap();
                let p: Vec<(u32, String)> = e
                    .props
                    .iter()
                    .map(|p| (p.key_hash, p.value.clone()))
                    .collect();
                if is_leafpatch(&e.base) {
                    leaf.push(leaf_class(&e.name, &p));
                }
                entcs.push((
                    e.name.clone(),
                    p.into_iter().filter(|p| is_attach_prop(p.0)).collect(),
                ));
                classes.push(crate::map::class_from_entc(&e));
            }
            let cap = if game == "bf2" { 16 } else { 30 };
            attach(&entcs, &leaf, cap, &Skeletons::of(&chunks), &mut classes);
            for (name, n) in want {
                let Some(k) = classes.iter().find(|k| k.name.eq_ignore_ascii_case(name)) else {
                    // A class of a mode layer the flattened walk may name
                    // differently: say so rather than pass silently.
                    panic!("{lvl}: no class {name}");
                };
                assert_eq!(k.attach.len(), n, "{lvl} {name}: {:?}", k.attach);
            }
        }
    }
}
