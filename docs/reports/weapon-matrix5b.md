# weapon-matrix5b — the rebuild on master's matrix4b

Branch `s/weapon-matrix5b` (base `s/weapon-matrix5` @ 4cf0d3ef, `origin/master` @ d1e8f6a7 merged first).

## What was done

1. **The rebuild.** Merged `origin/master` (90 commits: weapon-matrix4b's runner, fix-samurai-fire's
   `Gun::has_ammo(…, clip_slot)` + the clip-slot unification, fix-aim-raise2's `aim_gate`, …) into the
   matrix5 base. `matrix.rs` and `scene.rs` auto-merged; the three harness fixes survived intact and
   were verified in place: the watch OR arm (scene.rs mode block on `action12|action9`
   [M `Player_Weapon` 0x1a24c8:188-201]), the real-path fire invariant (`step()` feeds
   `input.fire = trigger`), and the guided-camera ride-out before the reload/mode/away phases
   [M `Bullet_Delete` 0x12e660:27-29 + `Player_Update` 0x193600 case 10].

2. **The HELD failure root-caused and fixed.** On the merged tree,
   `upgraded_runner_checks_the_new_behaviours_on_07000005` failed at weapon_matrix_live.rs:227
   ("0x54: the watch's twin chain must operate") — reproduced first (exit 101, panic at :227).
   Trace (examples/probe_watch.rs, the cell-shaped hold phase): the machine wedged with
   `fire_state = 1` while the view played the IDLE script, forever.

   - **Cause [M]:** `Player_SetFiringAnim` 0x1a3c70's tail is
     `if (fp_model != 0 && hash != 0) { *(short*)(anim+0xf4) = 9; AnimScriptAdd(…); }` — the game
     enters view state 9 ONLY when the fire script exists. The watch pair 0x54/0x55 has
     `+0xB0/+0xB4 = 0` [D probe dump], so the game's `anim+0xf4` stays 0 and the mode block's
     `anim+0xf4 == 0` gate [M `Player_Weapon` 0x1a24c8:199-201] stays open. The port latched
     `gun.fire_state = true` unconditionally on every round (gun.rs update_with); its state-9 clear
     (scene.rs, [M 0x1a2e38 case 9:449-456]) needs `view_script == +0xB0` — which never plays for a
     script-less record — so the proxy stuck at 9 forever and the mode block refused every press.
     It only surfaced on master because fix-samurai-fire's clip-slot unification now lets the
     watch's round-to-fire actually run (ammo_type 0 rounds unconditionally [M 0x1a18e0]) — on
     matrix5's pre-unification tree the round never armed, so the wedge never showed.
   - **Fix:** the state-9 latch gates on the fire script existing — `GunParams` carries
     `fire_script`/`fire_last` (+0xB0/+0xB4), and gun.rs latches `fire_state` only when one is
     non-zero, matching 0x1a3c70. The spawn latch (+0x95C) is untouched: the +0x44 == 0 press-tick
     spawn is behind `Player_RoundToFire`, not behind the state-9 store [M 0x1a5460:688-693], so
     the camera still "photographs" through its path. The runner test then passed (118 s).
   - **Fail-first + sister:** the 227 assert is the fail-first; the new sister
     `cell(0x54).machine_idle` (recorded at the fire settle: fire_state clear + switch/reload at 0)
     pins the honest state-9 proxy and fails on the old latch.

3. **The in-suite budget holds.** The full `weapon_matrix_live` binary on the merged tree:
   **6 passed / 0 failed in 341.8 s** — matrix4b's 341 s, well under the 1200 s cap (the ride-out
   bound and the sister add nothing measurable).

4. **The sweep re-run.** All 24 ignored level tests ran on the final tree: **24 passed / 0 failed,
   0 panics** (510 s, `--ignored --test-threads=1`). Per-cell marks tables were captured for 16 of
   27 levels (01-09, 0a, 0b, 0d, 11, 12 + the in-suite 05; the --nocapture re-run hit the sandbox
   window at 13 more); the other 14 levels' tests passed with 0 panics but their tables were not
   captured in this window — matrix5's tables stand for them.

