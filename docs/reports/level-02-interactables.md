# level-02-interactables — 07000002 "The Exchange" (Castle B)

Branch `s/level-02-interactables` (from origin/master d628f01f). Every number below is [D] a disc
parse of FILES.BIN `07000002.bin` (nf-formats `Level::parse`, main map entry 760 `01000094`, 502
placements), [M] a decompile citation (work/decomp/ACTION.ELF), or [L].

## Done

1. **The census** — `docs/levels/07000002.md` rewritten as the level's full interactables table: every
   kind, its disc count, the game behaviour [M] and the port status. Headline counts: 3 proximity
   sliding doors (no locks/channels), 24 TouchOnce + 3 Touch + 8 AND/SEQ_AND muxes, 1 LOAD_LEVEL
   (key gate 0x4C), 15 SimpleScripts (4 E-usable hatches/grills + 11 presence/bullet deco), 8 pickups,
   2 sensor cameras + 1 monitor, 8 scriptless fusebox sockets (correctly inert), 50 breakables,
   6 destroyables, 4 ladders, 1 wire + icon, 1 Hurt, 2 ScriptPlayers, 21 rigid bodies (unported,
   levels.md §7.1), and **zero** of: switches, safes/locks, keypads, lasers, searchlights, turrets,
   mines, vents/creepwalls, grapple points, vehicles, weapon racks, keyed/locked/scripted doors.
2. **Two corrections to the old doc/tests** (both now fixed in `docs/levels/07000002.md`,
   walk_07000002_live.rs, walkreal_07000002_live.rs headers):
   - "kind-3 key pickups #275/#430/#481" was wrong twice: pickup kind 3 is ARMOUR [M Pickup_Handler
     0x18e650:220-225]; #430/#481 are `PICKUP_BodyArmourHeavy`; #275 is an SS plant. **07000002 has no
     key pickups at all** — the exit gate is channel 0x4C.
   - **ch 0x4C's writer IS on the disc**: AND trigger #480 (0xEC, out 0x4C = AND(0x48, 0x4B)) at
     (−10.77, 5.91, −25.61) [D], fed by Computer1 #14 (0x48) + AND #303 (0x4B). The old "writer is not
     on the disc" claim (doc + both walk tests) is corrected; what is genuinely off-disc are the
     0x49/0x4A writers (the dialogue layer) and 0x4D.
   - "SS kind 4 param 586" terminology: there is no sub-kind enum — param 2 is the E-activate mask
     (bit 2 = E-usable), param 4 the operate sound id [M SS_Create 0x1a9c58:33,50].
