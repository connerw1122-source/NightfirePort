# fix-death-restart — dying, failing and restarting like the game

Branch `s/fix-death-restart` (latest: `5fe1625`, merge of master `d8a10a6` + 3 task commits).
Everything below is implemented on the branch with tests; the full suite is green except the 3
expected `disc_golden` failures on the slim ISO.

## What's done

### 1. The restart loop (the 2026-10-12 playtest, top priority) — reproduced and fixed

The user's symptom: dying on 07000007 (after punching a guest) looped "replaying at the start of
the mission over and over again".

**Reproduction** (`carried_restart_replays_the_fail_but_the_cold_one_does_not`): punching a guest
kills them → their `OnInitDeath` writes the civilian-kill channel 0x60 → the 0x60/0x61 hook raises
the fail channel 99 [M 0x1827d8:98-117] → MissionState 2 [M 0x182c60 case 2] → the menu → Restart.
Capturing the RamSave at that moment and reloading the way the OLD restart did (same level, carry
applied) shows the loop: the saved ch 99 re-fails the mission on the first tick back, with no input
at all — and after the banner the menu restarts again, forever.

**Cause**: the port's restart was a WARM reload — it carried the RamSave (health 0, ch 99, the
failed mission states) into the fresh level.

**Fix**: the fail/death menu's Restart is the game's COLD arm — `ResetMap_LevelToLoad(
Mission_BaseMapHCode(), 0, 1)` [M 0x204408:139]: `IsWarmReset = 0` means `Init_SwitchChannels`
zeroes all 256 channels [M 0x134310, 0x1d5080:44-46], `Player_RamLoad`'s whole body is skipped (it
is gated on the warm flag [M 0x1957a8]), and `Mission_Init(BaseMap, 0)` resets every part's
objective states [M 0x182270:68-129]. The port marks the pending exit `level_exit_cold`
(drone.rs, own-line field) and `nf-view/src/lib.rs` drops the RamSave for that exit. The restart
also goes to the MISSION'S FIRST level, not the current one — `Mission_BaseMapHCode` = `BaseMap`
[M 0x1830a0]; the walk test's fail-restart target on 0700000a was corrected 0x0700000a → 0x07000009
accordingly.

### 2. The death sequence, end to end (new `crates/nf-game/src/death.rs`)

- `Player_CheckForDeath` 0x190810: on health ≤ 0 — ch 0x5e, `Music_Event(6,1)` once, the death
  sound 0x89, `Player_WeaponNone` (fists, weapon 0x47), substate 0xd; while the player state is 2/3
  `Player_Update` runs no input [M 0x193600:317-319] — `Sim::step_bond` gates the input while
  `death.active`/menu (scene.rs hook).
- The blood pane (pane 12) enables on the first dead tick (`HUD_DisableAll` + `HUD_Enable(12,1,0)`
  [M 0x190bb0:23-25]); the red fade grows 1 px/tick [M 0x19a6a8:24], full cover at 448 ticks
  (viewer.h = 448 [M 0x19f4e8:33], template width 512 [L BloodSprInfo 0x2bcfd8]) ≈ 7.5 s.
- ch 0x62 rises on the cover **or** the player's action 25 (✕/Start — the skip) [M 0x190bb0:29-33].
- `Mission_Update` reads the death CHANNEL 0x62 [M 0x182c60:21], **not health** — the port used to
  fire the whole restart the instant health hit 0 (also replaying music 6 every tick). States run
  1 → 3 (`FadeClr` 0xff0000ff) → 7 (`GS_PausePlayer`) → 4 → the menu [M 0x182c60:45-81].
- The death restart loads the plain 0x70000NN code (param 3 = 1: no 0x71xxxxx intro-variant remap
  [M 0x1d3800]).

### 3. The 0x40000042 fail/death menu + the API for 180-fix-menus

`Mission.menu_open` / `menu_choice` (`MenuChoice::Restart|Quit`) + `Event::MenuOpen` [M
0x182c60:77-81: `MenuManager_Create(-0x80000004, 0x40000042, 0, 0, 0x28, 0, 4)` + `GS_PauseGame(1)`,
InternalState 5]. The answer API is `Drones::fail_menu_open()` / `fail_choose_restart()` /
`fail_choose_quit()` (crate::death), mirroring `C_GCPAUSE_Handler` 0x204408:136-155: Restart → the
cold reset above; Quit → `ResetMap_LevelToLoad(0x7000048, 0, 0)` [M :141-149] — the port logs it
and holds (no front end). Until the menu is drawn, `Drones.auto_restart` (default **on**) answers
Restart immediately — the pre-menu behaviour, behind the flag exactly as asked.

### 4. What a restart resets and keeps

