//! Physical hero queries include the native exposed-rider collision type.

use super::Sim;

/// Immutable class data supplies the same seat frame as the figure. The
/// evaluator may read only vehicle state and baked data, never a clock/RNG.
pub trait HeroRiderFrames: Send + Sync {
    fn frame(&self, vehicle: &crate::vehicle::Vehicle, seat: u8) -> Option<glam::Mat4>;
}

impl Sim {
    pub(super) fn hero_body_prims(&self, i: usize) -> Option<crate::unit::BoltPrims> {
        if !self.hero_body_active(i) {
            return None;
        }
        let unit = &self.units[i];
        let mut prims = unit.bolt_prims();
        if let Some(mount) = unit.mount {
            let vehicle = self.vehicle(mount.vehicle)?;
            if vehicle.kind != crate::vehicle::VehicleKind::Turret {
                if let Some(frames) = &self.hero_rider_frames {
                    let frame = frames
                        .frame(vehicle, mount.seat)
                        .filter(|m| m.is_finite())?;
                    prims.pos = frame.w_axis.truncate();
                    let forward = frame.z_axis.truncate();
                    prims.yaw = crate::fmath::atan2(-forward.x, -forward.z);
                }
            }
        }
        Some(prims)
    }

    pub(super) fn hero_body_active(&self, i: usize) -> bool {
        let Some(unit) = self.units.get(i).filter(|u| u.alive) else {
            return false;
        };
        let Some(mount) = unit.mount else {
            return true;
        };
        let Some(vehicle) = self.vehicle(mount.vehicle).filter(|v| v.alive) else {
            return false;
        };
        vehicle
            .seats
            .get(usize::from(mount.seat))
            .is_some_and(|seat| seat.occupant == Some(self.handle(i)))
            && self
                .seat_setup
                .get(vehicle.class as usize)
                .and_then(Option::as_ref)
                .and_then(|setup| setup.body_active.get(usize::from(mount.seat)))
                .copied()
                .unwrap_or(false)
    }
}

#[cfg(test)]
pub(super) fn seat_fixture(
    sim: &mut Sim,
    unit: crate::unit::UnitHandle,
    exposed: bool,
) -> crate::vehicle::VehicleHandle {
    use crate::vehicle::{SeatKind, SeatSetup, VehicleKind, VehicleSpec};
    let vehicle = sim.spawn_vehicle(&VehicleSpec {
        kind: VehicleKind::Hover,
        class: 0,
        home_team: Some(sim.unit(unit).unwrap().team),
        neutral_when_empty: false,
        spawn_pad: None,
        pos: sim.unit(unit).unwrap().soldier.pos,
        yaw: 0.0,
        max_health: 500.0,
        health_type: crate::weapon::HealthType::Vehicle,
        seats: vec![SeatKind::Pilot],
    });
    sim.seat_setup = vec![Some(SeatSetup {
        radius: 1.0,
        boardable: vec![true],
        body_active: vec![exposed],
        exit_drop: 15.0,
        ai_size: 1,
        guns: vec![],
    })];
    assert!(sim.seat_unit(unit, vehicle, 0));
    vehicle
}

#[cfg(test)]
pub(super) fn pose_fixture(sim: &mut Sim) {
    struct Frames;
    impl HeroRiderFrames for Frames {
        fn frame(&self, vehicle: &crate::vehicle::Vehicle, _: u8) -> Option<glam::Mat4> {
            Some(glam::Mat4::from_rotation_translation(
                vehicle.rot,
                vehicle.pos,
            ))
        }
    }
    sim.hero_rider_frames = Some(std::sync::Arc::new(Frames));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounted_hero_targets_require_a_live_matching_exposed_seat() {
        let mut sim = Sim::empty(19);
        let unit = sim
            .spawn(&crate::unit::UnitSpec::trooper(1, glam::Vec3::ZERO))
            .unwrap();
        let i = unit.index as usize;
        assert!(sim.hero_body_active(i));
        let vehicle = seat_fixture(&mut sim, unit, false);
        assert!(!sim.hero_body_active(i));
        sim.seat_setup[0].as_mut().unwrap().body_active[0] = true;
        assert!(sim.hero_body_active(i));
        sim.vehicle_mut(vehicle).unwrap().seats[0].occupant = None;
        assert!(!sim.hero_body_active(i));
        sim.vehicle_mut(vehicle).unwrap().seats[0].occupant = Some(unit);
        sim.vehicle_mut(vehicle).unwrap().alive = false;
        assert!(!sim.hero_body_active(i));
        sim.unit_mut(unit).unwrap().mount = None;
        assert!(sim.hero_body_active(i));
        sim.unit_mut(unit).unwrap().alive = false;
        assert!(!sim.hero_body_active(i));
    }

    #[test]
    fn hero_queries_use_the_seat_frame_without_moving_the_ordinary_body() {
        let mut sim = Sim::empty(20);
        sim.heightfield = Some(crate::sim::tests::flat_ground());
        let unit = sim
            .spawn(&crate::unit::UnitSpec::trooper(1, glam::Vec3::ZERO))
            .unwrap();
        let vehicle = seat_fixture(&mut sim, unit, true);
        pose_fixture(&mut sim);
        sim.vehicle_mut(vehicle).unwrap().pos = glam::Vec3::new(3.0, 5.0, 7.0);
        let prims = sim.hero_body_prims(unit.index as usize).unwrap();
        assert_eq!(prims.pos, glam::Vec3::new(3.0, 5.0, 7.0));
        assert_eq!(sim.unit(unit).unwrap().bolt_prims().pos, glam::Vec3::ZERO);
        let before = sim.hash();
        sim.vehicle_mut(vehicle).unwrap().pos.x += 1.0;
        assert_ne!(sim.hash(), before);
        assert_eq!(sim.hero_body_prims(unit.index as usize).unwrap().pos.x, 4.0);
    }

    #[test]
    fn a_hero_kill_uses_the_visible_seat_position_for_rider_death_bookkeeping() {
        let mut sim = Sim::empty(21);
        sim.heightfield = Some(crate::sim::tests::flat_ground());
        let owner = sim
            .spawn(&crate::unit::UnitSpec::player(0, glam::Vec3::ZERO))
            .unwrap();
        let unit = sim
            .spawn(&crate::unit::UnitSpec::trooper(1, glam::Vec3::splat(100.0)))
            .unwrap();
        let vehicle = seat_fixture(&mut sim, unit, true);
        pose_fixture(&mut sim);
        let position = glam::Vec3::new(2.0, 3.0, -4.0);
        sim.vehicle_mut(vehicle).unwrap().pos = position;
        let mut damage = sim.weapon_classes[0].ordnance.damage_desc(0.0, false);
        damage.damage = 400.0;
        sim.hero_damage(owner.index as usize, unit, damage, glam::Vec3::NEG_Z);
        assert!(
            sim.unit(unit).is_none(),
            "dead handles cannot target a live unit"
        );
        let unit = &sim.units[unit.index as usize];
        assert!(!unit.alive);
        assert!(unit.mount.is_none());
        assert_eq!(unit.soldier.pos, position);
        assert!(sim.vehicle(vehicle).unwrap().seats[0].occupant.is_none());
    }
}
