fix-finale-blockers — the report

What was asked
The remaining mission blockers the per-level walkthroughs found: 07000016's Kiko defeat (ch 0x90),
0700001b's missile consoles (the AND on channels 21/22/27) and Drake's defeat, 0700004a's lift
lobby arrival (ch 0x4a), the unported drone start states 0x85 / 0xa7 / 0xb4, and the gate writers
of 07000002/03 (ch 0x4C / 0x52) and 07000008 (ch 0x4A / 0x4B). Each fix had to cite the doc entry
and the game address, and come with a headless test that fails before the fix and passes after,
driven through the real input path.

What is done

1. 07000016 — the final mission now completes. New module crates/nf-game/src/kiko.rs (all
   addresses and evidence tags) plus additive state arms in drone.rs: the KikoMission family
   (KikoMission 0x19 = 0x15df28, KikoMissionRun 0x1a = 0x15e0d8, StandBlind 0x18 = 0x172870,
   RunToAlarm 0x89 = 0x1736f8 — read from the decompile and the state table at 0x29b300), and the
   ninja chain 0xa6 → 0xa7 with the distance bands 0xa8/0xa9/0xaa/0xb2 (0x16b990, 0x16bb50 and the
   band states). The chaser is placement #261 (skin 0x0500001e, dtype 0xf — GetDTYPENEW case 0xf
   at 0x14c938 sends dtype 15 to the ninja chain, the same family as 07000004's assassin; the
   #294–297 pit squad share the skin but carry death channels 23..26). Her defeat raise is a
   documented stand-in: no writer of ch 0x90 exists in the level's triggers, its 300+ scripts
   including the three loadable packs (both channel-write events swept), the script players' out
   channels, any drone's death-channel param, or the decompile's complete enumeration of
   switch_channels writers (103 files) — so kiko.rs gives #261 death channel 0x90 at init. The
   spawn chain is the game's own and runs in the port: ch 0x81 → script player #262 plays the pack
   cinema 0x06000631 → ch 185 → #272's 0x0600090a → ch 240/242 → spawners #257/#293; the test
   fires only the unproven chain head (ch 0x81). Her death raises ch 0x90 → AND #271 → ch 100 →
   MissionState 6 → the driving hand-off 0x7300017 (Mission_Update 0x182c60:111-112).

2. 0700001b — the station mission now completes, with no stand-in on the defeat path. Drake is
   drone #186 (skin 0x050000ba, death-channel param 12 = 14 = 0x0e — the earlier doc note "no
   placed drone carries death channel 11" was looking at the wrong channel). Channel 0x0e is the
   fight gate: spawner #187 (trigger 10) spawns him, his own wait-switch (param 3 = 10) enables
   him, and the SpaceDrake state machine (0x16dee0) reads the same channel. Script player #198
   ("ScriptPlayer_Pause", out 0x0e, mask 1) then plays the defeat NIS 0x06000a20, whose
   conditional event-11 stream writes ch 0x0b ("Defeat Drake") at t600 and ch 0x64 — channel 100,
   the mission end — at t720 (Script_EventHandler 0x1d94c0 case 0x11). The mission ends into
   0x0730001b. The missile consoles (objective 1, ch 0x09) stay unproven and are documented as
   such (below); the mission completes without them because the NIS writes ch 100 directly.

3. 0700004a — the lift works and the level completes. The lift is script player #42
   ("SCRIPTPLAYER_Lift", mode 1) playing NIS 0x06000571: its platform entity descends 150 units
   from key t90 to key t5040, ch 0x40 fires at t1610 and unlocks the −52 floor doors. Two port
   bugs fixed on the way:
   - Script_Update blocked forever on a NEGATIVE delay. Event 3 stores "p0 − 1"; the lift script
     fires it with p0 = 0 at t1611, storing −1, and the port treated any non-zero delay as a wait.
     The game only decrements a positive delay and runs the streams otherwise
     (Script_Update 0x1da998:17-18,112-114). This froze the lift (and any other script firing
     event 3 with p0 = 0) mid-ride.
   - The lobby arrival: no writer of ch 0x4a exists in the level's 42 maps' triggers, scripts,
     script players or drone death channels, and the level has no lift/SS/car/monitor object at
     all — the machinery is the doors, the floor plates and the script. The port now writes ch 0x4a
     when the lift script ends (the platform parked at the lobby) — a documented stand-in in
     nis.rs's script-player state-2 completion, keyed to 0x0700004a + 0x06000571. The objective
     completes and the monitor fires ch 100 for the Tower-2 mission.
   Known gap, documented in the level doc: the shaft doors' auto-open on the lift's approach is
   not ported, so a rider gets scraped off around the −52 floor mid-ride; the test re-seats Bond
   along the script's key line (the same shape the existing walk test uses for the plates).

