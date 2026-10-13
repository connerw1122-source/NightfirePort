# fix-party-mission — the 07000007 party mission and the 07000008 mission flow

Branch `s/fix-party-mission`. All seven playtest items (2026-10-12) are done, each reproduced
fail-first through the real input path on the NF_ISO and then fixed against the decompile. The
tests live in `crates/nf-view/tests/fix_party_mission_live.rs` (15, all green); the full suite was
run before the final push.

## What's done (per item, with the game evidence)

1. **The entrance bouncers no longer shoot unarmed Bond.** The game's gate: `CivilianGuard` UPDATE
   0x1603a8 case 3 attacks only when `NDrone2_NearArmedPerson` 0x1787c8 fires (prop 0x3c + Bond in
   sight `+0x228 & 4` + `ObjectIsArmed` 0x178768 on Bond's class byte). Bond starts the party with
   the FISTS (InitWeapon 0x1a7ef8 case 0x7000007 → +0x63 = 1; WeaponSelect 0x1a2cc0 copies the
   request into the class byte at the switch's start), so `ObjectIsArmed` is false for the whole
   pre-hall phase and the bouncers BondTalk instead.
   Test: `the_party_entrance_guards_never_shoot_unarmed_bond_at_the_start` (fail-first: the port
   used to arm them from the level start).

2. **Punching a guard fails the mission at once; shooting a guest fails it.** Both fail writers
   ported through the real hit paths: `NDrone2_PunchImpact` 0x145f28:45-56 (an unarmed 07000007
   punch → SetMissionFailReason(5) → "Cover Lost", ch 99) and `DroneFunc_OnInitDeath` 0x145bd0
   (a killed prop-0x54 guest → reason 3 → "Killed Civilians", ch 0x60).
   Tests: `punching_a_bouncer_fails_the_mission_at_once`, `shooting_a_guest_through_the_bullet_path_fails_the_mission`.

3. **Punching a guest no longer T-poses the others.** Reproduced with the real punch dispatch
   (fists → msg 6), fixed; the other guests keep their animation scripts.
   Test: `punching_a_guest_keeps_the_other_guests_animated`.

4. **The cut-scene music takes over at NIS start and gives back at the end.** The per-level
   updaters `UpdateMFX_01CastleC` 0x113188 / `UpdateMFX_01CastleD` 0x1133b8 jump sections on the
   NIS 0xB/0xC script-hash events (07000008's intro: start → marker 2, end → 4); ported as
   `Music::castle_c/castle_d`, `UpdateMusicalEvents` runs in the sim's tick (with or without an
   output device), mission fail/success → Music_Event(7/8) 0x182c60.
   Test: `the_cutscene_music_jumps_at_nis_start_and_end`.

5. **The banners carry the objective text and wait for the cut-scene.** The text binding itself was
   already fix-mission's `Mission_MonitorObjectives` 0x1827d8 port — the user's build predates it
   (a probe pins the bound labels on 07/01/02/08: "Breach the castle walls", "Escort Mayhew to
   bunker", …). The NEW part is the NIS hold-off: `Script_CameraStart` 0x1d9140:41-43 zeroes the
   viewers' HUD flag (+0x2a, `Camera_Enable` 0x1b9920) and `Script_KillStream` 0x1d9860:71 restores
   it; while it is 0, `Text_UpdateMsg` 0x1b11d8:16-18 runs no timers and the status panes never
   open (`HUD_UpdateStatusPane` 0x19c310 case 0). In the port: `hud_frozen` (set/cleared by the NIS
   camera requests), the message timers freeze, the mission events flush at the NIS end, and the
   three status panes draw nothing mid-NIS.
   Tests: `the_07000001_banner_survives_the_intro_cutscene` (fail-first: the 07000001 banner
   expired at tick 300 of a 2120-tick NIS — "no banner came up") and
   `no_status_pane_draws_during_the_07000002_intro_cutscene` (fail-first: banners drew mid-NIS).

6. **The guards re-show where they stood after the Kimiko scene.** The game teleports NO drone at
   NIS end — only Bond, to the event-6 marker entity (`Script_Free` 0x1da880:44-47 → Player_Enable
   with the previous stream's matrix; 0x06000073's marker key 498 = (91.52, 43.85, −31.09) yaw 180°
   [D]) — and re-shows everyone at their pre-scene positions (`Drone_EnableAll(1)` 0x1383e8:14
   skips WaitSwitch drones; `NDrone2_Enable(1)` 0x149dd0:16-28 refuses corpses and restarts the
   anim clock at script time 1). The port's re-enable now does the clock restart (the fail-first
   showed a guard frozen mid-stride at anim.time 40 resuming at 40; now 40 → 0) and skips corpses.
   The hand-back trigger's ch 0x48 removes the scene's guests via their second_channel/alt_state
   0xb4 DeleteMe [M `NDrone2_PreDroneControl`] — game behavior, pinned in the test.
   Test: `the_guards_reappear_at_their_places_after_the_kimiko_scene`.

7. **The 07000008 safe chain works in play.** The user's three symptoms are one sealed chain:
   the "giant pillar" at the missile closet is the locked GuardRoom door #122 + bolt #217 + safe
   door #205 [D placements], with nothing to open them; the gondola's enable channel 0x4A is
   written only by the briefcase inside the safe; the rocket launcher sits behind the same door.
   Fixes:
   - The GuardRoom fusebox #219 (0x31, script 0x060008AD) never drew — the game's own fusebox
     object has no mesh (obj+0xD8 = 0 [M 001b0448:33-35]); the visible box is its ScriptPlayer's
     entity (FuseBox_Create 0x1b0448 ends with `SP_Create(script, 0, 0x40, 0, obj, FuseBox_Register)`,
     mode 0x40 preload). The port pushes that player at attach — after `Scripts::new`, whose rebuild
     wiped the first attempt — and the mesh renders and joins the collision world.
   - The Lazer Watch 0x4E/0x4F (owned, clip 500 [D loadout tail 0x1a7ef8:318-325]) now burns the
     safe bolts: `Lock_Update` 0x1ae3d8 case 0 (record word 1) takes class-5 contacts whose
     BU+0x44 = 0x4E/0x4F and `Lock_Unlock` writes `switch_channels[record+8]` = 134/135 [:194-195].
     The port's watch bullets already carried the weapon id; the bullet pass adds the
     segment-vs-sphere lock contact (the port's `Lock::burn`, instant — the create-time base is 0
     on this level's data [M 0x1ae1f8, #207/#208 param 2 = 0]).
   - The SimpleScripts (0xE2) join set_dynamic with their placement glist (`Ss.hash`), so the safe
     door (ch 136) and doorway bolt (ch 137) are solid like the doors.
   - The chain test drives the whole path: two E presses on the fusebox → ch 137 → the bolt SS
     opens; the watch burns both bolts (mux #278 is an AND [D]) → ch 136 → the safe door opens;
     the briefcase pickup writes ch 0x4A (`Pickup_Handler` 0x18e650:330-333) and the rocket
     launcher (0x2C) is taken; the gondola ride starts (ch 4 · 0x4A → mux → ch 1).
   Test: `the_07000008_safe_chain_works_in_play` (fail-first: no fusebox SP, no mesh entity, the
   watch burned nothing — demonstrated on the pre-fix tree).

## Shared files touched beyond the task's list (all small, additive)

- `crates/nf-view/src/scene.rs` (nf-view): the `hud_tick` freeze (`hud_frozen` early-return after
  the pickup-message conversion), the SsList::create call's new hash arg (`|_p| 0` — the render
  pass resolves placements itself), nothing else.
- `crates/nf-game/src/drone.rs` is a task-named file; within it the non-mission bits are the
  fusebox SP push, the SS set_dynamic line, the laser-burn contact + `segment_hits_sphere`, the
  `hud_frozen` field + Camera hook, and the re-enable body — all listed in the WORKLOG.
- `crates/nf-game/src/ss.rs` (new `Ss.hash` field on its own line + a resolver arg),
  `crates/nf-game/src/lock.rs` / `fusebox.rs` / `docs/fields/objective.md` (doc updates only).

## Verification

- Fail-first: every item's test was shown failing before the fix (items 1–6 before this session's
  fixes; item 5's and 6's fail-firsts ran against the stashed tree; item 7's ran on the pre-fix
  checkout).
- Full suite: `NF_ISO=… cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio
  -p nf-view -- --test-threads=1` — see the WORKLOG for the count; on the slim ISO only the 3
  allowed `disc_golden` failures.
- Note: the sandbox reset mid-session (everything unpushed was lost). The last two commits
  (the fusebox-SP move after `Scripts::new` and the item-7 chain test) were re-applied from the
  session notes and re-verified green on the restored tree; the item-7 fail-first had been
  demonstrated live before the reset and is quoted above.

## What's left / live checks for the main agent

Render-side work that needs the main agent's GPU (rule 6), per `--bond x,y,z,yaw,pitch`:

1. **07000008 --bond 152.35,43.13,-120.76,1.261,-0.126** — the GuardRoom fusebox should now DRAW
   next to the doorway (the mesh comes from the SP's script entity; its mesh hash is
   0x020008b9 [D]). The doorway bolt SS opens after the two E presses; the door #122 unlocks.
2. **07000008 --bond 165.5,44.5,-120.5,2.4,-0.1** — the safe door #205 (at 162.25,43.33,-117.13)
   should be visibly CLOSED until the bolts burn, then glide open; the two bolt meshes (#207/#208)
   are still NOT drawn (0x2E is not in the port's dynamic draw list — cosmetic gap, cited in
   lock.rs's doc).
3. **07000007 --bond 94.3,44.6,-45.9,3.14,0** — walking into the hall trigger starts the Kimiko
   scene; after it, Bond should stand at (91.5, 43.9, −31.1) facing −Z and the guards should be at
   their posts (the scene's guest set drains by the game's own rule).
4. **07000001 from load** — the "Escort Mayhew to bunker" banner should appear right after the
   intro cut-scene ends (it used to never appear).
5. **The laser watch in hand (07000008, gadget cycle)** — the fp view-model plays its fire script;
   the beam itself is not rendered (no `Draw_LaserBeam`), so burning shows only as the bolts'
   channel/door response. If the user wants a visible beam, that's a render-side follow-up.

Live checks (gdbwatch/fieldprobe) that would settle the remaining uncertainty:

- `switch_channels` at 0x26fc90 + 134/135/136/137 while doing the safe chain live — confirm the
  game's burn durations (the port burns instantly; the game's `Lock_Update` decrements
  +0x104/+0x105 by 3/10 per contact from 0xFF — if the live values disagree, `Lock::burn` should
  model the health bytes instead).
- The fusebox SP live: `SP_Update` on the box's SP (SI+0x98 hash 0x060008AD) — confirm the mesh
  entity persists (state 1) and the wires register into the two groups (FuseBox_Register 0x1b0970).
- The Kimiko scene's guest drain: watch drone +0x135/second-channel for the ch-0x48 deletion live
  (the port pins it from the data; a live pass would confirm the set).

## Known gaps (not this task)

- The cut-scene playback issues (the Kimiko scene skipping, the Drake scene's duplicate models, no
  voices) belong to `045j-fix-cutscenes2` per the driver.
- The watch beam's visual, the 0x2E bolt meshes and the 0xE1 explosion box's burn-by-laser path
  (the guardroom door's second opener) are render/gadget follow-ups.
- The level-0x07000008 rocket launcher's forced 4 rounds (`Pickup_Create` 0x18dda0:116-118) is not
  ported (cosmetic).
