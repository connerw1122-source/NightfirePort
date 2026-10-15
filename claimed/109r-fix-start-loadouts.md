## Your task: `start-loadouts`, every level's start loadout from the game

The main agent found in PCSX2 that the real start loadouts differ by level, while the port gives nearly every story level
the same owned set `[1, 2, 4a, 4e, 50, 54]` (read `docs/live/2026-10-14-weapon-measurements-pcsx2.md`, "Which levels
carry what"): 07000014 starts with the Delta Repeater crossbow 0x11 in hand ("3-9 Single"); 07000015's gadget cycle is
Laser -> Micro-Camera -> Decryptor (no stunner — the port starts Bond on 15 with the stunner); 07000005's is Stunner ->
Laser -> Grapple. Find where the game sets the per-level loadout (`Player_InitAmmoWeapons` 0x1a17f8, the mission/level
table it reads, the carried-over inventory between a mission's levels via RamSave/RamLoad) and port it exactly: weapons,
gadgets, clips and spare ammo, the held weapon at the start. Table in the report: level -> the game's loadout vs the
port's before. Tests (NF_ISO): for 05, 14 and 15, the start inventory and held weapon equal the game's; sister
assertions.

Files: `crates/nf-game/src/{player,pickup,progress}.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/start-loadouts`.

---
claimed-by: sandbox2 at 2026-10-15T05:51Z
