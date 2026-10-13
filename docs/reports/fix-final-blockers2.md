# Report: fix-final-blockers2

**Branch** `s/fix-final-blockers2` (from origin/master 71d1496). Task: the 0700004a lift-landing
enable (first priority) and the rest of the 07000016 walk. Status: **4a deeply root-caused — the
enable mechanism is now fully mapped and the remaining question is precisely posed for a live
check; a candidate fix was implemented, verified end-to-end, then REVERTED on counter-evidence.
07000016 was not started.** Everything below is committed and pushed.

## 1. 0700004a, the lift landing — what was established

**The mechanism, fully mapped [M]:**
- `Script_EventHandler` 0x1d94c0 case 6 (125-134): the enable matrix `SCRIPTINFO+0x40` is written
  ONCE at EVENT time — `Mat_Copy(iVar5+0x90 → SI+0x40)`, where `iVar5` = the previous stream
  record's object (gated on the record's kind ∈ {1,2}); if there is no prev entity the fallback
  source is `glb_players+0x90` — the player's own live matrix at event time.
- `Script_Free` 0x1da880 (46-49): `Player_Enable(glb_players, SI+0x40, +0xab6)` — **the game does
  NOT re-read the entity at free time**. The previous session's second hypothesis is dead; the
  snapshot's SOURCE is the whole story.
- `Player_Enable` 0x195498: places Bond at the matrix translation (obj+0x30/+0x40), zeroes his
  velocities, and — when mode ≠ 0 (here mode 1, the event-6 param) — runs a
  `Collide_SphereIntersect` resolve with `Player_CollWire`/`CollCreepWall` follow-ups.

**The snapshot source, measured in the port (probes `walkreal_probe_fb6/fb7/fb8`, committed):**
- Cut-scene #289 (0x060006cf, SP placement #289, base (0,−53.53,−3.38)) actually runs
  t1611..t2186 (clock 576 — the previous session's "ends ~t2163" was off). Event 6 fires at tick
  575 (frame 2187) on stream 26, params [1]; the previous stream (25) holds an invisible marker
  entity (hash 0, obj_type 0x19, no parent, single key at t575: pos (1.80, −54.128, −3.453)).
- The port snapshots that marker's matrix → the enable. With the port's base composition the
  snapshot is (1.80, −107.66, −6.83) (the parked symptom); with the keys raw it is (1.80, −54.13,
  −3.45) — Bond's freeze pose.

**The candidate fix (implemented, verified, reverted):** the case-1 decompile puts
`Concat(SCRIPTINFO, obj+0x90)` INSIDE the `parent != -1` branch [M 0x1d9a08:106-133], and case 4
(cameras) writes the keys into the CAM_INFO with no base at all [M 0x1d9a08:190-200 +
`Script_UpdateCamera` 0x1db518] — i.e. **unparented keys are world-space**. Supporting geometry:
#289's camera key[0] (2.889, −7.829, −4.583) is Bond's boarding spot (spawn (4.26,−7.71,−4.76),
telephoto scale 0.344 — the shot films the −52 landing from the shaft top), and the marker key's
y is EXACTLY Bond's freeze pose. With the fix the whole flow works end-to-end: the enable drops
Bond at his freeze pose 17.5 above the still-descending cab, he falls back ONTO it (probe fb6i:
airborne t2188..2312, grounded t2313 at y −75.50, 0.4 under the key line, riding on to the lobby),
ch 0x4a fires at t5040, and the walk out — **`walkreal_0700004a_live` un-ignored and GREEN**.

**Why it was reverted:** the same change displaced 07000014's fusebox wire to the origin
(`[sab] wire at [0.0, 0.0, -0.0]` — `the_07000014_sabotage_cuts_through_the_real_laser`,
`the_laser_cuts_the_guardroom_fusebox_wire_to_ch137`) and broke `the_07000008_safe_chain_works_in_play`.
Those entities key LOCAL offsets and are render-verified with the base applied — the game DOES
compose the base for unparented streams (the decompile's tail-Concat structure loses a step, the
same way the camera's "*suggested*" note did). **The two evidence sets are contradictory as a
general rule**: the fusebox entities need the base; #289's marker base-composed lands at
(1.80,−107.66,−6.83), which probe fb6j shows is a VOID column (no floor −52..−160 within the
shaft at that x/z, only a north wall at z −6.0 — no SphereIntersect rescue surface), while raw it
works perfectly. The marker's single key at t575 with obj_type 0x19 may be special (world-space
authored, or the create-time interp ordering differs).

**What is on the branch:** the reverted `nis.rs` (all prior behaviour + the new NF_SCRIPTLOG
event-6/free diagnostics + `Script::stream_bindings()`), the ride-loop fall exemption in
`walkreal_0700004a_live.rs` (harmless with the revert; the test re-`#[ignore]`d with the full
chain in its header), and the three probes.

## 2. 07000016, door[0] onward

