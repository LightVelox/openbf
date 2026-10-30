//! Independent channel-activation review: exercise the consumer, not its flags.
use super::*;

fn choke() -> (Sim, UnitHandle) {
    let (mut sim, owner) = scene(emitter());
    let power = sim.weapon_classes[0].force.as_mut().unwrap();
    power.kind = ForceKind::Choke {
        angle: 10.0 * crate::weapon::DEG,
        damage_per_second: 10.0,
        height: 4.0,
        falloff: 1.0,
        drain_rate: 15.0,
    };
    power.energy = 10.0;
    sim.weapon_classes[0].initial_salvo_delay = 0.0;
    let _ = victim(&mut sim, 3.0, 0.0);
    (sim, owner)
}

#[test]
fn a_blocked_held_choke_still_pays_when_it_can_start() {
    let (mut sim, owner) = choke();
    assert!(sim.force_input(owner.index as usize, None, true, true));
    assert!(sim.unit(owner).unwrap().weapon.force_channel.is_none());
    assert_eq!(sim.unit(owner).unwrap().soldier.energy, 100.0);
    sim.force_input(owner.index as usize, None, true, false);
    assert!(sim.unit(owner).unwrap().weapon.force_channel.is_some());
    assert_eq!(sim.unit(owner).unwrap().soldier.energy, 90.0);
}

#[test]
fn recovery_during_a_hold_cannot_create_a_free_choke_channel() {
    let (mut sim, owner) = choke();
    let u = sim.unit_mut(owner).unwrap();
    u.soldier.energy = 0.0;
    u.soldier.energy_depleted = true;
    sim.force_input(owner.index as usize, None, true, false);
    assert!(sim.unit(owner).unwrap().weapon.force_channel.is_none());
    let u = sim.unit_mut(owner).unwrap();
    u.soldier.energy = 100.0;
    u.soldier.energy_depleted = false;
    sim.force_input(owner.index as usize, None, true, false);
    assert!(sim.unit(owner).unwrap().weapon.force_channel.is_some());
    assert_eq!(sim.unit(owner).unwrap().soldier.energy, 90.0);
}

#[test]
fn a_paid_choke_costs_once_per_live_channel_and_restarting_pays_again() {
    let (mut sim, owner) = choke();
    for _ in 0..9 {
        sim.force_input(owner.index as usize, None, true, false);
    }
    assert_eq!(sim.unit(owner).unwrap().soldier.energy, 90.0);
    assert!(sim.unit(owner).unwrap().weapon.force_channel.is_some());
    sim.force_input(owner.index as usize, None, false, false);
    assert!(sim.unit(owner).unwrap().weapon.force_channel.is_none());
    sim.force_input(owner.index as usize, None, true, false);
    assert!(sim.unit(owner).unwrap().weapon.force_channel.is_some());
    assert_eq!(sim.unit(owner).unwrap().soldier.energy, 80.0);
}
