# fix-objective-banner — the objective banner's timing, text and the fail message

Branch `s/fix-objective-banner` (note: the driver message's final line said `s/objective-banner`; the
task header, the push instructions and the finish id all say `s/fix-objective-banner`, so that is the
branch). Playtest items (dist-side results, 2026-10-12), all on 07000007:

1. "obj doesnt wait for kimiko cutscene and plays the sound immediately" — the banner and its SFX
   fired the moment the Kimiko cut-scene started instead of after it ended;
2. banners said a generic "NEW OBJECTIVE" instead of the objective's own text;
3. the fail message was off-centre.

## What was done

**The mechanism, from the decompile.** `Text_AddMsg` 0x1b0c30 only QUEUES a `TXT_MSG` node; every
sound and every drawn pixel belong to `HUD_UpdateStatusPane` 0x19c310, whose case 0 polls
`Text_UpdateMsg` 0x1b11d8 — which returns 0 while the viewer's +0x2a HUD gate is off
[0x1b11d8:16-18] (set by `Script_CameraStart` 0x1d9140:41-43, restored by `Script_KillStream`
0x1d9860:71). `Mission_Update` runs from `Game_Run` 0x1c9790:17 BEFORE the object handler
(0x1336f8:30-35), so the monitor sees a trigger's channel one tick after the trigger — after the
scene's camera gate is up. The SFX schedule lives in the pane: case 2 schedules PlayMe1st/PlayMe2nd
per display (new objective 1532/469; "OBJECTIVE COMPLETE" 468 with `ObjectiveSFXTriggered2`
preset so the 2nd never plays) [0x19c310:148-164]; case 3 fires PlayMe1st in the message's first 60
ticks, only while `ScriptCam == 0` [0x19c310:219-229], and PlayMe2nd (469) once the steady phase
begins (remaining + 60 ≤ life) [0x19c310:232-235]. `Text_UpdateMsg` ticks only the displayed head
[0x1b11d8:48-73].

**Fixes** (each cites the entry it implements — docs/systems/mission.md §3/§4/§5, updated in this
task):

- `nf-game` `Drones::tick` now runs `Mission_Update` at the TOP of the tick, before fx/triggers/
  scripts/the death path — the game's `Game_Run` order. Before, the port ran it at the end, so the
  monitor saw the Kimiko trigger's ch 0x48 on the trigger tick itself and the announce drained one
  tick before the scene's `Script_CameraStart` set the freeze. Reproduced with a frame trace:
  frame 1 = ch 0x48 set + the announce drained with the sound; frame 2 = `hud_frozen`.
