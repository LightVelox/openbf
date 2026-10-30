//! The soldier animation-set resolvers against the keys the checked Python
//! resolvers wrote (card ANIMS, `analysis/re-soldier-anim-layers.md` §2.4,
//! §6.3): `tests/data/anim_sets_bf1.json` is `sets_bf1.py`'s six tables (equal
//! to PS2 memory in 979 of 979 slots), `anim_sets_bf2.json` `sets_bf2.py`'s
//! model of three save states' sets (equal to PS2 memory in every slot) with
//! the registry each state holds. Disc data is gitignored: SKIP when absent.

use bake::anim_sets::{bf1, bf2, ClipRef, Inventory};
use serde_json::Value;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");
const DISC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../extracted");

fn key(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(format!("{DATA}/{name}")).unwrap()).unwrap()
}

/// Add every bank of an LVL to `inv`; `false` when the file is absent.
fn add_lvl(inv: &mut Inventory, path: &str) -> bool {
    let Ok(buf) = std::fs::read(path) else {
        return false;
    };
    for b in mungefs::anim::find_banks(mungefs::open(&buf).unwrap()).unwrap() {
        inv.add(&b);
    }
    true
}

fn bf1_tables(fallback: &[usize; 9]) -> Option<bf1::Tables> {
    let buf = std::fs::read(format!(
        "{DISC}/bf1/lvl/DATA/_LVL_PS2/COMMON/ENG/COMMON.LVL"
    ))
    .ok()?;
    let banks = mungefs::anim::find_banks(mungefs::open(&buf).unwrap()).unwrap();
    Some(bf1::build_with(&bf1::inventory(&banks), fallback))
}

/// (slots compared, slots that differ) of one table against its key, any
/// nesting depth.
fn diff_bf1(got: &Value, want: &Value, n: &mut (usize, usize)) {
    match (got, want) {
        (Value::Array(g), Value::Array(w)) if w.first().is_some_and(|x| !x.is_string()) => {
            assert_eq!(g.len(), w.len());
            for (g, w) in g.iter().zip(w) {
                diff_bf1(g, w, n);
            }
        }
        _ => {
            n.0 += 1;
            n.1 += usize::from(got != want);
        }
    }
}

fn slot(c: &Option<ClipRef>) -> Value {
    c.as_ref()
        .map_or(Value::Null, |c| serde_json::json!([c.name, c.frames]))
}

fn bf1_value(t: &bf1::Tables) -> Value {
    let v1 = |r: &Vec<Option<ClipRef>>| Value::Array(r.iter().map(slot).collect());
    let v2 = |t: &Vec<Vec<Option<ClipRef>>>| Value::Array(t.iter().map(v1).collect());
    let v3 = |t: &Vec<Vec<Vec<Option<ClipRef>>>>| Value::Array(t.iter().map(v2).collect());
    serde_json::json!({
        "base": v2(&t.base), "special": v1(&t.special), "W": v3(&t.w),
        "ALERT": v2(&t.alert), "U": v3(&t.u), "S": v2(&t.s),
    })
}

fn bf1_diff(t: &bf1::Tables) -> (usize, usize) {
    let (got, want) = (bf1_value(t), key("anim_sets_bf1.json"));
    let mut n = (0, 0);
    for k in ["base", "special", "W", "ALERT", "U", "S"] {
        diff_bf1(&got[k], &want[k], &mut n);
    }
    n
}

#[test]
fn bf1s_six_tables_equal_the_ps2s_in_979_slots() {
    let Some(t) = bf1_tables(&bf1::FALLBACK) else {
        eprintln!("SKIP: extracted BF1 COMMON.LVL not present");
        return;
    };
    let (slots, differ) = bf1_diff(&t);
    println!("bf1: {} of {slots} slots equal", slots - differ);
    assert_eq!((slots, differ), (979, 0));
    // Section 5: the super battle droid's own shoot over the tool set's rows.
    let name = |c: &Option<ClipRef>| c.as_ref().map(|c| (c.name.clone(), c.frames));
    assert_eq!(
        name(&t.u[0][6][1]),
        Some(("human_sbdroid_stand_shoot".into(), 7))
    );
    assert_eq!(name(&t.u[0][2][5]).unwrap().0, "human_pistol_stand_reload");
    // `idle` is empty in every set: no such clip is on the disc.
    assert!(t.u.iter().flatten().all(|w| w[0].is_none()));
}

