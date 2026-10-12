Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `fix-actors`, every non-guard actor: allies, civilians, VIPs, bosses and the spawners that bring actors in

Guards (hostile drones) are mostly ported. The other actors that make missions playable are not, or only partly. Build
the shared framework the per-mission tasks (032–037) use, and port each actor type the game has.

1. **Inventory every actor type on the disc.** For each, give its drone type (`dtype`, placement param 15), team, mode,
   skin, the levels it appears on, and its role. Use `nf-extract drones` and the placements on every level; `docs/systems/drone-ai.md` lists the
   modes. Write the table into `docs/systems/actors.md`.
2. **Allies who follow or lead Bond** (team 3; for example the "follow me" NPCs, and 07000001's skin-0x4F NPC): their
   follow and lead states, waiting for Bond, opening doors, dialogue triggers, and what happens if they die (mission
   failure).
3. **Civilians and VIPs:** fleeing, cowering, surrendering (`docs/systems/drone-surrender.md`), the scared radius,
   and the mission-failure rule for killing them. Bond's bullets do hurt team 2/3 actors in the game:
   `NDrone2_DealWithObjHit` 0x13fff0 case 5 gates only drone shooters, not the player.
4. **Bosses:** each boss's states, attacks, health, phases and defeat condition, and what defeat triggers (a cut-scene,
   the level end).
5. **Spawners** (G-025, `DroneSpawner_Control`): spawn timing, caps, the channels that start and stop them, and
   reinforcements on alarm. Actors must arrive when the game sends them.
6. **Scripted actors** (driven by script players / NIS): hand-over between a script and the AI, and walking a
   scripted path.

Tests (NF_ISO): for each actor type, a scenario through `nf_view::load` + `dev::scenario` that drives its key behaviour
(an ally follows Bond through a door; a civilian flees at a shot and the mission fails when one is killed; a spawner's
wave arrives on its channel; a boss takes damage and ends its fight), with sister assertions.

**Working alongside the other map tasks.** The per-mission tasks 032–037 depend on your framework, and run at the same time, and
`031-fix-mission` builds the objective system they all use.
- Pull `master` at the start and before every push; if `crates/nf-game/src/mission.rs` exists, build on it instead of
  writing your own.
- Keep your level-specific code in its own file (`crates/nf-game/src/mission/<level code>.rs`, or a table entry), and
  your docs in `docs/levels/<level code>.md`.
- Keep edits to shared files (`scene.rs`, `drone.rs`, `triggers.rs`, `nis.rs`, `lib.rs`) small and additive, and list
  them in your report.
- Never reformat or reorder a shared file.

Files: `docs/systems/actors.md` (new), the drone AI modules in `crates/nf-game/src/` (keep new behaviour in new
files such as `ally.rs`, `boss.rs`, `spawner.rs`), and minimal hooks in `drone.rs` / `scene.rs`. Update G-025 and G-026 in
`docs/GAPS.md`.

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

**Branch:** `s/fix-actors`.

---
claimed-by: sandbox3 at 2026-10-12T10:45Z

---
## Report (done, 2026-10-12T11:58Z)

