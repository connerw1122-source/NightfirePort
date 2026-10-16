# net-match-host — the online MP match in the running app

Status: implemented (session + app wiring + tests). Evidence tags: [M] code (function + address),
[D] disc, [L] EE snapshot, [S] suggested / port-only design.

## 1. What this is

The PS2 game has no online mode — every multiplayer path is couch play on four pads [D: the join
page joins agents through per-pad messages, P_MPJOIN 0x20ca00; there is no network page in the
menu-def 08000002 of 07000048.bin, parsed 2026-10-15 by the lobby task]. Online play is a port
feature (the user, 2026-10-15: "I want online support where we can host a mp server and other
clients can join"). The plan (main agent): **deterministic lockstep over UDP** — every machine runs
the same MP match from the same seed and settings and exchanges only each agent's per-tick input; a
per-tick state hash detects desyncs. Direct connect: the host listens on a UDP port, clients join
by IP:port (internet play needs the host to forward the port; no relay/infrastructure).

This task wires the lockstep session into the running app: the match start (the same level from
the host's settings + seed), the per-tick input exchange with the sim gate, the per-machine
camera, the state hash, and the desync/disconnect path back to the lobby.

## 2. The session layer (consolidated 2026-10-16 into `crates/nf-net`)

> **net-consolidate**: this module (`nf_game::net`) is DELETED. The ONE session layer is 051's
> `crates/nf-net` (stronger protocol: acks + 32-tick bitmap + full unacked-window retransmit,
> input delay, hash piggyback, keepalives, deterministic shim — see `net-lockstep.md` §9); the
> features below that the app depends on moved into it or into `nf_view::mpnet`:
> the `MpSettings` handover (a tagged DATA tunnel from `NetCore::start_match`), the FNV `Hasher`
> (now `nf_net::hash::Hasher`, `Sim::net_state_hash` uses it unchanged), the lobby codenames
> (`Session::set_display_name` + `agents()`), the refuse-when-started gate, and the full
> `MpSettings` wire fidelity (protocol 2). The app surface lives in `nf_view::mpnet`
> (`NetCore` + `SessionProxy`): the flag path and the lobby's `MpSession` trait object wrap the
> same core; `App.net` adopts it when a match starts. The drop of a peer is NO LONGER terminal:
> `DropPolicy::Neutral` feeds the dropped slot NEUTRAL inputs and the match runs on (the game's
> own disconnected-pad shape [M `psiInput_PollDevices` 0x10ca78]) — the old `PeerLost` ends the
> match instead; the app's `CoreEvent::ClientLeft` just logs it.

The original 053 design (kept for the record): the networking lived in `nf_game::net`
(crates/nf-game/src/net.rs), not a new `nf-net` crate, for three reasons [S]: (a) it consumes
`mp::MpSettings` and `player::Input` directly — a crate would either depend on nf-game or
duplicate the types; (b) the lobby task's design doc already noted that building the same
networking module twice in parallel collides — a module inside nf-game merges more easily with
051's `s/net-lockstep-core` work when it lands; (c) no rendering and no new dependencies
(`std::net::UdpSocket` only) either way. The API is the one task 051
specifies: `poll() -> Vec<NetEvent>`, `local_input(tick, input)`, `ready_inputs(tick) ->
Option<[Input; 4]>`, plus `host(port)` / `connect(addr, codename)`, `start_match(&settings)`,
`send_state_hash(tick, hash)`, `agents()`, `close()`.

Wire protocol (one tag byte, little-endian; `PROTOCOL_VERSION`/`BUILD_ID` in the handshake so
mismatched exes refuse cleanly):

| tag | packet | direction | contents |
|-----|--------|-----------|----------|
| 1 | HELLO | client → host | version u16, build u32, codename (≤15 bytes) |
| 2 | WELCOME | host → client | the assigned slot u8 (host is 0; clients 1..3) |
| 3 | DENY | host → client | reason u8: 1 version/build, 2 lobby full, 3 match started |
| 4 | START | host → all | the serialized `MpSettings` (+ seed) — every machine builds the same level |
| 5 | INPUT | both ways | n u8, then n × { tick u32, packed `Input` (20 bytes) } — each packet repeats the last `REDUNDANCY` (3) ticks, so a lost packet is repaired by the next one |
| 6 | HASH | both ways | tick u32, hash u64 |
| 7 | LEAVE | both ways | — the receiver ends the match (PeerLost) |

`Input` packs to 20 bytes (4×f32 LE, a bit byte, i8 cycle pairs, i8 switch_to with −1 = None) —
the same fixed-size idea as the replay record's `TickInput` [M replay.rs]. `MpSettings` serializes
the match-defining fields explicitly (seed, scenario, limits, counts, players[8], …, 168 bytes).
The round trips are pinned by `net_session.rs::input_and_settings_pack_round_trip`.

Lockstep rule: `ready_inputs(t)` returns `None` while any live peer's input for t is missing — the
sim **stalls that tick** (never guesses). A peer silent for 5 s (wall clock, outside the Sim — the
session never feeds sim state, so the no-wall-clock determinism rule holds) is dropped: PeerLost,
the match ends. The game's nearest behaviour: the join page resets a panel when its agent leaves
[P_MPJOIN's per-pad leave messages], so the port ends the match and returns to the lobby rather
than impersonating the leaver with a bot — the bot groundwork (bots.rs) is data-only and not wired
into the sim yet.

## 3. The app wiring (`crates/nf-view`)

**Match start.** The menu's confirm builds `MpSettings` and sets `mp_match_settings` +
`requested_level` [M mp-menu-flow's `mp_confirm_start`, menu.rs]. Before this task the settings
were **never read**: the load path always built `MpSettings::deathmatch(level_id)` [M lib.rs's
`load_impl` mp block], and the menu path loaded through `start_async_load` → `load_with_carry`
with `mp = false` — the MP core never ran from the menu at all. Now `menu_after`/`movie_tick` move
`mp_match_settings` into `App::pending_mp_settings` alongside `requested_level` → `pending_level`;
`start_async_load`'s job calls the new `load_mp_carry(iso, archive, all, save, settings)` →
`load_impl(..., mp_settings)` which feeds `MpMatch::init(settings)` and seeds the level build.

**The seed.** `MpSettings` gains a port-only `seed` field ([S], no game field; new fields on their
own line at the struct end per the WORKLOG lesson). `DroneContext` carries `mp_seed` into
`Scene::build`, which seeds the shared generator: `Rng::new(if d.mp_seed == 0 { 0x1234_5678 } else
{ d.mp_seed })` — local play keeps the pre-net constant bit-exactly (mp_core_live's pinned
folding), an online match builds from the host's seed. The seed comes from the host's wall clock
**outside the Sim**: it is matchmaking data carried to every machine inside START, never sim state.

**All human slots spawn in slot order.** `load_impl`'s mp block now spawns every human slot's
`Bond::stand_at` in slot order 0..human_count with the same `get_spawn_point(team, slot, rng)`
draw sequence on every machine (the game's MP_Start calls `MP_GetSpawnPoint(team, NULL)` before
each `Player_Init` [M 0x183a50:120-123]). Slot 0's bond takes `sim.bond` (the pre-net behaviour);
the rest park in the new `Sim::mp_bonds: [Option<Bond>; 4]` until each machine calls
`Sim::swap_local_slot(its slot)` — the per-machine move of ITS bond into `sim.bond` (the camera
and the full local path) with the old local one back at slot 0. Sim state content is identical on
every machine; only the local/remote VIEW differs.

**The tick gate.** In the frame loop's tick (`for _ in 0..n_ticks`), an online session:
1. checks `ready_inputs(net_tick)` — `None` → the frame runs no tick (`ticked = false`, render
   interpolation unchanged) and **no input capture runs** (key edges survive the stall);
2. captures the local input as usual (keyboard + mouse; the dev-test merges are local-only), then
   hands it to `local_input(net_tick, input)` after all merges, so the submitted input IS what
   `step_bond` consumes;
3. runs the local `step_bond` (unchanged full path), then `Sim::step_net_remotes(&inputs, slot)`:
   each remote slot's `Bond::tick_ctx` with that slot's session input (the same movement core the
   local path runs — the local machine's weapon/view pipeline stays local in this first cut, see
   §5), then `MpMatch::set_slot` for every live slot (the slots' live positions were stale
   before), and `send_state_hash` every `NET_HASH_EVERY` = 15 ticks (4 Hz at the fixed 60 Hz).

**Desync / disconnect → the message box and the lobby.** The frame pump (`session.poll()`
per frame) reports `Desynced` / `PeerLost` / `Refused`; `App::net_fail` closes the session, shows
the front end's `net_notice` (the game's `Menu_CreateOptionBox` error box pattern [M mp-menus;
the port-only strings have no label ids, so the MpJoin page draws them raw]) and returns to the
lobby page (`Page::MpJoin`, panels reset). `MatchStarted(settings)` drives the same handover a
menu confirm drives: `pending_mp_settings` + `pending_level`, the menu closes, the level loads
through `load_mp_carry`.

**Runnable path.** `--net-host [port] [arena]` (default 27420, Arena 0x07000023) listens and arms
the demo match for the first join; `--net-join host:port` dials. The lobby pages themselves are
task 052's (`s/net-online-lobby`'s design doc: mode → host/join pages around the game's own join
flow); the session behind them implements the same surface this task wired.

## 4. The state hash

`Sim::net_state_hash(local_slot)` folds, in a fixed order ([S] — the field set is the port's
choice; the game's own MP has no netplay to copy): the drones' frame counter, the shared RNG
stream state (`Rng.{x,y,draws}` — the most sensitive desync detector: any extra/missing draw
diverges it at once), then per slot 0..3 the bond pose (pos ×3, yaw, pitch, substate, crouched;
the local slot reads `bond`, the others `mp_bonds`, empty slots fold a sentinel), then the MP
match state (game.state, clock, limit, all 8 slots' pos/score/kills/deaths/dead, the settings'
level/scenario/limits/seed/human_count). FNV-1a 64 (`nf_game::net::Hasher`). Equal hashes at a
tick = the shared state agrees; the exchange compares both directions at the same tick (whichever
side's hash arrives first is stored; the second one compares — pinned by
`net_session.rs::forged_hash_desyncs`).

## 5. Deviations and limits of this first cut (all [S], all for the follow-up)

- **The shared core is the movement**: remote agents tick `Bond::tick_ctx` (walk/strafe/turn/look/
  jump/crouch, the same ladder/wire/creepwall context as the local body, `water = None` — no swim
  state machine on a remote body yet). The LOCAL agent keeps the full `step_bond` (weapons, view,
  gadgets). Weapon FIRE from a remote agent is not simulated on this machine in this cut: its
  bullets/hits are the firing machine's local state, outside the hashed core. The follow-up is a
  per-agent shared weapon path (the game's per-player BLData block) — the MP rules API is ready
  for it (rules.rs's `player_killed`/`register_bullet_hit` are not called from gameplay yet).
  Interactions that mutate shared world state locally (doors via use_key, pickups) surface as a
  hash desync rather than silently diverging — that is the hash doing its job.
- **`ready_inputs` returns `Option<[Input; 4]>`** (051's text) with absent slots defaulted; the
  app ticks bonds only for live slots (human_count from the settings).
- **2–4 agents**: tested host + 1 client (two sims in one process, the 3 GB sandbox cannot hold
  four full sims); the session layer handles up to 3 clients (slots 1..3) and the level spawn
  loop is slot-count driven.
- **No input delay yet**: pure lockstep (advance t only when all inputs for t arrived). On
  loopback/LAN the stall is invisible; WAN latency stretches frames. 051's task carries the
  2–4-tick input delay + ack tuning — the session's INPUT redundancy is already in.
- **The lobby UI is 052's**: until its pages land, `--net-host`/`--net-join` run the match.

## 6. Tests

`crates/nf-game/tests/net_session.rs` (headless, no ISO, loopback, each with a sister assertion):
the pack round trips (a different input / a truncated settings blob disagree); host + client 1 000
lockstep ticks with the hash exchange (per-tick input equality on both sides); a forged hash
desyncs BOTH sides at that tick; a wrong build id is denied (reason 1) before any slot is
assigned; a silent peer is dropped inside a shrunk timeout (PeerLost, Failed).

`crates/nf-view/tests/net_match_live.rs` (NF_ISO-gated; serial — two full arena sims per test,
a 3 GB sandbox OOMs on parallel runs): two app sims in one process over loopback play 1 000 ticks
of scripted input through the real `load_mp_carry` path (the host's settings + seed on both
machines, both slots spawned in slot order, each machine swapped to its own slot) — the hashes
agree at every 15-tick exchange and the final poses cross-check in slot order; a forced desync
(an extra shared-RNG draw on one side at tick 30) is detected within one cadence; a disconnect is
reported as PeerLost and the match ends.

## 7. Live checks for the main agent

Nothing visual ships in this cut (no drawing changed; the MpJoin message box is the only new
pixels). If the main agent wants to see the flow: two machines (or two processes) with
`cargo run --release -- --net-host 27420 07000023` + `--net-join <host>:27420`, then:
- both windows load the arena with **two bonds at two spawn points**; each window's view starts
  at ITS agent's spawn (the host's at spawn 0);
- moving in one window moves that agent's body in BOTH windows (the remote body is the
  darker/no-weapon one — it has no view model in this cut);
- killing one process mid-match → the survivor pops the message box ("Player N left the match")
  over the lobby page.
