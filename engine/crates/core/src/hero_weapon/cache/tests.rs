fn cache_attack(record: u64, id: u32, end: f32) -> crate::combo::AttackWindow {
    crate::combo::AttackWindow {
        index: 0,
        id,
        record,
        cache_end: end,
        edge: 0,
        damage: Some(100.0),
        length: Some(1.2),
        length_from_edge: false,
        width: Some(0.1),
        width_from_edge: false,
        push: 0.0,
        play_explosion: false,
    }
}

#[test]
fn melee_cache_uses_record_or_packed_id_without_extending_the_first_end() {
    use crate::projectile::Target;
    let mut state = MeleeState::default();
    let first = cache_attack(11, 0, 1.0);
    assert!(state.record_hit(&first, Target::Station(1)));
    let repeat = cache_attack(11, 0, 100.0);
    assert!(
        !state.record_hit(&repeat, Target::Station(1)),
        "reentering same record retains its cache"
    );
    assert_eq!(
        state.hit[0].end, 1.0,
        "reuse does not extend native end+.3 lifetime"
    );
    let other = cache_attack(12, 0, 2.0);
    assert!(
        state.record_hit(&other, Target::Station(1)),
        "id0 never aliases distinct records"
    );
    let shared = cache_attack(21, 77, 3.0);
    assert!(state.record_hit(&shared, Target::Station(1)));
    let next_state = cache_attack(22, 77, 9.0);
    state.runtime.state = 100;
    state.runtime.state_serial += 1;
    state.runtime.chain_serial += 1;
    assert!(
        !state.record_hit(&next_state, Target::Station(1)),
        "nonzero id crosses state and idle serials"
    );
    assert_eq!(state.hit.len(), 3);
    assert_eq!(state.hit[2].end, 3.0);
    for victim in 2..=8 {
        assert!(state.record_hit(&next_state, Target::Station(victim)));
    }
    assert!(
        !state.record_hit(&next_state, Target::Station(9)),
        "eight cached victims is the actual damage cap"
    );
}

#[test]
fn melee_cache_expires_at_window_end_plus_point_three_even_in_idle() {
    use crate::projectile::Target;
    let mut state = MeleeState::default();
    state.runtime.state = 0;
    let keep = cache_attack(1, 0, -0.299);
    let expire = cache_attack(2, 0, -0.3);
    assert!(state.record_hit(&keep, Target::Station(1)));
    assert!(state.record_hit(&expire, Target::Station(2)));
    state.expire_hits();
    assert!(state.already_hit(&keep, Target::Station(1)));
    assert!(!state.already_hit(&expire, Target::Station(2)));
}

#[test]
fn every_melee_cache_field_and_gravity_latch_changes_future_state_hash() {
    let mut state = MeleeState::default();
    let attack = cache_attack(1, 3, 2.0);
    assert!(state.record_hit(&attack, crate::projectile::Target::Station(1)));
    let bytes = |s: &MeleeState| {
        let mut b = Vec::new();
        s.hash_into(&mut |v| b.extend_from_slice(v));
        b
    };
    let original = bytes(&state);
    for field in 0..5 {
        let mut changed = state.clone();
        match field {
            0 => changed.hit[0].record += 1,
            1 => changed.hit[0].id += 1,
            2 => changed.hit[0].end += 0.1,
            3 => changed.hit[0].victims[0] = crate::projectile::Target::Station(2),
            _ => changed.gravity_done = true,
        }
        assert_ne!(bytes(&changed), original, "future field{field}");
    }
}
