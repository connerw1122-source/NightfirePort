## Your task: `gadgets-live`, the laser/stunner/AIMS-20/Oddjob findings on today's master

The user's playtest (2026-10-13 night) on today's master, after 101e-fix-laser-beam and 101f-fix-qclaw merged:
- **Laser (07000008 safe, `--bond 155.19,43.12,-113.38,1.57,0 --give 0x4e`): "the laser isn't visible".** PCSX2 at the
  same spot, holding fire (main agent, 2026-10-13): the watch hand comes up; a SHORT red beam leaves the watch toward
  the crosshair; a strong RED GLOW lights the nearby wall (a dynamic light at the muzzle/hit); a vertical red CHARGE BAR
  appears on the HUD right with the number counting down (31 -> 25 -> 19 -> 9 -> 0 over ~1 s at 1 per tick) and the
  beam stops when it reaches 0. The port shows none of that. Port the beam draw (`Draw_LaserBeam` 0x1a4560 via
  `Player_MuzzleFlash` 0x1a4298:131), its light, and the charge bar + its regen (the record's clip/regen fields).
- **Stunner (07000005, `--give 0x4a`, a guard spawned from the dev window): "doesn't work".** The stunner only fires
  at a class-2/3 target within `weapon_data+0x1c` (5 / 10) [M Player_WeaponInitBullet 0x1a4bf0:187-200, per
  fix-laser-beam]. Reproduce with a dev-spawned guard (panel `spawn` path) at 3 units: does the ray find the spawned
  guard (class, collision volume), does the beam draw (`Draw_TaserBeam` 0x1a46a8), does the guard react (stunned
  state)? Fix what's missing; same HUD charge bar as the laser.
- **AIMS-20 (0x1a), any level: "the lights on the AIMS-20 aren't on"** (PCSX2: the gun's side has a row of RED lights,
  lit; the port's model shows them dark: an emissive/additive material or a texture-animation the view model draw
  skips). And its scope (102a-fix-zoom-overlays owns the scope; leave it).
- **Oddjob's Hat (0x45): "should be a one-shot kill but it acts like a grenade".** Find the hat's projectile class and
  damage in its record and the game's hit path (a thrown hat that kills on contact and returns?), and port it.
Tests per item through the real input path; sister assertions. The PCSX2 capture is described above (no images in the
repo).

Files: `crates/nf-game/src/{gun,gadget,drone,bullet}.rs`, `crates/nf-view/src/{scene,hud}.rs`.

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

**Branch:** `s/gadgets-live`.

---
claimed-by: sandbox4 at 2026-10-14T06:24Z

---
## Report (done, 2026-10-14T07:59Z)

Closed by the driver: the report is on `s/fix-gadgets-live` (docs/reports/fix-gadgets-live.md); the worker didn't run finish.sh.
