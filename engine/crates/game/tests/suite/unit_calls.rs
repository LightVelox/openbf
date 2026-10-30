//! Card UCALL's referee for the voice of BF1's combat calls
//! (`analysis/re-unit-chatter.md` 2.1-2.4, 4): every class the three BF1
//! showcase maps' missions field resolves `AcquiredTargetSound`, `FleeSound`
//! and `HeardEnemySound` through the BAKED class db to its own faction's
//! property, and each has a take in the map's bank; the Gungans' to
//! `gungan_chatter_*` (data-gated: skips without a bake).
//!
//! The rule's referees live beside the code: the sim's events
//! (`core::sim::step` tests: the tracker's take, the grenade's one call a
//! second) and the bank's gates (`game::sounds`'
//! `a_unit_line_is_gated_by_its_take_then_its_property`).

use game::classdb::ClassDb;
use game::mission::MissionDef;
use game::roster::{cries, sound_key, FactionRoster};
use game::sounds::SoundLib;
use std::path::PathBuf;

/// The three BF1 showcase maps' missions (PLAN §Milestones M-ALPHA).
const SHOWCASE: [&str; 3] = ["bf1_nab1_con", "tat1_con", "bf1_yav2_con"];

/// Classes whose calls are not their side's trooper's, as the RE's census
/// of the disc reads them (`lead_tmp/usnd/bf1_classes.tsv`): (class,
/// acquired, flee, heard); "-" = the class authors none.
const PINNED: [(&str, &str, &str, &str); 3] = [
    ("all_inf_marksman", "-", "-", "-"),
    (
        "all_inf_smuggler",
        "-",
        "all_inf_com_chatter_flee_wookie",
        "all_inf_com_chatter_heard_wookie",
    ),
    ("all_inf_lukeskywalker", "-", "-", "-"),
];

const KEYS: [&str; 3] = ["acquired", "flee", "heard"];

