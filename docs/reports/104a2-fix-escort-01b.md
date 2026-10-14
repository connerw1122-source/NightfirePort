# 104a2-fix-escort-01b — the escort fix breaks weapons on 05

**Status: blocked** — the root cause is proven to the exact delta and the trigger chain is pinned to
the tick, but the game-side fix did not fit the window. The next session can start at "The fix to
land" below. Branch `s/104a2-fix-escort-01b` (the protocol-line name; the task tail said
`s/fix-escort-01b`, the same precedent as last round's oddjob task).

## What was done

1. **The tree.** `s/104a2-fix-escort-01b` = `origin/s/fix-escort-01` (a8803b30) merged with
   `origin/master` twice (first at b9ed8b8a, then again at 62c17571 — the oddjob-hat merge block;
   WORKLOG conflicts resolved append-only, both sides kept).
2. **The probe (DONE).** `probe_escort_root::probe_root_block` panicked at line 22
   (`.expect("run script")`) because the slim ISO's 07000001 does not carry script `0x06002056`.
   Fixed to skip cleanly when its input is absent (`let Some(...) else { eprintln!(...); return }`),
   which is the task's own prescription. Green: `1 passed; 0 failed; 0.25s`.
3. **All three failures reproduced** on the merged tree before touching anything:
   - `weapon_matrix_live::upgraded_runner_checks_the_new_behaviours_on_07000005` — panics at
     weapon_matrix_live.rs:138 "0x2a must spawn and blast its grenade". The 0x2a cell is
     `D vm1 draw0 shots0 fx(0b/0i/0e/0g)` — the DRAW fails, so the fire loop (which counts the
     grenade spawn and the blast) never runs.
   - `fix_weapon_sounds_live::every_weapon_on_07000005_draws_fires_and_reloads_with_sounds` —
     "0x41: [639], 0x42: [1051, 1481], 0x43: [1486, 1487, 1488] never played".
4. **Causality proven by experiment (the core finding).** The escort delta on the merged tree is
   **nav.rs ONLY** — `git diff origin/master HEAD -- crates/nf-game/src/{drone,triggers}.rs` is
   EMPTY (the branch's history never touches them; its unique commits are the LinkCreep port in
   nav.rs + tests/probes/docs). Reverting **only** `crates/nf-game/src/nav.rs` to master's version
   makes BOTH weapon tests pass:
   - matrix: `test result: ok` (60.45s) — the 0x2a cell becomes `- vm1 draw1 ... fx(0b/0i/2e/1g)`:
     the grenade spawns and blasts.
   - sounds: `3 passed; 0 failed`.
   So the escort's game-accurate LinkCreep port (Decrement 0x15a148 step-back, the SqDist2D<1.0
   passes, the MOVING-not-BLOCKED refusal — vs the old port's `return BLOCKED` that FROZE drones
   at the first boundary post their stand-in bounds graph rejects) is what changes 05's behaviour.
5. **The mechanism, pinned to the tick.** With `NF_MATRIX_DBG=1`: from matrix cell **0x06**
   onward every draw fails with the gun machine IDLE and the switch input swallowed
   (`held=05 req=05 sw=0 rel=0 fire=0` for all 60 draw ticks) — Bond is script-disabled, not
   machine-stuck; the runner's unstick/cold-reload machinery correctly does not fire. With
   `NF_SCRIPTLOG=1`: the **truck script `0x06000A22`** (player #37) fires at **tick 5412** —
   exactly the 0x05→0x06 cell boundary — stream 9, event `0x0D` params `[73, 1]`
   (`ch.set(73, 1)` = channel 0x49 [M Script_EventHandler 0x1d94c0; the port's own NF_SCRIPTLOG
   doc-comment names 06000a22 on 07000005]), then stream 3 event 0x0C at tick 5890. **No event 4
   (PlayerDisable) is logged anywhere** — the input death is downstream of ch 0x49 / the truck
   cut-scene path, not a logged script event. The intro script 06000047 (events 0x0f/0x0c at
   ticks 0/286) completed normally before it. Chain: the escort unfreezes a drone → it reaches the
   truck chain early (a guard walking, or the truck's own stream released by a touched trigger) →
   ch 0x49 → Bond's input dead for the rest of the sweep → 94 cells mark D → the matrix and sounds
   asserts fail. On master the same drones stay BLOCKED-frozen, the truck chain never fires, the
   tests pass — the tests were passing on top of the freeze bug the escort fixed.

## How it was verified

- Reproduction runs quoted above are from this tree, NF_ISO headless, the real input path
  (`matrix::run_level` → `Sim::step_bond`).
- The revert experiment ran the two weapon binaries directly (`--test-threads=1`), results quoted.
- `probe_escort_root` green after the fix; the other escort probes compiled and their inputs
  (contact skin, exit trigger) are present in the ISO.
- **The full four-crate suite did NOT run** — the window went to the causality experiments and the
  known ~36-minute suite would not fit before the 20:29 stop-work line (the same limitation the
  escort-01 and weapon-fx sessions hit: background jobs die between tool calls in this sandbox, so
  the suite needs foreground chunks). The next session must run it on the fixed tree.

## The fix to land (next session, in order)

1. Dump 05's script set (nf-extract or the level parse) and pin **who consumes ch 0x49** and what
   the truck script's full stream plan is; confirm whether the truck chain firing at tick 5412 is
   itself game-accurate (i.e., the game's guard/truck drone could reach it that early) or a second
   divergence.
2. Verify the port's `Route` field mapping against the game's `AINetwork_FollowRoute` 0x15b110
   gate (`(short)[0x40] < (ulong)[0x41] || (flags&7)==2 || f[0x46] <= f[0x44]` — index / links
   count / radius / remaining at 0x80/0x82/0x8c/0x88), and scope the creep block to that gate if
   the mapping shows the port creeps where the game returns state 3. The escort walk must stay
   green (walkreal_07000001_live, escort_01/0a, ai_live seed 2) while the two weapon tests come
   back.
3. Then the full `tools/test/suite.py` on the final merged tree (total in this report's place).

## Live checks for the main agent

1. **PCSX2, 07000005, the truck chain**: from the level start, fire 4 Wolfram PP7 shots at the
   start spot and wait ~90 s without moving. Expected (game): any guard reaction happens, but the
   truck script does not fire and the player never loses input. With gdbwatch on the truck
   script's release channel write (ch 0x49 set): no write within the window. If the game DOES run
   the truck script there, the port's divergence is not the walk but the ch-0x49 consumer.
2. **PCSX2, the guard at the boundary post**: walk a guard route into the first bounds post the
   stand-in graph rejects (the walkreal-a stall spot generalizes). Expected (game): the guard
   pushes/dithers through (LinkCreep MOVING), never freezes — this is the behaviour the escort
   port added and must keep.
3. **nf-view `--bond`** (after the fix lands): 07000005 start spot (the matrix's cell spot) — the
   sweep cells 0x06+ should draw/fire again; and 07000001's escort lead still walks onto the
   extraction plate (the escort work kept).

## What's left

- The game-side fix (the three steps above) — nothing landed for the two weapon tests; the escort
  work is intact and untouched; the probe fix is landed and green.
- The full suite on the final tree.
