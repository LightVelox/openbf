use super::*;

#[derive(Clone)]
struct Props(Vec<(String, String)>);
impl PropSource for Props {
    fn prop(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    fn prop_hash(&self, hash: u32) -> Option<&str> {
        self.prop(&format!("{hash:08x}"))
    }
}
impl Props {
    fn new(values: &[(&str, &str)]) -> Self {
        Self(
            values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }
    fn set(&mut self, key: &str, value: &str) {
        self.0.iter_mut().find(|(k, _)| k == key).unwrap().1 = value.into();
    }
}
fn weapon() -> Props {
    Props::new(&[("EnergyDrain", "60")])
}
fn throw() -> Props {
    Props::new(&[
        ("Velocity", "30"),
        ("LifeSpan", "1.333"),
        ("0dda9b5b", "0.8"),
        ("Damage", "400"),
        ("DamageWidth", "0.75 FromEdge"),
        ("DamageLength", "1.2 FromEdge"),
        ("RotationRate", "4"),
    ])
}

#[test]
fn hostile_throw_numbers_are_refused_before_flight_division_or_substeps() {
    assert!(SaberThrow::from_props(&weapon(), &throw()).is_some());
    let mut accepted = Vec::new();
    for (is_weapon, key) in [
        (true, "EnergyDrain"),
        (false, "Velocity"),
        (false, "LifeSpan"),
        (false, "0dda9b5b"),
        (false, "Damage"),
        (false, "DamageWidth"),
        (false, "DamageLength"),
        (false, "RotationRate"),
    ] {
        for value in ["NaN", "inf", "-inf"] {
            let (mut w, mut o) = (weapon(), throw());
            if is_weapon {
                w.set(key, value);
            } else {
                o.set(key, value);
            }
            if SaberThrow::from_props(&w, &o).is_some() {
                accepted.push((key, value));
            }
        }
    }
    for key in ["Velocity", "LifeSpan"] {
        for value in ["0", "-1"] {
            let mut ord = throw();
            ord.set(key, value);
            if SaberThrow::from_props(&weapon(), &ord).is_some() {
                accepted.push((key, value));
            }
        }
    }
    assert!(
        accepted.is_empty(),
        "accepted invalid throw fields: {accepted:?}"
    );
}

fn emitter() -> Props {
    Props::new(&[
        ("ConeLength", "20"),
        ("ConeAngle", "120"),
        ("MaxTargets", "5"),
        ("Push", "-15"),
        ("FirstRadius", "1.5"),
        ("JumpDeduction", "0"),
        ("DamageThreshold", "0"),
        ("MaxJumpDistance", "10"),
        ("6824f471", "0"),
        ("1076dfd6", "0"),
    ])
}
fn choke() -> Props {
    Props::new(&[
        ("5c1fc8bc", "Choke"),
        ("2efc37e9", "20"),
        ("3891df5a", "10"),
        ("MaxTargets", "1"),
        ("EnergyDrain", "40"),
        ("d2ea8a45", "150"),
        ("894a0e77", "4"),
        ("3b5eb7fa", "1"),
        ("8909415b", "15"),
    ])
}

#[test]
fn hostile_force_numbers_are_refused_but_signed_pull_is_kept() {
    let power = ForcePower::emitter(&weapon(), &emitter()).unwrap();
    assert_eq!(power.push, -15.0);
    assert!(ForcePower::choke(&choke()).is_some());
    let mut accepted = Vec::new();
    for key in emitter().0.iter().map(|(k, _)| k.as_str()) {
        for value in ["NaN", "inf", "-inf"] {
            let mut ord = emitter();
            ord.set(key, value);
            if ForcePower::emitter(&weapon(), &ord).is_some() {
                accepted.push(format!("emitter {key}={value}"));
            }
        }
    }
    for key in choke()
        .0
        .iter()
        .map(|(k, _)| k.as_str())
        .filter(|k| *k != "5c1fc8bc")
    {
        for value in ["NaN", "inf", "-inf"] {
            let mut w = choke();
            w.set(key, value);
            if ForcePower::choke(&w).is_some() {
                accepted.push(format!("choke {key}={value}"));
            }
        }
    }
    for key in ["2efc37e9", "894a0e77"] {
        let mut w = choke();
        w.set(key, "0");
        if ForcePower::choke(&w).is_some() {
            accepted.push(format!("choke {key}=0"));
        }
    }
    assert!(
        accepted.is_empty(),
        "accepted invalid Force fields: {accepted:?}"
    );
}

#[test]
fn finite_throw_flags_and_zero_return_time_remain_valid() {
    let mut ord = throw();
    ord.set("0dda9b5b", "0");
    let parsed = SaberThrow::from_props(&weapon(), &ord).unwrap();
    assert!(parsed.width_from_edge && parsed.length_from_edge);
    assert_eq!(parsed.return_seconds, 0.0);
}

#[test]
fn finite_throw_work_is_bounded_by_the_combined_motion() {
    for (velocity, rotation, accepted) in [
        ("30", "4", true),
        ("945", "0", true),
        ("960", "0", false),
        ("500", "70", true),
        ("600", "70", false),
        ("1000000000", "0", false),
        ("30", "1000000000", false),
        ("30", "-4", true),
    ] {
        let mut ord = throw();
        ord.set("Velocity", velocity);
        ord.set("RotationRate", rotation);
        let class = SaberThrow::from_props(&weapon(), &ord);
        assert_eq!(class.is_some(), accepted, "{velocity}, {rotation}");
        if let Some(class) = class {
            assert!(class.contact_substeps(crate::TICK_DT).unwrap() <= MAX_THROW_SUBSTEPS);
        }
    }
    let mut ord = throw();
    ord.set("Velocity", "945");
    ord.set("RotationRate", "0");
    let boundary = SaberThrow::from_props(&weapon(), &ord).unwrap();
    assert_eq!(boundary.contact_substeps(crate::TICK_DT), Some(64));
}

#[test]
fn bounded_throw_contacts_keep_native_spacing_and_signed_last_update() {
    let class = SaberThrow::from_props(&weapon(), &throw()).unwrap();
    assert_eq!(class.contact_substeps(crate::TICK_DT), Some(4));
    for seconds in [-crate::TICK_DT, -0.02, -0.0001, 0.0, 0.0001, crate::TICK_DT] {
        let native_step = (0.5 / (class.velocity + class.rotation_rate)).min(0.0333);
        let native = ((seconds / native_step) as i32 + 1).max(0) as usize;
        assert_eq!(class.contact_substeps(seconds), Some(native), "{seconds}");
    }
    for seconds in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e30] {
        assert!(class.contact_substeps(seconds).is_none());
    }
    let mut malformed = class;
    malformed.velocity = 1e9;
    assert!(malformed.contact_substeps(crate::TICK_DT).is_none());
}

#[test]
fn blade_visibility_uses_the_same_eight_edge_domain_everywhere() {
    assert_eq!(MAX_BLADE_EDGES, 8);
    for edge in 0..MAX_BLADE_EDGES {
        assert!(blade_visible(0, edge));
        assert!(!blade_visible(1 << edge, edge));
        assert!(!blade_visible(u8::MAX, edge));
    }
    for edge in [MAX_BLADE_EDGES, 31, 32, usize::MAX] {
        assert!(!blade_visible(0, edge));
    }
}