**The ch-0x24 panel writer is FOUND** (probe `walkreal_probe_fb9`, committed): trigger #0 — kind
0x1 (the mask-0x04 activate panel), out 0x24, mask 0x4 (the player's E), placement **250** at
**(−15.20, 6.52, −94.56)** — NOT beside door[0]: it is an UPPER WALKWAY west of the trench
(the corridor's trench dips to −4.8; the walkway floor holds y ≈ 5.71–6.44 for x ≤ −10 across
z −88..−112). For contrast the 0x27/0x28 panels sit beside their doors as documented
(placement 337 at (3.36,0.84,−141.50), 338 at (3.34,0.84,−151.95)).

The route fix attempted: the door[0] approach was re-laned through the leaf's span tightly
(`door[0]'s west approach` (2.3,−110) r 0.7 → `door[0]'s threshold` (2.3,−113.2) r 0.7 — both
arrive cleanly now), but the through-leg still wedges: door[0] is CLOSED and LOCKED (ch 0x24
never fires in the port — the panel is up on the walkway and the walk never reached it), so the
stuck-mitigation's hop puts Bond on the east furniture (5.20,1.56,−113.63) again. Probe fb9's
last grid also shows an upper deck at y 5.71 spanning x 2..8 across z −106..−112 (over door[0]'s
approach) — the walkway access (how the player climbs from the corridor/east lane to the deck)
is the next session's first probe (candidates: a stair at the deck's east edge x 8..9.5, or from
the room past door[0] — the deck may also be the −52-style landing above the doorway).

The test keeps its tightened lane (an improvement — the approach legs now arrive) and stays
`#[ignore]`d with the panel evidence added to the route comments.

## Verification

- The candidate fix's end-to-end pass WAS demonstrated before the revert (the 4a test green at
  3.20 s, the fall-onto-cab evidence in probe fb6i) — the fail-first is the previous session's
  parked failure (−107.66 vs the ride).
- After the revert: `fix_fusebox_cut_live` 4 passed, `fix_safe08_live` 5 passed (the three broken
  tests restored); `nf-game` 190 passed / 0 failed; `nf-formats`+`nf-audio` 80 passed with only
  the 3 allowed `disc_golden` trio; `nf-view` fast chunk 331 passed / 3 failed — ALL THREE were
  the change's own casualties (the two 07000014 laser/fusebox tests + the 07000008 safe chain),
  green again after the revert (verified directly). The walk-family chunk was not re-run in full
  (time); `walkreal_0700004a_live` compiles and is ignored.

## Files

- `crates/nf-game/src/nis.rs` — the NF_SCRIPTLOG event-6/free diagnostics (which stream/object the
  snapshot came from, the enable matrix at free), `Script::stream_bindings()` (probe accessor),
  and the revert note on the case-1/case-4 comments. Behaviour: UNCHANGED from master.
- `crates/nf-view/tests/walkreal_0700004a_live.rs` — the header rewritten to the resolved chain,
  the ride loop's fall exemption (disabled window + 5 grounded ticks), re-parked.
- `crates/nf-view/tests/walkreal_probe_fb6/fb7/fb8/fb9.rs` — the evidence probes (the cut-scene
  stream/matrix dump, the NIS stream/key dump, the object homes + SP placements, the 07000016
  trigger/panel scan + the walkway grids).
- `crates/nf-view/tests/walkreal_07000016_live.rs` — the tightened door[0] lane (the approach
  legs arrive now).

## What's left

1. **4a's last step**: settle the marker's base question (live check below); if the game's enable
   really is the raw freeze pose, re-apply the world-space reading SCOPED to the create-time
   snapshot path (or make `Player_Enable`'s sphere-resolve real) and re-land the (already
   written and verified) ride fix + fall exemption.
2. **07000016**: the walkway access up to the y 5.71 deck (probe x 8..9.5 at z −104..−108 and
   the deck's ends), the E on the panel at (−15.2,6.52,−94.56) → ch 0x24 → door[0] opens on the
   E; then doors [1]/[3]/[2] (their panels ARE beside them — placements 337/338), the silo plate,
   the launch chain, the chaser.
3. 0700001b's hub pit and 07000015's door[32] (the walkreal-c remainder) — untouched.

## Live checks for the main agent

1. **THE enable matrix** (settles 4a): ride the real lift past the −52 landing with
   `gdbwatch.py` on the player object's y (obj+0x30..0x38) through frames ~1611-2400-equivalent,
   plus a break on `Player_Enable` 0x195498 reading its MATRIX* arg (SI+0x40). Expected if the
   marker is raw: the enable translation ≈ (1.80, −54.13, −3.45) and Bond falls ~1.5 s onto the
   passing platform (grounded again ≈ 5.3 s after the cut-scene started). Expected if composed:
   ≈ (1.80, −107.66, −6.83) — and then WHERE does he land? (the port's probes show no floor on
   that column; if the game catches him, the rescue mechanism is the missing piece).
2. **The cut-scene camera**: screenshot at the landing cut-scene's start — if the camera looks
   down the shaft from the boarding spot (telephoto), the world-space keys reading is right; if
   it rides beside the platform at the landing, the composed reading is right.
3. **07000016 door[0]** (from the previous report, still open): walk to door[0], watch the ch-0x24
   writers — expected: a mask-0x04 panel/switch object writes it on use.

## `--bond` spots for the render

- `--bond 4.26,-7.71,-4.76,2.6,-0.9` (0700004a t0): the shaft top lobby, the lift platform below
  south.
- `--bond 1.8,-53.5,-3.45,2.8,0.2` (0700004a, the cut-scene's end with the candidate fix): Bond
  on the −52 landing area with the platform ~17 below — shows whether the framing matches the
  real game's post-cut-scene state.
