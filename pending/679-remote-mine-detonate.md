## Your task: `remote-mine-detonate` — the Remote Mine's detonator and throw cadence

Playtests 1014-1725 and 1015-0046: "the detonator, for some reason, you have it set to the watch rather than the
alternate fire of the mine"; "there's a delay between when I click to throw ... and a heavy delay between me being
able to throw multiple". Port from the game:
- how detonation is triggered while the mine weapon is held (its alternate-fire/mode action vs a separate
  detonator item) [M Player_Weapon / the mine's weapon record +0x6C/+0x70, the detonate arm in `Bullet_update`];
- the throw → release frame and the re-throw interval [M the fire script, `Player_WeaponFiring` 0x1a5460].

Measure the frames and table them. Headless tests: throw 3 mines at the game's cadence, then detonate with the alt
fire and all three explode. Sister: firing while no mine is out does nothing.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/remote-mine-detonate`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
