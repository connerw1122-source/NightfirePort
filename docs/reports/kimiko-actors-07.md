# kimiko-actors-07 — report

Branch `s/kimiko-actors-07` (from origin/master a4156da0, merged origin/master de6110cc).
Playtest 1012-2337: "models in that cutscene arent accurate" (the 07000007 Kimiko cut-scene;
earlier reports from the same scene: duplicates in the Drake scene, models present before they
should be).

## The actor inventory, from the script data [D]

Both 07000007 cut-scenes decode from their packs (`nf-extract nis 07000007.bin <hash>`; the
format is docs/formats/nis-script.md). Every `Script_AnimStart` (0x1d8f40) spawns at the stream's
clock 0 and every stream ends (kill) at the scene's length — neither script sets keep-objects.

**The Kimiko scene 0x06000073** (pack 07800007.bin, length 4326, camera flag set, 15 streams;
SP #420 "Script_Trigger" via TouchOnce #412 → ch 0x48):

| stream | actor | skin | anim script | spawn | despawn |
|---|---|---|---|---|---|
| 1 | script actor (the leading lady) | 0x0500003b | 0x060004f1 | 0 | 4326 |
| 2 | script actor (the fireplace woman's double) | 0x05000015 | 0x060004f2 | 0 | 4326 |
| 3 | script actor (the tuxedo Bond double) | 0x0500000d | 0x06000225 | 0 | 4326 |
| 14 | keyed entity 0x02000acb, type 0x31 (plain object), keys 499–504 | — | — | 0 | EntityEnd 4326 |
| 0 | camera (492 keys) | — | — | 0 | 4326 |

Events: 5 (`Drone_EnableAll(0)`) + 4 (`Player_Disable`) at tick 0; 6 (`Player_Enable`) + 7 at
4326. No keep-objects: `Script_Free` deletes all four objects.

**The Drake fly-through 0x06000072** (pack 07900007.bin, length 3000, camera flag set, 37
streams; SP #419 "NIS_Trigger" via TouchOnce #356 → ch 0x15): 13 `Script_AnimStart` guests, all
spawn 0 / despawn 3000 — stream 1 skin 0x05000016 on 0x060001fd (Drake), 2: 0x05000015/0x060003b2,
3: 0x0500000d/0x060001fe (the tuxedo Bond double), 4: 0x05000022/0x060003b3, 5: 0x05000019/0x060001ff,
6: 0x05000020/0x060003b4, 7: 0x05000021/0x06000200, 8: 0x05000024/0x060003b5, 9: 0x05000023/0x060003b6,
10: 0x05000019/0x06000201, 11: 0x05000019/0x06000202, 12: 0x05000022/0x060003b7, 13: 0x05000020/0x060003b8
— plus keyed entity 0x020004d0 (stream 14, type 0x19, 620 keys) and the op-0x14 sprites. Same
event shape: 5 + 4 at 0, 6/7 at 2999/3000.

All nine distinct skins resolve in 07000007's own archive [D bin-list]; every one builds a
character (asserted by fix_cutscene_actors_live and re-asserted here).

## Script actor or level drone reused? [M]

The game separates them with two mechanisms, both in the decompile:

1. **The freeze hide**: event 5 → `Drone_EnableAll(0)` 0x1383e8 → `NDrone2_Enable(0)` 0x149dd0:8-15
   sets `obj+0xF0 |= 0x10` on every drone's object — the bit `View_AddObjects` 0x1e62d8:33
   excludes from the draw list — plus `NPCGlobals = 0`. Event 4 → `Player_Disable` 0x195f70:17
   hides the player's body and view weapon the same way; `Player_Enable` 0x195498:23 restores.
   So during a scene ONLY the script's own actors draw: no duplicates, and the level drones are
   "not present" for the scene's duration.
2. **The second switch** (`NDrone2_PreDroneControl` 0x148db0:49-78): drones whose second channel
   (placement param 6) is the scene's trigger channel are cleaned up when the freeze lifts —
   tag 100 deletes them. On 07000007: Drake's drone (skin 0x05000016, second ch 0x15) stands
   enabled at [126.82, 47.0, −94.33] — out of sight from the start — and DELETES when the
   fly-through's freeze lifts; the fireplace Kimiko drone (skin 0x05000015 at [92.06, 43.75,
   −31.65], second ch 0x48) parks `WaitSwitch` (0x01) DISABLED — she never visibly stands; her
   scene double (the same skin, stream 2) replaces her, and she deletes at the free.

So "a level drone drawn instead of the script actor" cannot happen while the freeze-hide is
ported: every drone is frozen out of the draw for the scene's whole length.

## The port verdict

**The binding on the current master is faithful; no fix was needed.** The probes drove both
scenes through the real path (`step_bond` → TouchOnce → ch → SP → the script) and read the draw
sets through the render's own gate: exactly the scripts' actors visible, once each, correct
skins, drones frozen out, the doubles dead at the scenes' ends, Drake's and the fireplace
Kimiko's drones deleted by their second switches, Bond at the hand-back entity [91.52, 43.85,
−31.09], and the walks' root endpoints matching the measured [D] values. The timeline explains
the playtest: the freeze-hide landed on master 10-11 17:10 UTC (fix-core), the playtest is
10-12 23:37, and fix-cutscene-actors (the inventory task) merged 10-14 — the complaint is
explained by a pre-fix-core build. What the task adds is the render-side gate as a readable API
plus the per-phase pinning tests.

