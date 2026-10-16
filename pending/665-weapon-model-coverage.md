## Your task: `weapon-model-coverage` — every weapon has its first-person model on every level

The user: "Every single weapon needs to have a first person model" (the dev panel's weapon gives on any level), and
load logs still print "27 weapon-model hashes still missing" (07000005's session mesh merge). The port merges
weapon assets from other archives (`weapon assets: merged 07000029.bin …`).
- Census every weapon id 1..0x73: its view-model skin hash, the archive(s) that hold it, and whether the port
  resolves it on each single-player level.
- Find and fix the remaining misses (archives not merged, hash aliases, a different archive per weapon class).
- Report the census table: weapon → model hash → source archive → resolved yes/no, before and after.
- Tests: on 3 levels, every weapon the dev panel can give resolves a view model.
- Sister: a bogus hash must report missing.
Data only: the main agent checks the look.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/weapon-model-coverage`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