/// The mutation: without the fallback table (every set its own fallback) the
/// pistol, grenade, sabre and super-battle-droid sets lose the tool set's rows.
#[test]
fn bf1_without_the_fallback_table_is_not_the_ps2s() {
    let own = [0, 1, 2, 3, 4, 5, 6, 7, 8];
    let Some(t) = bf1_tables(&own) else {
        eprintln!("SKIP: extracted BF1 COMMON.LVL not present");
        return;
    };
    let (slots, differ) = bf1_diff(&t);
    println!("bf1 without the fallback table: {differ} of {slots} slots differ");
    assert_eq!(slots, 979);
    assert!(differ > 100, "{differ}");
}

fn bf2_inventory() -> Option<Inventory> {
    let dir = format!("{DISC}/bf2/lvl/DATA/_LVL_PS2");
    let mut inv = Inventory::default();
    if !add_lvl(&mut inv, &format!("{dir}/INGAME.LVL")) {
        return None;
    }
    let mut sides: Vec<_> = std::fs::read_dir(format!("{dir}/SIDE"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "LVL"))
        .collect();
    sides.sort();
    for s in sides {
        add_lvl(&mut inv, s.to_str().unwrap());
    }
    Some(inv)
}

/// The registry a key state holds in RAM.
fn key_registry(st: &Value) -> bf2::Registry {
    let idx = |v: &Value, me: usize| {
        let p = v.as_i64().unwrap();
        (p >= 0 && p as usize != me).then_some(p as usize)
    };
    bf2::Registry {
        families: st["families"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, f)| bf2::Family {
                name: f["name"].as_str().unwrap().into(),
                parent: idx(&f["parent"], i),
            })
            .collect(),
        classes: st["classes"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, c)| bf2::WeaponClass {
                name: c["name"].as_str().unwrap().into(),
                parent: idx(&c["parent"], i),
                combo: c["combo"].as_i64().unwrap() >= 0,
                alert: c["alert"].as_i64().unwrap() != 0,
            })
            .collect(),
        sets: st["sets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                (
                    s[0].as_u64().unwrap() as usize,
                    s[1].as_u64().unwrap() as usize,
                )
            })
            .collect(),
    }
}

/// A key set's 268 slots as names.
fn key_slots(k: &Value, slots: &Value) -> Vec<Option<String>> {
    slots
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            let i = i.as_i64().unwrap();
            (i >= 0).then(|| k["names"][i as usize].as_str().unwrap().to_string())
        })
        .collect()
}

#[test]
fn bf2s_sets_equal_the_ps2s_for_three_battle_states() {
    let Some(inv) = bf2_inventory() else {
        eprintln!("SKIP: extracted BF2 LVLs not present");
        return;
    };
    assert_eq!(inv.len(), 632);
    let k = key("anim_sets_bf2.json");
    let (mut total, mut differ) = (0, 0);
    for st in k["states"].as_array().unwrap() {
        let reg = key_registry(st);
        let built = bf2::build(&inv, &reg);
        let (mut n, mut bad) = (0, 0);
        for (name, slots) in st["slots"].as_object().unwrap() {
            let (f, c) = name.split_once(',').unwrap();
            let got = &built[&(reg.family(f).unwrap(), reg.class(c).unwrap())];
            let want = key_slots(&k, slots);
            assert_eq!(got.len(), want.len());
            for (w, (g, wnt)) in got.iter().zip(&want).enumerate() {
                n += 1;
                if g != wnt {
                    bad += 1;
                    if bad <= 5 {
                        println!(
                            "  {name} {} {}: {g:?} | key {wnt:?}",
                            bf2::base_name(w / 2),
                            w % 2
                        );
                    }
                }
            }
        }
        println!(
            "{}: {} sets, {} of {n} slots equal",
            st["state"].as_str().unwrap(),
            st["slots"].as_object().unwrap().len(),
            n - bad
        );
        total += n;
        differ += bad;
    }
    assert_eq!((total, differ), (11524, 0));
}

/// The `entc` / `wpnc` records of BF2 side LVLs.
fn side_classes(sides: &[&str]) -> Option<Vec<manifest::ClassDef>> {
    let buf = std::fs::read(format!("{DISC}/bf2/lvl/DATA/_LVL_PS2/INGAME.LVL")).ok()?;
    let mut out = bake::anim_sets::bf2::side_classes(&buf).unwrap();
    for s in sides {
        let buf = std::fs::read(format!("{DISC}/bf2/lvl/DATA/_LVL_PS2/SIDE/{s}.LVL")).ok()?;
        out.extend(bake::anim_sets::bf2::side_classes(&buf).unwrap());
    }
    Some(out)
}