A restart equals a fresh in-mission load: nothing is carried (cold arm — channels zeroed, no
RamLoad, `Mission_Init` resets current+future parts only [M 0x182270:68-129]). Test
`restart_matches_a_clean_load`: die on 07000005 after picking a pickup up, taking damage and
flipping objective channel 0x45 → restart through the real menu path → the reloaded sim's health,
armour, channels, carried/owned inventory, mission states and MissionState all equal a clean load;
the WARM carried reload provably does NOT match (the loop's mechanism — the sister).

### 5. Falling deaths and the scripted ones

`lethal_fall_reaches_the_death_path_and_the_cold_restart`: a real 400-unit fall on 07000011 (the
atrium level; part 0 of its own mission) lands through `Player_CollisionHandler` 0x1949a8:117-126
(`Player_HandlePain(timer − 60, type 6)`) → health 0 → the full sequence (pane, ch 0x5e/0x62, the
menu) → the COLD restart to 07000011. Sister: an 8-unit drop triggers none of it. The scripted
death NISs are already non-skippable in `nf-game::nis` (`Script_IsDeathNIS` 0x1dae60: the three
hashes 0x0600_0952/0x0600_098a/0x0600_0991) and play over this same sim sequence.

## How it was verified

`crates/nf-view/tests/fix_death_restart_live.rs` — 8 NF_ISO tests, all passing (5.7–6.6 s):

1. `fail_restart_targets_the_mission_start_cold` — punch-out fail on 07000007 → the banner's 300
   ticks [M 0x182c60:52-57, `FRAME_RATE_MUL` 1.0 [L 0x30d0d8]] → auto-restart → 07000005, cold.
2. `carried_restart_replays_the_fail_but_the_cold_one_does_not` — the loop mechanism (warm carry
   re-fails with no input; the cold reload comes back clean).
3. `death_sequence_stages_then_the_menu` — stage by stage, with the ✕ skip, plus the near-fatal
   sister (nothing triggers).
4. `death_menu_applies_by_itself_after_the_red_fade` — the no-skip 448-tick arm.
5. `restart_matches_a_clean_load` — restart ≡ clean load (with the warm-carry sister).
6. `fail_menu_waits_for_the_view_answer` — the menu API on 07000009 (alarm channel 0x7C — the OR
   trigger re-drives 0x61 every tick, a direct write would be stomped [L probe]); Quit → 0x7000048.
7. `near_fatal_hit_triggers_nothing` — the alarm-level sister.
8. `lethal_fall_reaches_the_death_path_and_the_cold_restart` — the fall entry (above).

Pre-fix failure: the 7 original tests fail to compile against the pre-fix tree (the API is new),
and the pre-fix behaviour they contradict is in the branch's parent (instant ch 0x62 on health 0 +
the carried warm restart); test 2 demonstrates the old mechanism directly. Full suite
(`cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view --
--test-threads=1`, run on `5fe1625`): green except the 3 expected `disc_golden` failures
(slim ISO).

## Shared files touched (all small, additive hooks)

- `drone.rs` — `Drones` fields on their own lines (merge lesson 4): `death` (the sequence state),
  `bond_fire` (this tick's action 25 for the skip gate), `auto_restart`, `level_exit_cold`; the
  `death_tick` call + the ch-0x62 read replacing the health read; the `fail_*` answer API lives in
  death.rs.
- `scene.rs` — the death-tick hook in `Sim::step_bond`, the dead-Bond input gate, the
  `Event::PlayerDead`/menu handling on the view side.
- `lib.rs` (nf-game) — `pub mod death;`.
- nf-view `lib.rs` — `load_with_carry` drops the RamSave when `level_exit_cold` is set; the reload
  loop handles the menu exit.
- `triggers.rs`, `player.rs` — untouched by this task.
- New files: `crates/nf-game/src/death.rs`, `crates/nf-view/tests/fix_death_restart_live.rs`.
- Docs: `docs/systems/mission.md` §8 (the whole path + the known-gaps update), `docs/WORKLOG.md`
  (entry `fix-death-restart`, appended).

## What's left / known gaps

- The menu DRAWING is 180-fix-menus' (the API above is ready); the red fade drawing (pane 12's two
  sprites growing 1 px/tick, layer 9, textures 0x0300005a/0x03000002 [L 0x2bcfd8]) is render-side —
  the sim holds `death.pane_on` / `death.ticks` / `death.covered` but nothing draws it yet.
- The death substate's animation (substate 0xd) is not animated by the sim; Bond stops taking input
  while `death.active` (the game runs `AnimObjectUpdate` [M 0x193600:317-319]).
- Quit (0x07000048) is logged and held — the front end is not ported.

## Live checks for the main agent

1. **The red fade** (07000005, kill Bond — fall or guards, no ✕): the screen should creep red from
   the centre over ~7.5 s. Watch `BL+0xf4` (player state → 2), the pane's state word reaching 1 at
   cover [M 0x19a6a8:34]. GDB watch on 0x30cdd8 (`switch_channels[0x62]`, the address
   `Mission_Update` 0x182c60:21 reads): write 0→1 exactly at cover or on ✕, never at the pain tick.
2. **The ✕ skip**: press fire while dying — the menu/restart comes immediately instead of the 7.5 s.
   Access: read `obj+0xf4` (2/3 while dying); expected: 0x62 rises the same tick.
3. **The 07000007 punch case** (die after punching a guest): the fail banner ~5 s, then ONE restart
   to 07000005 (the mission's first level), COLD — full health, fists, objectives re-announced, and
   NO further restart without a new death. This is the playtest's replay loop, now gone.
4. **Render spots** (`--bond`): on 07000005 at the death, (a) ~2 s in: a red pane starting to grow
   from the screen centre behind the HUD; (b) at cover: full red, then the 0x40000042 menu
   (Restart/Quit) once 180-fix-menus draws it; (c) on 07000011, the atrium fall: the camera follows
   the fall, the landing cuts to the same fade.

## ntfy

(sent after this report — see the finish steps' output in the session log if it failed)
