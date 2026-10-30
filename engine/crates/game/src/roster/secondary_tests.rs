use crate::classdb::ClassDb;

fn bf2_db() -> Option<ClassDb> {
    let root = crate::assets::baked_root();
    let mut db = ClassDb::default();
    for side in ["ingame", "imp", "tur"] {
        let path = root.join(format!("classes/bf2_{side}.ron"));
        if !path.is_file() {
            eprintln!("SKIP: {} is not baked", path.display());
            return None;
        }
        db.load(&path).unwrap();
    }
    Some(db)
}

#[test]
fn the_sonic_blaster_rides_a_fat_ray_with_its_blast_effect() {
    let Some(db) = bf2_db() else { return };
    let w = super::weapon_from_db(&db, "imp_weap_inf_sonic_blaster").unwrap();
    let sec = w.secondary.as_ref().expect("the sonic blast");
    assert_eq!(sec.name, "imp_weap_inf_sonic_blaster_ord");
    assert_eq!(sec.period, 1);
    let fat = sec.ordnance.fat_ray.expect("a fatray class");
    assert_eq!(fat.max_damage, 100.0);
    assert_eq!(fat.radius, 1.0);
    assert_eq!(fat.damage_deduction, 5.0);
    assert_eq!(sec.ordnance.velocity, 100.0);
    assert_eq!(sec.ordnance.life_span, 0.25);
    assert_eq!(
        sec.ordnance
            .impact
            .as_ref()
            .and_then(|l| l.trail.as_deref()),
        Some("imp_sfx_weap_sonicblast_ord")
    );
    assert!(w.ordnance.fat_ray.is_none());
}

#[test]
fn the_geonosian_turret_fires_a_fat_ray() {
    let Some(db) = bf2_db() else { return };
    let w = super::weapon_from_db(&db, "tur_weap_geoturret").unwrap();
    let fat = w.ordnance.fat_ray.expect("a fatray class");
    assert_eq!(fat.max_damage, 400.0);
    assert_eq!(fat.radius, 5.0);
}

#[test]
fn the_award_sniper_rifle_fires_a_beam() {
    let Some(db) = bf2_db() else { return };
    let w = super::weapon_from_db(&db, "imp_weap_award_sniper_rifle").unwrap();
    let beam = w.ordnance.beam.as_ref().expect("a beam class");
    assert_eq!(beam.range, 500.0);
    assert_eq!(beam.life, 0.15);
    assert!(beam.pierce);
    assert_eq!(beam.impact, None);
    assert_eq!(w.ordnance.max_damage, 600.0);
    let look = w.ordnance.laser.as_ref().expect("the beam's look");
    assert_eq!(look.texture.as_deref(), Some("com_sfx_laser_purple"));
    assert_eq!(look.width, 0.1);
    let impact = w.ordnance.impact.as_ref().unwrap();
    assert_eq!(
        (impact.soft.as_deref(), impact.terrain.as_deref()),
        (None, None)
    );
}

#[test]
fn the_beam_turret_strikes_with_its_impact_effect() {
    let Some(db) = bf2_db() else { return };
    let w = super::weapon_from_db(&db, "tur_weap_beam").unwrap();
    let beam = w.ordnance.beam.as_ref().expect("a beam class");
    assert_eq!(beam.life, 0.75);
    assert_eq!(beam.impact.as_deref(), Some("com_sfx_weap_beam_exp"));
    let impact = w.ordnance.impact.as_ref().unwrap();
    assert_eq!(impact.soft.as_deref(), Some("com_sfx_weap_beam_exp"));
    assert_eq!(impact.water, None);
}
