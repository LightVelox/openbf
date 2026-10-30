# BF2 hero combo programs

PS2 BF2 has authored `comb` resources for saber actions. A weapon's
`ComboAnimationBank` selects the program; the soldier's animation bank supplies
its clips. Data names and values below are observations of the PS2 disc. Function
addresses refer to `SWBFRNT2.ELF`; line numbers refer to its current decompilation.
[V] denotes a primary-data or instruction check, [D] a decompiler reading, and
PROVISIONAL an implementation behavior still awaiting an original-game referee.

## Resource format and census

[V] A `comb` holds a NAME hash followed by ordered DATA records and SCOP
children. DATA uses the common config argument slots: string slots are offsets
into the trailing blob, numeric slots are raw f32 words. A SCOP belongs to the
preceding DATA. The bake retains order, repeated fields, every argument and
nested If/Or group. Unknown keyword hashes, malformed records, duplicate states,
missing IDLE, orphan SCOPs and excessive nesting are errors.

All SIDE resources contain 27 raw combo payloads and 18 definitions after
side-local exact duplicate folding: ALL1, CIS3, GAM1, GEO1, IMP2, JED4, REP5,
SNW1. The census observes60 keyword hashes and11,862 records. Repeated copies
within a side are identical. The CIS and JED copies of Maul's same-name program
have20 and19 states respectively, so global hash-only folding would lose data.

[V/D] The PS2 IMP program 0xe4c031da is Darth Vader's. ATTACK2 plays
`stand_attack1b`, offers the conditional ATTACK3 chain, then names missing
RECOVER2 as its terminal destination. The raw readers retain that authored
reference. FUN00309d10,298549–298585, searches the state table and uses index0
when unresolved; MIPS 0030a3e0 stores zero explicitly. An original loaded Vader
class has all 17 expected states, state 0 = IDLE, and ATTACK2 targets [10, 0], namely
ATTACK3 and IDLE. This is compiled-class evidence, not a new active-attack trace.

[V] Shipped PC and Xbox IMP.LVL Vader payloads are byte-identical to one another
(SHA256 43b44a257445046c9fa95481eae262394020ee7c21a2ce113d713e7ed104ae5c).
Both have those same 17 state names and explicitly target IDLE at that exit;
neither supplies an extra RECOVER2 animation state. Their four additional
MustShowOneFrame flags on deflect posture adapters are separate platform data.

[ours] Emitted gameplay data corrects only Vader ATTACK2's bare terminal label
RECOVER2→IDLE, through `baked_combos_from_lvl` and `combo/compatibility.rs`.
Both the side dictionary and soldier companion use this path; lossless raw
parsing stays separate. No PC/Xbox retail data flags, animation, timing, damage or other PS2
field are copied. This makes the already-effective idle exit explicit and does
not add a recovery animation or change play: the third attack still chains on
the authored input window, otherwise the second attack ends in idle.

[V] Game hashes fold each byte with OR0x20 before FNV multiplication. This is
not equivalent to ASCII lowercase for underscores, punctuation or digits.

[V] Mace's program hash is0x84d86099. ATTACK1 authors Damage300,
DamageTime2..9 Frames, DamageLength1.5, DamageWidth.75 and Push3. DEFLECT authors
EnergyRestoreRate−10 and DeflectAngle[−110,+110]. Tests read the actual side
resource and mutate a real DamageTime keyword to prove unknown data fails.

No AI-specific keyword occurs among the60 observed hashes. Hero-authored
Button modes are Press and Down. GAM has a Hold record; GAM is not fielded by
the hero roster. The data census does not establish how an AI generates input.

## Schema and animation resolution

`manifest::combo` is a lossless typed-key tree. It intentionally supplies no
runtime defaults. `bake::combo` reads side definitions and follows soldier /
weapon class inheritance to bind a model to the appropriate program.

BF2 animation-set resolution includes saber locomotion families. Soldier output
loads its own hero-specific bank and every clip referenced by that soldier's
combo, in addition to ordinary locomotion. Scoped animation metadata can refer
to an already-declared full clip. An actual-content check resolves334 references
for the saber heroes with zero missing. The noncombo `anim_sets::build` API stays
available for the older slot referee.

Raw combo side files and per-model program files are separate from resolved
clip metadata. Per-state duration comes from the actual soldier rig. Root-motion
and weapon-attachment frames preserve their authored animation samples.

## Native parser roles and packed values

