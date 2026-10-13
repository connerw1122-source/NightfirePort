## Your task: `fix-progress`, scoring, 007 bonuses, mission ratings, difficulty and the save file

The game tracks a score per mission (time, accuracy, kills, health, 007 bonus moves), rates it, and unlocks things. Difficulty (Agent / Secret Agent / 00 Agent) changes damage, guard accuracy, armour and the objective list. The port has the 007 bonus sound, but no tally, no rating, no difficulty effects and no save. `180-fix-menus` owns the screens; you own the data and the logic underneath, and give the menus an API to call.

1. **The mission tally:** every counter the end-of-mission screen shows, where the game increments it (`Player_*`, `Bonus_*`, `Mission_*`, `Stats_*`; find them in the symtab), and the 007-bonus events per level.
2. **The rating:** the thresholds per difficulty and per level, the medal and point totals, and what each unlocks.
3. **Difficulty:** the global and where it's read. The damage and accuracy multipliers, the armour and health pickups, the per-difficulty objective lists (with `070-fix-mission`, which owns objectives: use its data, don't fork it), and anything that spawns differently per difficulty.
4. **The save:** what the memory-card save holds (progress, unlocks, options, best scores) and its layout. Store it as a local file in the same fields, and keep it deterministic (no wall-clock time in the Sim).

Tests: a scripted run through `nf_view::load` and `step_bond` on 07000005 that kills N guards and takes one 007 bonus gives the tally you'd expect from the decompile; a difficulty switch changes a guard's damage to Bond by the decompile's factor (with a sister assertion at the default difficulty); and the save round-trips.

Files: a new `crates/nf-game/src/progress.rs`, minimal hooks in `crates/nf-view/src/scene.rs`, and `docs/systems/progress.md` (new).

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

**Branch:** `s/fix-progress`.

---
claimed-by: sandbox2 at 2026-10-13T13:30Z
