## Your task: `fix-npc-doors-escort`, NPCs and doors, and the Mayhew escort

These items are from the user's playtest of 2026-10-12 (the user's own words are quoted). Screenshots are referenced by their `--bond` spot: run `nf-view <iso> <level> --bond x,y,z,yaw,pitch` to stand there.

**Start from `s/walkreal-a` (812a0fd, unmerged).** It has a first port of `NDrone2_OpenDoor` 0x14aa38 from `ControlSTANDARD` 0x1490f0:242 in `drone.rs` (the +0xBB0 60-tick retry), but it stands in the game's collision hit object (+0xBA8, class 0x1C) with "the nearest door within 2.0" [S] and has no fail-first test. Port the real hit-object path, test it fail-first, and bring over its walker harness (`crates/nf-view/tests/common/walker.rs`) and the 07000001 walk if they help. Do NOT bring `agent-ctx/` (scratch copies of the field docs).

1. **NPCs can't open doors**, on every level the user played ("npcs cant open doors"; on 07000007 "none of the NPCs can open the doors"). Port how a drone opens a door on its path (the door's NPC-use path, the AI's door handling) so guards, guests and Mayhew walk through.
2. **07000001, the escort:** "mahew doesnt really follow properly"; the screenshot (`07000001 --bond 6.28,6.47,6.20,6.338,-0.083`) shows Mayhew stuck facing a shoji door. Fix his follow behaviour against the game's ally route code (`docs/fields/hitdata-spawner-ally.md`).
3. **The level-exit door** "should be able to open without him walking all the way to it". Find the exit's real condition.
4. **07000002: the grate above the bed in the bunker** "should be able to be opened but it wont open"; also "There should be grates that you can interact with that will open up before you climb into the vent". Port that interaction.

Files: `crates/nf-game/src/{door,ally,drone,useobj}.rs`

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in your report. Pull `master` before you start and before every push: other workers merge into it all day.

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

**Branch:** `s/fix-npc-doors-escort`.

---
claimed-by: sandbox5 at 2026-10-13T02:59Z