## The delta this task

- `crates/nf-view/src/scene.rs`: the `frame_state` hidden-closure hoisted into
  `pub fn Scene::char_hidden(ci, opts)` (one source of truth; the render path and the tests read
  the same gate). No behaviour change (the closure's logic is byte-identical; the inspect path
  keeps its script-only gating).
- `crates/nf-view/tests/kimiko_actors_live.rs` (2 tests, green):
  - `the_kimiko_scene_draw_set_matches_the_script_per_phase` — pre-scene (the fireplace Kimiko
    parked disabled on ch 0x48, Drake standing enabled but not pictured from the start, nothing
    script-made drawn), mid-scene (the 3 doubles once each, every level drone frozen out, the
    no-duplicates invariant visible-count == live-object-count per skin, the [D] root endpoints
    91.97,−31.67 / 94.13,−35.76 within 0.15), post-skip (the doubles deleted, the freeze lifted,
    the ch-0x48 drone deleted, Bond at the hand-back spot).
  - `the_drake_flythrough_draw_set_matches_the_script_with_no_duplicates` — the 13 live actors
    pinned stream-for-stream against the [D] decode, no duplicates, the Bond double at its
    [D] terrace root, and at the end the doubles dead + Drake's drone deleted.

## How it was verified

- The inventory cross-checked twice: an independent Python decode of the pack bytes and the
  runtime's own `Cmd` decode (nf-formats), agreeing stream-for-stream.
- Tests run before push (on the merged tree): kimiko_actors_live 2/2, fix_cutscene_actors_live
  5/5, fix_cutscenes_live 4/4, fix_cutscenes2_live 5/5, fix_kimiko_handback_live 1/1,
  models_characters_live 6/6, fix_party_mission_live 9/9, scenario_live 6/6 — 38 green across
  the 8 draw-path binaries.
- Full fast-runner suite, pre-change tree: `1864 passed, 6 failed, 72 ignored (50 min)` — the 3
  allowed disc_golden + the 3 pre-existing pylon render probes. The final-tree fast-runner suite
  (after the merge) was started 16:41 UTC; its total line is appended below.

## What's left

- The final-tree suite total (running; appended below when done).
- Nothing else in the port: the binding is pinned. If the user still sees the symptom on the
  current build, the live renders below decide where.

## Live checks for the main agent

1. **nf-view render, Kimiko scene mid-play**: `--bond 90.5,44.3,-33.5,1.2,0` ~10 s after the
   trigger — exactly three figures (the tuxedo Bond, the two women), no duplicates, no frozen
   party guests or hall guards in the picture, no white unposed characters.
2. **nf-view render, fly-through mid-play**: `--bond 34.25,44.13,-70.71,1.549,-0.070` — the 13
   guests once each on the terrace, Drake among them, no placed-drone copies.
3. **nf-view render, pre-scene**: `--bond 33.4,44.5,-70.6,0,0` — no Drake on the far terrace, no
   woman at the fireplace.
4. **PCSX2 gdbwatch** (only if the renders disagree): the fireplace Kimiko drone's obj+0xF0 at
   the scene's start — expect bit 0x10 set within one tick of event 5 [M 0x149dd0:10-12].

## Final-tree suite total (appended at close-out)

The final-tree fast-runner suite (the merged tree with this task's draw-gate refactor and the
pinning tests, started 16:41 UTC): `1877 passed, 6 failed, 72 ignored (40 min)` — the 6 are the
3 allowed disc_golden + the 3 pre-existing pylon render probes. No regressions.
