use super::*;

fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = tag.to_vec();
    out.extend((payload.len() as u32).to_le_bytes());
    out.extend(payload);
    while !out.len().is_multiple_of(4) {
        out.push(0)
    }
    out
}
fn record(key: &str, slots: &[u32], blob: &[u8]) -> Vec<u8> {
    let mut raw = mungefs::fnv1a_lowercase(key).to_le_bytes().to_vec();
    raw.push(slots.len() as u8);
    for slot in slots {
        raw.extend(slot.to_le_bytes())
    }
    raw.extend((blob.len() as u32).to_le_bytes());
    raw.extend(blob);
    chunk(b"DATA", &raw)
}
fn named(key: &str, value: &str) -> Vec<u8> {
    let mut blob = value.as_bytes().to_vec();
    blob.push(0);
    record(key, &[4], &blob)
}
fn program(field: &[u8]) -> Vec<u8> {
    let body = [
        chunk(b"NAME", &7u32.to_le_bytes()),
        named("State", "IDLE"),
        chunk(b"SCOP", field),
    ]
    .concat();
    chunk(b"comb", &body)
}
fn accepted(bytes: &[u8]) -> bool {
    let chunk = mungefs::chunks(bytes).next().unwrap().unwrap();
    parse_combo(&chunk).is_ok()
}
#[test]
fn non_finite_numeric_combo_arguments_are_rejected() {
    assert!(accepted(&program(&record(
        "Duration",
        &[1.0f32.to_bits()],
        &[]
    ))));
    let mut wrong = Vec::new();
    for key in ["Duration", "Damage", "Gravity", "VelocityZ"] {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            if accepted(&program(&record(key, &[value.to_bits()], &[]))) {
                wrong.push((key, value.to_bits()));
            }
        }
    }
    assert!(wrong.is_empty(), "accepted nonfinite values: {wrong:?}");
}
#[test]
fn mandatory_names_and_compiled_string_boundaries_are_checked() {
    assert!(accepted(&program(&named("Animation", "stand_idle"))));
    let mut wrong = Vec::new();
    for key in [
        "Animation",
        "Sound",
        "Transition",
        "AttackId",
        "DeflectAnimation",
        "ThrowAnimation",
    ] {
        if accepted(&program(&record(key, &[1.0f32.to_bits()], &[]))) {
            wrong.push(key.to_owned());
        }
    }
    for (label, slots, blob) in [
        ("unterminated", vec![4], b"stand_idle".to_vec()),
        ("invalid_utf8", vec![4], vec![0xff, 0]),
        ("middle_of_string", vec![5], b"stand_idle\0".to_vec()),
    ] {
        if accepted(&program(&record("Animation", &slots, &blob))) {
            wrong.push(label.to_owned());
        }
    }
    assert!(wrong.is_empty(), "accepted malformed names: {wrong:?}");
}
#[test]
fn authored_numeric_looking_time_string_is_preserved_as_text() {
    let mut blob = b"-12\0Frames\0".to_vec();
    let bytes = program(&record("TimeStart", &[8, 12], &blob));
    let chunk = mungefs::chunks(&bytes).next().unwrap().unwrap();
    let combo = parse_combo(&chunk).unwrap();
    let field = combo.state("IDLE").unwrap().find(K::TimeStart).unwrap();
    assert_eq!(field.text(0), Some("-12"));
    assert_eq!(field.number(0), None);
    blob.clear();
}
