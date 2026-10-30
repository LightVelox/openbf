use core::collision::{CollisionWorld, HullPart, Layers, Shape};
use core::vehicle::VehicleHandle;
use game::camera_collide::{see_through_vehicles, SEE_THROUGH_OPACITY};
use glam::{Mat4, Vec3};

fn fixture() -> (CollisionWorld, VehicleHandle) {
    let h = VehicleHandle { index: 12, gen: 1 };
    let mut world = CollisionWorld::default();
    let parts = [
        HullPart::new(
            Mat4::from_translation(Vec3::new(-4.0, 0.0, 0.0)),
            Shape::Sphere { radius: 1.0 },
            Layers::ALL,
        ),
        HullPart::new(
            Mat4::from_translation(Vec3::new(4.0, 0.0, 0.0)),
            Shape::Sphere { radius: 1.0 },
            Layers::ALL,
        ),
        HullPart::new(
            Mat4::from_translation(Vec3::new(0.0, 5.0, 0.0)),
            Shape::Box {
                half: Vec3::new(4.0, 1.0, 2.0),
            },
            Layers::ALL,
        ),
    ];
    world.set_hull(h, &parts, Mat4::IDENTITY);
    (world, h)
}

#[test]
fn standing_below_the_body_between_legs_keeps_the_walker_visible() {
    let (world, h) = fixture();
    for x in [-2.0, -1.0, 0.0, 1.0, 2.0] {
        let eye = Vec3::new(x, 1.5, 0.0);
        assert!(see_through_vehicles(
            &world,
            std::iter::once(h),
            eye,
            &[eye + Vec3::Z * 3.0],
            None
        )
        .is_empty());
    }
}

#[test]
fn a_sphere_bounding_box_corner_is_air_and_a_real_obstruction_still_fades() {
    let (world, h) = fixture();
    let eye = Vec3::new(-2.8, 1.2, 0.0);
    assert!(see_through_vehicles(
        &world,
        std::iter::once(h),
        eye,
        &[eye + Vec3::Z * 3.0],
        None
    )
    .is_empty());
    assert_eq!(
        see_through_vehicles(
            &world,
            std::iter::once(h),
            Vec3::new(0.0, 5.0, -5.0),
            &[Vec3::new(0.0, 5.0, 5.0)],
            None
        ),
        vec![(h, SEE_THROUGH_OPACITY)]
    );
    assert_eq!(
        see_through_vehicles(
            &world,
            std::iter::once(h),
            Vec3::new(-4.0, 0.0, 0.0),
            &[Vec3::Z],
            None
        ),
        vec![(h, 0.0)]
    );
    assert!(see_through_vehicles(
        &world,
        std::iter::once(h),
        Vec3::new(-4.0, 0.0, 0.0),
        &[Vec3::Z],
        Some(h)
    )
    .is_empty());
}
