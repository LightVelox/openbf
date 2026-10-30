use super::*;

#[test]
fn native_hold_short_release_double_press_and_pruning_are_distinct() {
    let mut h = ButtonHistory::default();
    h.update(1.0, 2);
    assert_eq!(h.events[0].flags, 0);
    h.update(1.3, 2);
    assert_eq!(h.events.len(), 1, "Hold uses strict > .3");
    h.update(1.31, 2);
    assert_eq!(h.events[1].flags, 2);
    h.update(1.32, 0);
    assert_eq!(h.events[2].flags, 3);
    h.update(2.0, 2);
    h.update(2.1, 0);
    assert_eq!(h.events.last().unwrap().flags, 1);
    h.update(2.2, 2);
    assert_eq!(h.events.last().unwrap().flags, 4);
    assert_eq!(h.query(2.2, 2.2, 2.2, 1, 0), Some(2.2));
    h.prune(2.2);
    assert!(h.events.is_empty());
    assert_eq!(h.held, 2);
    h.update(2.21, 2);
    assert_eq!(
        h.events[0].flags, 2,
        "held after pruning is never a fresh Press"
    );
    h.update(2.22, 0);
    assert_eq!(h.events[1].flags, 3);
    assert_eq!(h.query(2.22, 2.2, 2.3, 1, 0), None);
}

#[test]
fn native_ring_cap_and_continuous_down_up_windows() {
    let mut h = ButtonHistory::default();
    for i in 0..12 {
        h.update(i as f32, if i % 2 == 0 { 2 } else { 0 });
    }
    assert_eq!(h.events.len(), 8);
    assert_eq!(h.events[0].time, 4.0);
    let h = ButtonHistory {
        held: 2,
        events: vec![ButtonEvent {
            time: 1.0,
            button: 1,
            flags: 0,
        }],
    };
    assert_eq!(h.query(2.0, 1.5, 2.5, 1, 10), Some(1.5));
    assert_eq!(h.query(2.0, 1.5, 2.5, 1, 8), None);
    assert_eq!(
        h.query(2.0, 2.1, 2.5, 1, 10),
        Some(2.1),
        "earlier down event spans a future start"
    );
    assert_eq!(h.query(2.0, 0.0, 0.5, 1, 10), None);
}
