//! Static station bodies and dynamic vehicle hulls share the flight query.

use super::*;
use crate::collision::{Body, HullPart, Layers, Shape};
use crate::sim::stations::{Station, StationClass};
use crate::vehicle::{SeatKind, VehicleKind, VehicleSpec};
use crate::weapon::{Health, HealthType};

#[test]
fn thrown_saber_hits_map_and_vehicle_pools_once_and_turns_back_with_authored_scales() {
    for (layers, target_team, factor) in [
        (Layers::ORDNANCE, 1, 1.0),
        (Layers::SOLDIER, 1, 0.0),
        (Layers::ORDNANCE, 0, 0.25),
    ] {
        let (mut sim, owner, _) = scene();
        sim.friendly_fire_scale = 0.25;
        sim.weapon_classes[1].ordnance.scales[HealthType::Vehicle as usize] = 0.2;
        sim.weapon_classes[1].ordnance.scales[HealthType::Building as usize] = 0.5;
        let vehicle = sim.spawn_vehicle(&VehicleSpec {
            kind: VehicleKind::Hover,
            class: 0,
            home_team: Some(target_team),
            neutral_when_empty: false,
            spawn_pad: None,
            pos: Vec3::new(0.25, 1.15, -1.0),
            yaw: 0.0,
            max_health: 500.0,
            health_type: HealthType::Vehicle,
            seats: vec![SeatKind::Pilot],
        });
        sim.collision.set_hull(
            vehicle,
            &[HullPart::new(
                Mat4::IDENTITY,
                Shape::Sphere { radius: 0.1 },
                layers,
            )],
            Mat4::from_translation(sim.vehicle(vehicle).unwrap().pos),
        );
        let class = sim.add_station_class(StationClass::soldier("fixture", 1.0, 0.0, 0));
        let mut station = Station::new(class, Vec3::new(-0.25, 1.15, -1.0), 0.0);
        station.instance = Some(17);
        station.team = Some(target_team);
        station.health = Health::new(500.0, HealthType::Building);
        let mut body = Body::with_layers(
            Mat4::from_translation(station.pos),
            Shape::Sphere { radius: 0.1 },
            layers,
        );
        body.instance = Some(17);
        sim.stations.push(station);
        sim.collision.push_body(body);
        sim.collision.bind_station(17, 0);
        sim.launch_saber_throw(owner.index as usize, Some(0), 1);
        for _ in 0..10 {
            sim.tick_saber_throws();
        }
        assert_eq!(
            sim.vehicle(vehicle).unwrap().health.cur,
            500.0 - 40.0 * 0.2 * factor
        );
        assert_eq!(sim.stations[0].health.cur, 500.0 - 40.0 * 0.5 * factor);
    }
}
