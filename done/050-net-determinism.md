## Your task: `net-determinism`, prove the MP sim is lockstep-safe

Before any netcode: lockstep needs the MP match simulation (nf-game's mp code — 400 mp-core, 401/s mp-bots, 402
match rules, 611 menu flow's MpSettings) to be a pure function of (settings, seed, per-tick inputs). Audit and fix:
every source of nondeterminism in the MP tick — host time / wall clock reads, HashMap iteration order, uninitialised
or address-dependent ordering, thread timing, float paths that differ by build — and the RNG seeding (one seeded
stream per match, the game's own `Rand_*`). Deliver `state_hash(&MpWorld) -> u64` (stable across runs and machines:
hash integers and the f32 bit patterns of the sim state, not floats via formatting), a recorded-input replay harness,
and tests: two independent sims fed the same inputs for 10 000 ticks (bots on, every scenario) hash identically every
tick; a sister test proves the hash catches a one-bit state difference. Document in docs/systems/net-lockstep.md.

**Context — online multiplayer (the user, 2026-10-15: "I want online support where we can host a mp server and other
clients can join").** The PS2 game has no online mode; this is a port feature. The plan (main agent): **deterministic
lockstep over UDP** — every machine runs the same MP match from the same seed and settings and exchanges only each
agent's per-tick input; a per-tick state hash detects desyncs. Direct connect: the host listens on a UDP port, clients
join by IP:port (internet play needs the host to forward the port; no relay/server infrastructure). **No new crate
downloads** — use `std::net::UdpSocket` (ask in the report if something truly needs a crate; don't add one). Keep the
networking in a new `crates/nf-net` crate (or `nf_game::net` if a crate is overkill — say which and why), with no
rendering. Keyboard + mouse only (no pads). Tests run on loopback (127.0.0.1) and must not need a second machine.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/net-determinism`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-15T22:38Z

---
## Report (done, 2026-10-15T23:47Z)

