# fix-cutscene-models — the cut-scene actor regression

Branch `s/fix-cutscene-models` (from `origin/master` 8144bd6). Session 01:16–02:55 UTC 2026-10-14.

## The headline

**The regression does not reproduce headlessly.** Every cut-scene on 07000005/06/07/08 draws all
of its actors on today's master (8144bd6) **and** on the playtest-era master (3c77a9d, run in a
worktree) — through the sim, the palette and the room-mask paths, with the game's own SP start
mechanism. The draw guarantees are now pinned by tests that would have caught the reported
symptom in any of the covered paths. What could NOT be covered in this sandbox: the actual GPU
skinning draw and the playtest launcher's exact build — see "Live checks" (re-running the
playtest on the current master is the cheapest next step; three merges landed after it).

## What was built (`crates/nf-view/tests/cutscene_models_live.rs`, new)

The draw path for a script actor (scene.rs `frame_state` + `skin_palette`): the character draws
only while (a) `Sim::script_anim(src, skin_hash)` resolves a live `Obj::Anim` (else
`frame_state`'s `hidden` collapses it), (b) its pose (`script_pose_together` → `world_pose` →
`skin_matrices`) is finite, and (c) the palette's `vis_room` gate (the portal walk's strict room
mask from the draw camera) does not move it to FAR (y −10000). Four tests:

1. `probe_kimiko_scene_actor_census` / `probe_flythrough_actor_census` — the two known 07000007
   scenes through their real triggers (TouchOnce #412 → 0x06000073, #356 → 0x06000072 [D
   fix_cutscenes2_live]); per-actor: live `Obj::Anim` + finite pose. Master: Kimiko drawn 3 /
   hidden 0; fly-through drawn 13 / hidden 0 (the 13 "hidden" in each census are the *other*
   scene's actors — its SP is not running — which is correct).
2. `every_scene_on_05_06_07_08_draws_its_actors` — EVERY scene hash the level pre-builds actors
   for, started the game's own way: the SP's `out` channel set on the sim's switch channels plus
   the SP's activation flag (`SP_Update` 0x1dbda8 state 0: `flags = out-channel | 4`, `trig` on
   `mask`), then censused at three checkpoints (90/210/390 ticks):
   - 07000005: 060008c8, 06000952, 06000994, 06000a22
   - 07000006: 06000071
   - 07000007: 06000072, 06000073
   - 07000008: 06000077, 06000078, 06000991 (the briefcase/safe scene pair + the death NIS)
   All 10 scenes: every actor live and finite on both trees.
3. `probe_render_path_census_07` — the render chain headless (scene::skin_layout → eye_cell →
   `portal_visible_cels_from_cell`'s strict room mask → `skin_palette`), carrying ONE persistent
   `ViewerCel` across the whole scene exactly like the live render loop (per-frame `eye_cell`
   from the script camera). The script camera holds at (94.3, 44.4, −35.6), eye cell 6, the
   palette real 3 / far 0 / collapsed 13 (the fly-through's, correctly off). The fly-through's
   own camera (95.4, 51.3, −84.1, eye cell 20): real 13 / far 0.

## The bisect evidence

- `git log --merges` scoped to the candidates; each read:
  - **fix-gpu-skinning (942e2ca)**: the merged diff touches only `lib.rs` (+26: `lerp_yaw` +
    `ticks_for_frame` extraction, verbatim) and a new test. No shader/palette/gpu.rs change —
    the G-043 palette path predates it. Behaviour-preserving.
  - **fix-anim-events (dd1cd7b)**: drone weapon events (`DroneAnim_EventFunc`, weapon swaps) —
    merged ~30 min AFTER the playtest; not actor creation.
  - **fix-script-commands (0a7268d)**, **fix-facial/facial2 (366ceff/bc231e8)**,
    **fix-anim-blend2 (e679d6e)**, **fix-portal6 (8ee0791)**, **fix-spawners/-remainder**,
    **fix-cover/nav-remainder**: all predate the playtest; the paths they changed (script
    decode, face tracks, blend timing, the room mask, spawner hold-back) are exactly what the
    census covers — green on both trees.
- The playtest-era master (3c77a9d) ran the full census in a worktree (`git worktree add`,
  shared target dir): identical results, all scenes draw.

## What this means

The reported symptom (only Bond draws in cut-scenes) is not reachable through the sim, the
palette or the room-mask logic on either tree. Two candidates remain, both outside this
sandbox's reach:

1. **The GPU skinning draw** — the palette upload (`pal_buf`) and the shader consuming it. The
   CPU palette content is verified correct; a wrong `a_skin` index or a bad palette binding
   would blank exactly the skinned characters while the world draws. No Vulkan ICD here, so no
   `--shot`/render test was possible (the attempt is not even reachable: `device()` fails
   headless per docs/systems/levels.md §6).
2. **The playtest launcher's build** — the results (1013-1758) predate three merges
   (fix-anim-events 00:30 UTC, the merge2 syncs, 8144bd6 01:03 UTC). If the launcher was older
   than 3c77a9d, the tree it tested differs from both trees verified here.

## Live checks for the main agent

1. **Re-run the playtest on the current master first** (the launcher build may simply be stale).
   Spot: 07000007 trigger #412 (the Kimiko scene) and #356 (the fly-through); 07000008's
   briefcase/safe scene. Expected on this branch: Bond + Kiko + the guests + the safe scene's
   cast all draw.
2. **`--shot` renders** (needs a GPU or lavapipe): `--bond` at the Kimiko scene's trigger
   (94.2, 43.7, −45.9 on 07000007) mid-scene — Kiko beside Bond; the fly-through camera
   (95.4, 51.3, −84.1) — the 13 guests on the party floor below.
3. **If the actors are still missing on current master**: the GPU path is the remaining suspect
   — gdbwatch/fieldprobe the pal_buf upload against the CPU palette (this branch's
   `cutscene_models_live` prints the expected per-character palette: `char_base[ci]`..`+bones`,
   FAR = rows all `[[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,-10000,0,1]]`). Check `a_skin` in the
   character vertices for a script character vs its `char_base`.
4. **PCSX2 ground truth** for the 08 safe scene: which actors does the real game show? (the
   port's census needs the expected cast list for 06000077/078 to assert counts, not liveness.)

## What's left

- For **291-fix-cutscene-actors** (positions/skins in general): untouched here — the census
  found no skin/position gap in the covered scenes; 291 owns the broader audit.
- The GPU-path verification (live check 3) if the playtest still reproduces on current master.
- The full suite could not complete in this sandbox (background runs are reaped within minutes;
  the foreground window fits only per-binary runs). Run per-binary before merge:
  `cutscene_models_live` (4 tests, ~20 s), `fix_cutscenes_live`, `fix_cutscenes2_live`,
  `fps_determinism_live` all green on this branch; the fast runner (`tools/test/suite.py`) on
  the merged tree is the pre-merge gate as usual.

## ntfy

Delivered via curl at 02:28 UTC (id dwcfoKId8Nyd): "done — the cut-scene actor draw census
(10 scenes, 4 levels) does not reproduce the regression on master or the playtest build; draw
guarantees pinned by tests; live checks filed". finish.sh attached this report to the already-
closed queue entry ("OK already closed in done/, report attached").
