## Your task: `net-online-lobby`, the Host / Join flow in the front end (no drawing)

Add the front-end model for online play on top of 611's MP menu flow (crates/nf-view/src/menu.rs): from Main →
Multiplayer, a choice Local / Host Online / Join Online; Host: the port number (default e.g. 27420) and start
listening, then the Join Game page shows remote agents joining in panels 2–4 (their codenames), the host runs the
usual Scenario → Map → Options flow and starts the match; Join: an address entry (typed with the keyboard —
digits, dots, colon; paste with Ctrl+V if simple), connecting / refused / timed-out states and messages, then a lobby
view mirroring the host's choices read-only until the match starts; Back cancels cleanly. State + getters + actions
(mouse support follows the existing `mouse_move`/`mouse_click` pattern — add the new pages' hit rows), wired to 051's
session API (merge `origin/master`; if 051 hasn't landed, put the session behind a trait and test with a fake). The
main agent draws the pages. Tests: the page flow host side and join side with a fake session.

**Context — online multiplayer (the user, 2026-10-15: "I want online support where we can host a mp server and other
clients can join").** The PS2 game has no online mode; this is a port feature. The plan (main agent): **deterministic
lockstep over UDP** — every machine runs the same MP match from the same seed and settings and exchanges only each
agent's per-tick input; a per-tick state hash detects desyncs. Direct connect: the host listens on a UDP port, clients
join by IP:port (internet play needs the host to forward the port; no relay/server infrastructure). **No new crate
downloads** — use `std::net::UdpSocket` (ask in the report if something truly needs a crate; don't add one). Keep the
networking in a new `crates/nf-net` crate (or `nf_game::net` if a crate is overkill — say which and why), with no
rendering. Keyboard + mouse only (no pads). Tests run on loopback (127.0.0.1) and must not need a second machine.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/net-online-lobby`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-15T22:45Z

---
## Report (done, 2026-10-15T23:55Z)

