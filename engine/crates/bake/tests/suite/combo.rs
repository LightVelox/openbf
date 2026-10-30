//! Every PS2 side combo, with an unknown-keyword mutation of real disc bytes.

use bake::combo::{combos_from_lvl, dangling_transitions, model_bindings, resolve_animation_clips};
use manifest::combo::{ComboDef, ComboKeyword as K, ComboSet, ComboValue};
use std::path::{Path, PathBuf};

fn disc() -> Option<PathBuf> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../extracted/bf2/lvl/DATA/_LVL_PS2");
    if !root.is_dir() {
        eprintln!("SKIP: extracted BF2 PS2 disc absent");
        None
    } else {
        Some(root)
    }
}

#[test]
fn every_ps2_side_combo_keyword_is_known() {
    let Some(disc) = disc() else {
        return;
    };
    let expected = [
        ("ALL", 1),
        ("CIS", 3),
        ("GAM", 1),
        ("GEO", 1),
        ("IMP", 2),
        ("JED", 4),
        ("REP", 5),
        ("SNW", 1),
    ];
    let mut total = 0;
    let mut dangling = Vec::new();
    for (side, count) in expected {
        let buf = std::fs::read(disc.join("SIDE").join(format!("{side}.LVL"))).unwrap();
        let set = combos_from_lvl(&buf).unwrap();
        println!("{side}: {} unique comb definitions", set.combos.len());
        assert_eq!(set.combos.len(), count, "{side}");
        total += count;
        for c in set.combos {
            dangling.extend(
                dangling_transitions(&c)
                    .into_iter()
                    .map(|p| (side, c.name_hash, p)),
            );
        }
    }
    assert_eq!(total, 18);
    // Preserve this authored PS2 target rather than silently invent a state.
    assert_eq!(dangling.len(), 1);
    assert_eq!(dangling[0].0, "IMP");
    assert_eq!(dangling[0].1, 0xe4c0_31da);
    assert_eq!(dangling[0].2 .1, "RECOVER2");
}

#[test]
fn actual_requested_resources_distinguish_empty_from_missing_combo_loads() {
    let Some(disc) = disc() else { return };
    for (side, request, combo) in [
        ("ALL", "all_hero_luke_jedi", Some("all_hero_luke_jedi")),
        ("ALL", "all_hero_hansolo_tat", None),
        ("GEO", "gen_inf_geonosian", None),
        ("GAM", "gam_inf_gamorreanguard", Some("gam_inf_axe")),
        ("WOK", "wok_inf_basic", None),
        ("EWK", "ewk_inf_basic", None),
    ] {
        let raw = std::fs::read(disc.join("SIDE").join(format!("{side}.LVL"))).unwrap();
        let set = combos_from_lvl(&raw).unwrap();
        assert!(set.scope_provenance && set.load_provenance);
        let expected: Vec<_> = combo.into_iter().map(mungefs::fnv1a_lowercase).collect();
        assert_eq!(
            set.loads.get(&mungefs::fnv1a_lowercase(request)),
            Some(&expected),
            "{side}/{request}"
        );
        for name in set.loads.values().flatten() {
            assert!(set.combos.iter().any(|c| c.name_hash == *name));
        }
    }
}

