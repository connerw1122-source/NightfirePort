The user's playtest 2026-10-15 (results id 1015-1738) said: the Delta Repeater (the crossbow): "model is missing the
arrows" and "when you shoot, it sounds like a grenade is bouncing when the crossbow flies out and bounces against the
wall when it shouldn't be bouncing". 1) The bolt's world hit: port what the game does for this weapon's projectile
(stick/die vs ricochet; the bounce sound must not play) [M Bullet_CollisionHandler + weapon_data +0x6C/+0x70
flags]. 2) The missing arrows: find which sub-meshes/datums/attachments carry the loaded bolts on the view model
and why they're absent (a skin entry not merged, a datum attachment not created, a script op not ported). Deliver
the data/attachments; the main agent checks the look against PCSX2.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/delta-repeater`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
