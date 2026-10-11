# Plan v2: verify against the real game, port subsystems whole

Written 2026-10-10 after the first multi-session week (WORKLOG A-1..A-11, B-1..B-11, C-1..C-15). This supersedes the per-playtest workflow; `docs/PLAN.md` still holds the phase roadmap.

## 1. What the first week showed

| Problem | Evidence | Fix in this plan |
|---|---|---|
| Nobody could check the work except the user. | Sandboxes have no GPU and no PCSX2, and their tests asserted their own assumptions. "Fixed, tests pass" was followed by "still broken in play" for most items: the truck level end, doors, the wire, pickups, scenarios. | Reference traces from PCSX2 (§3). Sandboxes test against recorded game state, not their own reading. |
| Wrong facts about data layouts. | The trigger gate param read as 10 instead of 2; pickup params read from `extra` instead of `params`; wire 0x3A vs 0x3E; the door proximity radius; `+0x4F8` vision flags. | A shared structure atlas (§2), verified against EE snapshots, before more porting. |
| Features built only as far as the playtest needed. | Guard AI: 32 of 268 functions ported, 205 partial. | Port subsystems whole, in dependency order (§4). Progress is measured by port-map completion. |
| Sessions breaking each other's work. | C-13 overwrote B's `dev.rs`/`hud.rs`. B-10 froze every cut-scene. A-11's frustum cull hid the truck and the HUD. | One integrator (§5): sessions work on branches; the main agent merges after the full suite plus trace and image checks. |

## 2. Phase 0a: the structure atlas (sandbox work, about 2 days)

A typed reference for every runtime structure the port touches, in `docs/atlas/<struct>.md`. One file per struct:

- name, size and allocation site (the creating function and its address);
- a field table: offset, type, name, meaning, writers (function@address), readers, evidence tag;
- the placement-param → field mapping for structs created from placements (`*_Create` reading `level_tag+0x2C+4k`);
- a check against a live EE snapshot (`work/ee_start_*.bin`): at least 3 fields per struct read from a real instance, with values that make sense.

Structs, in priority order:

1. the object/control list entry and its handler table;
2. the Drone (`NDrone2`);
3. the player (`BLData`, player object, `PlayerSetting`);
4. `weapon_data` (0x10C records);
5. the trigger (`DOORINFO`-based ext);
6. the door;
7. the pickup;
8. the script player and the NIS stream state;
9. the switch channels;
10. the searchlight;
11. the wire/grapple;
12. the cel/room;
13. the GS leaf state.

The Rust code then names fields after the atlas. Any `+0xNNN` in a comment must exist in the atlas.

## 3. Phase 0b: reference traces from PCSX2 (main agent, this machine)

Tools in `tools/trace/` (`goto_level.py`, `ai_trace.py`, `pad_log.py`, `pine.py`) extended into one harness:

1. **`trace_record.py`**:
   - load a level with `goto_level`;
   - drive the pad from a script (an input file of per-frame pad states, written into the game's pad buffer through PINE);
   - every in-game frame, dump a fixed field set:
     - Bond's position and yaw;
     - the health, state and position of every drone;
     - the RNG state;
     - the switch-channel bitmap;
     - the trigger and door states;
     - the script-player frames.
   - Output `tests/traces/<name>.trace` as text TSV; numbers only, no game bytes.
2. **`nf-view --replay-trace <name>`** plus a headless test helper: run the same inputs through Sim and compare tick by tick. The first divergence is reported as the tick and the field, e.g. "tick 5412: channel 0x73 game=1 port=0".
3. **Reference image pairs:**
   - a PCSX2 screenshot and an `nf-view --frame --shot` at the same camera (`--cam` from the trace), compared per region;
   - stored as numbers (mean and variance per tile), not images, so nothing copyrighted is committed;
   - only the main agent runs these (GPU).

First traces:

| Trace | Inputs | Covers |
|---|---|---|
| `05-truck` | Gatehouse plate, ride, park, level end. | Scripts, channels, LOAD_LEVEL, riding. |
| `05-gate-fight` | Walk to the gate; guards engage. | Drone AI, firing, damage, RNG. |
| `01-doors` | Open and close two doors with use. | Door state machine and channels. |
| `01-raptor` | Kill the Raptor guards; pick up the drops. | Drops, pickups, inventory. |
| `05-weapons` | Cycle all weapons; fire; reload. | Weapon state machine, ammo. |
| `02-grenade` | Stand in range of a grenade carrier. | The grenade decision and throw. |

## 4. Phase 1: port subsystems whole, in dependency order

The order follows the dependencies: each one rests on the ones before it.

1. **The core loop:** `Game_Run`, the control list and update order, the ScriptCam pause, triggers, switch channels, script players, the level flow.
2. **The player:** movement, collision, use, ladders, wire, grapple, damage, armour, death.
3. **Weapons:** all `weapon_data` weapons, firing, reload states, melee, grenades, scope, pickups and drops.
4. **Guards:** the rest of the 268 functions, in port-map order.
5. **Objects and levels:** doors, searchlights, explosions, all 36 on-foot levels.
6. **Rendering (main agent):** character and view-model lighting (TWEAK, ambient, VU1), lanterns and glows, the HUD remainder, correct culling (cels and portals, G-016).

"Done" for a subsystem means every function in its port map is ported or justified, and the relevant traces match. A playtest is not the bar.

## 5. Workflow

- **Branches:** each sandbox session works on `s/<letter>-<topic>` and pushes there, never to `master`.
- **The integrator (main agent):** merges branches into `master` after:
  - a rebase;
  - the full NF_ISO suite;
  - the trace replays;
  - for rendering or HUD changes, image checks.

  The integrator also owns `nf-view` window and GPU code, rendering, and the trace harness.
- **Session size:** a 2-hour sandbox gets one bounded task: an atlas struct, a port-map section, or a set of functions against a trace. Not an open-ended "fix the playtest".
- **The user:** playtests at milestones with `dist/check.ps1` (one checklist per milestone), not every round.

## 6. Guardrails for the long-term goals (mods, multiplayer, co-op)

- **Determinism is a hard rule:**
  - the shared bit-exact RNG;
  - the fixed 60 Hz;
  - no wall-clock time, HashMap iteration order or float non-determinism in Sim;
  - no rendering or I/O state in Sim.

  Lockstep or rollback netcode depends on it. The trace replays enforce it.
- **Players are a list internally** (the game's `glb_players`), even with one player. That keeps co-op a smaller step later.
- **The game's multiplayer comes first:** ACTION.ELF has split-screen multiplayer and bots (drone team 2). Port them before designing new netcode.
- **Content goes through our own data structures:** disc formats load into port-owned structs (weapons, levels, placements). New guns and maps later use the same structs, which gives a mod format.
- **Distribution:** the user supplies their own disc image; the port never ships game files. Servers and mods follow the same rule.

## 7. Week schedule

| Day | Sandboxes | Main agent |
|---|---|---|
| 1 | Atlas structs 1–4 (one per session). | `trace_record.py` with pad injection; the `05-truck` trace. |
| 2 | Atlas structs 5–13; core-loop port map. | The trace replay test helper; `05-gate-fight` and `01-doors` traces; the branch workflow. |
| 3–5 | The core loop against `05-truck` and `01-doors`; then the player and weapons against their traces. | Merging; character lighting; image checks. |
| End of week | | The user's milestone playtest. |
