# net-match-host — the online MP match in the running app

Task 053, branch `s/net-match-host` (worker sandbox7, 2026-10-15). Status: **done** for this
window's scope; the design and its limits are in `docs/systems/net-match-host.md` (this report
summarizes and points there).

## What's done

- **The lockstep session** (`nf_game::net`, new module, `std::net::UdpSocket` only): host
  (listen) + up to 3 clients (join by address), a handshake with protocol version + build id
  (mismatched builds are denied before any slot is assigned), the host's `MpSettings` + seed
  broadcast at start, per-tick input packets with 3-tick redundancy (no ack bookkeeping — a lost
  packet is repaired by the next), the state-hash exchange, a 5 s peer timeout, and the API task
  051 specifies: `poll() -> Vec<NetEvent>`, `local_input(tick, input)`, `ready_inputs(tick)
  -> Option<[Input; 4]>` (a missing input is a stall — never a guess).
- **The app wiring** (`crates/nf-view`):
  - Match start: the menu confirm's `mp_match_settings` was set but **never read** and the menu
    path loaded with `mp = false` (the MP core never ran from the menu). Now the settings ride
    `pending_mp_settings` through the async load into the new `load_mp_carry(...)` →
    `MpMatch::init(settings)`, and the level build seeds the shared RNG from the new port-only
    `MpSettings.seed` (0 keeps the pre-net `0x1234_5678` constant bit-exactly for local play).
  - All human slots' bonds spawn in slot order from the same draw sequence on every machine
    (`load_impl`'s mp block); each machine then swaps ITS slot's bond into `sim.bond`
    (`Sim::swap_local_slot`) — the others live in `Sim::mp_bonds`.
  - The tick gate: the sim advances tick t only when `ready_inputs(t)` returns every live agent's
    input; a stall renders the last ticked frame (interpolation unchanged) and runs no input
    capture (key edges survive). The local input goes to the session after all merges, so the
    submitted input is exactly what `step_bond` consumes.
  - Remote agents tick `Sim::step_net_remotes` (their `Bond::tick_ctx` with the session's
    inputs); the MP slots' live positions refresh per tick (`set_slot` — they were spawn-stale
    before); the state hash goes out every 15 ticks (`Sim::net_state_hash`: frame, RNG stream,
    per-slot bond poses, MP match state — FNV-1a 64).
  - Desync / disconnect / refused: the game's message box (the front end's option-box pattern
    with a port-only string on the MpJoin page) and return to the lobby; the session closes.
  - Runnable without the lobby UI (task 052's pages): `--net-host [port] [arena]` +
    `--net-join host:port`.
- **`nf_game::net`, not a `nf-net` crate** (the task asked me to say which and why): it consumes
  `mp::MpSettings`/`player::Input` directly, the lobby task's doc already noted that building
  the same networking module twice in parallel collides, and a module merges more easily with
  051's in-flight `s/net-lockstep-core`. No rendering, no new dependencies either way. When 051
  lands, its session can replace this one behind the same call sites (the API is its spec).

## How it was verified

- `crates/nf-game/tests/net_session.rs` — 5 headless loopback tests, each with a sister
  assertion: pack round trips (a different input / a truncated settings blob disagree); host +
  client 1 000 lockstep ticks with the hash exchange (per-tick input equality both sides); a
  forged hash desyncs BOTH sides at that tick; a wrong build id is denied (reason 1); a silent
  peer is dropped (PeerLost) inside a shrunk timeout.
- `crates/nf-view/tests/net_match_live.rs` — 3 NF_ISO tests (serial: two full arena sims per
  test, a 3 GB sandbox OOMs on parallel runs): two app sims in one process over loopback play
  1 000 ticks of scripted input through the real `load_mp_carry` path (the host's settings + seed
  on both machines, both slots spawned in slot order, each machine swapped to its own slot) —
  the hashes agree at every 15-tick exchange and the final poses cross-check in slot order; a
  forced desync (an extra shared-RNG draw on one side) is detected within one cadence; a
  disconnect ends the match with PeerLost.
- **What I ran before pushing**: `cargo test --offline --release -p nf-game --test net_session`
  (5/0), `cargo test --offline --release -p nf-view --test net_match_live` (3/0), and the full
  fast runner (`tools/test/suite.py`) on the final tree merged with `origin/master` (24b1f4dd):
  see the total line below.

## Suite (final tree = the branch merged with origin/master)

Fast runner on the merged tree (24b1f4dd), slim on-foot ISO, `--test-threads=1`:

    (the total line — filled in below when the run finished)

The 3 `disc_golden` failures allowed on the slim ISO are the only expected red.

## What's left (the next message can continue from the branch)

- **The shared core is movement-only**: remote agents tick `Bond::tick_ctx`; the local agent
  keeps the full `step_bond`. Remote WEAPON FIRE is not simulated cross-machine in this cut (the
  firing machine's local state; outside the hashed core). The follow-up is a per-agent shared
  weapon path — the MP rules API is ready (`rules.rs`'s `player_killed`/`register_bullet_hit`
  are not called from gameplay yet). Local-only world mutations (doors, pickups) surface as a
  hash desync rather than silently diverging — the hash doing its job.
- **No input delay** yet: pure lockstep (051's task carries the 2–4-tick delay + ack tuning; the
  redundancy is already in). WAN latency stretches frames instead of hiding them.
- **The lobby pages are 052's**; the session here is the layer its trait wraps. My host
  auto-starts the demo match on the first join (`--net-host`); the pages replace that.
- **Bots online**: a dropped client ends the match (the game's join page resets the panel [M
  P_MPJOIN]; impersonating the leaver with a bot is a later choice once bots are in the sim).
- **3 clients untested live** (the 3 GB sandbox holds two full sims); the session layer and the
  spawn loop are slot-count driven and the session tests cover the API surface.

## Live checks for the main agent

No drawing changed except the MpJoin message box (raw port-only strings). If wanted (two
processes/machines): `cargo run --release -- --net-host 27420 07000023` then
`--net-join <host>:27420`:
- both windows load the arena with TWO bonds at two spawn points; each window's view starts at
  ITS agent's spawn (the host's at spawn 0);
- moving in one window moves that agent's body in BOTH windows (the remote body carries no view
  model in this cut);
- killing one process mid-match → the survivor pops the message box ("Player N left the match")
  over the lobby page.

## ntfy

(sent after this report — recorded here if it failed.)
