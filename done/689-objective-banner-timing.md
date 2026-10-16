## Your task: `objective-banner-timing` — objective banners wait for cut-scenes

Playtests 1012-1723 and 1012-2337 (07000007): "The objective banner keeps playing during the cutscene. It needs to wait
until after the cutscene's finished", and "obj doesnt wait for kimiko cutscene and plays the sound immediately".
Port the game's gate:
- how `Text_AddMsg` / the objective-message queue hold while an NIS / script camera runs [M the message update's
  NIS check, `Mission_ObjectiveState`];
- when the "new objective" sound plays.

Tests on 07000007's Kimiko scene: the banner and its sound appear only after the scene hands back. Sister: outside a
cut-scene the banner shows immediately.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/objective-banner-timing`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T11:51Z

---
## Report (done, 2026-10-16T13:46Z)

# objective-banner-timing — the objective banner waits for the cut-scene

Task (playtests 1012-1723, 1012-2337, 07000007): "The objective banner keeps playing during the
cutscene. It needs to wait until after the cutscene's finished", and "obj doesnt wait for kimiko
cutscene and plays the sound immediately". Port the game's gate — the message update's NIS check
and `Mission_ObjectiveState` — and pin when the "new objective" sound plays, cold AND carry.

## What was found

**The gate itself was already on master** (fix-objective-banner, merged): `Text_UpdateMsg`
0x1b11d8:16-18 returns 0 while the viewer's +0x2a HUD gate is off, so `HUD_UpdateStatusPane`
0x19c310 case 0 can never open the objective pane mid-cut-scene; the port's `hud_frozen` mirrors
the gate (set/cleared by the script camera requests [M `Script_CameraStart` 0x1d9140:41-49 — all
seven viewers disabled, `Camera_Enable` 0x1b9920 writing +0x29/+0x2a — restored by
`Script_KillStream` 0x1d9860:71-72]) and all four HUD panes skip while it is set;
`Mission_Update` runs at the top of `Drones::tick` so the monitor's announce lands behind the
scene's camera gate [M `Game_Run` 0x1c9790:17 → `control_movement_object_handler` 0x1336f8]. All
13 prior banner/cutscene tests passed unchanged on the fresh master.

**The gap this task closed: the open chime's exact shape.** The game's case 2 (the pane open)
only SCHEDULES the pair [M 0x19c310:148-164: new objective → PlayMe1st 1532 / PlayMe2nd 469,
"OBJECTIVE COMPLETE" → PlayMe1st 468 with Triggered2 preset]; **case 3 — the NEXT pane frame —
fires PlayMe1st, and only while `ScriptCam == 0`** [M 0x19c310:219-229]. The port played the chime
at open with no camera check, so a cut-scene starting the tick after a banner's open chimed during
the scene where the game holds it (the same +0x2a gate freezes the head timer [M 0x1b11d8:16-18],
so the banner stays inside its flash window — remaining + 60 > life — until the hand-back, and the
chime fires then). Ported: `Msg::sfx1` (new field), case 2 = open only; case 3 = the chime on the
first unfrozen pane frame inside the flash window; 469 rides the steady branch after the 1st
[M 0x19c310:232-235 — no camera check there; the frozen timer cannot newly reach it mid-scene].

The probes also pinned the load shape (superseded by the tests): a cold 07000007 has NO intro
cut-scene (hud_frozen 0 ticks), so the load announce ("Rendezvous…") shows at once — the game's
own shape, since no camera runs; the 06 → 07 carry announces a carried COMPLETE at t=0 (468) and
queues "Rendezvous…" at t=1 — the monitor's state-3 arm on the carried record [M 0x1827d8 case 3],
again with no camera. `Mission_ObjectiveState` 0x182630 is the objective-state enumerator for the
pause/HUD list (states 2/3/4 → 0x2000001/0x2000000/0x16f) — not a second gate; the banner's
header ids come from case 2's strcmp [M 0x19c310:134-147].

## Verification — `crates/nf-view/tests/objective_banner_timing_live.rs` (3 tests, green)

- **the sister** — `outside_a_cutscene_the_banner_shows_and_sounds_at_once`: with no camera
  anywhere (hud_frozen false throughout), the level's own announce opens the tick it queues and
  the SFX play in the case-3 shape: 1532 one pane frame after the open, 469 ~60 ticks later.
- **the carry** — `the_carry_holds_the_kimiko_announce_until_the_handback`: the 06 → 07 carry with
  a finished-06 mission snapshot (`RamSave::with_mission` — the game persists the mission globals
  across the load [M LS_MakeMission/LS_LoadMission 0x1cd720/0x1cd880]), then the real trigger
  (TouchOnce #412 → ch 0x48 → SP #420, the Kimiko scene): no banner SFX and no banner display
  from the trigger to the hand-back (+4326 ticks); at the hand-back the COMPLETE announce opens
  (468 at +0) and the "Spy on secret meeting" announce opens when the pane frees (+177), with
  1532@178 and 469@237 — the full playtest flow.
- **the flash-window hold** — `a_scene_starting_in_the_flash_window_holds_the_open_chime`: a
  banner opened at t=0 with the scene from t=1 keeps the pane open, holds the chime through the
  whole 4325-tick scene (no 1532/469/468 while frozen) and fires it at the hand-back (+0) — the
  game's held Triggered1 [M 0x19c310:219-229].

The prior banner/cutscene tests all still pass (16 tests across fix_objective_banner_live,
fix_objective_messages_live, fix_cutscenes2_live, objective_banner_timing_live — green in the
suite run below too).

## What I ran

- `cargo test --offline --release -p nf-view --test objective_banner_timing_live` (+ the three
  prior banner/cutscene binaries): 16 passed.
- `cargo test --offline --release -p nf-game --lib -- --test-threads=1`: **237 passed** (nf-game
  is the crate the pane machinery reads; no nf-game source changed this task).
- The full four-crate suite (the `tools/test/suite.py` logic) on the final tree (this branch =
  origin/master e416e5f0 + this task's diff): **this sandbox kills background jobs between tool
  calls**, so the suite ran in resumable foreground chunks with suite.py's exact per-binary logic.
  In the window available it completed **278 of 408 binaries — 955 passed, 3 failed, 55 ignored**;
  the 3 failures are the known GPU-less trio (`probe_pylon_render`,
  `fix_0c_pylon_live::the_0c_pylon_fills_the_view_from_the_closeup_spot`,
  `fix_0c_pylon_live::the_0c_pylon_is_on_screen_at_the_user_spot` — wgpu finds no adapter in this
  sandbox), the same set the prior full-tree run recorded. The 130 binaries not reached are the
  tail of cargo's order (the heavy nf-view level binaries and the nf-game integration binary);
  my diff touches only nf-view's `scene.rs`/`hud` paths, whose binaries all ran green.

## What's left

- The suite tail above (130 binaries) for completeness — nothing in it touches this task's paths;
  the next sandbox can resume the same chunked runner from the branch.
- The render-side look: the tests pin the SIM state and the draw-side skip (`hud.rs` checks
  `hud_frozen` on every pane), but the on-screen letterbox + banner timing during the Kimiko
  scene is the main agent's render check (below).

## Live checks for the main agent

- `--bond 94.2,43.7,-45.9,<into the room>,0` walking into TouchOnce #412 on 07000007 (from the
  06 → 07 carry): during the Kimiko scene NO objective banner and no letterbox-area banner text;
  the letterbox bars are up; at the hand-back the "OBJECTIVE COMPLETE / Rendezvous with
  undercover…" banner appears first, then ~180 ticks later "NEW OBJECTIVE / Spy on secret
  meeting" — each with its flash blink in the first 60 ticks.
- Same scene, sound on: the completion chime (468) and the new-objective chime (1532, then 469)
  only AFTER the scene hands back — never over the scene's voice lines.
- A cold 07000007 load (no cut-scene): the "Rendezvous…" banner shows within a second of the
  level appearing, with its chime — the banner must NOT be delayed outside cut-scenes.
