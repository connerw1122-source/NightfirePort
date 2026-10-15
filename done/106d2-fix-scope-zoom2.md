## Your task: `fix-scope-zoom2`, scope zoom: the sniper mode-press test fails on the merge

Base: `s/fix-scope-zoom` — merge origin/master FIRST. HELD (bisected: master + fix-samurai-fire + weapon-matrix5
passes this test; adding this branch fails it): `fire_modes3_live::sniper_mode_press_resets_zoom_and_swaps_damage`
"the scope zoomed in (zoom 1)" — after this branch the sniper's zoom stays 1.0 in the test's window. Decide from the
game which is right: your zoom model (the aim-start sqrt(mag), the zoom keys, the scoped gate) vs the test's
expectation (fire_modes3_live.rs ~line 452, written for the old zoom); master also has fix-weapon-anims' aim raise and
fix-aim-raise2 may land first (the aim bit set at the raise's end — the zoom should start then). Fix the code or the
test with the game's numbers; keep fix_scope_zoom_live green. Run the full `tools/test/suite.py`; total in the report.

Files: `crates/nf-view/src/{scene,hud}.rs`, `crates/nf-view/tests/fire_modes3_live.rs` as justified.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/fix-scope-zoom2`.

---
claimed-by: sandbox6 at 2026-10-14T23:49Z

---
## Report (done, 2026-10-15T01:36Z)

# fix-scope-zoom2 — the sniper mode-press test on the merge

Branch `s/fix-scope-zoom2` (base `s/fix-scope-zoom` 2c4864b0 + `origin/master` merged twice:
43e65549 first merge, 84e0719f/c845094d the pre-push pull). Task: HELD on the merge —
`fire_modes3_live::sniper_mode_press_resets_zoom_and_swaps_damage` "the scope zoomed in (zoom 1)";
after `s/fix-scope-zoom` the sniper's zoom stays 1.0 in the test's window. Decide from the game:
the zoom model vs the test's expectation.

## Verdict from the game

**The zoom model is right; the queue's edge source was wrong.** Two separate things were conflated
by the HELD bisect:

1. **The zoom model survives the merge.** `Player_Weapon` 0x1a24c8:154-187 + `Player_Zoom` 0x1a1a70
   + `Player_InitAmmoWeapons` 0x1a17f8:51 (the base branch's model, decompile re-verified this
   session): aim start targets the BASE weapon's slot `BL+0x1B4 + base·0xC` filled with
   `sqrt(+0x88)` [M 0x1a24c8:173-177 — the read gated by input option 0x2a38ce, default 1
   [M Input_Init 0x17edd8:22, L ee_start_05 = 1]], clamped down to +0x88 [M :179-183]; the zoom
   axis scales target and value ×1.025/tick [M 0x1a1a70:35-39,56]; release keeps, aim end targets
   1.0 [M 0x1a24c8:158-166]; the reached zoom persists into the slot [M 0x1a1a70:72-74].
   **The aim-start slot write is NOT gated on the scoped flag** — the decompile's toggle block runs
   for every weapon [M 0x1a24c8:168-187]; the record's own +0x88 does the gating (mag 1.0 keeps the
   target at 1.0), which is why the unscoped zoomables 0x18/0x19 (3) and 0x2a/0x2b (2) zoom with no
   overlay [L ee_start_05 census]. Master's pre-merge `if input.zoom && self.aiming { target =
   max_zoom } else { target = 1.0 }` block was the old coarse model and lost the slots/persistence;
   the merge resolution restores the game's model (scene.rs, the zoom block).

2. **The actual bug: the raise queue edged on the raw button instead of the game's latched input
   bit.** The game edges the aim raise/lower on `+0x60` **bit0 — the latched input** — and the
   view-state gate CLEARS bit0 while the machine is busy [M `Player_Weapon` 0x1a24c8:64-99 — the
   +0xf4 switch clears bit0 for every non-idle view state (scoped list: 1-4, 6-8, 10, 13-16)] and
   the raise queue consumes it [M :144-152 `uVar8 & 0xfffe`]. So a press that lands while the DRAW
   still runs (and stays held) re-edges on the tick the machine returns to idle [M :117-152 — the
   edge detect compares the latch against last tick's stored latch]. The port's raw-button edge
   (`aiming && !prev_aiming`) fired mid-draw, was refused by the queue arm (busy), was consumed,
   and never came again → the raise never queued → the aim bit never set → the zoom target stayed
   1.0 → "zoom 1". Every fix-scope-zoom test that aims from tick 0 (mid-draw) hit this.

## The fix (scene.rs, 3 hunks)

- **The edge source is the latch** (`run`, the aim-edge block): `aim_latch = raw aim input AND the
  fully idle machine` (view_script == idle, switch/reload/fire idle, no raise playing) — mirrors
  bit0; `aim_on_edge`/`aim_off_edge` key on the latch; `prev_aiming` now stores the latch
  (re-documented; sole consumer is the raise queue — grep-verified).
- **The raise's finish writes the latch** (`aim_gate`, the raising finish):
  `prev_aiming = true` [M `Player_SetWeaponAnimObj` case 0xf 0x1a2e38:554-561 — the finish's
  `+0x60 |= 1` at :557] — without it the finish tick produces a fresh latch rise and re-queues the
  raise forever. The lower's finish writes no bit [M 0x1a2e38:562-567] — mirrored (no write).
- **The merge's zoom block** keeps the base's game model (slots, edges, persistence, no scoped
  wrapper) with master's aim-gate architecture (the edges run on the GATED aim bit, so a
  raise-script weapon's zoom starts at the raise's finish [M 0x1a2e38:554-561]; `Player_Zoom`
  0x1a1a70:18 reads the axis only while the bit is set — the `zooming` gate).

Documented gap (pre-existing, unchanged): the fidget is a script swap at view state 0, so the
game's bit0 is KEPT during it and a press displaces the fidget; the port's latch includes
`view_script == idle_hash`, so the raise waits for the fidget's end instead. Also pre-existing:
a switch while aimed keeps the port's zoom target (the game's aim bit drops in the holster and the
slot re-read on the raise's finish lands at the same value — the twins test exercises the same
visible outcome through the kept target).

## Tests (fail-first shown, sister assertions in)

- **New: `fire_modes3_live::sniper_aim_held_through_the_draw_still_zooms`** — the draw request,
  the aim press lands MID-DRAW and stays held: the aim bit sets (the latch re-edged at idle, the
  raise finished) and the zoom eases to the slot sqrt(10) ≈ 3.1623 [M 0x1a24c8:173-177; L 0x1E
  +0x88 = 0x41200000]; sister: the held zoom axis then climbs to the cap 10.0 [M 0x1a1a70:35-39,56;
  L 0x1E +0x88]. Shown FAILING pre-fix (`git stash` of the scene.rs hunks): panicked at
  fire_modes3_live.rs:494 "the aim bit set once the draw ended and the raise finished".
- **`fix_scope_zoom_live` 4/4 green** (was 1/4 on the naive merge — `zoom_levels` "w 0x1a:
  aim-start zoom 1 != sqrt(+0x88) 2.4494898", `the_twins_share_the_base_zoom_slot` "0x1b held
  zoom 1 != 6.0", the overlay gates). The overlay test's window was updated to the game's timing,
  not its old assumption: the HUD gate reads `+0x60` bit0 [M `HUD_UpdateCrossHair` 0x19e6d0:27-148],
  the raise queue CONSUMES bit0 on the press tick [M 0x1a24c8:144-152] and the raise's finish
  writes it back [M 0x1a2e38:557] — so the pane appears at the raise's end, not on the press tick.
  Measured per weapon in 07000014 [L probe]: 0x24's raise script is absent from that level's set
  (bit at tick 1), 0x1a raise 20 @1.25 → tick 16, 0x54 raise 71 @1.25 → tick 56, 0x32 has no raise
  (immediate); the hold is 80 ticks for every case.
- **`fire_modes3_live` 6 passed / 4 ignored** — including the HELD test
  `sniper_mode_press_resets_zoom_and_swaps_damage` (its window holds the aim from a drawn, settled
  weapon, so the raw edge always fired; it passes under the game model).
- **`fix_aim_raise_live` 5/5, `fix_viewmodel_zoom_live` 3/3** — master's raise tests unaffected.

## Files

`crates/nf-view/src/scene.rs` (the named file; the latch hunks + the merge resolution of the zoom
block and the two new fields), `crates/nf-view/tests/fire_modes3_live.rs` (the named file; the new
test), `crates/nf-view/tests/fix_scope_zoom_live.rs` (the base branch's own test file — the task's
"as justified": the window fix with the game's timing). **hud.rs was NOT touched** — its gate reads
`sim.aiming`, which for raise weapons equals bit0 in every steady state (press/raise/finish/hold/
release; the merge's one-tick divergence is the test's window, fixed there). No shared file
(drone.rs/player.rs/lib.rs) was touched.

## Suite

`tools/test/suite.py`'s background mode dies in this sandbox between tool calls (suite.out empty,
process gone — the 102c/100f/107d precedent), so it ran as foreground chunks
(scripts/chunked_suite.py, per-binary 600 s limit, --test-threads=1) **on the final tree
(c845094d, master merged)**:

```
197/293 binaries run: 904 passed, 2 failed, 47 ignored, 0 TIMEOUT
failed: fix_0c_pylon_live (2 — pre-existing, on master too)
```

The remaining 96 binaries (among them the 3 `disc_golden`, allowed to fail on the slim ISO) did
not fit the window. Full-tree `cargo test --no-fail-fast --release` (the four crates,
--test-threads=1, on 555544a4 = the same code before the second master pull): **677 passed,
5 failed, 15 ignored** — the 5: the 3 allowed `disc_golden` + the same 2 pre-existing
`fix_0c_pylon_live`. No zoom/aim/mode regression anywhere.

## Live checks for the main agent

PCSX2 (07000005, Winter Tactical Sniper 0x1E via the dev menu's Give, pad scheme):
- Spawn, press-and-HOLD aim (L1) the moment the draw starts (circle during the holster): the scope
  should raise on its own once the draw ends, then the zoom sits at ~3.16 (sqrt of the ×10 mag) —
  NOT stuck at 1.0. Expected: the raise anim plays, then the overlay appears.
- With aim held, hold the zoom axis (action 6): the zoom climbs ×1.025/tick to 10.0 and STAYS when
  the axis releases; release aim → eases back to 1.0; re-aim → resumes at 10.0 (the slot).
- Mid-raise release: the scope lowers via the reverse raise; no zoom flicker beyond one tick.
- Mode press (circle on keyboard B) while scoped+aimed must be REFUSED [M 0x1a24c8:204-212];
  released, it lands the twin and resets the zoom to 1.0 [M :213-228].

nf-view `--bond` spots (07000005 spawn, give 0x1E first):
- `--bond 105.9,25.3,108.1,0,0` hold aim → the scope overlay draws, the crosshair hides, the
  view-model hides while scoped; after the raise, FOV divided by ~3.16.
- Same spot, aim + zoom held 300 ticks → FOV/10, the overlay full-bleed.
- The AIMS-20 (0x1A) at the same spot: the OICW circle texture (0x03000141), not the Sight scope.

## What's left

- The 96 suite binaries not reached before the sandbox deadline (the 3 disc_golden allowed fails
  among them) — the next session should finish the chunked run (`python3
  scripts/chunked_suite.py 0 293`, it resumes from /tmp/suite_state.txt) and confirm no new fail.
- The fidget-vs-raise displacement gap and the switch-while-aimed bit-drop gap (both documented
  above, neither breaks a test).