#[test]
fn mace_attack_and_block_are_read_from_ps2_data() {
    let Some(disc) = disc() else {
        return;
    };
    let buf = std::fs::read(disc.join("SIDE/REP.LVL")).unwrap();
    let set = combos_from_lvl(&buf).unwrap();
    let combo = set
        .combos
        .iter()
        .find(|c| c.name_hash == mungefs::fnv1a_lowercase("rep_hero_macewindu"))
        .unwrap();
    let block = combo.state("DEFLECT").unwrap();
    assert_eq!(
        block.find(K::EnergyRestoreRate).unwrap().number(0),
        Some(-10.0)
    );
    let angle = block
        .find(K::Deflect)
        .unwrap()
        .find(K::DeflectAngle)
        .unwrap();
    assert_eq!(
        (angle.number(0), angle.number(1)),
        (Some(-110.0), Some(110.0))
    );
    let attack = combo
        .state("ATTACK1")
        .unwrap()
        .all(K::Attack)
        .next()
        .unwrap();
    assert_eq!(attack.find(K::Damage).unwrap().number(0), Some(300.0));
    let window = attack.find(K::DamageTime).unwrap();
    assert_eq!(
        (window.number(0), window.number(1), window.text(2)),
        (Some(2.0), Some(9.0), Some("Frames"))
    );
    let mut mutant = buf.clone();
    let original = mungefs::fnv1a_lowercase("DamageTime").to_le_bytes();
    let replacement = mungefs::fnv1a_lowercase("DamageTimeTypo").to_le_bytes();
    let at = mutant.windows(4).position(|s| s == original).unwrap();
    mutant[at..at + 4].copy_from_slice(&replacement);
    let error = combos_from_lvl(&mutant).unwrap_err();
    assert!(format!("{error:#}").contains("unknown combo keyword"));
}

fn vader_terminal(combo: &ComboDef) -> &manifest::combo::ComboField {
    combo
        .state("ATTACK2")
        .unwrap()
        .all(K::Transition)
        .last()
        .unwrap()
}

fn expected_vader_repair(mut combo: ComboDef) -> ComboDef {
    assert_eq!(combo.name_hash, 0xe4c0_31da);
    let attack = combo
        .fields
        .iter_mut()
        .find(|f| f.keyword == K::State && f.text(0) == Some("ATTACK2"))
        .unwrap();
    let terminal = attack
        .children
        .iter_mut()
        .rfind(|f| f.keyword == K::Transition)
        .unwrap();
    assert_eq!(terminal.text(0), Some("RECOVER2"));
    assert!(!terminal.scoped && terminal.children.is_empty());
    assert_eq!(terminal.args.len(), 1);
    // Independent expected edit: both shipped PC and Xbox Vader payloads
    // name IDLE here, with the same seventeen state names and no RECOVER2.
    terminal.args[0] = ComboValue::Text("IDLE".into());
    combo
}

#[test]
fn vader_terminal_recovery_dictionary_corrects_only_the_proven_target() {
    let Some(disc) = disc() else { return };
    let source = disc.join("SIDE/IMP.LVL");
    let raw = combos_from_lvl(&std::fs::read(&source).unwrap()).unwrap();
    let mut expected = raw.clone();
    let vader = expected
        .combos
        .iter_mut()
        .find(|c| c.name_hash == 0xe4c0_31da)
        .unwrap();
    *vader = expected_vader_repair(vader.clone());
    let out = std::env::temp_dir().join(format!(
        "openbf-vader-dictionary-{}.ron",
        std::process::id()
    ));
    let actual = bake::combo::bake_combos(source.to_str().unwrap(), out.to_str().unwrap()).unwrap();
    let written: ComboSet = ron::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    std::fs::remove_file(out).unwrap();
    assert_eq!(actual, written);
    let vader = actual
        .combos
        .iter()
        .find(|c| c.name_hash == 0xe4c0_31da)
        .unwrap();
    assert_eq!(vader_terminal(vader).text(0), Some("IDLE"));
    // Entire dictionary equality prevents copying the four PC/Xbox-only
    // MustShowOneFrame flags or changing any other PS2 field.
    assert_eq!(actual, expected);
    assert_eq!(
        vader_terminal(
            raw.combos
                .iter()
                .find(|c| c.name_hash == 0xe4c0_31da)
                .unwrap()
        )
        .text(0),
        Some("RECOVER2"),
        "the raw reader must retain the original PS2 reference"
    );
}

