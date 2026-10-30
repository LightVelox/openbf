//! Physical contacts distinguish exposed riders from enclosed crews.

use super::*;

#[test]
fn a_hand_swing_can_damage_only_the_exposed_seated_body() {
    for exposed in [false, true] {
        let (mut sim, owner, target) = scene();
        sim.weapon_classes[0] = saber(program(vec![
            field(K::Animation, vec![text("swing")], vec![]),
            field(
                K::Attack,
                vec![],
                vec![
                    field(K::DamageTime, vec![V::Number(0.0), V::Number(1.0)], vec![]),
                    field(K::Damage, vec![V::Number(100.0)], vec![]),
                ],
            ),
        ]));
        sim.unit_mut(owner).unwrap().weapon.melee = None;
        sim.unit_mut(target).unwrap().soldier.pos = Vec3::new(0.0, 0.0, -1.0);
        let vehicle = crate::sim::hero_bodies::seat_fixture(&mut sim, target, exposed);
        crate::sim::hero_bodies::pose_fixture(&mut sim);
        sim.unit_mut(target).unwrap().soldier.pos = Vec3::splat(100.0);
        let before = sim.unit(target).unwrap().health.cur;
        let mut pads = vec![PadState::default(); sim.units.len()];
        pads[owner.index as usize].buttons = buttons::FIRE;
        sim.saber_inputs(&mut pads);
        sim.saber_hits();
        let target = sim.unit(target).unwrap();
        assert_eq!(
            target.health.cur,
            before - if exposed { 100.0 } else { 0.0 }
        );
        assert_eq!(target.mount.unwrap().vehicle, vehicle);
    }
}
