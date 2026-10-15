## Your task: `net-lockstep-core`, the UDP lockstep session

Build the session layer on 050's determinism (merge `origin/master` first; if 050 hasn't landed, coordinate through
its state-hash API and say so). Deliver: a host that listens on a UDP port and up to 3 clients (4 agents, the game's
MP limit) that join by address; a handshake (protocol version + build id check so mismatched exes refuse cleanly,
codename, the host's MpSettings + seed sent to clients); per-tick input packets with an input delay (2–4 ticks,
tunable), redundancy (each packet repeats the last N unacked inputs), acks, and a stall when an input is missing
(never guess); a state-hash exchange every N ticks with a desync error; disconnect/timeout handling (a dropped client
becomes a bot or the match ends — pick the game's nearest behaviour and say why); the session API the host app will
call (`poll() -> Vec<Event>`, `local_input(tick, input)`, `ready_inputs(tick) -> Option<[Input; 4]>`). Tests over
loopback: host + 3 clients run 2 000 ticks with simulated loss/latency (a test shim dropping/delaying packets) and stay
in sync; a version mismatch is refused; a client timeout is reported. Document the protocol in docs/systems/net-lockstep.md.

**Context — online multiplayer (the user, 2026-10-15: "I want online support where we can host a mp server and other
clients can join").** The PS2 game has no online mode; this is a port feature. The plan (main agent): **deterministic
lockstep over UDP** — every machine runs the same MP match from the same seed and settings and exchanges only each
agent's per-tick input; a per-tick state hash detects desyncs. Direct connect: the host listens on a UDP port, clients
join by IP:port (internet play needs the host to forward the port; no relay/server infrastructure). **No new crate
downloads** — use `std::net::UdpSocket` (ask in the report if something truly needs a crate; don't add one). Keep the
networking in a new `crates/nf-net` crate (or `nf_game::net` if a crate is overkill — say which and why), with no
rendering. Keyboard + mouse only (no pads). Tests run on loopback (127.0.0.1) and must not need a second machine.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/net-lockstep-core`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
