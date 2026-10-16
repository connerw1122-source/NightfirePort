## Your task: `grates-02` — the openable grates/vents on 07000002

Playtests 1011-1732 and 1012-1723: "There should be grates that you can interact with that will open up before you
climb into the vent behind this NPC" and "a grate above the bed in the bunker that should be able to be opened but it
wont open". Find the grate objects on 07000002: placement type/hash, door-class or switch-class, the use prompt,
what opens them (the use key, a gadget, a script channel) [M the object's create/update/activate]. Port what's
missing so E (or the right gadget) opens them as the game does, with the collision freed. Headless tests: Bond at
each grate, use, and the grate opens with a passable volume. Sister: a locked grate stays shut.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/grates-02`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T08:25Z

---
## Report (done, 2026-10-16T10:33Z)