[D] Conditions: FUN00304048; attack: FUN003058c0; deflect:
FUN00306c48; state: FUN00307a80. Callers connect conditions from003054a8 and
00309178, attacks/deflect from00307a80, and states from00309d10.

[V] DamageDistance and DamageLength are native aliases. In FUN003058c0,
295956–296006, hashes0xf081f01b and0xc3b311dc enter the same LAB00305dfc.
The last authored alias determines the stored length and FromEdge flag. Keeping
an independent unused "distance" value would misread Emperor's air attack.

[V] AnimatedMove X/Z velocities are signed12-bit values after multiplication
by50, saturated to[−2048,+2047]. FUN00309178,297830–297967, and fresh
003093e0–0030944c instructions establish that authored VelocityZ150 loads40.94m/s.
The bake retains150; only the runtime decoding applies the native packing.

[V] Empty AnimatedMove scopes also carry both velocity defaults: parser307a80,
297525–297539, stores integer50 and FromAnim bits22/23 for X and Z. Loader30a480
resolves each animation record's signed15 centimetres/second with EE multiply
by.01, then computes `(packedFactor * .02)`, animation speed times that factor,
and finally `*50` before integer conversion and signed12 saturation. The
instruction order is30a704–30a730 for Z and30a7ac–30a7dc for X. The runtime
getter33c4d0 multiplies the resolved integer by.02. Replacing those operations
with division by50 or bypassing the final pack for an absent field changes the
result. The evaluator keeps this EE arithmetic local to combo numeric decoding.

A fixed source-derived40cm/s control decodes to float bits3ecccccc, multiplies
by default-factor bits3f7fffff, packs19 and returns3ec28f5b (about.38m/s).
An exact2m/s control packs99, distinguishing native factor multiplication from
a division-produced unity factor. These are instruction-chain controls; they
do not claim a native Maul RAM movement observation.

[D] FUN00535e78,665582–665610, copies raw config argument words.
FUN00304048 reads TimeStart's first word as float and packs hundredths. The
five literal-string TimeStart("-12") records therefore do not mean−12frames:
the tiny positive offset bits quantize to0. The evaluator does not parse a
numeric-looking string. A live RAM packed-field check remains open.

## Posture masks constrain an action

[V] Native parser hashes establish Roll=0x20 (index5) and Jet=0x40
(index6), not the reverse:295430/297683 and295468/297721. Other retained bits
are stand1, crouch2, prone4, sprint8, jump0x10. Mask0 is unrestricted.

[D] Consumer FUN0033cda8,333418–333465, reads current posture from owner
flags at original bits35..40. If an action's nonzero mask excludes the current
posture, it does not force the body to another stance. Disallowed stand through
sprint mark runtime flags. Roll and postures index≥7 interrupt toIDLE. Jump or
Jet interrupt only if the action admits neither Jump nor Jet (mask0x50).
A focused test distinguishes an allowed jump/jet pair, severe interruptions,
and a disallowed crouch which must not force standing.

## Button events and transition ordering

[D] FUN0033f780,334858–334885, maps combo controls1..6 to controller
words+38,+3c,+44,+4c,+48,+40. FUN0033f828 copies each low held bit into the
combo mask, with its remap/lock flags applied. Physical button mapping is a
separate controller concern; it cannot be inferred from the combo's logical name.

[V] FUN00304048 maps Press(hash01fcf9b4) to mode0, Hold to2, Down to10,
Up to8 and Release to9. FUN0033fb80,335071–335228, mode0 matches event flags
whose bit1 is0; this includes initial press0 and short release1. Down/Up also
consult held state and prior events across the requested time window.

[D] FUN0033f908,334945–335068, maintains an eight-entry ring. A new held
edge records flags0; continuing held can record2 after.3s (or immediately if prior transition pruning left no event for that button). A release can be
short tap1 or longer release3, and a second press can record4 within.5s.
It does not maintain the port's provisional per-button "consumed" bits.

[D] Native transition consumer FUN0033cda8 invokes FUN0033fe38 at333625
with cutoff=(selectedRelativeTime+weaponClock)−oldStateElapsed. That helper,
335229–335275, removes ring entries at or before the cutoff. FUN00340148,
335383–335446, converts between absolute event times and state-relative windows.
The valid five-control trace below checks these equations against the current
evaluator; it supersedes the earlier provisional latching/consumption prototype.

The evaluator now follows that ring and cutoff model. It retains eight logical
entries of `(absolute time, button, flags)` plus the held mask; it has no
consumed-event bits or early-input latch. Matching groups compete by selected
event time, with authored priority for ties. A nonbreaking group is examined at
the state's duration. A previous state's positive FromEnd window remains
available during recovery, using its original duration as the offset.

