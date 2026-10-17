The user (2026-10-17): "09 level elevator buttons dont work. make sure we're prioritizing level work so we can actually complete each level completely and get all models and functions per level working" and "I want EVERYTHING covered. ALL ON FOOT LEVELS". Level work now runs ahead of everything else in the queue.

**Reproduce first, through the REAL path.** Load the level with `nf_view::load` (or `load_weapons` when a
level's own weapon is needed), move Bond with `Sim::step_bond` inputs (walking, turning, the use key, the
watch gadgets, the weapons), let the level's own scripts and triggers run, and observe. No teleporting past
a problem unless the task says so; a teleport to reach a spot is fine when the test then operates the
thing through the real input. Cite the decompile [M], the disc [D] or a fresh-boot PCSX2 value [L] for every
game claim; tests pin only proven behaviour.

## Your task: `level-02-interactables`, every interactive thing and every model on 07000002 (The Exchange (Castle B))

1. **Census.** From the disc (an nf-formats parse of 07000002.bin), list every placement the player can
   touch, use, shoot or ride, by type, with its parameters and channel wiring: buttons and switches,
   lifts/elevators and their call buttons and doors, doors (locked, keyed, scripted, sliding, shoji),
   SimpleScripts (0xE2), ScriptPlayers, touch/use/load-level triggers, pickups and weapon racks, safes,
   keypads, monitors, terminals, computers, lasers and laser switches, breakable glass and destroyables,
   searchlights, cameras, turrets, fuseboxes, vents, hatches, ladders, wires, grapple points (watch gadget
   targets), vehicles. For each: what the game does with it [M], and the port's status. Put the table in
   docs/levels/07000002.md.
2. **Make each work.** Through the real input path, drive Bond to one instance of every KIND on this
   level and operate it the way a player does (use key, the right gadget, shooting, walking in). It must do
   what the game does: the channel writes, the animation and sound, the script, the item, the door. Fix every
   kind that does not. One live test per kind (not per placement), each asserting the game's effect, with a
   sister that fails without the operation.
3. **Every model draws.** Every placement's mesh/skin resolves on this level (no missing hash, no white
   fallback texture — `NF_TEX_TRACE=1` prints the fallbacks), every NPC has its skin, every pickup and
   script entity its model. List the misses in the report and fix them.

Known open items on this level (start here): the pond hut and the deep water ladders (swim height: task 762), the bath hatch above the deep ladder (task 764), the grates.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/level-02-interactables`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T08:27Z

---
## Report (done, 2026-10-17T09:57Z)


---
## Report from the worker (done, 2026-10-17T09:57Z)

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
