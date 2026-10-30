//! Optional original-content referee. The private fixture is not distributed.

#[test]
fn original_bf2_throw_replays_every_recorded_outgoing_boundary_and_return_point() {
    use crate::hero_weapon::{SaberFlight, SaberThrow};
    use glam::{Mat4, Vec3};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Reference {
        dt: f32,
        velocity: f32,
        outbound_seconds: f32,
        return_seconds: f32,
        rotation_rate: f32,
        rows: Vec<Row>,
    }
    #[derive(Deserialize)]
    struct Row {
        clock: f32,
        previous: [f32; 3],
        position: [f32; 3],
        velocity: [f32; 3],
        owner: [f32; 3],
        remaining: f32,
        outgoing: f32,
    }
    let Some(dir) = std::env::var_os("OPENBF_THROW_REFERENCE_DIR") else {
        eprintln!("SKIP: original BF2 throw reference directory not supplied");
        return;
    };
    let path = std::path::PathBuf::from(dir).join("native_throw_reference.ron");
    let reference: Reference = ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let class = SaberThrow {
        energy: 60.0,
        velocity: reference.velocity,
        outbound_seconds: reference.outbound_seconds,
        return_seconds: reference.return_seconds,
        rotation_rate: reference.rotation_rate,
        damage: 0.0,
        width: 0.0,
        length: 0.0,
        width_from_edge: false,
        length_from_edge: false,
    };
    assert_eq!(reference.rows.len(), 60);
    let mut counts = [0; 3];
    let mut distinguishes_signed_boundary = false;
    for row in reference.rows {
        let before = row.remaining + reference.dt;
        let mut flight = SaberFlight {
            pos: Vec3::from_array(row.previous),
            velocity: Vec3::from_array(row.velocity),
            basis: Mat4::IDENTITY,
            spin_axis: Vec3::Y,
            angle: 0.0,
            blade_class: 0,
            remaining: before,
            outbound_seconds: row.outgoing,
            return_age: (-before).max(0.0),
            returning: before <= 0.0,
            hit: vec![],
        };
        assert!(flight.advance(&class, Vec3::from_array(row.owner), reference.dt));
        let group = if before <= 0.0 {
            2
        } else if row.remaining <= 0.0 {
            let truncated = Vec3::from_array(row.previous);
            distinguishes_signed_boundary |=
                truncated.distance(Vec3::from_array(row.position)) > 0.005;
            1
        } else {
            0
        };
        counts[group] += 1;
        for (axis, (actual, expected)) in flight
            .pos
            .to_array()
            .into_iter()
            .zip(row.position)
            .enumerate()
        {
            // At most four component ULPs cover the native EE truncation
            // versus host rounding across the multiply/add/divide sequence.
            let magnitude = expected.abs();
            let ulp = f32::from_bits(magnitude.to_bits() + 1) - magnitude;
            assert!(
                (actual - expected).abs() <= ulp * 4.0,
                "clock {} group {group} axis {axis}: {actual} != {expected} (ULP {ulp})",
                row.clock
            );
        }
    }
    assert_eq!(counts, [35, 2, 23]);
    assert!(distinguishes_signed_boundary);
}
