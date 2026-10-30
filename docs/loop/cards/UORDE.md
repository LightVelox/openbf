---
id: UORDE
title: Emitter weapons (BF1 clone pilot's arc caster; BF2 dark trooper / Jawa arc casters, the officer's incinerator, the hero flamethrowers) fire no bolt: each shot is one instant cone query at the muzzle that chains damage from target to target (MaxTargets, JumpDeduction, DamageThreshold, MaxJumpDistance, LOS per hop); BF2 adds hit-all, team / alive filters, class scales, a blast per hit, pull and burn damage over time
milestone: M3
value: 3
phase: build
status: blocked
deps:
files: engine/crates/core/src/weapon.rs, engine/crates/core/src/weapon/emitter.rs, engine/crates/core/src/emitter.rs, engine/crates/core/src/sim/emitter.rs, engine/crates/core/src/sim/ordnance.rs, engine/crates/core/src/sim/state.rs, engine/crates/core/src/sim/guns.rs, engine/crates/game/src/roster.rs, engine/crates/game/src/fx.rs, engine/crates/game/src/fx/arc.rs
declares: sim; sweep; arena
needs: the H2 PS2 checks (lead_tmp/uord/helpers/H2_emitter.md) and an 8 x 32 sweep 256 / 256; the burn / deflection / buff keys are uncracked
effort:
branch: uord-missiles
resume: 2026-10-30: implementation LANDED on branch uord-missiles (70da755 + uncommitted tree) - the emitterordnance class, the BF1 / BF2 query law, the soldier_shots dispatch, one Event::Emitter per link and the lightning-arc renderer, with 22 core tests and the game arc test. Status stays blocked: burn / deflection / per-hit buffs / pull have no RE model, the flamethrowers' flame and the burn bone effects are not drawn, and the mutation check and the 8 x 32 sweep are not run. Restore the H2 checks first; the PS2-side files are absent (Test-Path false).
created: 2026-10-22
---
## Why
analysis/re-ordnance-kinds.md §8 (card UORD; full reading lead_tmp/uord/helpers/H2_emitter.md): the emitterordnance class's create slot (B1 FUN_00278208, B2 FUN_00315f78) runs one query per weapon shot and returns 0 - no projectile. The port built these as straight bolts before this branch (see What landed).

## Evidence
- BF1 (B1:198922-199272): box gather at muzzle + 0.9 L x aim, mask 0x812 (no vehicles), cone tan < sin(ConeAngle/2) at the body origin with no front test, nearest along the aim first; D = lerp(Damage, MaxDamage, charge); hop range ConeLength then MaxJumpDistance (axial); LOS mask 0x4c from the chain origin to body + 1 m; FirstRadius on the first; all type / shield scales forced to 1.0; D -= JumpDeduction; stop at MaxTargets or D < DamageThreshold. Verifier batch 4: 8 / 8 SUPPORTED.
- BF2 (B2:305952-307007): mask 0x102a (vehicles), 3-D cone (cos 0.1 within 2.5 m), alive bit, AffectFriends / AffectEnemies, the class's scales, hit-all mode (D = max(Damage, D/N)), Explosion blast per hit, pull (Push < 0), burn timer = max(timer, T), rate x dt per update while alive (FUN_00236f50).
- Fielded: BF1 rep_weap_inf_arccaster (1 / 2 / 3 targets by charge: 100 -> 250, JD 75, DT 75); BF2 imp / tat arc casters, all_weap_inf_incinerator (30 per shot, 15-shot burst), cis / imp_weap_hero_flamethrower (5 per shot + burn 50/s for 5 s).

## Acceptance
- Unit tests on the read rules, mutation-checked: the BF1 arc caster's target count by charge (1 below 1/3, 2 below 5/6, 3 above), a skipped (LOS-blocked) candidate ends nothing, no vehicle in BF1's gather; BF2 hit-all split, the burn's damage over 5 s, team filter.
- A scripted scene: a clone pilot's full charge into three soldiers in a line deals 250 / 175 / 100.
- 8 x 32 sweep 256 / 256 (bots hold these weapons: the clone pilot, the dark trooper, the Rebel officer).

## Notes
- Visual only (no sim): the lightning segments, BF2's extra arcs, the flame effect, the burn bone effects.
- Named-deviation candidates (H2 §6): BF1's missing front test, its feet-point cone test, its dead scale keys, corpses as targets - measure on the PS2 before keeping or dropping any.

## What landed 2026-10-30 (branch `uord-missiles`, 70da755 + uncommitted tree)
An `emitterordnance` class is now one instant query per shot, not a bolt. Line numbers are the working tree's.

- **The class** (`engine/crates/core/src/weapon/emitter.rs:27-95`): `EmitterOrdnance` reads `ConeLength`,
  `ConeAngle` (degrees -> radians), `MaxTargets`, `JumpDeduction`, `DamageThreshold`, `MaxJumpDistance`,
  `FirstRadius`, `Radius`, and the hashed `Push`, `LightningEffect`, `AffectFriends`,
  `AffectEnemies` and `0x4c387ca1` (hit-all). `bf2` selects the rule set; `is_cone()` is `ConeLength > 0`.
  Loaded on a `WeaponClass` when the ordnance's engine base is `emitterordnance`
  (`engine/crates/game/src/roster.rs:897-902`), and its `Damage` becomes the hit-all floor
  (`roster.rs:760-766`).
