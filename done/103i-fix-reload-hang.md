## Your task: `reload-hang`, weapons: the reload state machine hangs (state 6, clip 0)

Base: origin/master. If `s/weapon-matrix2` is not on master yet, merge it into your branch first (it carries the
upgraded runner `nf-extract weapon-matrix` and docs/reports/weapon-matrix2.md, which this task builds on).

docs/reports/weapon-matrix2.md ("The remaining red cells", "The upgraded runner's full sweep") pins ONE bug behind
most of the upgraded sweep's red (D 1278, R 226, M 152): after a switch lands on the P2K's silenced twin 0x03 with an
empty clip, the auto-reload starts and the shell loop sticks in state 6 with clip 0 forever (`rel=6`, carried 4), and
every later switch is refused while the view anim isn't idle [M `Player_WeaponChange` 0x1a6f80:33-47]. The AT-420
Sentinel 0x2D's reload (the 5/6/7 machine) never returns either: same family. In play this is a weapon that can never
fire or be put away again.

Find the game's reload loop and its exits: `Player_WeaponFiring` 0x1a5460 (the shell refill), the reload restart gate
(the report's `reload_loop_restarted`), and the view-script queue (+0xA4/+0xA8/+0xAC) the port's scene.rs view-script
block replays. Port the exits so the loop refills a shell per pass and returns to idle when the clip is full or carried
is empty, for every reload kind (magazine, per-shell, rockets). Tests (NF_ISO, failing first on master): empty 0x03 ->
reload -> idle with clip refilled; switch away mid-reload and back; 0x2D fire -> reload -> idle; a shotgun's per-shell
loop interrupted by fire. Sister assertions. Then re-run `nf-extract weapon-matrix` on the full 27 levels and put the
new red counts next to weapon-matrix2's in the report.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report; a test on the drawn set alone is not enough (several fixes passed their tests while the rendered
frame still showed the bug).

Files: `crates/nf-game/src/gun.rs`, the view-script block in `crates/nf-view/src/scene.rs` (keep the edit small and
local; other branches touch scene.rs), `crates/nf-view/tests/fix_reload_hang_live.rs`.

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

**Branch:** `s/reload-hang`.

---
claimed-by: sandbox4 at 2026-10-14T12:50Z

---
## Report (done, 2026-10-14T15:45Z)

