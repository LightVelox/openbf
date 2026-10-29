---
id: SPL17
title: Split the core crate's over-cap files: soldier.rs 1744, ai/route.rs 1618, sim/combat.rs 1588, sim/ordnance.rs 1570, ai/tactic.rs 1521
milestone: M0
value: 2
phase: cleanup
status: todo
deps:
files: engine/crates/core/src/soldier.rs, engine/crates/core/src/ai/route.rs, engine/crates/core/src/sim/combat.rs, engine/crates/core/src/sim/ordnance.rs, engine/crates/core/src/ai/tactic.rs
declares:
needs:
effort:
branch:
resume:
created: 2026-10-29
---
Found by the 2026-10-29 cleanse (Pass 2): these files are over the 1,500-line cap with no split card or waiver. One file, one session, a pure move (engine/artifacts/briefs/SPLIT_RECIPE.md); CI and the sweep byte-identical after each.
