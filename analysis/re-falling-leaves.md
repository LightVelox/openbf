# Airborne leaves (LEAFF)

The airborne particles are separate from canopy sprites and from the ground
leaves stirred by gusts. This note narrows `re-leafpatch.md` §8; it does not
claim acceptance of the whole forest view.

## Source reading

[V] BF1 `FUN_001b1f00` fills a drawn patch's missing slots, using byte +0x80
of its class. Property `0xfd7fae90` caps positive counts at 50
(`FUN_001b27b8`, lines 57630–57637). Positions are independent world-axis
samples around the patch centre: x/z ±Radius, y ±Height×Scale. The pool has
100 records (`FUN_001b0900` / `FUN_001b0c30`). This is a refill as slots
become free, not a one-time emission on the first draw.

[V] `FUN_001af8d0(...,4)` chooses mode 1 with probability 0.2, otherwise
mode 0. Birth yaw and phase are uniform through 2π; scalar r is uniform
through [0,1). The initializer stores the terrain height below the birth
point. Mode 3 belongs to ground leaves and snaps y to terrain+0.1.

[D] For an airborne leaf, let B be its birth yaw, p its phase, and r its
random scalar. The update reads p before advancing it by dt modulo 2π:

- Mode 0: s=sin(2p), Q=B·Rz(r·π/4·s). If r<0.15, append
  Ry((r−1)·p·10). Velocity is lerp((0,−0.5,0), Q·X·(−1.8s), |rs|).
- Mode 1: Q=B·Rx((r+2)·p·2π). Velocity is B·Z·(0.3r+0.3)−Y·(r+1).

[V] The sine helper is the interpolated 2048-entry table
`FUN_004452c0`; the table is initialized in RAM, not present at that
address in the ELF. The generic update also supports external impulse
vectors and their decay. This implementation excludes those interactions.

[V] The three-second word at BF1 0x5174c0 is **time without a draw**:
`FUN_001b03e0` increments +0x120, but `FUN_002eda78` clears it whenever
that leaf draws. A visible leaf may live longer. Falling below cached
terrain−0.5 retires it. Its owning patch's count is decremented on removal.

[V] The draw selects `leaf` (BF1 string 0x52d5e8), grey 0.4+0.6r and full
vertex alpha.

[D] The horizontal quad's packed coordinates correspond to
±0.17 inside bounds ±0.25, with the whole texture. Its GS state is
0x100044 / TEST 0x5120d / PRIM attributes 0x50. BF2's constructor and two
motion functions are twins (`FUN_001c80e0`, `FUN_001c85c0`,
`FUN_001c8938`); its generic state initializer `FUN_003c7d50` writes the
same draw state. The port uses its existing unlit, fogged blend path.

## Checks and limits

`python3 scripts/re_check.py fn b1 55519 55673 55707 55870 56020 56234 56461 57236 57265 276931 277024`
anchors initialization, motion, retirement, spawning and draw. Also:
`word b1 0x5174bc 2 --f32`, `word b1 0x52d5e8 1 --str`,
`fn b2 73535 73682 73721 73885 422977`.

[D] Native quaternion/trig math approximates the original table. Two
original sampled frames constrain its orientation/velocity: the flutter
referee allows 1e−5 in the right vector and 4e−5 m/s in velocity; the tumble
referee allows 2e−4 in its up vector (the original table's rounding).
The rare r<0.15 flutter extension is source-derived, not separately sampled.

[ours] Cosmetic random seed is fixed. Clock discontinuities take at most
100 ms per presentation update. Invalid negative authored counts become
zero. The pool's frustum/bounds visibility is conservative; exact original
edge-cull timing is not claimed. Ground-leaf gusts, sound anchors and unit
interactions remain outside this airborne-only implementation.