## The red list (numbered findings; from the 16 captured tables — identical shape on every level)

The `n` marks (record has no fp model, by design — the game's cycle cannot reach them either) are
not findings. The real reds:

1. **0x32 F — the Phoenix overcharge's fire shows nothing observable** (every level that carries
   it). matrix5's finding stands: the cell's fire phase is target-gated (no live guard in reach at
   the harness spot), the fire path itself works (matrix5's live check).
2. **0x38 F — the mine family's quiet fire** (every level). matrix5's live check stands: the mine's
   fire spawns no observable effect on the harness row.
3. **0700000b only: 0x2c W + 0x2e/0x30 D — the guided-missile lifetime.** matrix5's root cause
   stands: the port's guided missile never dies (the game's `Bullet_update` 0x12e7b0 lifetime is
   unported), so on 0b the camera rides into the later cells (gcam1) and the Sentinel's away switch
   reads W even with the 700-tick ride-out bound (the bound records the honest mark instead of
   hanging — as designed).
4. **0700000d only: 0x2a X — the MGL blast-gap mark.** matrix5's re-run explains it: gap 97 ticks =
   the contact flight time at that spot, not the 210 fuse. Not a bug.
5. **The give refusals (`.` at 0x01 and friends)** — the dim no-fp give-refusal family from
   matrix3/5, unchanged.

**Improvement over matrix5:** the watch pair 0x54/0x55 is now CLEAN on every captured level — the
state-9 fix removed the M reds the pair carried on master's tree, and the twin chain (0x54↔0x55 via
the plain mode press, the OR'd fire press and the pair's oscillation) operates everywhere.

## Verification

- The failing-first: the 227 panic reproduced on the merged tree before the fix (log kept),
  passes after (118 s); the in-suite binary 6/0 in 341.8 s; the sweep 24/24 passed, 0 panics.
- Full suite on the final tree (the 3 direct chunks + in-process runs, `--test-threads=1`):
  **1136 passed / 3 failed / 0 unexpected** — the 3 are the allowed `disc_golden` trio on the slim
  ISO; `pss_movies_live` and `driving_golden` passed. 288 binaries ran; `weapon_matrix_live` ran
  separately (341.8 s binary + the 24-level sweep + the upgraded runner) and is excluded from the
  direct-binary total. The `nf_view`/`nf_extract` "binaries" in deps include the nf-extract CLI
  (windowed browser, no test harness — N/A headless); the real nf_view lib harnesses: 8/0.

## What's left

- The 14 sweep levels whose per-cell tables were not captured this window (tests passed, 0 panics;
  matrix5's tables stand) — one `NF_MATRIX=all … --ignored --nocapture` run captures them.
- Findings 1-3 need their game-side work (the mine/overcharge observable effects, the missile
  lifetime 0x12e7b0) — separate tasks; matrix5's report and this one carry the citations.

## Live checks for the main agent (PCSX2)

1. **Micro-Camera 0x54 mode chain** (the fix): spot any level carrying the Micro-Camera (e.g. 05's
   gadget loadout). Equip it, press fire once — the game should swap to the twin 0x55 (the mode
   block ORs action 9 [0x1a24c8:188-191]); press fire again — back to 0x54. The view must NEVER
   stick "busy" (the port's old wedge): after any press the weapon must re-press/react within a
   second. Also verify the plain mode button (action 12) swaps the pair the same way.
2. **The camera's "photograph"** (unchanged by the fix, guards the spawn path): with 0x54 held,
   press fire aiming at a wall — the game shows the photo/view arm; no projectile spawns.
3. **0b guided family** (finding 3): fire the Sentinel 0x2c and the 0x2e/0x30 records on 0700000b —
   watch whether the game's missile self-destructs after a fixed flight time (the port needs that
   lifetime, `Bullet_update` 0x12e7b0).
4. **nf-view `--bond` spots**: `07000005` `--bond 94.2,44.6,-45.9,0,0` with the Micro-Camera given —
   the mode press and the fire press should both land the twin swap in the render (the HUD ammo
   pane label changes).
