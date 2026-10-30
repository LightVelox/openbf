//! What a hand blade's contact reports (B2 FUN_00340c60, 336287-336347): the
//! `HitEffect` at each struck victim's contact, one `HitSound` an update; and
//! `DeflectEffect` at a block (FUN_0033e8b8, 334743-334751).

use super::*;
use crate::combo::AttackWindow;
use crate::sim::Event;

fn attack() -> AttackWindow {
    AttackWindow {
        index: 0,
        id: 3,
        record: 12,
        cache_end: 1.0,
        edge: 0,
        damage: Some(10.0),
        length: Some(1.2),
        length_from_edge: false,
        width: Some(0.1),
        width_from_edge: false,
        push: 0.0,
        play_explosion: false,
    }
}

/// The blade runs from the body at chest height forward; a soldier stands on it.
fn swinging() -> (Sim, UnitHandle, UnitHandle) {
    let (mut sim, defender, attacker) = scene();
    let class = sim.weapon_classes[0].saber.as_mut().unwrap();
    class.hit_sound = Some("hit".into());
    class.deflect_sound = Some("deflect".into());
    let a = sim.unit_mut(attacker).unwrap();
    a.weapon_class = 0;
    a.soldier.pos = Vec3::NEG_Z;
    a.soldier.yaw = std::f32::consts::PI;
    a.weapon.melee = Some(MeleeState {
        animation: Some("swing".into()),
        attacks: vec![attack()],
        previous_position: a.soldier.pos,
        previous_yaw: a.soldier.yaw,
        ..Default::default()
    });
    (sim, defender, attacker)
}

fn contacts(sim: &Sim) -> Vec<(Vec3, bool)> {
    sim.events
        .iter()
        .filter_map(|e| match *e {
            Event::SaberContact { pos, deflect, .. } => Some((pos, deflect)),
            _ => None,
        })
        .collect()
}

fn combo_sounds(sim: &Sim) -> usize {
    sim.events
        .iter()
        .filter(|e| matches!(e, Event::ComboSound { .. }))
        .count()
}

#[test]
fn a_cut_reports_its_contact_on_the_blade_at_the_victim() {
    let (mut sim, defender, _) = swinging();
    sim.unit_mut(defender).unwrap().weapon.melee = None;
    sim.saber_hits();
    let found = contacts(&sim);
    assert_eq!(found.len(), 1, "{found:?}");
    let (pos, deflect) = found[0];
    assert!(!deflect);
    assert!(pos.z.abs() < 0.8 && (pos.y - 0.9).abs() < 0.05, "{pos}");
    assert_eq!(combo_sounds(&sim), 1);
}

#[test]
fn two_victims_in_one_update_get_two_contacts_and_one_sound() {
    let (mut sim, defender, _) = swinging();
    sim.unit_mut(defender).unwrap().weapon.melee = None;
    let second = sim
        .spawn(&UnitSpec::trooper(1, Vec3::new(0.3, 0.0, 0.0)))
        .unwrap();
    let before = sim.unit(second).unwrap().health.cur;
    sim.saber_hits();
    assert!(sim.unit(second).unwrap().health.cur < before);
    assert_eq!(contacts(&sim).len(), 2);
    assert_eq!(combo_sounds(&sim), 1, "one HitSound an update");
}

#[test]
fn a_block_reports_a_deflect_contact_and_no_hit() {
    let (mut sim, _, _) = swinging();
    sim.saber_hits();
    let found = contacts(&sim);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1);
}

#[test]
fn a_deflected_bolt_reports_a_deflect_contact() {
    let (mut sim, defender, shooter) = scene();
    let bolt = impact(&sim, defender, shooter, Vec3::Z);
    assert!(sim.deflect_bolt(defender, &bolt));
    let found = contacts(&sim);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1);
}