[V] State field+0x1c is the resolved animation index: the Animation hash
0xe145ee5d branch at297287–297347 resolves the animation record and stores its
index byte. FUN0033cda8 records events at the **old** weapon clock (333290),
advances elapsed before evaluating transitions (333306), then advances the
weapon clock at the end (333643). Entering a different animation resets elapsed
and adds this frame's dt; the same animation preserves elapsed (333625–333631).
Thus Yoda's ATTACK3→RECOVER3 continues stand_attack1c's clock. Clip durations
resolve to packed hundredths in FUN0030a480,298652–298681; using an unquantized
clip fraction changes the window. Contact overlap is `previous <= end` and
`start < current`, not an inclusive start (335912–335914).

[V] Timing operations use EE round-toward-zero arithmetic. This matters even
for pruning the first press: ordinary float rounding can leave the cutoff one
ULP below the event. The original tick is separately established from saved RAM:
DAT00661378 and the scheduler argument DAT0066af14 both contain0x3d088888;
DAT00661318 is the scheduler's base quantum0x38e90453. FUN00175d88 passes the
argument to00425840, which stores it at482446. The replay supplies that tick;
it does not change the port's general fixed-tick constant. Across retained rows,
EE addition matches364/364 weapon-clock increments and345/345 stable same-state
elapsed increments exactly; ordinary rounding matches0/364 clock increments.
The scoped runtime helpers reproduce timing additions/subtractions and packed
window multiplication without widening the ordinary sim's float policy.

Clean retained PS2 trials have97 advancing-clock rows each over3.199951s,
all starting from the same verified quickslot with X11 repeat disabled. Native
state hashes resolve to IDLE→ATTACK1→RECOVER1→IDLE for held Fire, but
IDLE→ATTACK1→ATTACK2→ATTACK3→RECOVER3 for repeated presses. Held block
stays DEFLECT through the long release; two short presses enter then leave it.
FIFO data shows the initial event pruned at entry and held event2 on the next
update. Thus the current hero program is a press toggle for block despite the
tutorial's physical hold instruction. The original failed-load/auto-repeat
recording is invalid for that claim. The native owner controller word can also
be rewritten later in the frame; FIFO/held-mask observations are stronger than
one post-update word sample. A last-clean-per-clock rerecord retains all485
observations across the five controls. Its instrument requires the native dt and
exact EE-rounded clock/FIFO grids on every row; no mixed-row exclusion is used.
The evaluator matches state, previous state, elapsed bits, weapon-clock bits,
held mask and every logical FIFO entry's button/flags/time bits for485/485 rows.
The input is the observed logical held stream; five entry-pruned presses are
located from their first entered-dt row. This does not claim physical
keyboard/controller latency parity. The older first-clean recording and its
477-row replay remain an earlier instrument check; its eight preclassified
mixed-clock rows are superseded by the complete last-clean rerecord.
Other programs, button modes and compound transition cases still rely on source
and discriminating tests rather than an original-game replay.

## Shared custom-animation metadata

[V/D] Animation references and Animation scopes are different loader operations.
FUN00307a80,297286–297376, resolves a name through0027aed8/0027af48 and records
the animation index in state+1c. It parses following SCOP metadata only while
descriptor bit21 is clear. FUN00279780 sets that lock at entry, including an
empty scope. A forward reference without SCOP does not lock the descriptor.
The bake therefore retains `ComboField.scoped` separately from `children` and
marks checked sets with `scope_provenance`. Older RON can load using defaults;
it does not establish fresh metadata provenance and must be rebaked for that use.

The side bake also preserves each native hashed `lvl_` resource request's
ordered COMB names, including resources with no combo. The mission's SIDE file
and request order selects these lists directly; a requested alias is not assumed
to name a gameplay class. Equal combo hashes within one side must have identical
definitions before dictionary lookup is allowed. `load_provenance` distinguishes
this checked resource index from older combo dictionaries or model-only subsets.

[D] The descriptor array DAT005fdcf8 is global, with index
`2*customAnimationIndex+layer`; the name lookup has no class-name component
(0027aed8,199736–199755;0027af48,199756–199799). The first scoped definition
therefore wins across references and classes in a mission's actual load order.
00279498 copies to the other layer while clearing lower-layer aim bits.
0027f458,202437–202456, reads AimType from the current upper descriptor:
None0, Head1, Torso2, FullBody3. FullBody becomes flag4 and is excluded from
00280158's head/torso mask0x33. An absent state child does not reset that shared
descriptor, and FullBody must not be interpreted as Torso.

