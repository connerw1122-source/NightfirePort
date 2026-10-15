## Your task: `hud-ammo-odd`, the ammo pane's numbers for the Samurai, the detonator and thrown weapons

The main agent's dry run of the playtest checks on master (`nf-view <iso> 07000005 --dev --give <id>`) showed ammo
pane numbers that look wrong:
- **Phoenix Samurai** (`--give 0x32`, "Overcharge"): the pane reads just `0` (with a red bar beside it) while the gun
  fires bolts fine.
- **Remote Mine detonator** (`--give 0x37,0x38`, press B for "Detonate"): the pane reads `500-4`.
- **Thrown weapons** (Remote Mine `1-4`, frag `1-3`, satchel `1-3`): the counts never drop after a throw. The dev
  window's infinite ammo is ON by default in `--dev` (`dev::Dev { infinite_ammo: true }`), which may explain this one:
  check it with infinite ammo OFF before calling it a bug.

Find what the game's ammo pane draws for each of these records [the pane code: `HUD`/`AmmoPane` in the decompile; the
records' +0x90 ammo type, +0x92 clip cap, the twin rules — see `docs/formats/weapon_data.md`], measure where you can
from the EE snapshots in `work/` (`ee_snapshot.bin`, `ee_start_05`), and make the port's pane match. Tests (NF_ISO)
for the pane text of each record through `nf_view::hud::overlay`. If something can only be settled in PCSX2, list the
exact live check (level, weapon, what to read on screen) in your report instead of guessing.

Files: `crates/nf-view/src/hud.rs` (or wherever the ammo pane is drawn), a new test file.

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

**Branch:** `s/hud-ammo-odd`.
