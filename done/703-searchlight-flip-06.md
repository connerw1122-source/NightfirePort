## Your task: `searchlight-flip-06` — a searchlight beam that flips on its side (07000006)

Playtest 1013-1825 (07000006): "Sometimes a spotlight will cover a piece of ground and it will like flip on its side,
which looks really weird". Trace each 06 searchlight's beam matrix per tick over its sweep (`searchlight.rs`, the
parts' matrices). Find a sign flip or gimbal discontinuity: the `align2dir` up-vector fallback when the beam points
straight down, an atan2 wrap, or the roll [M Searchlight_Update 0x1cb900, Mat_Align2Dir with its fallback vector].
Fix it to the game's math. Test: every searchlight's beam basis is continuous tick to tick over 1800 ticks (no
column flips sign while the direction moves smoothly). Sister: a synthetic straight-down beam uses the game's
fallback.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/searchlight-flip-06`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T16:13Z

---
## Report (done, 2026-10-16T17:39Z)

# searchlight-flip-06 — the searchlight beam basis to the game's Mat_Align2Dir math (2026-10-16)

Branch `s/searchlight-flip-06` from `origin/master`. Playtest 1013-1825 (07000006): "sometimes a
spotlight will cover a piece of ground and it will like flip on its side".

## What's done

`Mat_Align2Dir` 0x1df2a8 now matches the game's disassembly exactly (`tools/crosscheck/mipsdis.py`):

1. **A degenerate cross ZEROES the row** — the game's `sqrt(x) == 0` arms store 0.0 to all three
   components [M 0x1df2a8 disasm 0x1df2d8–0x1df310 (the dir), 0x1df3a8–0x1df3e0 (row 0): `c.2.s`
   against zero, the taken arm `swc1 f3(=0.0)` into the vector's slots]. The port had
   `[1,0,0]`/`[0,1,0]` stand-ins behind a `|cross|² < 1e-20` window — a snap onto the world axes
   where the game keeps the (exact-FP) cross direction sound and continuous. That stand-in is the
   only port-vs-game divergence found in the basis, and the only one that can read as a "flip on
   its side" (a world-axis row is the beam cone lying sideways for a tick or two).
2. **The fallback threshold is 0x3f7fbe77 (0.998993)** — `lui at,3f7f; ori at,at,be77`
   [M 0x1df2a8 disasm 0x1df320–0x1df354] — not 0.999.
3. **Normalization multiplies by the reciprocal** [M 0x1df2f4 / 0x1df3c4: `div.s f0,1.0,len` then
   the VMUL], it does not divide per-component.
4. The zero-dir arm also lands on zeros (a sum of squares is 0.0 only when every component is 0,
   so the game's lane shuffle moves zeros).

The beam call's arguments were verified at the instruction level: `Mat_Align2Dir(beam+0x90, dir,
up, up)` with up = Bond − head [M Searchlight_Update 0x1cb900 disasm 0x1cc020 `daddu a3,a2,zero`]
— the port already matched, and with fallback == up the fallback branch is inert for the beam; the
degenerate handling is what differs.

## How it was verified

- **The 1800-tick continuity tests** (`crates/nf-view/tests/searchlight_flip_07000006_live.rs`,
  the real path `nf_view::load("07000006.bin")` + `Sim::step_bond`): every 06 searchlight's
  head/mount/beam matrix rows must not jump more than 0.35 rad tick-to-tick while the head
  sweeps/tracks. Two scenarios: Bond parked inside head #2's sweep (the boot NIS lights the heads,
  the sweep passes over him, the spot arms the tracker, the tracking leg included) and Bond
  WALKING the courtyard (the arm/track/lose cycle at walking speed). Both green post-fix.
  The assert compares only same-state active tick pairs (active_run ≥ 3): the light-up tick builds
  the beam from the stale state-3 angles and the next tick's first sweep jumps — the game's own
  case-3 fall-through does exactly that [M 0x1cb900], so it is not a sweep flip. (The first two
  "flips" the trace caught were create→first-update and turn-on→first-sweep — both the game's own.)
- **The sister** (`a_straight_down_beam_uses_the_games_fallback`): a straight-down beam with a
  REAL fallback (the game's other Mat_Align2Dir callers pass CONSTANT_UP1/UP2 0x2d6210/0x2d6220
  [M symtab; Bullet_init 0x12c5a0]) gets the cross(fallback, row2) basis [[1,0,0],[0,0,1]]; the
  beam's own parallel-up arguments zero the rows; a near-parallel cross normalizes to its own
  sound direction, never a world axis.
- **No regression** (all on the final tree): nf-game FULL (lib 240/240 — the
  `align2dir_matches_the_live_beam` [L] pin against the live EE beam matrix still holds with the
  game's math — plus every integration binary, 0 failures), nf-formats (79 lib + all binaries ok;
  only the 3 expected slim-ISO `disc_golden` failures), nf-audio (all ok), and the nf-view
  families the change touches: searchlight_flip_07000006_live 3/3, fix_searchlight_alarm_live 3/3,
  fix_searchlight_remainder_live 2/2, models_world_live 2/2, alarm_panel_06_live 2/2, cutscene_06_live
  1/1, fix_06_alarm_live 2/2, fix_06_driver_live 2/2, decal_order_live 2/2, casings16_live 2/2,
  impacts_live 1/1, weapon_matrix_live 4/4, worldobj_live 3/3, wire_live 6/6 (+24 ignored),
  character_lighting_live 2/2, character_room_light_live 5/5.
- **The full-suite total line is NOT available**: the fast runner (`tools/test/suite.py`) was
  started twice and killed twice by this sandbox's background-job reaping between tool calls (the
  first one also died at a session break). The per-crate foreground runs above cover nf-formats,
  nf-audio, nf-game and the align2dir-touching nf-view families; the next session can re-run the
  runner for the total.

## What's left / live checks for the main agent

The headless sim shows a continuous basis on every traced path, and the port now matches the
game's math instruction-for-instruction — but the playtest symptom is VISUAL and this sandbox has
no GPU (lesson 6). Please render and watch:

1. `--bond <courtyard 07000006 spot>` mid-sweep, a few seconds apart, each courtyard searchlight:
   the beam cone must keep its roll steady across the sweep (no sudden sideways pose).
2. The moment a beam points near-straight-down (the sweep's pitch crosses −π/2, once per period
   per head): the head/beam matrices there are the near-degenerate window.
3. `tools/trace/gdbwatch.py` on the beam part's `obj+0x90` row 0 (16 bytes) while the sweep's aim
   crosses Bond's direction in play: the game's row 0 should swing fast but continuously (or zero
   for the exact-degenerate tick), never hold a world axis.
4. If the render still shows a flip the headless path does not, the next candidates are the spot's
   Euler builder at the Euler gimbal (pitch → ±π/2 on wall/sphere normals — the
   fix-searchlight-remainder reading's aivolume::matrix) and the head matrix's construction path
   (the game builds head/mount matrices outside Searchlight_Update — no Mat_Align2Dir call exists
   in it; the port's align2dir-for-heads was pinned [L] on one live head only).
