# fix-final-blockers — the two walkreal-c blockers

Branch `s/fix-final-blockers`, cut from `origin/master` (59d466c). Task: the two port bugs
walkreal-c parked on the final levels — 07000016's door[7] never opening, and 0700004a's lift
losing its rider mid-shaft. Status: **door[7] RESOLVED (with a focused passing NF_ISO test);
0700004a root-caused past the task's hypothesis to the real mechanism and parked with the full
evidence chain.** Everything is committed and pushed; the next session continues from the branch.

## 1. 07000016 door[7] — RESOLVED (two stacked causes)

The walkreal-c report's symptom ("14 `use_door` calls, state stays 0x2") is STALE: at the stuck
dump on current master the door IS open — state 0x4, pos01 1.000, matrix row3 y 2.175 (the leaf's
open pose; the keys slide it up 2.28). The straddle walk-in that landed in `door.rs` after that
report (the +0x3c param → obj+0xfd bit 8 [M `Door_Create` 0x136100]) fixed the E path.

The REAL blocker was the route lane. Evidence (probe `walkreal_probe_fb2/fb3`, committed):

- The doorway's operable half is the leaf's own span x 4.35..5.62 (the closed leaf's world bbox;
  the placement pos is the leaf's WEST edge, not the doorway centre).
- A static-only crossing sweep (the dynamic set emptied) shows x 4.4..5.5 OPEN at the door plane
  while x 2.8..4.2 is a SEALED wall: the corridor cel mesh `Start_corridor1` (map 63 object 158)
  carries 311 triangles at z −74.13±0.2 — the doorway's left leaf is fixed geometry (a
  permanently-closed half; the game's own meshes do this too, so the real player uses the east
  half). `Door_Link2Portal`'s portal seals are NOT needed to explain this — the seal is ordinary
  cel geometry.
- The walkreal-c route aimed Bond at x 3.5 — into the sealed half — and his capsule wedged on it
  at (3.5, 0.9, −73.58) (segment_to_target 0.124 → the wall at z −74.13).