4. The unported start states — all three behave like the game now: 0xb4 DeleteMe (0x173f00)
   despawns the drone on ENTER (07000007's mode-21 party placement); 0x85 EnemyRunToPoint
   (0x168910) resolves to its return state — 0x56 Attack, or Idle/Alert on 07000011/12/13/41 while
   the player is unseen (the decompile's own fallback at 0x168910:32-36 is the proof that the
   sweep levels' mode-7 drones stand: the port has no 0xe9 AI-point registry, so FindRunToPoint's
   none-found arm is the one that runs); 07000004's assassin (dtype 0xf → dtypenew 0xd →
   DroneTypeSettings[13] = 0xa6/0xa7) enters NinjaAttack at alert 1.0 and re-routes by distance.
   The gate-waiters among them sit in WaitSwitch (state 1) exactly as the game leaves them.

5. Two cross-cutting death fixes, both from the decompile: Death_Anim raises the drone's death
   channel on its first UPDATE while the pending flag is set (0x1637f8 case 2), not when the death
   animation finishes — before this, a drone whose death danim never completes (Drake) never
   raised its channel at all; and the ninja band re-dispatch arms now yield to a pending
   transition, because the game's damage messages run inside the state router (0x175778) while the
   port's explosion damage sat outside it — a same-tick UPDATE re-dispatch used to clobber the
   pending death and leave a zero-health ninja standing.

6. The gate writers the task named (07000002 ch 0x4C, 07000003 ch 0x52, 07000008 ch 0x4A/0x4B)
   are bounded, not ported: the complete enumeration of switch_channels writers in the decompile
   (103 files, each checked for what it writes) contains no path to those channels on those levels
   — the castle chat guard writes ch 0x1a, the Zoe control writes ch 0x9B, and every other writer
   is level machinery the data scans already covered. Player_Activate's E-dispatch (0x1a5170) and
   its per-class activators write no switch channels themselves. The level docs now carry this
   evidence; the existing walkthrough tests keep their documented channel drive.

How it was verified
- The new test file crates/nf-view/tests/fix_finale_live.rs (opt-in via NF_ISO) drives the real
  path — nf_view::load / load_with_carry plus Sim::step_bond — through four tests, each with
  sister assertions: the 07000016 spawn chain and chaser kill (sister: a non-chaser death never
  raises ch 0x90), 0700001b's Drake gate → kill → NIS timing (sister: ch 0x0b waits for the NIS's
  t600, not the death tick), 0700004a's ride and arrival (sisters: Bond descends with the
  platform; ch 0x4a waits for the script's end), and the three start states on their own levels.
- Reproduce before fix: with the fixes stashed, all four tests fail with exactly the symptoms
  (the chaser never spawns, Drake's channel never fires, ch 0x4a never completes the objective,
  the 0xb4 placement never despawns); after the fixes all four pass. Both runs recorded.
- The full suite (NF_ISO set, all four crates, single-threaded) runs before the push; on the slim
  ISO only the three disc_golden tests may fail, per the standing rule.

What is left
- The ch-0x90 stand-in on 07000016 and the ch-0x4a stand-in on 0700004a are the two documented
  substitutions; both need the writer the port cannot see (the launch's own script link and the
  lift's arrival logic) if the real mechanism ever surfaces.
- 0700001b's console objective (ch 0x09) is unproven end to end: the AND's inputs are the space
  missiles' per-missile channels, written by the missile state machine only after the launch order
  ch 19 = AND(0xc1, 0xc2), whose writers are not on the disc data; the task's Monitor hypothesis
  does not hold on this level (no Monitor or Sensor placements with channels on any of its 42
  maps; the NIS console entities are plain objects — Script_CreateEntity's dispatch has no case
  for their stream class). The mission completes without it.
- 07000002/03's key-gate channels and 07000008's boarding/ride stay with the un-ported
  mission/dialogue layer, now with the code-side enumeration as evidence.
- The ninja chain's melee/throwing-star animations are not modelled (the bands advance and
  re-dispatch); the shaft doors' auto-open on 0700004a is not ported.
