use super::*;
use crate::fx::Library;
use core::hero_weapon::Burn;
use core::unit::UnitSpec;
use core::weapon::{BurnClass, EmitterOrdnance, WeaponClass};

fn library() -> Option<Library> {
    let root = crate::assets::baked_root();
    let common = root.join("common_bf2");
    if !common.join("effects.ron").exists() {
        eprintln!("SKIP: {} is not baked", common.display());
        return None;
    }
    Some(Library::load(&root, &common, "bf2", &[]))
}

fn flame(effect: Option<&str>, burn: BurnClass) -> WeaponClass {
    WeaponClass {
        emitter: Some(EmitterOrdnance {
            cone_length: 10.0,
            cone_angle: 45.0 * core::weapon::DEG,
            max_targets: 10,
            jump_deduction: 0.0,
            damage_threshold: 0.0,
            max_jump_distance: 0.0,
            first_radius: 2.0,
            push: 0.0,
            lightning: None,
            bolts: 1,
            affect_friends: false,
            affect_enemies: true,
            hit_all: true,
            radius: 0.0,
            effect: effect.map(str::to_owned),
            burn,
            bf2: true,
        }),
        ..Default::default()
    }
}

fn hero_burn() -> BurnClass {
    let fire = "com_sfx_inf_damage_fire".to_owned();
    BurnClass {
        time: 5.0,
        rate: 50.0,
        effects: vec![
            ("bone_ribcage".into(), fire.clone()),
            ("bone_head".into(), fire.clone()),
            ("bone_pelvis".into(), fire),
        ],
    }
}

#[test]
fn a_flame_shot_starts_its_effect_at_the_muzzle_along_the_shot() {
    let Some(lib) = library() else { return };
    let mut fx = Fx::with_library(lib);
    let mut sim = Sim::empty(1);
    let lit = sim.add_weapon_class(flame(Some("com_sfx_weap_flamethrower_ord"), hero_burn()));
    let bare = sim.add_weapon_class(flame(None, BurnClass::default()));
    let unit = sim.spawn(&UnitSpec::trooper(0, Vec3::ZERO)).unwrap();
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let shot = |weapon_class: usize| Event::Shot {
        unit,
        weapon_class: weapon_class as u32,
        origin: Vec3::new(0.0, 1.5, 0.0),
        dir,
    };
    sim.events = vec![shot(bare)];
    fx.ingest_sim(&sim);
    assert!(fx.live.is_empty(), "no effect named");
    sim.events = vec![shot(lit), shot(lit)];
    fx.ingest_sim(&sim);
    assert_eq!(fx.live.len(), 2, "one per shot");
    let e = &fx.live[0];
    assert_eq!(e.name(), Some("com_sfx_weap_flamethrower_ord"));
    assert_eq!(e.frame.pos, Vec3::new(0.0, 1.5, 0.0));
    assert!((e.frame.rot.z_axis - dir).length() < 1e-6);
    assert!((e.frame.rot.y_axis - Vec3::Y).length() < 1e-6);
}

#[test]
fn a_burning_soldier_wears_one_effect_per_slot_until_the_burn_ends() {
    let Some(lib) = library() else { return };
    let mut fx = Fx::with_library(lib);
    let mut sim = Sim::empty(1);
    let class = sim.add_weapon_class(flame(None, hero_burn()));
    let shooter = sim.spawn(&UnitSpec::trooper(0, Vec3::ZERO)).unwrap();
    let h = sim
        .spawn(&UnitSpec::trooper(1, Vec3::new(3.0, 0.0, 0.0)))
        .unwrap();
    fx.ingest_sim(&sim);
    assert!(fx.burns().is_empty());
    sim.units[h.index as usize].burn = Some(Burn {
        left: 5.0,
        damage_per_second: 50.0,
        owner: shooter,
        team: 0,
        weapon_class: class,
    });
    fx.ingest_sim(&sim);
    assert_eq!(fx.burns(), vec![(h, 0), (h, 1), (h, 2)]);
    let at = sim.units[h.index as usize].soldier.pos + Vec3::Y;
    assert!(fx
        .live
        .iter()
        .all(|e| e.name() == Some("com_sfx_inf_damage_fire") && e.frame.pos == at));
    fx.ingest_sim(&sim);
    assert_eq!(fx.burns().len(), 3, "a running burn spawns no more");
    sim.units[h.index as usize].soldier.pos = Vec3::new(5.0, 0.0, 0.0);
    fx.ingest_sim(&sim);
    assert!(fx
        .live
        .iter()
        .all(|e| e.frame.pos == Vec3::new(5.0, 1.0, 0.0)));
    sim.units[h.index as usize].alive = false;
    fx.ingest_sim(&sim);
    assert!(fx.burns().is_empty(), "a dead soldier stops burning");
    sim.units[h.index as usize].alive = true;
    fx.ingest_sim(&sim);
    assert_eq!(fx.burns().len(), 3);
    sim.units[h.index as usize].burn = None;
    fx.ingest_sim(&sim);
    assert!(fx.burns().is_empty());
    assert!(fx.live.iter().all(|e| e.is_stopped()));
}
