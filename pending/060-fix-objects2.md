Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-objects2`, 07000008 (the gondola level) objects

User playtest after 6faffb4, on 07000008:
1. **Invisible wall:** after the opening cut-scene, jumping down and going down the stairs is blocked by an invisible wall. The user has reported it several times. Find the collision with no drawn mesh there (log the blocking triangle's object and placement on contact, the way `NF_COLL` does), and port what it really is: a door, a barrier opened by a channel, or an object the cut-scene should remove.
2. **The safe and the gondola:** inside, there's a safe Bond opens with the laser watch, and from there he can activate the gondola so it runs. Find the safe and gondola objects (their placement types, scripts and channels), the laser-watch interaction (it depends on the gadget port; if that isn't ported yet, port the object side so a channel can drive it, and document the gadget hook), and the gondola's movement (a script-driven moving platform: collision and riding, like the 07000005 truck).

Files: object modules under `crates/nf-game/src/`, plus trigger and script hooks.

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

**Also, confirmed by the user on 07000002** (after `fix-drone2`; they didn't press R): the guard in the bathhouse standing next to a civilian woman is still drawn, never attacks, and can't be damaged. Towards the end of 07000002, another NPC next to a hostage does the same. `fix-drone2` couldn't reproduce this headless, so its test doesn't match play.
- Run the real viewer path (`nf_view::load`, the level start, then walk there) and dump every drone that is drawn but either not hittable or not running AI, with present / started / enabled / in_llist / held / drawn / team / mode / dtypenew / channel.
- These may be scripted or ally characters (team 3, mode 5/9, or NIS-driven), not guards. Find what the game does with them, and port it.
**More from the user's latest playtest:**
- **07000005, between the gatehouse and the castle gate:** a patrolling guard walks one way while his upper body looks the opposite way, as if his spine were flipped. Check the patrol look-at and aim-blend (torso twist) direction and clamp.
- **07000006:** stationary guards rotate to follow Bond unrealistically (their whole body or torso spins fully around), never attack, and can't be hit. Probably the same class as C2: find what they are, and port their real behaviour.
- **07000006:** standing in a searchlight beam doesn't raise the alarm. Port the searchlight's spotted → alarm path (channel, drone alert, music event) from `docs/atlas/searchlight.md` and `Searchlight_Update`.

**User retest:** the 07000007 guests no longer T-pose, but they **don't move or walk** at all, despite `fix-drone3`'s CivilianPatrol and PartyGirl port. Reproduce it through `nf_view::load` and the real tick path: check that they have patrol paths, that the drone update runs for them (`npc_enabled`, the llist, start state), and that the walk animation and movement are applied. Fix it.
**Grenade follow-ups (user retest after `fix-player3`):**
- The thrown grenade kills a guard on impact, bounces back and damages Bond, and there's a bounce sound but **no explosion sound or effect**. Check the impact behaviour per grenade type (the frag fuse versus contact), the explosion sound id and the listener, and port the explosion visual (script 0x6000052, `Explode_Create`).
- **The flashbang (stun grenade) kills enemies**, even when it isn't thrown at them. A stun grenade must stun, not deal lethal damage: per-type damage (`weapon_data`, the 0x36 flags), and the stun receiving states (`StunGrenadeImpact`/`Loop`/`Recover`).
**Branch:** `s/fix-objects2`.
