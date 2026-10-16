## Your task: `party07-start-rules` — 07000007's party rules from the first frame

Playtests 1011-1535, 1011-1616 and 1012-1723 (07000007):
- "the guards immediately start shooting at me because I start with my gun drawn";
- "If you punch a guard, it should be an automatic mission fail";
- guests T-posing or not reacting;
- one male guest shooting.

Verify every rule against the game [M the party's script channels, the weapon lock `Player_Weapon` 0x1a24c8:242-275,
the guards' friendly/alert state at start, the punch → fail hook, the guests' drone classes and reactions].
Headless tests from a cold start AND from the 06 → 07 carry:
- Bond holds fists, and the guards are not hostile until the trigger;
- a punch on a guard fails the mission with the game's fail text;
- no guest fires a weapon;
- guests react to gunfire the way the game does.

Fix what fails, and cite the evidence.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/party07-start-rules`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T08:29Z

---
## Report (done, 2026-10-16T11:51Z)


---
## Report from the worker (done, 2026-10-16T11:51Z)

Closed by the driver: the report is on `s/party07-start-rules` (docs/reports/party07-start-rules.md); the worker didn't run finish.sh.