3. **Models: everything resolves.** 785/785 placements across all 52 maps (0 by-hash misses, 0 index
   overruns), 25/25 character skins, 8/8 pickups, **0 of 5,507 materials on the white fallback** (the
   NF_TEX_TRACE predicate evaluated over scene.materials/textures headlessly — gpu.rs:525 is the
   wgpu-only path this sandbox cannot reach). Hashcode-0xFFFFFFFF placements (sniper #398, flashbang
   #486, the doors, monitor #323, Computer2 #15, the breakables) resolve by placement index exactly as
   the game resolves them [D + M Pickup_Create's glist hand-off]. **No model misses to fix.**
4. **Hurt volumes (0xE4) ported** — the level's one missing hazard kind. `crates/nf-game/src/hurt.rs`:
   `Hurt_Create` 0x17e870's record fields (gate channel, damage, pain type, sound, one-shot), the
   create-time disabled bit [M 0x17e870 tail], `Hurt_Update` 0x17e5d0's gate maintenance, and the
   contact damage `Player_DealWithObjHit` 0x194458:94-101 → `Player_HandlePain` (type 1..7 bypass
   armour [M 0x1902c8:84-104]) wired into `Sim::step_bond`. The disc chain, verified end to end:
   smashing cable light #392 writes ch 0x82 [M Destroy_CollisionHandler 0x12bd48:42] → Emitter #9
   enables (0x17b080:94, the on-channel gate) → its out ch 0x8D rises while particles live
   (0x17b080:163) → Hurt #13's gate (0x8D [D param 2]) opens → the cable gap above the bunker becomes
   lethal. The port's WorldFx emitter + props destroy already carried their halves; hurt.rs closed it.
   - `hurt_02_live` (nf-view, ISO-gated): fresh boot → the volume parks disabled, 20 real ticks inside
     hurt nothing; the 0x82 smash raises ch 0x8D through the port's own WorldFx emitter; standing in
     the volume drains 1.0×0.5 (difficulty) per contact tick; stepping out stops it. 3 unit tests in
     hurt.rs (gate off, capsule-vs-box, ungated volumes). All pass.
   - Param→field split for p0/p1/p4 is [S] (all three read 1 on the disc's only instance — the split
     cannot change #13's behaviour); the gate (p2) and sound (p3=0xffff) fields are [D]+[M].
5. **Census verified kinds that already work on 02** (through the real input path, existing tests):
   doors (3 sliders — door.rs golden + walkreal), TouchOnce/muxes/LOAD_LEVEL (walk_07000002: the
   refusal + the load both ways), E-grates (fix_ss_grate_live ×4 ON this level), pickups (pickup_live),
   ladders (ladder_02_live, ladder_underwater_02_live), wire + icon (wire_live, fix_wire2_live),
   sensors (sensor_live), fusebox stubs (correctly inert in game and port), ScriptPlayers
   (briefing/walk tests).

## What ran

- `NF_ISO=… python3 tools/test/suite.py` (fast runner, merged tree = my branch; origin/master is my
  branch's base): **2057 passed, 9 failed, 74 ignored (51 min)**, 496 binaries built ok.
  - 3 `disc_golden` — allowed on the slim ISO.
  - `probe_pylon_render` + `fix_0c_pylon_live` ×2 — the documented no-GPU environmental set (verified
    failing in prior sessions' logs: docs/reports/astronaut-guards-1b.md, attract-mode.md, ai-points.md).
  - `fix_death_restart_live`, `walk_07000013_live`, `walkreal_probe13` — CRASH exit −9 (the runner's
    kill signal, not an assertion): resource limits of this sandbox; none of them load 07000002 and my
    diff adds no code to their paths (the hurt loop is empty on every other level). Needs a re-run on
    the next machine to confirm.
- Directly: `cargo test -p nf-game --lib hurt::` 3/3, `--test hurt_02_live` 1/1, both re-run after the
  trace cleanup; nf-game + nf-view build clean (release).

## Remaining (next message continues from this branch)

Ordered by value:
1. **Breakables 0x20 shatter** (50 props on 02): drawn + solid + the glass pierce rule, but per-object
   health/shatter is not wired — blasts pass `breaks: &[]` (drone.rs:8350). Game side: Break_Update
   0x12bfd8, Break_ApplyDamage 0x12c128, Break_Kill 0x12b970.
2. **SS mask-2 operate** — the 11 deco SS (akari lamps, pavillion plants, Gong #293, lantern, poster)
   freeze: the obj+0xD0 bullet-presence half of `SS_Operate` 0x1a9df8:47-48 is not evaluated (same gap
   fix_0c_vent2 documented for 0700000c's pulleys).
3. **Monitor 0x30** (#323 secretbunker_radio2, p0=2) — E → camera view (`Monitor_Activate` 0x1b0058:
   find the sensor whose +0x5E == monitor's +0xF6, Camera_SetUpdator + Input_SetAction(0xE)); the
   port draws the radio static. Needs the camera-view state/flow + a test; the draw is the main
   agent's.
4. **Walk tests to add**: Computer1 #14 smash → ch 0x48 (objective 1's condition channel; props.rs
   Destroy works, no 02 test); the bunker plate chain #385/386/387 → AND #388 → 0x66 → #389 lock.
5. Rigid bodies 0xD8 ×21 (buckets, pans, hanging lamps) — unported everywhere (levels.md §7.1), drawn
   at rest.
6. The mission layer's ch 0x49/0x4A/0x4D writers (dialogue/chat) — off-disc, un-ported layer (031).

## Live checks for the main agent

1. **07000002, fresh boot, walk to the bunker cable gap** (the wire #187 crossing, bond view from the
   garden side): BEFORE shooting any cable light the arc should NOT hurt (the emitter is gated off —
   ch 0x82 off [M 0x17b080:94]). gdbwatch on `switch_channels[0x8D]` (EE 0x2a1e0d area — the
   switch_channels base per the map) expecting 0 while Bond stands in the Hurt #13 box
   (−60.99, 7.33, −30.16).
2. Same spot AFTER shooting the middle cable light #392: `switch_channels[0x82]` → 1, then
   `switch_channels[0x8D]` → 1 within a tick, and standing in the box drains Bond's health (type-1
   pain flashes, no armour absorb). This pins the arc-starts-on-smash polarity — the code says the
   smash ENABLES the emitter; if the playtest shows the arc burning before the smash, the on-channel
   gate needs a second look (Emiter_Update 0x17b080:94's piVar1[0xc] slot mapping).
3. E on the bunker radio #323 (−24.08, 1.14, 29.08): expect the camera-view mode of sensor #406/#423's
   group (Monitor_Activate 0x1b0058) — today the port does nothing; the check pins the target
   behaviour for item 3 above.

## Usage

not visible to me. (This sandbox exposes no token/context-window/cost figures to the worker; the
tool reports in this session showed no usage panel.)
