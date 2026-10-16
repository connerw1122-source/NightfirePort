## Your task: `net-consolidate` — one lockstep session layer, not two

The online-MP chain (050-053) is being merged together on 2026-10-15. Two workers built the UDP session
independently:
- **051** `net-lockstep-core`: a new std-only crate `crates/nf-net` (session.rs/proto.rs/input.rs/net.rs/shim.rs,
  17 tests). It has a handshake with protocol + build id, input delay 2-4, redundancy with acks and a 32-tick
  bitmap, a hard stall, state-hash desync detection and timeouts.
- **053** `net-match-host`: its own `nf_game::net` module (UdpSocket, 3-tick redundancy with no ack bookkeeping,
  a 5 s peer timeout). This is the one the running app wires: menu → `pending_mp_settings` → `load_mp_carry`, the
  seeded RNG, all human bonds spawned in slot order.
- **052** `net-online-lobby`: the Host/Join pages talk to an `MpSession` trait object (`FrontEnd::mp_session`),
  which its report says "the host wires 051's real session".

Start from `origin/master` (once the chain has landed; if `crates/nf-net` or `nf_game::net` is missing there,
start from the merge of `origin/s/net-determinism`, `s/net-lockstep-core`, `s/net-online-lobby`,
`s/net-match-host`). Make ONE session layer:
- Pick the stronger protocol (justify it in the report: loss/reorder/latency behaviour, measured by tests).
- Port the other's unique features into it.
- Wire the app (053's path) and the lobby's `MpSession` to it.
- Delete the other layer.
Keep or port every test from both. Add a loopback end-to-end test: host + 2 clients in-process over 127.0.0.1
with packet loss and reordering injected, play 600 ticks, and require identical per-tick state hashes on all
peers. Sister: a corrupted input must trip the desync detector. Update docs/systems/net-lockstep.md and
net-match-host.md to describe the single design.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/net-consolidate`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