- **The law** (`engine/crates/core/src/emitter.rs:70-239`): BF1's gather (soldiers and riders only, no vehicles,
  32 cap), the cone at the body point, nearest along the aim first, `FirstRadius` on the first link, LOS per
  hop, `D -= JumpDeduction` with stops at `MaxTargets` and `DamageThreshold`; BF2 adds vehicles, the team filter
  (`AffectFriends` / `AffectEnemies`), the class's damage scales, and hit-all (`max(Damage, D/N)`, nearest
  first). A skipped (LOS-blocked) candidate does not end the chain.
- **The sim** (`engine/crates/core/src/sim/emitter.rs:24-152`): one `Event::Shot` (the fire sound, muzzle flash
  and trigger animation read it), the gather of alive soldiers and hulls at body + 1 m, the query, BF1's forced
  1.0 scales, per-hit damage with the friendly-fire scale, BF2's `Explosion` per body hit, `Push` through
  `kick_and_push`, and one `Event::Emitter` link per hit (`sim/state.rs:188-199`; headless skips it,
  `main/headless.rs:277`).
- **The draw** (`engine/crates/game/src/fx.rs:1244-1268`): each `Event::Emitter` link whose class names a
  `LightningEffect` becomes an `fx::arc::Arc` (`engine/crates/game/src/fx/arc.rs:117-176`), seeded from the sim
  tick and the link's index, alive for the record's 18 ticks and drawn as the PS2 ribbon (`arc.rs:1-28`, `186`);
  a class that names none (the flamethrowers) starts no arc.
- **The dispatch** (`engine/crates/core/src/sim/ordnance.rs:192-203`): a cone emitter with `max_damage > 0`
  spends the shot in `fire_emitter` and spawns no bolt; a zero-damage cone (Force push / pull) stays on the old
  bolt-and-blast path.
- **Tests, run 2026-10-30** from `engine/`: `cargo test -p core --lib emitter` -> 22 passed, 0 failed (10 law
  tests in `emitter.rs`, 9 scene tests in `sim/emitter.rs`, 3 loader tests in `weapon/emitter.rs`; 1,384
  filtered). The card's own scene is asserted twice, law and sim: a clone pilot's full charge deals 250 / 175 /
  100 down a line (`emitter.rs:355`, `sim/emitter.rs:292`); 1 / 2 / 3 targets by charge
  (`emitter.rs:329`, `sim/emitter.rs:276`); a wall skips one link (`sim/emitter.rs:312`); BF1 zaps an ally
  (`sim/emitter.rs:336`); BF2 hit-all splits 15 into 7.5 / 7.5 and never goes under `Damage` when split
  (`emitter.rs:392`, `:401`); the team filter both ways (`emitter.rs:409`, `:419`; `sim/emitter.rs:371`); a
  vehicle is a candidate on BF2 only (`emitter.rs:428`, `sim/emitter.rs:395`); the zero-damage class stays a
  bolt (`sim/emitter.rs:431`).
- **The arc draw test, run 2026-10-30** from `engine/`: `cargo test -p game --lib
  emitter_links_draw_their_lightning` -> 1 passed, 0 failed (576 filtered): two links become two arcs, a class
  with no `LightningEffect` starts none, and the arc is gone at tick 23 after its 18 (`fx.rs:2042-2079`).

## What is deferred (not built, with the reason)
- **Burn over time** (BF2's soldier timer = `max(timer, T)`, then `rate x dt` per update while alive, doc §8):
  the ODF burn keys are uncracked in the RE text. §8 states the effect in one line, with no key hashes, no
  units and no class table, so there is no model to implement against. No soldier burn field exists: `grep -i
  burn` in `engine/crates/core/src` finds the sticky's fuse (`sticky.rs:253`, ticked at `sim/ordnance.rs:423`)
  and no soldier or emitter burn.
- **Deflection, per-hit buffs, Force push / pull**: §8 names each in a clause - "deflection by the target's
  weapon", "pull (Push < 0)", "buffs" - with no key, unit or class table, and the impact section's deflector
  vcall (+0xd0 / +0xdc, §3.2) is the impact handler's, not the emitter's. No code models them: `kick_and_push`
  returns on `push <= 0.0` (`sim/blast.rs:311`), so a negative `Push` (a pull) does nothing; the zero-damage
  (Force push / pull) emitter is kept on the bolt path (`sim/ordnance.rs:192-199`).
- **The flame and the burn bone effects**: not started. The arc renderer covers a class that names a
  `LightningEffect`; an emitter that names none (the incinerator, the hero flamethrowers) draws no effect, and
  no code draws the burn bone effects (the only `burn` in `engine/crates/core/src` is the sticky's fuse,
  `sticky.rs:253`).

## Evidence (2026-10-30)
- Source: the files and lines above; the test runs quoted above (`22 passed, 0 failed` in core; `1 passed` in
  game for the arc draw).
- The PS2 side the card cites is **ABSENT**: `Test-Path` false for
  `lead_tmp/uord/helpers/H2_emitter.md`, and there is no `lead_tmp` at the repo root
  (`engine/artifacts/lead_tmp/` holds ten other directories and no `uord`). The read rules are therefore
  checked against the doc's text (`analysis/re-ordnance-kinds.md` §8) only.
- Mutation check (the card's "mutation-checked" clause): NOT RUN. `lead_tmp/audit-uord/mut/M1` is false.
- 8 x 32 sweep 256 / 256: NOT RUN.
- Status stays `blocked`: the sim build is in, but burn / deflection / buffs / pull, the flame and the two
  acceptance runs (mutation, sweep) are not.
