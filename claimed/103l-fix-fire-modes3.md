## Your task: `fire-modes3`, weapons: every fire mode against the PCSX2 table

Base: origin/master. If `s/weapon-matrix2` is not on master yet, merge it into your branch first (it carries the
upgraded runner `nf-extract weapon-matrix` and docs/reports/weapon-matrix2.md, which this task builds on).

docs/live/2026-10-13-weapon-modes-pcsx2.md holds the main agent's PCSX2 table of all 32 multiplayer weapons and their
modes (the Sentinel's rocket camera, the MGL's impact vs ~3-4 s delay, the AIMS-20 burst/grenade, the Samurai
overcharge/beam, the crossbow 0x11, the mode button). For EVERY row: find the game code behind the mode (weapon data
fields, `Player_Weapon` 0x1a24c8's mode switch, `Bullet_init` / `Bullet_DoTrails` flags), then check the port does the
same: what the press does, the label, projectile kind, fuse/impact, burst count and interval, charge time, beam
behaviour, zoom steps, ammo drawn per shot. Table in the report: weapon x mode -> game vs port, every mismatch fixed or
explained with evidence. The weapon-matrix runner's M/G/X marks are the starting signal.

Tests (NF_ISO): one test per weapon family with modes (pistol silencer, burst rifle, grenade attachment, launcher
guided/unguided, MGL impact/delay, Samurai charge/beam, crossbow, sniper zoom) asserting the measured numbers; sister
assertions.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report; a test on the drawn set alone is not enough (several fixes passed their tests while the rendered
frame still showed the bug).

Files: `crates/nf-game/src/gun.rs`, `crates/nf-view/src/scene.rs` only where the mode reaches the view (small local
edits), a new `crates/nf-view/tests/fire_modes3_live.rs`.

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

**Branch:** `s/fire-modes3`.

---
claimed-by: sandbox1 at 2026-10-14T14:36Z
