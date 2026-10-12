## Your task: `fix-spawner-p7`, the spawner's param 7 is a KILL channel (measured live)

**The measurement** (main agent, 2026-10-12, real game in the GDB-server PCSX2, 07000005; recorded in
`docs/fields/hitdata-spawner-ally.md` §Live result): the gate spawner (data block at obj+0xE0; p4 = 0x22, p7 = 0x43,
maxalive 2) spawned its two guards on ch 0x22. Ch 0x43 was then raised. Ch 0x43 first plays a script whose event
calls `Drone_EnableAll(0)` (`Script_EventHandler` 0x1d94c0): `NPCGlobals` = 0, so `DroneSpawner_Control` 0x137e68
returns at its first line until `Script_Free` 0x1da880 re-enables NPCs. Then `Control` latched `+0x20 = 1` and took
the cleanup block (0x137e68:235-288): **both spawned guards were removed** (15 → 13 drones), the spawner's slot array
was zeroed, `+0x1E` = 0, and the spawner set its own `obj+0xFE |= 1`. That is triggers.md's original "kill channel"
reading. fix-actors' "ARM channel" reading (`crates/nf-game/src/spawners.rs` `arm`/`armed`, docs/systems/actors.md §5)
is wrong.

**The level split that explains fix-actors' playtest** (map-tower2's stairwell spawners on 07000011 "died instead of
arriving" under the old kill code): the cleanup is level-dependent [M 0x137e68:235-262]:
- level < 0x07000011, or 0x07000014/15, or > 0x07000016 except 0x0700004A: **every** spawned drone gets
  `obj+0xFE |= 1` (removed), the slot is cleared, and the spawner deletes itself;
- 0x07000011..0x07000013, 0x07000016 and 0x0700004A: only drones whose `obj+0xFC` byte is 0 are removed. The
  spawner counts the rest (`sVar8`) and returns; it deletes itself only once none remain. On 0x07000016 each removed
  drone first gets `DroneFunc_SetDeathChannel`.
Find what `obj+0xFC` is for a drone (every writer; the field docs in `docs/fields/drone.md` /
`docs/fields/hitdata-spawner-ally.md` are the starting point) and port it.

**Also port the early-outs** at the top of `Control`: `NPCGlobals` byte 0 (cleared by `Drone_EnableAll(0)` during
scripts), `Drone_bDisableSystem`, the spawner's own `obj+0xFE` bit 0, the block's +0 < 2 / +4 / +0x34 checks.
Check how the port's script player models `Drone_EnableAll` and wire the spawner to it.

Tests (NF_ISO, through the real input path or `dev::scenario`, fail-first shown):
1. 07000005: ch 0x22 → the two gate guards spawn; ch 0x43 → after the wall-hug script ends, both are gone and the
   spawner never spawns again (the sister: without ch 0x43 they stay).
2. 07000011: map-tower2's stairwell case. The guards the game keeps (`obj+0xFC` ≠ 0) stay; the stairwell guards
   still arrive as they do now.
3. A spawner gated while NPCs are disabled does nothing until they're re-enabled.

Files: `crates/nf-game/src/spawners.rs`, `docs/systems/actors.md` §5 (correct it), `docs/systems/triggers.md`.
**Shared files** (`drone.rs`, `scene.rs`): minimal hooks, listed in your report. Pull `master` before you start and
before every push.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-spawner-p7`.
