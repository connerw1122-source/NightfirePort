# fix-trigger-unknowns2 — the held 110f sight pause vs the live 61 (and the escort it broke)

Branch `s/fix-trigger-unknowns2` (110f's `s/fix-trigger-unknowns` @ 29108414 merged with origin/master). Finish ID `110f2-fix-trigger-unknowns2`.

## What was wrong

110f ported the 15-frame sight pause of `DroneVision_ProcessDroneSight` 0x176a28:68-69 as a
`Drones.sight_pause_until = 15` stamp at construction ("the fb8 load stamp") plus a re-stamp at every
`DroneEnable(1)` ("the fbc stamp"). On master+110f that broke two live-matched tests:

- `nf-game/tests/ai_live.rs::guard_notices_and_fires_like_the_game` — noticed after **Some(64)** ticks (live 61).
- `nf-view/tests/walkreal_07000001_live.rs::walk_from_the_spawn_to_the_exit_with_the_contact` (line 218) — the escort
  contact no longer reached the LoadPointActivator plate.

## Root cause: the load stamp is consumed before the first AI tick

1. **The gate's `now` is the boot-global frame counter, not a per-level one.** The disasm at the gate reads
   `0x2a379c` — `GameState+0x34` [M ACTION.ELF 0x176a7c: `lui v1,0x002a` + `lw v1,14236(v1)`; the ELF symtab pins
   `GameState` = 0x2a3768/88 bytes]. `tools/trace/ai_trace.py` samples the exact same word as its "once per in-game
   frame" clock (line 92: `fr = p.read32(GAMESTATE + 0x34)`), which is how the tool time-stamps every trace row.
2. **`DAT_00298fb8`/`DAT_00298fbc` are NPCGlobals fields** (+0x7a8/+0x7ac; NPCGlobals = 0x298e10, 7440 bytes per the
   ELF symtab). Writers, whole-decompile grep [M]: fb8 is written ONLY by `Drone_PostLoad_Init` 0x136938:46
   (`DAT_00298fb8 = GameState+0x34`), fbc ONLY by `Drone_EnableAll` 0x1383e8:25 (the `param_1 != 0` arm; the sole
   gameplay caller is `Script_Free` 0x1da880:41-43 under the `+0xaa8 & 0x80` flag). Readers: the 0x176a28 gate, plus
   the 30-frame speech mute (`NDrone2_PlaySFX` 0x141848:74, `NDrone2_SpeechSFX` 0x141b00:23).
3. **The stamps hold boot-scale values, and the counter is already past them at the snapshot.** [L]
   `work/ee_start_05.bin` (file offset = EE address): fb8 = **108150**, the counter at 0x2a379c = **108153** (fb8 − 3),
   fbc = **0** (EnableAll(1) had never run — BSS init). Same shape in ee_start_07000001 (fb8 = 139550),
   ee_start_0700000a (139701), ee_snapshot (26905) — each a `PostLoad_Init` stamp taken with the boot-global counter
   during the load sequence. A per-level counter would have made the gate `108165 <= 0..15` — sight closed for ~30
   minutes — which is absurd; the counter is global and the stamps expire 15 frames after `PostLoad_Init`, i.e.
   **during the loading/fade frames, before the first AI tick**.
4. **The live 61 is an ungated measurement, and that is the only reading consistent with the decompile.**
   `ai_trace.py` attaches to a RUNNING game (the operator executes it against live PCSX2 after the level is up),
   teleports Bond once, and records [M tools/trace/ai_trace.py:61-92; WORKLOG 0019: "noticed after 61 ticks"].
   At trace time the counter is tens of thousands of frames past fb8+15, so the game's sight chain ran UNGATED in the
   measured window: reaction 59 [M `DroneFunc_ReactionTime` chopped arithmetic, mirrored in drone.rs] + the round-robin
   line-of-sight turn landing 2 ticks after the teleport = 61. A gameplay-side 15-frame pause would push the same
   chain to 64 — exactly what the broken port measured, and exactly what the port's construction stamp produced:
   with the pause active the guard's first LOS check moved from frame 12 to frame 16 (logged empirically), shifting
   `seen_ticks` accumulation +4 and the ATTACK log entry 61 → 64.
5. **The escort break was the same stamp.** walkreal_07000001 runs Bond at his spawn from tick 1; gating frames 1-15
   delayed every guard's sight/alert chain and the run wedged the contact's lead 3.9+6 units short of the plate
   (line 218 assert). With the construction stamp removed the walk completes as on master (re-verified below).

## The fix (crates/nf-game/src/drone.rs, one field)

- `Drones.sight_pause_until` construction value **15 → 0**: the port's frame 0 is the post-load state, the state the
  game reaches only after the load stamp has expired; the fb8 stamp therefore never consumes a simulated tick.
- The **`DroneEnable(1)` re-stamp is kept** (`sight_pause_until = frame + 15` in the DroneEnable handler): that is the
  fbc stamp, which really does land in gameplay (`Script_Free`'s mid-level re-enable) and really does mute sight for
  15 ticks — the pause stays in the port where the game applies it.
- Field doc rewritten with the evidence chain; the unit sister renamed
  `the_load_sight_pause_never_consumes_gameplay_ticks` (asserts 0 at construction; would fail on the old tree).
- Hearing remains ungated, `FindAlertedDrones` remains inside the gate, all per [M] 0x176a28:68-70 vs 0x139520:172+ —
  unchanged from 110f.

## Reproduce-before-fix / verification

- The failing-first test is the driver's own symptom: `ai_live::guard_notices_and_fires_like_the_game` printed
  `noticed after Some(64) ticks (live 61)` on the merged 110f tree (reproduced at 12:20 UTC), and
  `walkreal_07000001_live` failed line 218 on master+110f (driver's suite). Both are NF_ISO tests through the real
  input path (`nf_formats` load + `Drones::tick`, and `nf_view::load` + the walker).
- After the fix: **ai_live 35/35** (guard notice back in the live 59-63 band with the seed's 61, the level-start
  snapshot test, the patrol test, the sister), **walkreal_07000001_live 2/2** (the contact settles 3.87 from the
  plate, the `< 6.0` soft assert green, the sister holds).

## Full suite on the final tree (merged with origin/master 0e73a820's master = 96405b35)

**1294 passed, 7 failed, 66 ignored** — 329/329 test binaries, run per-binary with `--test-threads=1`
(resumable foreground chunks — this sandbox reaps background jobs; the runner mirrors suite.py's isolation).

The 7 failures:
- 3 × `disc_golden` — the allowed slim-ISO set.
- `probe_pylon_render` (1) + `fix_0c_pylon_live` (2) — wgpu `adapter: NotFound`, no GPU in the sandbox (documented
  pre-existing on master by earlier sessions).
- `fix_death_restart_live` (1) — **flagged, not resolved in this window**: the binary's first 5 tests pass; the 6th
  (`restart_of_a_cold_loaded_level_restores_the_post_load_entry_checkpoint`) stalls (no result line, killed at the
  wall-clock cap); inside the chunk runner the binary died twice with no failing test (SIGKILL-class — the same binary
  was OOM-killed in a previous session's notes on this 3 GB sandbox). It passed on the driver's master+110f run and in
  fix-death-retry-boost's final-tree suite, so the stall is new with THIS session's tree — either this task's stamp
  change or master's newest death-path merges (fix-death-anim-events / fix-death-retry-boost landed in the same
  merge). **The next session must bisect those two candidates first** (`git stash` the `sight_pause_until: 0`
  construction value, re-run the binary; then master alone).

## Shared files

- `crates/nf-game/src/drone.rs` — the task's named file; the one-field fix + doc + unit sister.
- `docs/WORKLOG.md` — appended entries only (two merge-conflict resolutions, both kept append-only).
- `docs/UNKNOWNS.md` — U-032's sight-pause paragraph corrected (the load-stamp timeline), no other rows touched.
- No other shared file touched; master's merges during the session (start-loadouts, death-anim-events,
  progress-remainder, death-retry-boost, shells) came in via `git merge origin/master` before each push.

## Live checks for the main agent (PCSX2)

1. **fb8 vs the counter across a load** (gdbwatch on 0x298fb8 + 0x2a379c, write/read watchpoints): expected — fb8
   written once during the load sequence, then ≥ 15 counter ticks elapse before the first `ProcessDroneSight` body
   execution; `ProcessDroneSight` 0x176a28's gate branch should be TAKEN (sight runs) on the first AI tick after the
   fade.
2. **fbc re-stamp** (fieldprobe, 07000001's intro free): at `Script_Free` of script 0x06000087, fbc should jump to the
   current counter and the guard sight columns in a trace should stay flat for the next 15 frames, then resume.
3. **The 61 re-measure**: re-run the WORKLOG 0019 teleport trace and confirm notice ≈ 61 (it must not be 64 — if it
   ever measures 64 on real hardware, this report's conclusion is wrong and the construction stamp should come back).

## nf-view render spots (no visual change expected — the fix is timing-only)

- `--bond 104.95,24.96,108.83,-2.15,0` (07000005 cold start): the courtyard guards idle/patrol as before; nothing
  visible changes in the first 15 frames (that is the point — the old stamp delayed their sight unseen).
- `--bond 67.31,26.5,99.47,-2.0,0`: the x=61.34 guard raises and fires from ~61 ticks as on master.

## What's left

- Nothing on this task's two tests; both pass with the pause still in place for the fbc (re-enable) path.
- U-032's msg-0x1B sender remains open (unchanged from 110f; no evidence found this session — ACTION.ELF still has no
  literal 0x1B sender).
- `fix_death_restart_live`'s in-runner flake deserves a look on a machine with more RAM (solo runs green).

## ntfy / finish notes

- finish.sh first run used a repo-relative report path (the script `cd`s to the queue worktree before `cat`), so the
  report body was missing from the queue entry; the idempotent re-run with the absolute path attached it ("OK already
  closed in done/, report attached").
- ntfy: the direct curl got `{"code":42908,"http":429,"error":"limit reached: daily message quota reached"}`; the
  python-urllib fallback delivered the notification ("sent").