#[test]
fn vader_terminal_recovery_soldier_companion_uses_the_same_single_edit() {
    let Some(disc) = disc() else { return };
    let source = disc.join("SIDE/IMP.LVL");
    let raw = combos_from_lvl(&std::fs::read(&source).unwrap()).unwrap();
    let vader = raw
        .combos
        .iter()
        .find(|c| c.name_hash == 0xe4c0_31da)
        .unwrap();
    let expected = ComboSet {
        scope_provenance: true,
        combos: vec![expected_vader_repair(vader.clone())],
        ..ComboSet::default()
    };
    let out = std::env::temp_dir().join(format!("openbf-vader-companion-{}", std::process::id()));
    let ingame = disc.join("INGAME.LVL");
    bake::soldier::bake_soldier_anims(
        source.to_str().unwrap(),
        "imp_inf_darthvader",
        Some(ingame.to_str().unwrap()),
        out.to_str().unwrap(),
        None,
    )
    .unwrap();
    let path = out.join("soldiers/imp_inf_darthvader.combos.ron");
    let actual: ComboSet = ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    std::fs::remove_dir_all(out).unwrap();
    assert_eq!(vader_terminal(&actual.combos[0]).text(0), Some("IDLE"));
    assert_eq!(actual, expected);
}

#[test]
fn every_hero_combo_resolves_its_animation_references() {
    let Some(disc) = disc() else {
        return;
    };
    let ingame = std::fs::read(disc.join("INGAME.LVL")).unwrap();
    let mut bytes = vec![ingame];
    for side in ["ALL", "IMP", "REP", "CIS", "JED"] {
        bytes.push(std::fs::read(disc.join("SIDE").join(format!("{side}.LVL"))).unwrap());
    }
    let mut banks = Vec::new();
    for buf in &bytes {
        banks.extend(mungefs::anim::find_banks(mungefs::open(buf).unwrap()).unwrap());
    }
    let inv = bake::anim_sets::Inventory::new(&banks);
    let mut checked = 0;
    let mut missing = Vec::new();
    for (side, buf) in ["ALL", "IMP", "REP", "CIS", "JED"].iter().zip(&bytes[1..]) {
        let classes = bake::anim_sets::bf2::side_classes(buf).unwrap();
        let set = combos_from_lvl(buf).unwrap();
        for class in &classes {
            use bake::classes::ClassProps;
            if class.kind != "entc" || !class.name.contains("_hero_") {
                continue;
            }
            let Some(model) = class.prop("GeometryName") else {
                continue;
            };
            for binding in model_bindings(&classes, model) {
                let combo = set
                    .combos
                    .iter()
                    .find(|c| c.name_hash == binding.name_hash)
                    .unwrap();
                let (clips, absent) = resolve_animation_clips(&inv, &binding, combo);
                checked += clips.len();
                missing.extend(
                    absent
                        .into_iter()
                        .map(|name| format!("{side}:{}:{name}", binding.family)),
                );
            }
        }
    }
    println!("hero combo clips resolved: {checked}; missing: {missing:?}");
    assert!(checked > 300);
    assert!(missing.is_empty());
}

#[test]
fn bf2_emitter_lightning_is_preserved_as_a_bolt_not_a_particle() {
    let Some(disc) = disc() else { return };
    let bytes = std::fs::read(disc.join("INGAME.LVL")).unwrap();
    let root = mungefs::open(&bytes).unwrap();
    let chunks = mungefs::content_chunks(root).unwrap();
    let fx = bake::effects::Collected::from_chunks(&chunks).unwrap();
    let bolt = fx
        .lightning
        .iter()
        .find(|b| b.name == "com_sfx_lightning")
        .expect("PS2 lightning definition");
    assert_eq!(bolt.texture.as_deref(), Some("com_sfx_lightning"));
    assert_eq!(bolt.width, Some(0.5));
    assert_eq!(bolt.fade_time, Some(0.6));
    assert_eq!(bolt.color, Some([130.0, 170.0, 255.0, 250.0]));
    assert_eq!(bolt.child_color, Some([140.0, 180.0, 255.0, 100.0]));
    assert_eq!(bolt.max_branches, Some(3.0));
    assert_eq!(
        bolt.branch_length, None,
        "keep absence until the runtime's native ctor default"
    );
    assert!(fx.textures().iter().any(|t| t == "com_sfx_lightning"));
    assert!(!fx
        .effects
        .iter()
        .any(|e| e.name_hash == mungefs::fnv1a_lowercase("com_sfx_lightning")));
    println!("PS2 lightning: {bolt:?}");
}
