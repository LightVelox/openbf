---
id: SPL16
title: Split the game crate's over-cap files: roster.rs 1924, fx.rs 1857, event_feed.rs 1849, animator.rs 1590, bolt_vis.rs 1512
milestone: M0
value: 2
phase: cleanup
status: todo
deps:
files: engine/crates/game/src/roster.rs, engine/crates/game/src/fx.rs, engine/crates/game/src/event_feed.rs, engine/crates/game/src/animator.rs, engine/crates/game/src/bolt_vis.rs
declares:
needs:
effort:
branch:
resume:
created: 2026-10-29
---
Found by the 2026-10-29 cleanse (Pass 2): these files are over the 1,500-line cap with no split card or waiver. One file, one session, a pure move (engine/artifacts/briefs/SPLIT_RECIPE.md); CI and the sweep byte-identical after each.
