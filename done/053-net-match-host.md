## Your task: `net-match-host`, run a networked MP match in the app

Wire 051's lockstep session into the running app (crates/nf-view/src/lib.rs, the MP match path from 611/402): when a
match starts online, every machine builds the same MP level with the host's settings + seed; each frame the local
agent's input (keyboard + mouse) goes to the session, and the sim advances one tick only when `ready_inputs(tick)`
returns all agents' inputs (fixed 60 Hz tick, render interpolation unchanged); each machine renders ITS agent's
view (one camera — no split screen online), the others as remote characters; the state hash goes out every N ticks;
desync / disconnect show the game's message box and return to the lobby. Tests: two app sims in one process over
loopback play 1 000 ticks of scripted input and end in the same state; a forced desync is detected.

**Context — online multiplayer (the user, 2026-10-15: "I want online support where we can host a mp server and other
clients can join").** The PS2 game has no online mode; this is a port feature. The plan (main agent): **deterministic
lockstep over UDP** — every machine runs the same MP match from the same seed and settings and exchanges only each
agent's per-tick input; a per-tick state hash detects desyncs. Direct connect: the host listens on a UDP port, clients
join by IP:port (internet play needs the host to forward the port; no relay/server infrastructure). **No new crate
downloads** — use `std::net::UdpSocket` (ask in the report if something truly needs a crate; don't add one). Keep the
networking in a new `crates/nf-net` crate (or `nf_game::net` if a crate is overkill — say which and why), with no
rendering. Keyboard + mouse only (no pads). Tests run on loopback (127.0.0.1) and must not need a second machine.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/net-match-host`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-15T22:56Z

---
## Report (done, 2026-10-16T00:20Z)

Closed by the driver: the report is on `s/net-match-host` (docs/reports/net-match-host.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T00:53Z)

The report is on the branch: `docs/reports/net-match-host.md` at s/net-match-host (73b946fd + the test-file commit). Summary: the nf_game::net UDP lockstep session (051's API), the app wiring (settings+seed handover — mp_match_settings was never read before; load_mp_carry; slot-order spawn + Sim::mp_bonds + swap_local_slot; the ready_inputs tick gate; step_net_remotes; net_state_hash every 15 ticks; the message-box + return-to-lobby path; --net-host/--net-join). Tests: net_session 5/0 headless loopback, net_match_live 3/0 NF_ISO (two sims, 1000 ticks same state; forced desync detected; disconnect). Suite: run cut at the window end — observed FAILED walkreal_07000001_live (window-sensitive long walk, not touched by the diff) and probe_pylon_render (render probe, no drawing changed); no TIMEOUT/CRASH. Details + live checks in the report.