[D] Within the first scope,279780 applies recognized AimType and BlendInTime
properties in authored order, so the last recognized write wins. Unknown
AimType strings leave the previous value. BlendInTime optionally converts Frames
using literal0x3d088889, multiplies100, clamps0..255, converts and stores byte+3
(MIPS279b2c–279bf0);27f458 decodes it with0x3c23d70a. These multiplications use
local EE rounding. BlendTimeIn is a distinct unrecognized hash in279780 and must
not be treated as an alias. The consumed scalar values in the current side
corpus are unchanged by the ordered/unit correction; repeated properties and
Frames have separate discriminating controls.

[V] Original loaded Aayla data demonstrates the mission scope. In assault,
ATTACK3, ATTACK3_LAND and RECOVER3 all reference animation149, whose locked
upper descriptor05200095 has AimTypeNone. A fresh Felucia CW load gives those
same states animation148 and locked upper05220094, AimTypeTorso. Its ATTACK1
and DASH controls are None and FullBody respectively. RECOVER3 has no authored
AimType child but inherits the shared descriptor. These observations establish
loaded metadata, not an active-pose or complete animation-fidelity comparison.

## Contact-cache identity and lifetime

[V/D] FUN00340c60,335915–335937, finds a cache by the authored attack-record
pointer or equal nonzero packed AttackId. FUN003058c0 packs FNV into the upper20
bits of the second attack word, so only the low20 hash bits survive; zero means
record identity alone. It creates an empty cache when the window activates,
not only after damage. At most eight entity pointers are recorded, before the
deflect call (336282–336304); a blocked contact consumes its victim entry.
The candidate gather also requests at most eight with mask0x3a (336191), which
includes soldiers, vehicles, stations and exposed riders. The port's stable
candidate enumeration remains a separate ordering limitation.

[D] Ordinary exit FUN00340738 subtracts old elapsed from every cache end
(335624–335631), and FUN00340838 removes entries after contact processing when
end+.3<=current elapsed (335709–335713). Ordinary IDLE entry does not clear this
list. Same-animation continuation therefore requires a deadline rebase as well
as elapsed continuation; treating each state serial as a fresh hit would permit
repeated damage. The port stores deadlines on its weapon-clock axis and rebases
existing deadlines on state exits; severe posture interruption clears them.
Tests distinguish same-record reentry, distinct records sharing a nonzero ID,
id0 separation, the eight-victim cap, expiry, cross-state continuation and
blocked-contact consumption.

[D] Hand melee constructs all eight health-type scales as1 (336205–336211),
then dispatches contact/block/damage without another LOS or perception gate.
Throw damage has a different descriptor path:0031e4c8's vtable slot+0x30 points
to00310178, which copies the ordnance class descriptor and optional charge
interpolation (302145–302201). Authored throw scales must therefore remain.

## Runtime outputs and verification boundary

The evaluator returns selected state/animation/time, control and posture masks,
explicit transition energy cost, optional authored restoration rate, movement,
attack windows with record/id/edge/damage/FromEdge values, native cache-deadline
rebasing, deflect arcs/cost and sounds. Animated movement exposes both its target
and whether its blend is active; an Until stops the blend without discarding
the target during fade-out.
It does not assume that default energy recovery already occurred. The sim owns
normal recovery and applies an authored override separately.

`step_with_animation` resolves the movement source after transitions. It normally
uses the destination state; while the native getter retains the previous
AnimatedMove during fade-out, that previous state supplies its own FromAnim clip
speed and alignment flag. The game supplies each clip's constant resolved ANM
root speed, rather than instantaneous visual root-key velocity. The older `step`
wrapper accepts a constant velocity for fixtures.

Meaningful checks include real-disc unknown-key mutation, all side definitions,
all hero clip references, exact Mace values, out-of-window presses, transition
cost, root motion on the transition tick, packed dash saturation, raw string
arguments, DamageDistance alias order, posture interruption and field-by-field
future-state hashing. Actual15-class simulation checks prove the loaded programs
can reach animated damage windows and authored deflect arcs; they do not prove
complete original-game combat or presentation fidelity. The five Yoda controls establish only their named state/time/FIFO scope. Other
original-game input programs, contact ordering and final gameplay/render review
remain required before claiming full hero combat parity.
