## Your task: `aim-raise`, aiming: the raise animation gates the aim

Two merged fixes left the same deviation: the game's view state 0xF (`Player_SetWeaponAnimObj` 0x1a2e38:554-561) sets
the aiming bit only when the aim-raise script (+0xB8 @1.25) finishes, and `Player_AutoAim` 0x1a7c98 keeps scanning
until then (fix-guard-misc2's report, "What's left"; fix-weapon-anims' deviation 2). The port turns the aim camera,
the free-aim window and the auto-aim lock on the instant the button goes down. Port the gate: the aim bit, the zoom
start and the auto-aim lock wait for the raise to end, for weapons with +0xB8; weapons without it aim at once as the
game does. Tests (NF_ISO): sniper and AIMS-20: zoom and free-aim begin on the raise's last tick (numbers); the P2K
aims immediately; the 0c sniper test still passes; sister assertions.

Files: `crates/nf-view/src/scene.rs` (the aim block, small and local), `crates/nf-game/src/player.rs` if needed.

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

**Branch:** `s/aim-raise`.

---
claimed-by: sandbox3 at 2026-10-14T16:50Z

---
## Report (done, 2026-10-14T18:26Z)

Closed by the driver: the report is on `s/fix-aim-raise` (docs/reports/fix-aim-raise.md); the worker didn't run finish.sh.
