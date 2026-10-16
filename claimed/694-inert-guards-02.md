## Your task: `inert-guards-02` — invincible, AI-less guards on 07000002

Playtests 1010-2055 and 1011-1128 (07000002):
- "there are npcs that shouldnt be able to be shot and some of the enemies are invincible with no AI";
- "one specific guard ... standing next to a civilian that won't move, no AI, doesn't do anything".

List every drone on 07000002 with its class, state, health and enabled/present flags at start and after 600 ticks.
Find the ones that are present but never update (a missing state handler, a disabled AI waiting on a channel that
nothing sets, a script actor left in the world) [M their create flags + the waking channel's writers]. Fix them to
the game. Tests: each guard listed either runs AI or is legitimately dormant (cite the channel), and none is
invincible unless the game makes it so.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/inert-guards-02`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T13:36Z
