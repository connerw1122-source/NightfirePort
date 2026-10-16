## Your task: `net-e2e-loopback` — host and join an online match end to end over loopback

Two FrontEnds plus two Sims in one test, over 127.0.0.1 through nf-net (661):
- host: Multiplayer → Host Online → port → Start Listening → the lobby;
- client: Join Online → types 127.0.0.1:port → Connect → the lobby;
- the host picks the scenario/map/options and starts; both load the arena via `load_mp_carry` with the same seed;
- 1200 lockstep ticks with scripted inputs on both;
- the per-tick state hashes match; a scripted kill updates both scores; the match end hands both to the debriefing.

Inject 5% packet loss and reordering in one variant. Every step goes through FrontEnd actions (no direct page
writes). Fix what breaks.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/net-e2e-loopback`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T16:50Z
