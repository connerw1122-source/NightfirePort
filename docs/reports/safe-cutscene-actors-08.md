# safe-cutscene-actors-08 — 07000008's cut-scene shows its actors

Task 702. The playtest (1013-1758, 07000008): "Kiko is the only one showing in this cutscene
besides me... I can't see any other models. Any character models."

## The 08 station intro's actors (the script census)

The intro cut-scene is **0x06000077** (pack 07800008.bin: camera script, 3880 ticks, 30 streams)
[D the pack's stream decode], started the real way: spawn #0 (139.5, 50.5, −76.4) → TouchOnce
**#237** (139.2, 49.5, −81.2, out 0x49) → SP #294 (mask 1 on ch 0x49) [D docs/levels/07000008.md].
Its `Script_AnimStart` actors — (stream, skin, anim script), every skin resolved from
07000008.bin's own archive [D bin-list] and pre-built by the port's per-`Cmd::Anim` character pass
(scene.rs:5328-5347):

| stream | skin | anim script | who (port-side) |
|---|---|---|---|
| 1 | 0x05000014 | 0x0600020a | intro cast |
| 2 | 0x05000013 | 0x06000209 | intro cast |
| 3 | 0x0500001b | 0x0600020c | intro cast |
| 4 | 0x0500000d | 0x06000207 | the tuxedo Bond double |
| 5 | 0x05000016 | 0x0600020b | intro cast |
| 6 | 0x0500003b | 0x060003ae | **Kiko** (the same skin as 07's Kimiko) |
| 7 | 0x05000012 | 0x06000208 | intro cast (stream also starts SubScript 0x06000052) |
| 8 | 0x05000026 | 0x06000206 | intro cast (also SubScript 0x06000052) |
| 9 | 0x05000018 | 0x060003af | intro cast |

No `Cmd::Event` 0xB weapon attaches in the intro's streams (nothing is held) [D the decode]. The
level's later scenes keep their own actors: 0x06000078 s8 (0x05000026/060001fa, the
bottom-of-mountain cinematic), 0x06000051 s1 (0x0500000d/06000992, the helicopter parts
assembly) — covered by the existing 10-scene drive (`cutscene_models_live`).

## The verdict: the current tree shows all nine

Every actor of the intro, driven through the REAL plate start (`nf_view::load` +
`Sim::step_bond` onto #237), at scene ticks ≈119/209/329/569/1049:

- **created** — `script_anim((0x06000077, st), skin)` resolves a live `Obj::Anim` for all nine at
  every checkpoint;
- **bound** — the live object's anim script equals the pack's stream decode (asserted), and each
  has its pre-built character with a skin from `sim.skins`;
- **posed** — `script_pose_together` returns a 73-bone finite pose whose **root sits at the body
  track's library mark** (e.g. stream 6 Kiko at (139.47, 53.79, −80.41), stream 4 the Bond double
  walking (141.21→141.46, 49.78, −87.34→−87.61)), never the world origin;
- **drawn** — the render-path palette (`skin_palette` fed by the script camera's eye cell +
  portal walk) is real for all nine at +30/+240/+960: none FAR, none collapsed.

## The regression (the bisect over master history)

The user's build had **fix-cutscenes2's d907aba1 (Oct 13 09:07)**: face tracks playing beside the
body with **first-writer-wins per bone** — the intro's narrow face-track windows won the shared
pose's low bones and their leading zero channels collapsed **every script actor's root to the
world origin**, off-camera in every shot, on 07000007 and 07000008 (mechanism and pre-fix repro:
docs/reports/fix-cutscene-draw.md). Kiko — a placed NPC drawn through the drone path, not a
script actor — was the only character left visible: exactly the playtest. **fix-cutscene-draw
7f5414aa (Oct 14 04:53)** gave the base (widest) track the bones it covers; the playtest build
predates that fix. The later **cutscene-faces** merge (Oct 16, in de6110cc) does not re-break the
scene — the tests below run green on the final tree. The pre-fix collapse was NOT re-run on a
pre-fix worktree in this sandbox (no time before the window closed); the draw report carries the
pre-fix repro for 07 and names 08.

## The tests

`crates/nf-view/tests/cutscene_08_actors_live.rs` (new, opt-in via NF_ISO, 2 tests, both green):
1. `the_station_intro_actors_live_and_posed_through_the_plate` — the real plate start, then every
   one of the nine (stream, skin, anim script) triples: live `Obj::Anim` with the pack's anim
   script, a pre-built skin-bound character, a finite non-zeroed pose, and the root NOT at the
   origin (the d907aba1 signature) at five checkpoints across the first 1049 ticks.
2. `the_station_intro_actors_survive_the_render_path` — the draw loop's palette chain
   (script camera → `eye_cell` → portal visibility → `skin_palette`) leaves all nine real, none
   FAR/room-hidden, none collapsed, at three checkpoints.

## What was run

- The full four-crate suite on the final tree (this branch = origin/master de6110cc + WORKLOG +
  this test), `tools/test/suite.py` driven in six foreground `--only` chunks (this sandbox kills
  background processes between tool calls — the nohup pattern from docs/tools/test-suite.md is
  unusable here; the binaries, `--test-threads=1` runner and 600 s per-binary limit are the
  runner's own):
  **1877 passed, 6 failed, 72 ignored** — the 6 are the pre-existing failures (the 3 allowed
  `disc_golden` on the slim ISO + the 3 pylon GPU probes `probe_render_pylon_only`,
  `fix_0c_pylon_live::the_0c_pylon_fills_the_view_from_the_closeup_spot`,
  `fix_0c_pylon_live::the_0c_pylon_is_on_screen_at_the_user_spot`), same set as the previous
  tasks' final trees (inert-guards-02: 1835/6; death-loop-07: 1838/6).
- `cargo test --offline --release -p nf-view --test cutscene_08_actors_live` (the new tests) and
  `--test cutscene_models_live` (the existing cut-scene net, 4/4 green) before the push.
- No library code changed; no crate outside nf-view's tests touched.

## Live checks for the main agent

Nothing is blocked on them (the headless palette census covers the draw path), but a rendered
frame would settle the user-visible truth:
- `--bond 139.2,49.5,-81.2` on 07000008, let the intro play to ~tick 120: the station interior
  should show the cast (the Bond double by the plate, Kiko in the group) — not an empty room with
  only Kiko.

## What's left

Nothing for this task's ask. If the user still sees missing actors on a build **after**
de6110cc, the next suspect is the GPU draw only (lavapipe `--shot` renders on the main agent's
side; the headless palette chain here is clean).