#[test]
fn every_fielded_bf1_class_calls_in_its_own_voice() {
    let root = game::assets::baked_root();
    let classes = root.join("classes");
    if !classes.join("bf1_all.ron").exists() {
        eprintln!("SKIP: no baked classes at {}", classes.display());
        return;
    }
    let missions = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../missions"));
    let (mut checked, mut voiced, mut pinned) = (0, 0, 0);
    let mut wrong = Vec::new();
    for name in SHOWCASE {
        for era in ["", "_cw"] {
            let path = missions.join(format!("{name}{era}.ron"));
            if !path.exists() {
                continue;
            }
            let m = MissionDef::load(&path).expect("mission");
            let Some(lib) = SoundLib::load_for(&root.join(&m.map), &m) else {
                eprintln!("SKIP {}: no sound bank", m.map);
                continue;
            };
            let f = m.factions();
            let (rosters, _) = FactionRoster::pair(
                &classes,
                &m.source_game,
                [f[0].as_str(), f[1].as_str()],
                Some(&m),
            )
            .expect("rosters");
            for r in &rosters {
                for c in r.classes.iter().filter(|c| c.droideka.is_none()) {
                    checked += 1;
                    let got = [&c.cries.acquired, &c.cries.flee, &c.cries.heard]
                        .map(|k| k.as_deref().unwrap_or("-").to_owned());
                    let want = match PINNED.iter().find(|p| c.class.starts_with(p.0)) {
                        Some(p) => {
                            pinned += 1;
                            [p.1, p.2, p.3].map(str::to_owned)
                        }
                        None => KEYS.map(|k| format!("{}_inf_com_chatter_{k}", r.faction)),
                    };
                    if got != want {
                        wrong.push(format!("{name}{era} {}: {got:?}, want {want:?}", c.class));
                    }
                    for n in got.iter().filter(|n| *n != "-") {
                        voiced += 1;
                        let take = lib.take_path(n, 0);
                        if !take.as_ref().is_some_and(|p| p.exists()) {
                            wrong.push(format!("{name}{era} {}: {n} has no take", c.class));
                        }
                    }
                    println!("CALLS {} {} {got:?}", m.map, c.class);
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {checked}:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert!(checked >= 30, "only {checked} classes checked");
    assert!(pinned >= 2, "only {pinned} pinned classes fielded");
    println!("{checked} fielded BF1 classes, {voiced} calls with a take");
}

/// NAB1's Gungans (the port fields no team-3 locals yet): their three
/// calls through the baked class db are `gungan_chatter_*`, in the bank.
#[test]
fn the_gungans_call_as_gungans() {
    let root = game::assets::baked_root();
    let file = root.join("classes").join("bf1_gun.ron");
    if !file.exists() {
        eprintln!("SKIP: no baked Gungan classes");
        return;
    }
    let mut db = ClassDb::default();
    db.load(&file).expect("bf1_gun.ron");
    let m = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../missions/bf1_nab1_con.ron"
    ));
    let m = MissionDef::load(&m).expect("mission");
    let lib = SoundLib::load_for(&root.join(&m.map), &m).expect("NAB1 bank");
    for class in ["gun_inf_soldier", "gun_inf_rider", "gun_inf_defender"] {
        for (key, k) in ["AcquiredTargetSound", "FleeSound", "HeardEnemySound"]
            .into_iter()
            .zip(KEYS)
        {
            let n = sound_key(&db, class, key);
            assert_eq!(
                n.as_deref(),
                Some(format!("gungan_chatter_{k}").as_str()),
                "{class} {key}"
            );
            let n = n.unwrap_or_default();
            assert!(lib.take_path(&n, 0).is_some_and(|p| p.exists()), "{n}");
        }
    }
}

/// Card UTALK (re-squad-orders.md 2, 5): the AI's own squad lines of every
/// class the three BF1 showcase maps field, through the BAKED class db -
/// `AISCFieldFollowSound`, `AISCFieldMoveOutSound`, `AISCDriverGetInSound`,
/// `AISCResponseYesSirSound` - are its side's `ai_<s>_inf_com_chatter_tac_*`
/// (the Wookiee and smuggler classes `wok_...`; the marksman none), each with a take in the
/// map's bank; and none of them authors a `LowHealthSound`.
#[test]
fn every_fielded_bf1_class_talks_as_its_sides_ai() {
    let root = game::assets::baked_root();
    let classes = root.join("classes");
    if !classes.join("bf1_all.ron").exists() {
        eprintln!("SKIP: no baked classes at {}", classes.display());
        return;
    }
    let missions = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../missions"));
    let (mut checked, mut wok, mut silent) = (0, 0, 0);
    let mut wrong = Vec::new();
    for name in SHOWCASE {
        for era in ["", "_cw"] {
            let path = missions.join(format!("{name}{era}.ron"));
            if !path.exists() {
                continue;
            }
            let m = MissionDef::load(&path).expect("mission");
            let Some(lib) = SoundLib::load_for(&root.join(&m.map), &m) else {
                eprintln!("SKIP {}: no sound bank", m.map);
                continue;
            };
            let f = m.factions();
            let (rosters, _) = FactionRoster::pair(
                &classes,
                &m.source_game,
                [f[0].as_str(), f[1].as_str()],
                Some(&m),
            )
            .expect("rosters");
            for r in &rosters {
                for c in r.classes.iter().filter(|c| c.droideka.is_none()) {
                    checked += 1;
                    let k = &c.cries;
                    let got = [&k.follow, &k.move_out, &k.need_gunner, &k.yes]
                        .map(|k| k.as_deref().unwrap_or("-").to_owned());
                    let ai = |t: &str| format!("ai_{}_inf_com_chatter_tac_{t}", r.faction);
                    let woks = |t: &str| format!("wok_inf_com_chatter_tac_{t}");
                    let want_ai = ["follow", "moveout", "needgunner", "yes"].map(ai);
                    let want_wok = ["follow", "moveout", "needgunner", "yes"].map(woks);
                    // The marksman (female) class authors no squad line
                    // (the disc census, `lead_tmp/usnd/bf1_classes.tsv`).
                    let mute = c.class.starts_with("all_inf_marksman");
                    if mute {
                        silent += 1;
                        if got.iter().any(|n| n != "-") {
                            wrong.push(format!("{name}{era} {}: {got:?}", c.class));
                        }
                        continue;
                    }
                    if got == want_wok {
                        wok += 1;
                    } else if got != want_ai {
                        wrong.push(format!("{name}{era} {}: {got:?}", c.class));
                    }
                    for n in &got {
                        if !lib.take_path(n, 0).is_some_and(|p| p.exists()) {
                            wrong.push(format!("{name}{era} {}: {n} has no take", c.class));
                        }
                    }
                    if k.low_health.is_some() {
                        wrong.push(format!("{name}{era} {}: a low-health voice", c.class));
                    }
                    println!("TALK {} {} {got:?} low {}", m.map, c.class, k.low_threshold);
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {checked}:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert!(checked >= 30, "only {checked} classes checked");
    assert!(wok >= 2 && silent >= 2, "{wok} wok_, {silent} silent");
    println!("{checked} fielded BF1 classes, {wok} in the wok_ voice, {silent} silent");
}

/// Card UTALK (re-unit-chatter.md 2.8): NAB1's three Gungan classes say
/// `gungan_chatter_heartbeat` as their low-health voice with threshold 1.1,
/// so for as long as they live, and the bank holds a take of it.
#[test]
fn the_gungans_low_health_voice_runs_while_they_live() {
    let root = game::assets::baked_root();
    let file = root.join("classes").join("bf1_gun.ron");
    if !file.exists() {
        eprintln!("SKIP: no baked Gungan classes");
        return;
    }
    let mut db = ClassDb::default();
    db.load(&file).expect("bf1_gun.ron");
    let m = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../missions/bf1_nab1_con.ron"
    ));
    let m = MissionDef::load(&m).expect("mission");
    let lib = SoundLib::load_for(&root.join(&m.map), &m).expect("NAB1 bank");
    for class in ["gun_inf_soldier", "gun_inf_rider", "gun_inf_defender"] {
        let n = sound_key(&db, class, "LowHealthSound");
        assert_eq!(n.as_deref(), Some("gungan_chatter_heartbeat"), "{class}");
        let t = db.prop_f32(class, "LowHealthThreshold");
        assert_eq!(t, Some(1.1), "{class}");
        // ...and the roster's class carries them.
        let c = cries(&db, class);
        assert_eq!(c.low_health, n, "{class}");
        assert_eq!(c.low_threshold, 1.1, "{class}");
        assert!(lib
            .take_path("gungan_chatter_heartbeat", 0)
            .is_some_and(|p| p.exists()));
    }
}
