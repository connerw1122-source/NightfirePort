# net-consolidate — one lockstep session layer, not two

Branch `s/net-consolidate` (2026-10-16). Task 661: the 050-053 online-MP chain had two UDP
session layers built in parallel — 051's `crates/nf-net` and 053's `nf_game::net` (the one the
running app wired) — plus 052's lobby talking to an `MpSession` trait object with no real
implementation. This task made ONE session layer, wired every consumer to it, and deleted the
other. Evidence tags follow the repo's rules (`docs/fields/README.md`).

## What's done

**Kept 051's `crates/nf-net` as the session layer** (justification, measured — see
`docs/systems/net-lockstep.md` §9 for the full argument):
- **Loss**: 051 carries the full unacked window in every packet + a contiguous ack with a 32-tick
  bitmap, so an isolated loss can never lose a tick and the sender always knows the peer's
  cursor. 053 had 3-tick redundancy with no ack bookkeeping — more than 3 consecutive losses
  lose an input forever (permanent stall → the 5 s timeout ends the match).
- **Latency**: 051 has a configurable input delay 2..=4 negotiated in WELCOME; 053 sampled and
  executed the same tick (every RTT is a stall).
- **Desync detection**: 051 exchanges hashes every `hash_every` ticks AND piggybacks the previous
  hash on each HASH packet (one lost HASH packet cannot lose the comparison); 053 sent a single
  HASH per cadence with no recovery.
- **Testability**: 051's `Transport` trait + seeded shim make the lossy tests bit-reproducible;
  053's live-socket tests were flaky under load (its own comments: "1-3 in 6 runs", "4 in 5 runs").
- **Fidelity**: 051's 24 B input carries `trigger`/`reload`/`use_key` — `Sim::step_bond`'s
  separate args [M `crates/nf-view/src/scene.rs:1818`]; 053's 20 B packing dropped them.

**Ported 053's unique features into nf-net** (protocol bumped 1 → 2):
- Full `MpSettings` wire fidelity: `MatchSettings` is now 208 B — per-player `radar_on`
  [M `MP_GetRadarObjects` 0x189840, `HUD_RadarUpdate` 0x19f6e8] and `registered_pickups` (+0x472)
  joined the 051 field set.
- The lobby display codename: the HELLO/WELCOME 32 B payload is a display name now (the
  compatibility gate is protocol + build only; `RefuseReason::Codename` removed), with
  `Session::set_display_name` + `Session::agents() -> Vec<AgentInfo>`.
- `RefuseReason::Started`: a HELLO after GO is refused (no mid-match join; the lobby owns joins).
- `nf_net::hash::Hasher` (moved verbatim from `nf_game::net`); `Sim::net_state_hash` uses it
  unchanged.
- The `MpSettings` match handover: the host's final settings ride a tagged DATA tunnel (new
  `PT_DATA` + `Session::send_data` / `Event::Data`) sent on arm — the WELCOME at join time
  cannot know the confirm page's settings (053's START packet semantics, re-created).
- `Session::set_host_match`: the lobby arms AFTER clients joined, so the handshake's placeholder
  settings must be replaceable pre-GO.

**Wired both consumers to the one layer** (`crates/nf-view/src/mpnet.rs`, new):
- `NetCore` owns `nf_net::Session<UdpTransport>`; `SessionProxy` (cheap `Rc` clone) is held both
  by the app (`App.net`, the lockstep gate) and by the front end (`FrontEnd::mp_session` as the
  real `MpSession` impl — 052's trait needed no change, so its 9 page tests pass untouched).
- The flag path (`--net-host`/`--net-join`) drives the same core (053's `App::net_host/net_join`
  surface, rewritten); the seeded-RNG + slot-order handover is unchanged (`pending_mp_settings`
  → `load_mp_carry`, the seed rides `MpSettings.seed` on every path).
- The lobby adoption: the online pages create the proxy lazily; when their match starts (the
  confirm page armed it / the tunnel payload arrived) `menu_after` adopts the proxy as the
  lockstep gate (`has_match()` is the exact cue — `match_start` is Some only after arm/payload).
- The hash cadence moved into the session (`Config::hash_every = 15` = the old `NET_HASH_EVERY`);
  the app submits every tick and the session sends + piggybacks on its own schedule.

**Deleted the other layer**: `crates/nf-game/src/net.rs` and its `tests/net_session.rs` are gone
(`pub mod net` removed from `nf-game/src/lib.rs`; nothing else in the workspace referenced it).

**Behaviour change, intentional**: a dropped peer is no longer terminal. 053's `PeerLost` ended
the match; 051's default `DropPolicy::Neutral` feeds the dropped slot NEUTRAL inputs and the
match runs on — the game's own disconnected-pad shape [M `psiInput_PollDevices` 0x10ca78 writes
buttons 0 / analog 0x7f for a dead pad]. The app logs `ClientLeft` instead of `net_fail`ing.