/// The registry the ODFs of a state's two sides give, against the one the PS2
/// holds in that state. Among the families the state loaded the sets are the
/// PS2's own, and resolving with the ODF registry (which holds more: both
/// sides' whole class lists, where a mission reads a few) gives them the same
/// clips. One set per state is NOT given by the rule here and is left out: a
/// Jedi's (family, rifle) - `kiadimundi`, `vader` - beside his sabre's combo
/// set. No hero is baked; what registers it is unread.
#[test]
fn the_side_odfs_register_the_ps2s_sets() {
    let (Some(inv), Some(gcw), Some(cw)) = (
        bf2_inventory(),
        side_classes(&["ALL", "IMP"]),
        side_classes(&["REP", "CIS"]),
    ) else {
        eprintln!("SKIP: extracted BF2 LVLs not present");
        return;
    };
    let k = key("anim_sets_bf2.json");
    let (mut total, mut differ) = (0, 0);
    for st in k["states"].as_array().unwrap() {
        let state = st["state"].as_str().unwrap();
        let ram = key_registry(st);
        let mut reg = bf2::Registry::shipped();
        reg.register(if state.contains("_cw_") { &cw } else { &gcw });
        let built = bf2::build(&inv, &reg);
        let named = |r: &bf2::Registry| -> Vec<String> {
            let mut v: Vec<String> = r
                .sets
                .iter()
                .filter(|s| !r.classes[s.1].combo)
                .map(|s| format!("{},{}", r.families[s.0].name, r.classes[s.1].name))
                .collect();
            v.sort();
            v
        };
        let (have, want) = (named(&reg), named(&ram));
        let fams: Vec<&str> = ram.families.iter().map(|f| f.name.as_str()).collect();
        // Among the families the state loaded, the sets are the PS2's own.
        let mine: Vec<&String> = have
            .iter()
            .filter(|s| fams.contains(&s.split_once(',').unwrap().0))
            .collect();
        println!("{state}: ODF sets of its families {mine:?}");
        let want: Vec<String> = want
            .into_iter()
            .filter(|s| !["kiadimundi,rifle", "vader,rifle"].contains(&s.as_str()))
            .collect();
        assert_eq!(mine, want.iter().collect::<Vec<_>>(), "{state}");
        for f in &ram.families {
            let mine = &reg.families[reg.family(&f.name).unwrap()];
            let parent =
                |r: &bf2::Registry, p: Option<usize>| p.map(|p| r.families[p].name.clone());
            assert_eq!(
                parent(&reg, mine.parent),
                parent(&ram, f.parent),
                "{}",
                f.name
            );
        }
        for (name, slots) in st["slots"].as_object().unwrap() {
            if !want.contains(name) {
                continue;
            }
            let (f, c) = name.split_once(',').unwrap();
            let got = &built[&(reg.family(f).unwrap(), reg.class(c).unwrap())];
            for (g, w) in got.iter().zip(&key_slots(&k, slots)) {
                total += 1;
                differ += usize::from(g != w);
            }
        }
    }
    println!("ODF registries: {} of {total} slots equal", total - differ);
    assert_eq!((total, differ), (11524 - 2 * 268, 0));
}

/// BF1's root speeds against the set object in PS2 memory (§2.1 [M]):
/// walkforward 3.687, runforward 6.338, walkbackward -1.245, runbackward
/// -3.724, crouch walk 2.075 / -2.050, prone 1.010 / -0.900, and the two
/// defaults prone walkleft +0.945 (measured) / walkright -0.900.
#[test]
fn bf1s_root_speeds_are_the_ps2s() {
    let Ok(buf) = std::fs::read(format!(
        "{DISC}/bf1/lvl/DATA/_LVL_PS2/COMMON/ENG/COMMON.LVL"
    )) else {
        eprintln!("SKIP: extracted BF1 COMMON.LVL not present");
        return;
    };
    let banks = mungefs::anim::find_banks(mungefs::open(&buf).unwrap()).unwrap();
    let speed = |name: &str| {
        let (b, k) = banks
            .iter()
            .find_map(|b| Some((b, b.clip_index(name)?)))
            .unwrap();
        bake::anim_sets::root_speed(name, &b.decode_clip(k).unwrap(), false).unwrap()
    };
    for (name, axis, want) in [
        ("human_rifle_stand_walkforward", 1, 3.687),
        ("human_rifle_stand_runforward", 1, 6.338),
        ("human_rifle_stand_walkbackward", 1, -1.245),
        ("human_rifle_stand_runbackward", 1, -3.724),
        ("human_rifle_crouch_walkforward", 1, 2.075),
        ("human_rifle_crouch_walkbackward", 1, -2.050),
        ("human_rifle_prone_walkforward", 1, 1.010),
        ("human_rifle_prone_walkbackward", 1, -0.900),
        ("human_rifle_prone_walkleft", 0, 0.945),
        ("human_rifle_prone_walkright", 0, -0.900),
    ] {
        let got = speed(name)[axis];
        println!("{name}: {got:.3} (PS2 {want:.3})");
        assert!((got - want).abs() < 0.0015, "{name}: {got} vs {want}");
    }
}

