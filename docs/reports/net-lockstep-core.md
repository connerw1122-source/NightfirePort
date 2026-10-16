# net-lockstep-core — report

Branch `s/net-lockstep-core`. Task: the UDP lockstep session layer (host +
up to 3 clients, handshake, per-tick inputs with delay/redundancy/acks/stall,
state-hash desync detection, timeout handling, session API), documented in
`docs/systems/net-lockstep.md`.

## What is done

**New crate `crates/nf-net`** (std-only, zero deps, no rendering — chosen
over `nf_game::net` so session tests build in milliseconds and the host app
can run a match without the game sim):

- `session.rs` — the lockstep session state machine: host/client roles,
  handshake (protocol version + build id + codename, mismatched exes refuse
  cleanly with `REFUSE{reason}`), WELCOME carries the host's
  `MatchSettings` + `seed: u32`, GO match-start (retransmitted until the
  client's first input proves it started), per-tick INPUT packets with
  tunable input delay (default 3, task range 2..=4), redundancy (full
  unacked window + last N inputs; oldest-first with END truncation when the
  span overflows the packet cap so the ack can always advance), contiguous
  acks with a 32-tick bitmap and a prune-safe sliding base, hard stall when
  a human input is missing (never guesses), state-hash exchange every
  `hash_every` ticks (default 30) with piggybacked previous hash and
  `Event::Desync`, keepalives, timeout handling, `SLOTMAP` broadcast, and
  the API `poll(now_ms) -> Vec<Event>` / `local_input(tick, input)` /
  `ready_inputs(tick) -> Option<[Input; 4]>` / `submit_hash(tick, hash)` /
  `start_match` / `leave`. `poll` takes logical ms from the caller — no
  wall clock anywhere (determinism rule).
- `proto.rs` — 9 packet types, strict decode (magic, length, count/mask
  consistency), all packets fit a 1500-byte MTU.
- `input.rs` — the 24-byte per-agent input (4 f32 as raw bit patterns for
  bit-exactness + flag bytes covering `player::Input` AND the separate
  `trigger/reload/use_key` args of `Sim::step_bond`) and `NEUTRAL`, the
  game's disconnected-pad shape (buttons 0 / sticks 0x7f,
  `psiInput_PollDevices` 0x10ca78).
- `settings.rs` — 176-byte `MatchSettings` wire encoding with a field-by-
  field mapping to `MpSettings`' game hex offsets (no wire encoding existed;
  the only prior save code is `LS_MakeMPSettings` 0x1ce508's 13-byte `MPSG`
  profile block).
- `net.rs` — `UdpTransport` over `std::net::UdpSocket` (non-blocking).
- `shim.rs` — seeded deterministic loss/latency/jitter network for tests.
- Workspace: `Cargo.toml` members + `Cargo.lock` gained `nf-net` (additive).

**Dropped-client behaviour: NEUTRAL continuation** (`DropPolicy::Neutral`,
default), justified from the game: a vanished pad is fed buttons 0 / analog
0x7f every poll and the match runs on (`psiInput_PollDevices` 0x10ca78,
`Player_HandleDeath` 0x190bb0 → `MP_ReSpawn` 0x185368); bot takeover has no
mid-match precedent (bots are fixed into slots 4..8 at `MP_Start` 0x183a50,
configured only pre-match by `Menu_PrepareBots` 0x1f94a0); match end on the
console is only the explicit pause-menu QUIT (`C_GCPAUSE_Handler` 0x204408).
`DropPolicy::EndMatch` exists for hosts that want it. A reconnect resumes
real input, like the pad state machine re-initialising on replug
(`ps2_handle_pad` 0x10c738).

**050 coordination**: `origin/s/net-determinism` contains only a WORKLOG
entry (checked 2026-10-15) — no `state_hash` exists yet, so the session
takes the per-tick hash as an opaque `u64` via `Session::submit_hash`;
050's `state_hash(&MpWorld) -> u64` and the seed→`Rng::new(seed)` routing
(`drone.rs:394`) wire in at the host app without session changes.

## Two real bugs the 2000-tick test caught (fixed)

1. **Send-window clamp wedge**: my first redundancy clamp rewrote the
   oldest-unacked window start to `newest-cap+1`, so after one big catch-up
   poll the host re-sent only the newest 8 ticks forever; ticks 0..11 were
   never re-sent and every client starved at tick 0. Fix: when the unacked
   span overflows the cap, send oldest-first and truncate the window END.
2. **Prune-broken acks**: a receiver pruned old `recv_ticks`, its ack scan
   restarted from tick 0, hit the hole, and acked `u32::MAX` ("nothing")
   forever — the peer then re-sent ticks 0..7 forever and the whole match
   froze (found frozen at host tick 524 / clients 480). Fix: the ack scan
   starts from a sliding `our_ack` (prune uses the same base), so pruning
   can never reset an ack.

## Verification (what I ran)

- `cargo test --offline -p nf-net --lib` — 9/9 unit tests green (codec
  round-trips incl. bit-exact f32, decode rejection of garbage/truncation,
  each with a failing sister assertion).
- `cargo test --offline --release -p nf-net` — **all green**: 9/9 unit
  tests and 8/8 integration tests (`four_agents_2000_ticks_lossy_stay_in_sync`,
  `one_flipped_input_byte_is_caught_as_desync`, `stall_never_guesses_then_resumes`,
  `client_timeout_reported_then_neutral_continuation`,
  `endmatch_policy_ends_match_instead`, `handshake_delivers_settings_seed_and_go`,
  `version_and_build_mismatch_refused`, `real_udp_loopback_sync`).
- Fast suite (`tools/test/suite.py`, background from 22:50 UTC): it never
  produced a result line in this sandbox (background jobs are killed
  between tool calls here); the only crates my code touches are `nf-net`
  (new) and the workspace files above, and `cargo build -p nf-net` is green,
  so no existing crate's tests can regress from this branch. Full-suite
  note: my branch is `origin/master` + additive files only.

## What's left (honest)

Nothing in this task's scope. **All 17 tests pass**: 9 unit + 8/8
integration, including the full `four_agents_2000_ticks_lossy_stay_in_sync`
(host + 3 clients, 2000 ticks, 5% loss + 30-50 ms latency, zero desyncs,
hash chains agree at every tick), the one-bit desync sister test, and the
real-`UdpSocket` loopback run. (An intermediate state of this report said
3 tests were red; the reds turned out to be TEST-harness bugs, not session
bugs, and are fixed: (a) the final-hash assertion compared chains of
DIFFERENT lengths because the host legally overshoots the target tick while
slower clients finish — hashes are now compared AT tick 1999/299 with
per-tick marks; (b) the desync sister test corrupted the input at SAMPLING
time, which propagates the flipped value to every machine through the input
packets — correct lockstep behaviour that can never desync; the corruption
now applies to one machine's sim view only, like a real sim bug. A third
find: a first-HASH piggyback default (`prev_tick == tick`, prev_hash 0)
was being stored as a remote hash and produced a bogus `Desync` at tick 0;
the receiver now skips that marker.)

Next steps beyond this task: wire `MpSettings`/seed into a real `nf-view`
host-app path, replace the fake `sim_hash` with 050's `state_hash(&MpWorld)`
once it lands, and (optionally) add reconnect-after-timeout as a follow-up
task.

## Live checks for the main agent

None — the PS2 game has no network mode; the cited game behaviours
(pad-disconnect shape, bot slot rules, quit path) were verified from the
decompile.

## ntfy

Sent (see the finish-step output in the session log; exact error would be
recorded here if all three transports had failed).
