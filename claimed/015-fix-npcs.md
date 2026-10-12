Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `fix-npcs`, characters that are drawn but don't act, and the AI items from the user's playtests

These items were attached to every queued task by mistake. They're yours alone now, so no other worker will touch them. Start with C2, because it blocks play on 07000002 and 07000006.

**Also, confirmed by the user on 07000002** (after `fix-drone2`; they didn't press R): the guard in the bathhouse standing next to a civilian woman is still drawn, never attacks, and can't be damaged. Towards the end of 07000002, another NPC next to a hostage does the same. `fix-drone2` couldn't reproduce this headless, so its test doesn't match play.
- Run the real viewer path (`nf_view::load`, the level start, then walk there) and dump every drone that is drawn but either not hittable or not running AI, with present / started / enabled / in_llist / held / drawn / team / mode / dtypenew / channel.
- These may be scripted or ally characters (team 3, mode 5/9, or NIS-driven), not guards. Find what the game does with them, and port it.
**More from the user's latest playtest:**
- **07000005, between the gatehouse and the castle gate:** a patrolling guard walks one way while his upper body looks the opposite way, as if his spine were flipped. Check the patrol look-at and aim-blend (torso twist) direction and clamp.
- **07000006:** stationary guards rotate to follow Bond unrealistically (their whole body or torso spins fully around), never attack, and can't be hit. Probably the same class as C2: find what they are, and port their real behaviour.
- **07000006:** standing in a searchlight beam doesn't raise the alarm. Port the searchlight's spotted → alarm path (channel, drone alert, music event) from `docs/atlas/searchlight.md` and `Searchlight_Update`.

**User retest:** the 07000007 guests no longer T-pose, but they **don't move or walk** at all, despite `fix-drone3`'s CivilianPatrol and PartyGirl port. Reproduce it through `nf_view::load` and the real tick path: check that they have patrol paths, that the drone update runs for them (`npc_enabled`, the llist, start state), and that the walk animation and movement are applied. Fix it.

**Notes from earlier sessions:**
- `fix-ladder2` researched the searchlight → alarm music chain (WORKLOG, the fix-ladder2 entry): `Searchlight_Update` 0x1cb900 → `ch[param3]` → `DroneFunc_CheckAlarmRaised` 0x1453a8 counts `ch[0x37]` → `Music_Event(2,1)` → `UpdateMusicalEvents` 0x114e70 → the per-level `UpdateMFX_*` (07000006: `UpdateMFX_01CastleB` 0x113060). The port runs the castle-A updater only for 07000005, so add the per-level dispatch in `crates/nf-audio/src/music.rs`.
- `fix-ladder2` also reports that `fix-drone4` already fixed the 07000007 guests (CivilianMission 0x16 and the route-chain fix). Retest that headless on master first; only work on it if they still don't walk.
- `fix-drone2`'s headless test didn't reproduce the 07000002 guard. Your test must run the same path as the window: `nf_view::load`, the level start (intro and spawners), then walking Bond to the spot.

Files: `crates/nf-game/src/drone.rs` and the drone AI modules, `crates/nf-view/src/scene.rs` (minimal), `crates/nf-audio/src/music.rs` (the per-level dispatch), and `docs/systems/drone-ai.md`. Update C1/C2/C3 status in your report.

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

**Branch:** `s/fix-npcs`.

---
claimed-by: sandbox1 at 2026-10-12T04:04Z
