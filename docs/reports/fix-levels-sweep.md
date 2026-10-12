# fix-levels-sweep — the health check of all on-foot levels

Branch **s/fix-levels-sweep** (from origin/master). Test: `crates/nf-view/tests/levels_sweep_live.rs`
(opt-in `NF_ISO`). Tables: `docs/systems/levels.md` §7. No product code needed changing — **the
sweep found no crash on any level**.

## What ran

Every on-foot archive in FILES.BIN — 27 on this ISO (the driving missions 07000023-29 and the
shared packs 07000500/07000999 are out of scope per levels.md §1/§3; 0700000e/f/10 are in the
mission data but their archives are not on this disc) — through the real Sim path
(`nf_view::load` + `Sim::step_bond`):

1. **The smoke** (the required test): load + 600 idle ticks, no panic. All 27 pass.
2. **The deep sweep**: 1800 ticks of a scripted walk along the AI path graph (nav waypoints,
   E pressed every half second for doors), then one CPU pose pass — the GPU-less `--shot` counts
   only (no Vulkan ICD in this environment, levels.md §6). No panic anywhere.
3. **The objective-channel writer scan**: for every record in `nf_game::mission::LEVELS`, each
   objective's condition channel (with `Mission_Init`'s init value) and enable channel, and the
   writers found **across the whole mission** (the monitor matches BaseMap, so a sibling level's
   mux can write this level's channel). Writers scanned: trigger outs, the OUT-mirror's target
   list, LOAD_LEVEL success/fallback writes, script event-0xD/0x11 writes (the libs include the
   fix-cutscenes loadable packs), pickups' +0x28 channel, drone death channels (placement param
   12 [M set_death_channel 0x13e660]), fuseboxes, cam-subjects, destroy props.

## Results

- **Panics: 0. Unresolved objects: 0. Pickups without a model: 0** on every level.
- **Unhandled placement types** (drawn as static geometry, gameplay skipped), each with its
  `parsemap_create_dynamic_objects` (0x1d04f0) creator: 0x21 Ripples, 0x35 PCQWorm (lands with
  s/map-tower), 0x3a Grapple, 0x3b DynamicObject, 0x3f CreepWall, 0x41 InverterTrigger, 0x4e
  Light, 0xd2 Shooter, 0xd8 RB (rigid bodies), 0xda Car, 0xe3 Flicker, 0xe7 ThirdCam, 0xf2
  Emitter, 0xf3 EnvTree, 0xfc Corona, 0x38/0x39 space laser/missile, 0x4a Apocalypse, 0xd4/d5
  Cloud/OneSided, 0xf4 MoviePlayer. 0x17/0x23 have no case in 0x1d04f0 (map-block data).
- **Unported drone state ids** (no constant in the port's 88-state machine): 0x85 (mode 7) on
  07000011/12/13/15, 0xa7 on 07000004, 0xb4 on 07000007 — start-state ids passed through from
  `DroneTypeSettings` with no handler arm; the game's handlers live in the DSTATE dispatcher
  (`NDrone2_ControlSTANDARD` 0x1490f0 + the per-dtype controls).
- **0700001b**: 11 of 74 ScriptPlayers waiting on their gate channel and 11 drones never enabled
  after 1800 ticks (`NDrone2_Enable` 0x149dd0 / the k15 wait-switch — the fix-npcs C2 class).
- **Objective channels**: 55 records scanned; the writers found are tabulated per level in
  levels.md §7.2.

## The brief's known gaps — resolved

- **07000005 "Breach the castle walls" (ch 0x46, enable 0x45)**: both written. ch 0x46 by
  **07000006's OR mux #498** [D: type 0xC out 0x46 list [0x32, 0x33, 0x34]] — the monitor matches
  BaseMap 07000005 while playing 06; enable 0x45 by 07000005's TOUCH_ONCE #36 [D]. The record
  bytes `46 00 01 45` misread in the brief: `Mission_Init` reads the init value from **+0x0D**
  [M 0x182270:118-124: `switch_channels[+0xc] = +0xd`], which is 0 here — the 0x01 sits at +0x0E,
  a different field. fix-mission's records are correct; nothing to port.
- **07000002/03's key-gated exits (ch 0x4C / 0x52)**: both written — 0x4C by an AND mux on
  07000002 [D trigger(out 3)], 0x52 by script 0x06000082's event 0xD on 07000003 [D].
- **07000008's boarding/ride channels 0x4A/0x4B**: 0x4A written by a pickup's +0x28 channel [D];
  0x4B remains a gap (below).

## Remaining gaps (levels.md §7.3, with the game function)

- 07000008 ch 0x4B (the cable-car ride): the SP chain `mux AND(0x4A, ch 4)` → ScriptPlayer #295
  (0x0600004C) — `SP_Update` 0x1dbda8's out-hold does not raise it; the mayhew walkthrough drives
  the channel.
- 07000003 ch 0x4E (the Dragon Safe): no writer in the castle data — the safe-object handler (the
  0x3b DynamicObject family) is unported.
- 07000016 ch 0x90: the Kiko-launch chain — s/map-finale's scope.
- 0700000a/0b's Q-Worm channels (ch 0x48/0x4C): written by s/map-tower's PCQWorm [M 0x1aee20] —
  unmerged at scan time; the walkthroughs prove them.
- 0700000e/f/10: not on this ISO.

## Verification

- The sweep test's two passes assert no panic (they fail the test on any panic).
- Full suite: 82 test binaries ok; the only failures are the 3 permitted slim-ISO `disc_golden`.
- An independent audit pass over the scan's own byte-reader caught the endianness bug (the port's
  stream reader is little-endian; my first big-endian pass reported garbage channel ids like
  0x47000000) — fixed and re-run before the tables were written.
