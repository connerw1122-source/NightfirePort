## Your task: `guard-blast-smoke` — guards react to blasts and smoke

Audit gap ②: grenades and smoke do nothing to guards today. Port the blast-stagger state and the smoke states:
DSTATE 0x54 and 0x7B-0x7D [M]. Cover:
- the entry from a nearby explosion, a stun or a smoke cloud;
- the anims;
- the timings;
- the exit back to combat.

Tests on 07000005: a frag a few units from a guard staggers him the game's way, and a smoke grenade puts guards
inside its cloud into the smoke state until it clears.

The uncited-gameplay audit (task 710, merged) has this gap's description, caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that section first and meet its criteria.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guard-blast-smoke`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T20:38Z

---
## Report (done, 2026-10-16T22:32Z)

Closed by the driver: the report is on `s/guard-blast-smoke` (docs/reports/guard-blast-smoke.md); the worker didn't run finish.sh.