**The fix is the test's lane** (x 3.5 → 4.9 through the leaf's span; the E still reaches because
the use sphere tests the door's placement pos with the 2.5 stand-in). The walk then crossed the
trench (the corridor dips to −4.8 across x ≤ 7 at z −88..−102; the east lane x 8..11 is a
continuous −0.2 floor — probe `walkreal_probe_fb4`'s ledge grid) and parked at a NEW blocker:
**door[0]'s approach** — Bond wedged east of that doorway at (5.16, 1.75, −113.63) (the
stuck-mitigation's hop put him ON furniture, grounded=false; segment_to_target 0.102 → a wall at
z ≈ −114.3 east of door[0]'s leaf span x 1.84..3.11). door[0] is locked by ch 0x24 [D] whose
mask-0x04 panel writer is not located yet. The full walk test is re-`#[ignore]`d there with the
evidence in its header (the walkreal-c pattern).

**Test (NF_ISO, fail-first)**: `walkreal_07000016_live::door7_opens_on_bonds_e_and_bond_passes`
— walks the real input path to the corridor, asserts the static floor holds Bond for a second
(the sister), presses E via `use_door`, asserts the door opens and the leaf travels (pos01 > 0.5),
then walks through the leaf's span and asserts Bond passed the doorway plane grounded. FAIL-FIRST
demonstrated: with the old x 3.5 lane the walk wedged on the sealed wall even with the door open
(the run logs in this session; the walkreal-c stuck dump), and before the straddle walk-in the E
never moved the leaf at all (the walkreal-c report). Sister: `walkreal_07000012_live` green
(1 passed), and the in-test static-floor assertion.

## 2. 0700004a the lift rider — root cause found, parked

The task's hypothesis (the door-solid vs rider interaction in the moving set) is **DISPROVED**:

- At t1615 (mid-freeze) the opening leaves d50/d51 span z −0.98..−0.82 while Bond stands at
  z −3.78 — a 2.8 gap; his capsule push is (0,0,0) with walkable=false (probe
  `walkreal_probe_fb5`). The leaves never touch the rider.

The real chain (all reproduced in this session):

1. t1610: the lift script (#42, 0x06000571) pulses ch 0x40 (event 0xd [NF_SCRIPTLOG]) — the −52
   doors [110]/[111] start opening. t1611: it pauses (event 3).
2. **t1612: cut-scene #289 (hash 0x060006cf) STARTS** — its opening events at its tick 0: stream 4
   kind 0xb (the view model, weapon 0x12), stream 8 kind 0x05 (the drone freeze), stream 9 kind
   0x04 (**PlayerDisable**) [NF_SCRIPTLOG]. The game plays the −52 landing as a NIS. Bond goes
   disabled and freezes mid-shaft (the game's `Player_CollisionHandler` early-returns while
   BLData+0x94a is set [M 0x1949a8:24] — the riding inherit legitimately stops; scene.rs gates on
   the same flag). The platform keeps descending (−45.8 → −63.3 over t1600-2180 [D keys]) — so
   the rider and the platform part ways during the cut-scene, exactly like the real game.
3. With the cut-scene PLAYED OUT (`skip_cutscenes = false` — the real player watches it; the
   walkreal-c session's harness skip made the enable fire early at t1633 instead), the cut-scene
   ends ~t2163 (551 ticks) and `Script_Free`'s enable (flag 0x100 [M 0x1da880], nis.rs `free()`)
   places Bond at **−107.66** — 36.8 BELOW the platform's live pose. He falls (the walkreal-c
   "falls to −107.66").
4. **The port bug**: cut-scene #289's event-6 `player_at` (nis.rs `event()` case 6 — the PREVIOUS
   stream's entity matrix, captured at event time) is not the platform's live pose at free time.
   Either the prev-stream resolution picks the cut-scene's own animated stand-in instead of the
   platform entity, or the game re-reads the entity's CURRENT matrix at `Script_Free` (0x1da880)
   and the port bakes it in early.

`walkreal_0700004a_live` stays `#[ignore]`d (parked) with the chain in its header and ignore
message. This session's ride-loop changes are kept: `skip_cutscenes = false` for the ride (the
real player watches the landing NIS) and the key-line scrape check exempts `sim.disabled` ticks
(the game freezes Bond too — a disabled freeze is not a scrape). The test currently fails at
t2163 with y −107.66 vs want −70.89 — the fail-first for the next session's enable-matrix fix.

## Verification

- `walkreal_07000016_live` (the focused door[7] test) + `walkreal_07000012_live`: 2 passed.
- `walkreal_0700004a_live`: compiles, ignored (parked at the enable matrix).
- The full four-crate suite ran to the session's time cap: **79 passed / 3 failed — exactly the 3
  allowed `disc_golden` tests** (bigf_header_fields_behave_as_documented,
  bigf_tables_match_the_golden_manifest, directory_tree_matches_the_golden_manifest) before the
  timeout cut it mid nf-game. **A complete four-crate pass is still owed before the merge.**

## Files

- `crates/nf-view/tests/walkreal_07000016_live.rs` — the lane fix, the focused door[7] test, the
  parked header.
- `crates/nf-view/tests/walkreal_0700004a_live.rs` — the parked root-cause chain, the ride loop's
  cut-scene handling.
- `crates/nf-view/tests/walkreal_probe_fb2/3/4/5.rs` — the evidence probes (the static-seal
  sweep, the world-space seal scan, the trench/ledge grids, the ride window).
- **Shared file touched**: `crates/nf-game/src/door.rs` — ONE additive diagnostic line in
  `log_doors` (the matrix row3 + keys presence, NF_DOORLOG only). No behaviour change.

## What's left

1. 07000016: door[0]'s approach (re-lane west of x 3.1, locate the ch-0x24 panel writer), then
   doors [1]/[3]/[2], the silo plate, the launch chain, the chaser — the rest of the 16 walk.
2. 0700004a: fix the cut-scene #289 enable matrix (dump its stream list over frames 1612-2170,
   match against `Script_EventHandler` 0x1d94c0 case 6 + `Script_Free` 0x1da880), un-ignore the
   ride test, then the lobby walk-out.
3. 0700001b's hub pit and 07000015's door[32] doorway (the walkreal-c remainder) — untouched here.
4. The full four-crate suite pass.

## Live checks for the main agent

1. **4a's landing enable**: ride the real lift past the −52 landing; `gdbwatch.py` on
   BLData+0x94a (the disabled byte) through frames ~1612-2170-equivalent and on the player's y
   (obj+0x30..0x38) at the enable moment. Expected: the game re-enables Bond ON the platform
   (his y ≈ the platform's live pose + 1.1), NOT at a fast-forwarded/mid-shaft matrix. If the
   game's enable y IS below the platform there, the cut-scene's entity layout differs from the
   port's reading and the stream list needs re-derivation.
2. **16's door[7] lane**: walk Bond down the south corridor at x ≈ 4.9 (east of the doorway's
   centre), press E. Expected: the door slides up and the player passes through the EAST half;
   walking at x 3.5 bumps an invisible (static) wall — confirm the west half is sealed in the
   real game too.

## `--bond` spots for the render

- `--bond 4.9,0.9,-76.5,2.6,-0.35` (07000016): door[7]'s doorway from the north — the east half
  open (the leaf up), the west half a fixed panel.
- `--bond 3.2,-6.97,-3.8,3.1,0` (0700004a, ~t1600): Bond aboard the platform passing the closed
  −52 leaves (z gap 2.8 to the leaves at z −0.9).
