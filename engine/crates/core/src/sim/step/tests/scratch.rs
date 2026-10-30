use super::*;

#[test]
fn retained_ai_scratch_matches_fresh_allocations_and_keeps_no_pending_writes() {
    let make = || {
        let mut s = Sim::empty(73);
        s.heightfield = Some(flat_ground());
        for team in 0..2 {
            for i in 0..8 {
                s.spawn(&ai(
                    team,
                    Vec3::new(i as f32 * 2.0, 0.0, team as f32 * 10.0),
                    0.0,
                ))
                .unwrap();
            }
        }
        s
    };
    let mut retained = make();
    let mut fresh = make();
    let mut storage = None;
    for tick in 0..120 {
        // The original lifetime: no buffer survives a tick in the control.
        fresh.ai_updates = Vec::new();
        retained.step(&[]);
        fresh.step(&[]);
        assert_eq!(retained.hash(), fresh.hash(), "tick {tick}");
        assert_eq!(
            format!("{:?}", retained.events),
            format!("{:?}", fresh.events)
        );
        assert!(retained.ai_updates.is_empty());
        if let Some(ptr) = storage {
            assert_eq!(retained.ai_updates.as_ptr(), ptr);
        }
        storage = Some(retained.ai_updates.as_ptr());
        assert!(retained.ai_updates.capacity() >= 16);
        // Capacity and allocation identity never enter a replay hash.
        let hash = retained.hash();
        let cache = std::mem::take(&mut retained.ai_updates);
        assert_eq!(retained.hash(), hash);
        retained.ai_updates = cache;
    }
}