- `nf-view` `hud_tick` drains the mission events BEFORE the frozen check (a banner queued
  mid-cut-scene waits in the queue with its timer untouched — the game's `Text_AddMsg` during the
  NIS) and plays the SFX when the message first DISPLAYS: 1532 at open (468 for "OBJECTIVE
  COMPLETE"), 469 sixty ticks later, new objectives only.
- The objective/mission banner queues tick head-only (`Text_UpdateMsg` ticks the head;
  0x1b11d8:48-73) — the port ticked every queue member in parallel, which expired the banners
  behind the front one before they ever showed.
- The objective banner's status label and body draw CENTRED on x = 256: the sprites' template x 128
  + `MsgObjectiveStatusPane` CREATE x 128 = 256, fmt 0x30cdb0's justify byte 3 = centre
  (`Font_ParseFormat` 0x1ee3e0 case 0xfe/3 → flag 2; `Font_DrawText` 0x1eee48 flag 2 → x −= w/2;
  templates dumped from ee_start_05.bin at 0x319608 entries 5/0 + CREATE 0x319710). The port drew
  both rows left-aligned from x=128 — the user's off-centre fail message. The bar itself (16..496)
  was already right: `HUD_UpdateStatusPane` case 1 re-pins the middle's x/w to the template on every
  open [0x19c310:101-111], which cancels the pane-x add for the bar but not for the text sprites.

**Text (item 2):** the binding was already correct on master (fix-mission): the events carry the
objective record's own label and `Lang::bind` resolves it. The user's dist build predated it. The new
test pins it per objective.

## How it was verified

New `crates/nf-view/tests/fix_objective_banner_live.rs` (all NF_ISO, all green), driven through the
real path (`nf_view::load` + `Sim::step_bond`):

- `the_kimiko_scene_holds_the_announce_until_the_handback` — walks into TouchOnce #412's volume
  (Bond at 94.2, 43.7, −45.9); from the trigger tick to the hand-back (`hud_frozen` clearing) no
  banner SFX (1532/469/468) plays and no queued banner has opened; at the hand-back exactly one
  announce ("Spy on secret meeting" — asserted equal to `Lang::bind` of the record's own text id
  0x04000003, status id 0x02000051), 1532 once at open, 469 once ~60 ticks later. (The earlier
  parts' objectives and 07000007's "Rendezvous…" are preset to their carried states — the state a
  player reaching the rendezvous has; a cold load also announces the earlier parts at tick 1, which
  is the game's own behaviour.)
- `every_objective_banner_shows_the_objectives_own_line` — 07000005 and 07000007: every banner body
  equals `Lang::bind` of the objective record's own text id (string ids in the asserts, not text),
  status 0x02000051.
- `the_fail_message_is_centred_on_the_banner` — the game's Fail-prefixed notice ("%s: %s",
  0x30cbb8 [0x1827d8:72-74]) drawn through the overlay: the status and body rows measure centred on
  256 ± 1.5, inside the 16..496 bar. (Before the fix both rows started at x=128.)

Suites: `nf-game` + `nf-formats` + `nf-audio` full: 296 ok, only the 3 allowed disc_golden failures.
`nf-view`: 173 binaries ran green except three 1-tick timing tests from other tasks that the
`Mission_Update` move shifts by design (fix_death_restart_live `death_sequence_stages_then_the_menu`,
fix_finale_live `the_0700004a_lift_ride_reaches_the_lobby`, fix_objects2_live
`the_safe_bolts_open_the_safe_and_the_briefcase_completes_the_boarding_objective`) — each updated
minimally (+1-2 monitor ticks, with a comment citing the `Game_Run` order) and re-run green along
with fix_objective_banner_live / fix_party_mission_live / fix_cutscenes2_live / fix_mission_live.
The nf-view run was cut by a sandbox timeout after those 173 binaries (the rest of the nf-view
binaries did not re-run in this session; they do not touch the banner path).

## Shared-file changes (all additive, minimal)

- `crates/nf-game/src/drone.rs` — the `Mission_Update` block moved to the top of `tick` (with
  fix-progress's `PlrStat` reset / `log_enemy_spawned` kept inside the moved latch during the
  rebase); no constant or match arm touched.
- `crates/nf-view/src/scene.rs` — `hud_tick`: the drain moved above the frozen return (no SFX
  there), the head-only tick for the objective/mission queues, the pane-open SFX block; `Msg` grew
  `life/opened/second` (own line); `Msg::new` made `pub` for the tests.
- `crates/nf-view/src/hud.rs` — the objective banner's two text rows: Anchor::Left @128 → centred
  @256 (the bar and every other pane untouched).
- Existing tests touched: `hud_playtest_live.rs` (two `Msg` literal constructions → `Msg::new`),
  the three timing tests above.

## What's left

- `fix_progress_live.rs` `killing_two_guards_and_taking_a_bonus_tallies_like_the_decompile` fails on
  the rebased branch: the guards die (health −42.5) but `PlrStat` shows `dispatched 0, hit 0`. This
  task's diff does not touch the kill/tally path; the failure appeared after rebasing onto master
  (fix-progress + the movie-decoder merges landed together). Needs an A/B against origin/master to
  assign it — left to the driver/next session.
- The remaining nf-view binaries (after the timeout cut) should re-run once on the merged master.
- Live checks for the main agent (PCSX2, 07000007): at the Kimiko rendezvous the "NEW OBJECTIVE /
  Spy on secret meeting" banner and its sting must appear only AFTER the cut-scene hands back; the
  banner's two text rows sit centred on the bar's centre (the bar spans 16..496); the fail notice
  ("Fail: …") is centred the same way. nf-view render spots: `--bond 94.2,43.7,-45.9,3.14,0` on
  07000007 right after the hand-back (banner up, texts centred), and any 07000005 fail moment for
  the Fail-prefixed row.

## ntfy failed

(not reached — the notification was sent; see the finish step.)
