# hatch-02-intro — the hatch in 07000002's intro never opens

Playtest 1014-1725 [intro02]: "the hatch still doesn't open". Branch `s/hatch-02-intro` (queue
715), cut from origin/master c8186aa1, merged forward to 170bcf01 during the session.

## TL;DR

**Current master already behaves like the game.** The playtest predates the grates-02 merge:
the hatch it means — `grill1` #440, the bunker-bed grate in the room the 02 level starts in —
opens on the use key now, and the test pinned here fails on the pre-grates-02 code (proven by
reverting `ss.rs` to 366f5d49 and re-running). The intro script itself animates **no hatch**;
that part of the task premise does not hold on the disc.

## What the disc says (all measured this session, through the real loaders)

- **The level-start NIS animates no hatch** [D]. Script 0x06000084 (pack 07800002.bin, SP #400
  mode 1, 1540 ticks): stream 0 camera, streams 1/2 anim actors (skins 0x0500004f Mayhew,
  0x05000009 the guard-skin), streams 3/4 the player/drones disable-enable event pair, stream 8
  ONE animated entity — `OBJECT_DrogonPearl` (8 cm prop, type 0x19, 1500 keys rolling through
  the secret bunker). No stream touches any hatch object.
- **The intro's only channel write is event 0xD [0x47, 1] at tick 1539** [D] — the objective
  announce gate ("Destroy Mayhew's computer" / "Rescue geisha girls" both announce on en 0x47,
  MissionData). (Earlier probes misread the param as decimal 47; the levellog pins
  `channel 0x47 = 1` at frame 1540.) The port sets it — pinned by the new test.
- **The hatches are the four 0xE2 SimpleScripts** (grates-02's table, re-verified): #272
  `sg_grill_large`, #308 `bath_hatch` (E, ch 2), #309 `bath_hatch1` (mask 0x1, ch 1 — no writer
  anywhere on the disc, locked by design), #440 `grill1` (E, ch 0 — the bunker-bed grate, 2.5
  above the bed Bond starts beside).
- **The second cut-scene 0x0600007f (the hatch-panel scene) is dormant on the disc** [D]:
  its chain is Touch #495 (out 0x3c) → MultiplexSIn #496 (out 0x35 = ch 53; trigger type 4 =
  sequential AND of ch 0x3b && 0x3c, `Trigger_Update` 0x1b1a78 case 4) → SP #399; and ch 0x3b
  is MultiplexIn #397's AND of ch 0x29/0x2a/0x88/0x49 — **no placement on any of the 52 maps
  writes any of those channels** (full param scan), so ch 53 can never rise and the panel skin
  0x05000034 (mesh 0x02000926, 73 bones, no placement of its own) never animates. Nothing for
  the port to fix there.

## Why the port already matches the game

- `nf_game::ss` (grates-02, merged) ports `SS_Update` 0x1a9948's true state machine and
  `SS_Activate` 0x1a9ea8; E on #272/#308/#440 sweeps the grate open in 150 ticks and holds the
  open pose; #309 refuses E and nothing writes ch 1. `fix_ss_grate_live` 4/4.
- The render path uploads the SS panels' **full posed matrix** (`scene.rs` door/SS region:
  `q.matrix` — quaternion swing included), so the open is visible, not just collision.
- The intro run through the real path (`nf_view::load` + `step_bond` × 1560) tracks the disc:
  the pearl entity follows its key path (final position within 0.03 of key 1719), the object
  set mid-intro is exactly {camera, the two anims, the pearl}, and ch 0x47 lands at the end.

## The pin (`crates/nf-view/tests/hatch02_intro_live.rs`, 2 tests)

- `the_intro_animates_no_hatch_and_ends_on_the_objective_gate` — the intro completes (SP state
  5), creates exactly the four objects above, moves no SS grate during the play, and ch 0x47
  rises at the final event. Fails if a regression drops the script's event path or invents a
  hatch stream.
- `the_bunker_bed_hatch_opens_on_e_and_holds_the_open_pose` — Bond settles onto the bunker bed
  (the player's real stance), the test aims head→grate, and the use key through `step_bond`'s
  E edge takes `grill1` #440 from key 0 (asserted closed first) to the last key's open pose,
  holding it (state 0 after the wrap). **Fail-first proven**: with `ss.rs` reverted to the
  pre-grates-02 channel-seek tick (366f5d49) the hatch stays at the closed transform and the
  state sticks at 1 — exactly the playtest symptom.

## What I ran

On the final merged tree (origin/master 170bcf01): batch 1 — `hatch02_intro_live` (2),
`fix_ss_grate_live` (4), `fix_0c_hatch_live` (2+2 ign), `fix_0c_vent2_live` (3),
`cutscene_skip_audio_live` (3), `fix_cutscenes_live` (4), `fix_cutscenes2_live` (5): 23 passed,
0 failed. Batch 2 — lib tests: nf-formats 79, nf-view 17, nf-game 241: 337 passed, 0 failed.
Batch 3 — the cutscene/level integration slice (`cutscene_*`, `fix_cutscene_*`,
`fix_ladder_live`, `fix_objects_live`, `civilian_no_shoot_live`, `inert_guards_02_live`):
43 passed, 0 failed. Total 403 passed / 0 failed (+2 ignored).

The full suite did **not** run: `tools/test/suite.py` in the background was reaped between tool
calls (empty `suite.out`, no process), and a foreground full run does not fit the sandbox's
tool-timeout windows. The three batches above are the affected surface (the merge touched
nf-formats/hud, nf-game/drone, nf-view/gpu+hud — all three crates' lib tests ran clean).

## What's left

- A re-playtest of 07000002 on a current build: the bed-grate E-open, the pavillion grill and
  the bath hatch (all three open and hold), `bath_hatch1` staying shut (by design).
- **Live checks for the main agent** (renders; this sandbox has no GPU):
  1. `--bond -22.35,0.9,32.95,0.2,0.85` on 07000002 — the bunker-bed grate #440 overhead; press
     E: the panel should swing ≈131° and hold (grates-02 live check #2, still unrendered).
  2. `--bond -16.1,4.3,33.8,1.57,0` — pavillion grill #272 opens ≈135° about z on E.
  3. The 02 intro (level start): the pearl prop visible rolling through the bunker near Bond's
     start spot, Mayhew at the computer desk — confirms the entity/anim draws on screen.

## Usage

not visible to me (no token, context-window or cost figures are exposed in this environment).
