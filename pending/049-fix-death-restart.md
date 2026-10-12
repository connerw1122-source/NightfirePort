Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `fix-death-restart`, dying, failing and restarting like the game

The mission layer (fix-mission, merged) raises ch 0x62 on death and ch 99 on failure, and the port restarts the
level by reloading it. The game does more, and the player sees all of it:

1. **The death sequence:** `Player_HandleDeath` 0x190bb0 and what follows it: the death camera and animation, the
   red fade (`Mission_Update` 0x182c60's MissionState 3 path), the "Mission Failed" presentation, the timing. Port it
   end to end in the sim; the main agent checks the render.
2. **The fail and restart menu:** the 0x40000042 menu `Mission_Update` opens (Restart / Quit). Port the logic and an
   API for `180-fix-menus` to draw it. Until the menu exists, the port's auto-restart stays, behind a flag.
3. **What a restart resets and keeps:** `ResetMap_LevelToLoad` with `IsWarmReset`, the level-start loadout versus
   the carried state (`Player_RamSave`), the mission states (`Mission_Init` resets the current and future parts only,
   [M 0x182270:68-129]), and the RNG. A restart must give the same start as a fresh load of that level within the
   mission. Test it: die on 07000005 after picking things up, restart, and assert the state equals a clean start.
4. **Falling deaths and the scripted ones** (the 07000011 atrium fall, the chasm volumes): they share this path.
   Check they all reach it.

Tests (NF_ISO): death → the sequence's stages tick by tick → restart → a state identical to a clean load; mission
fail (the 07000009 alarm) → the same; sisters: a near-fatal hit doesn't trigger any of it.

Files: `crates/nf-game/src/mission.rs` (the restart flow, additive), a new `crates/nf-game/src/death.rs`, `docs/systems/mission.md`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in
your report. Put new behaviour in new files. Pull `master` before you start and before every push: other workers
merge into it all day.

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

**Branch:** `s/fix-death-restart`.