## How it was verified

- **The required e2e** (`crates/nf-net/tests/loopback_e2e.rs`): host + 2 clients in-process over
  real 127.0.0.1 UDP with 8% injected packet loss and 25% injected reordering (a seeded
  delay-line transposition wrapper around real `UdpSocket`s), 600 ticks — **identical per-tick
  state hashes on all peers**. Sister test: one corrupted input in one machine's sim view trips
  the desync detector at/after the corrupted tick. Both PASS.
- **053's tests ported** (`crates/nf-net/tests/ported_session.rs`): the raw-wire build-mismatch
  REFUSE (a raw HELLO with a wrong build id gets REFUSE/2 back on a real socket), the forged-hash
  desync at a hash cadence boundary (both sides report `Event::Desync`), the silent-peer timeout
  (`ClientLeft/Timeout`) with the NEUTRAL continuation after the drop (the port's behavioural
  replacement for the terminal PeerLost), and 053's clean-link per-tick lockstep contract
  (`host_and_client_lockstep_300_ticks`, input_delay 1, per-tick equality both sides). 4/4 PASS.
- **051's tests** kept and green: the shim suite (handshake/refusal, 2000 ticks at 5% loss +
  30 ms/20 ms jitter with hash equality at every tick, stall-never-guesses, timeout, EndMatch
  policy, real-UDP loopback) — 8/8 — plus the crate's unit tests (proto round-trips incl. the
  new DATA truncation sister, settings v2 round-trip with flip-detection sisters, input
  bit-exactness, Hasher fold) — 11/11.
- **The app-level sims test ported** (`crates/nf-view/tests/net_match_live.rs`, NF_ISO-gated,
  run against the slim ISO): two real MP sims load the host's arena from the host's settings +
  seed through the real `load_mp_carry`, 1000 lockstep ticks with the hash every 15 — per-tick
  hash equality, final poses cross-checked in slot order; a forced extra RNG draw is detected
  within one cadence; a disconnect is reported and the match continues on NEUTRAL. 3/3 PASS.
- **The lobby tests** (`crates/nf-view/tests/mp_net_lobby.rs`, 052's suite): 9/9 PASS unchanged —
  the `MpSession` trait kept its shape, so the fake needed no edits.
- **Crates run before pushing** (the task's requirement): `cargo test --offline --release -p
  nf-net` (25 passed), `-p nf-game --lib` (234 passed), `-p nf-view --lib` (15 passed),
  `-p nf-view --test net_match_live --test mp_net_lobby` (12 passed).

## The fast suite

`tools/test/suite.py` could not run in this sandbox: background jobs are reaped between tool
calls (the runner died mid-build twice — same finding the drive-mission-scripts session logged),
and a foreground run of the whole suite did not fit the 2-hour window next to the port work. The
touched crates' tests ran in the foreground instead (listed above); the full suite should be
run on the merged tree.

## What's left

- The full `tools/test/suite.py` run on the merged tree (see above).
- `docs/systems/net-online-lobby.md` still describes the trait object as unwired; the wiring is
  real now — one paragraph there would finish the doc set (the two required docs are updated).

## Live checks for the main agent

None required — this task touched no game-referenced behaviour (the only [M]-tagged behaviour
kept is the disconnected-pad NEUTRAL shape, already cited from the decompile). If a render pass
is wanted anyway: hosting with `--net-host 27420 0x07000023` and joining with
`--net-join 127.0.0.1:27420` from a second copy should show two Bonds in an Arena deathmatch
(the flag path), and the menu route (Multiplayer → Host Online → port → Start Listening; Join
Online → 127.0.0.1:27420 → Connect) should reach the same match through the lobby pages.

## ntfy

(ntfy result appended below after the finish step.)