/// What a GLB's JSON chunk names its animations.
fn glb_animations(path: &std::path::Path) -> Vec<String> {
    let d = std::fs::read(path).unwrap();
    let n = u32::from_le_bytes(d[12..16].try_into().unwrap()) as usize;
    let j: Value = serde_json::from_slice(&d[20..20 + n]).unwrap();
    j["animations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap().to_string())
        .collect()
}

/// Bake one soldier of `game` and read back its sets and its GLB's clips.
fn baked_sets(
    game: &str,
    side: &str,
    model: &str,
) -> Option<(bake::anim_sets::SoldierSets, Vec<String>)> {
    let dir = format!("{DISC}/{game}/lvl/DATA/_LVL_PS2");
    let anims = if game == "bf1" {
        format!("{dir}/COMMON/ENG/COMMON.LVL")
    } else {
        format!("{dir}/INGAME.LVL")
    };
    let lvl = format!("{dir}/SIDE/{side}.LVL");
    if !std::path::Path::new(&lvl).exists() || !std::path::Path::new(&anims).exists() {
        return None;
    }
    let out = std::env::temp_dir().join(format!(
        "openbf-bake-test-anim-sets-{game}-{model}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&out);
    bake::soldier::bake_soldier_anims(&lvl, model, Some(&anims), out.to_str().unwrap(), None)
        .unwrap();
    let sets: bake::anim_sets::SoldierSets = ron::de::from_str(
        &std::fs::read_to_string(out.join(format!("soldiers/{model}.sets.ron"))).unwrap(),
    )
    .unwrap();
    let clips = glb_animations(&out.join(format!("soldiers/{model}.glb")));
    let _ = std::fs::remove_dir_all(&out);
    Some((sets, clips))
}

/// A baked BF1 soldier carries all nine weapon sets' clips, the 13-bone
/// upper mask and the PS2's record facts; the old rifle list is still there.
#[test]
fn a_baked_bf1_soldier_carries_its_sets() {
    let Some((sets, glb)) = baked_sets("bf1", "CIS", "cis_inf_sbdroid") else {
        eprintln!("SKIP: extracted BF1 LVLs not present");
        return;
    };
    assert_eq!(sets.game, "bf1");
    assert_eq!(sets.upper_bones.len(), 13);
    assert!(sets.lower_bones.iter().any(|b| b == "hp_weapons"));
    assert!(sets.lower_bones.iter().any(|b| b == "bone_pelvis"));
    // 151 clips in the tables; every one is an animation of the GLB.
    assert_eq!(sets.clips.len(), 151);
    for c in &sets.clips {
        assert!(glb.contains(&c.name), "{}", c.name);
    }
    for old in bake::soldier::SOLDIER_CLIPS {
        assert!(glb.iter().any(|n| n == old), "{old}");
    }
    let t = sets.bf1.as_ref().unwrap();
    let clip = |i: Option<u16>| &sets.clips[usize::from(i.unwrap())];
    // The super battle droid (set 6): its own 7-frame shoot, blend 0.02, no
    // loop; its own run on the upper body over the rifle's legs.
    let shoot = clip(t.u[0][6][1]);
    assert_eq!(
        (shoot.name.as_str(), shoot.frames),
        ("human_sbdroid_stand_shoot", 7)
    );
    assert_eq!((shoot.looping, shoot.blend), (Some(false), Some(0.02)));
    assert_eq!(clip(t.w[0][6][7]).name, "human_sbdroid_stand_runforward");
    let legs = clip(t.base[0][7]);
    assert_eq!(legs.name, "human_rifle_stand_runforward");
    assert_eq!((legs.looping, legs.blend), (Some(true), Some(0.15)));
    assert_eq!(clip(t.base[2][5]).blend, Some(0.4));
    assert_eq!(clip(t.base[0][3]).looping, Some(false));
    let z = sets.root_speed["human_rifle_stand_runforward"][1];
    assert!((z - 6.338).abs() < 0.0015, "{z}");
    assert!(!sets
        .root_speed
        .contains_key("human_sbdroid_stand_runforward"));
}

/// A baked BF2 soldier carries its family's sets: the super battle droid's
/// one set (sbdroid, rifle) with its own upper clips over `human_rifle` legs,
/// a clone's six `human` sets (including shared saber locomotion), the masks
/// and the `sanm` facts.
#[test]
fn a_baked_bf2_soldier_carries_its_sets() {
    let (Some((sbd, sbd_glb)), Some((clone, clone_glb))) = (
        baked_sets("bf2", "CIS", "cis_inf_sbdroid"),
        baked_sets("bf2", "REP", "rep_inf_ep3trooper"),
    ) else {
        eprintln!("SKIP: extracted BF2 LVLs not present");
        return;
    };
    for (s, glb) in [(&sbd, &sbd_glb), (&clone, &clone_glb)] {
        assert_eq!(s.game, "bf2");
        // All but DummyRoot, root, pelvis and the legs.
        assert_eq!(s.lower_bones.len(), 10);
        assert!(s.upper_bones.iter().any(|b| b == "hp_weapons"));
        assert!(!s
            .upper_bones
            .iter()
            .chain(&s.lower_bones)
            .any(|b| b == "DummyRoot"));
        for c in &s.clips {
            assert!(glb.contains(&c.name), "{}", c.name);
        }
        assert_eq!(s.bf2_animations.len(), bf2::PAIRS);
    }
    // The bake is reproducible: the same file from a second run (the sets
    // come out of a hash map; six of them here).
    assert_eq!(
        baked_sets("bf2", "REP", "rep_inf_ep3trooper").unwrap().0,
        clone
    );
    assert_eq!(sbd.bf2.len(), 1);
    assert_eq!(sbd.bf2[0].family, "sbdroid");
    assert_eq!(sbd.bf2[0].sets.keys().collect::<Vec<_>>(), ["rifle"]);
    let pair = (0..bf2::PAIRS)
        .find(|&p| bf2::base_name(p) == "stand_runforward")
        .unwrap();
    let rifle = &sbd.bf2[0].sets["rifle"];
    let name = |s: &bake::anim_sets::SoldierSets, i: Option<u16>| {
        s.clips[usize::from(i.unwrap())].name.clone()
    };
    assert!(name(&sbd, rifle[pair * 2]).starts_with("sbdroid_rifle_stand_runforward"));
    assert_eq!(
        name(&sbd, rifle[pair * 2 + 1]),
        "human_rifle_stand_runforward"
    );
    assert_eq!(
        clone.bf2[0].sets.keys().collect::<Vec<_>>(),
        ["bazooka", "melee", "pistol", "rifle", "sabre", "tool"]
    );
    assert_eq!(clone.bf2[0].family, "human");
    let sabre = &clone.bf2[0].sets["sabre"];
    assert_eq!(
        name(&clone, sabre[pair * 2]),
        "human_sabre_stand_runforward_full"
    );
    assert_eq!(
        name(&clone, sabre[pair * 2 + 1]),
        "human_sabre_stand_runforward_full"
    );
    let bazooka = &clone.bf2[0].sets["bazooka"];
    assert!(name(&clone, bazooka[pair * 2]).starts_with("human_bazooka_stand_runforward"));
    assert_eq!(
        name(&clone, bazooka[pair * 2 + 1]),
        "human_rifle_stand_runforward"
    );
    let run = &clone.bf2_animations[pair];
    let lower = run.lower.as_ref().unwrap();
    assert!(lower.looping);
    assert_eq!((lower.blend_in, lower.blend_out), (0.15, 0.15));
    assert_eq!(lower.sync.as_deref(), Some("forward"));
    assert_eq!(run.upper.as_ref().unwrap().aim, "Head");
    assert!(clone.root_speed["human_rifle_stand_runforward"][1] > 3.0);
}
