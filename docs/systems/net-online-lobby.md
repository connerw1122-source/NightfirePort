# net-online-lobby — the online Host / Join front end

Status: implemented (front-end model + session trait + fake; the real UDP session is 051's).
Evidence tags: [M] code (function + address), [D] disc, [L] EE snapshot, [S] suggested / port-only design.

## 1. What this is

The PS2 game has no online mode — every multiplayer path is couch play on four pads [D: the join
page joins agents through per-pad messages, P_MPJOIN 0x20ca00 msg 0x6a; there is no network page in
the menu-def (08000002 of 07000048.bin, parsed 2026-10-15)]. Online play is a **port feature**. The
plan is deterministic lockstep over UDP: every machine runs the same MP match from the same seed and
settings and exchanges only per-tick input (the plan is the main agent's [S]). This task adds the
**front-end model** — the pages, the state, the flow, the getters the draw code calls — and puts the
network session behind a trait so the real transport can land underneath it (051, net-lockstep-core)
without touching the pages again.

Nothing here renders. All drawing is the main agent's.

## 2. The page flow

The game's local flow is untouched where it stands [M mp-menu-flow, crates/nf-view/src/menu.rs]:

    Main row 1 "Multiplayer" → 0x40000019 Join Game → 0x4000001a Select Scenario → 0x40000013
    Select Map → 0x40000051 Setup → 0x40000012 Options → 0x40000049 Confirm → the match
    [M C_GOMULTIPLAYER 0x216a78 → P_MPJOIN 0x20ca00 → P_MPSCENARIO 0x217348 → P_MPMAP 0x217468
    → P_MPSETUP 0x20d7a8 → P_MPOPTIONS 0x217678 → P_MPCONFIRM 0x20ec00].

The port inserts ONE choice page between Main and the game's Join Game page [S — port-only]:

    Main row 1 → PAGE_MP_MODE 0x5000_0001 (Local / Host Online / Join Online)
      row 0 Local        → 0x40000019 (the game's couch flow, byte-identical to before)
      row 1 Host Online  → PAGE_MP_HOST 0x5000_0002 (port number + Start Listening)
                         → 0x40000019 (the Join Game page AS the host lobby)
                         → the usual Scenario → Map → Setup → Options → Confirm flow
      row 2 Join Online  → PAGE_MP_NETJOIN 0x5000_0003 (typed IP:PORT, Connect)
                         → PAGE_MP_LOBBY 0x5000_0004 (the host's choices, read-only)
                         → the match (the host's Confirm hands both machines over)

The port-only page ids sit in a spare `0x5000_0000` block so they can never collide with the
game's `0x4000_00xx` page ids [S].

### 2.1 Host

- `PAGE_MP_HOST` has two rows: **Port** (default 27420 [S]) and **Start Listening**. ◀ / ▶ on the
  port row steps it ±1 (clamped 1024..65535); the mouse's value columns step it too (§4).
- Start Listening calls `MpSession::listen(port)` [S §3]. On success the front end arms host mode:
  the host's own panel (slot 0) joins automatically as the active codename (or "Player 1" — the
  game's template name, Txt 0x1c3 + slot [M C_RBMPCNAME 0x20d448]), state 5, ready, and the page
  becomes the game's own 0x40000019. Remote agents arrive into **panels 2–4** (slots 1–3) as the
  session reports them; their codenames show in the panels through the existing panel getters
  (mp_join — the draw code needs nothing new).
- While hosting, the host walks the game's own Scenario → Map → Setup → Options flow unchanged.
  Every choice change is published to the session (`MpNetChoices`, §3) on the next `tick()`.
- Confirm on the host's own panel (already ready) proceeds to Select Scenario — the game's ready
  arm routes through Menu_MPAreWeReady → 0x4000001a [M P_MPJOIN 0x20ca00's msg-0x50 arm]; while
  hosting the port skips the panel's own ready toggle (the host is ready by definition [S]).
- P_MPCONFIRM's confirm (0x20ec00:53-94) builds `MpSettings` and hands the match over [M — the
  port's `mp_confirm_start`]; while hosting it also calls `MpSession::start_match(&settings)`.
- Back anywhere on the host path cancels: `MpSession::close()`, host mode off, the panels reset,
  Back to the mode page [S].

### 2.2 Join

- `PAGE_MP_NETJOIN` is one text row: the typed address. The keyboard types **digits, dots, colon**
  (an IPv4 `IP:PORT` needs nothing else), Backspace deletes, Ctrl+C / Ctrl+V copy and paste the
  page's own clipboard buffer [S — the system clipboard needs a crate, see the report]. Length cap
  22 (`255.255.255.255:65535`). Confirm with an unusable address keeps the page and reports it.
- Confirm (Connect) calls `MpSession::connect(addr)` and the page shows **Connecting…**. The front
  end gives the session 600 ticks (10 s at the fixed 60 Hz [M the menu runs on the game's frame
  tick]) to connect; past that it shows **Connection timed out** and closes the attempt [S].
  A refused handshake shows **Connection refused** [S]. Back cancels and returns to the mode page.
- Connected: the page becomes `PAGE_MP_LOBBY` — the mirror of the host's lobby, read-only: the
  agent panels (the same mp_join mirror as the host's page) and the host's current choices as
  `mp_lobby_lines()` (the confirm-page line labels 0x3d4 Map / 0x3d5 Scenario / 0x24d Weapon Set /
  0x3d8-0x3d7 limit / 0x3d6 Duration / 0x3da Friendly Fire [M P_MPCONFIRM's enter builds these
  pairs; the lobby reuses the labels]). The lobby updates as the host changes things, until the
  host starts the match.
- Match start: the session's `match_started()` yields the host's `MpSettings`; the client runs the
  same handover the host's confirm page runs (mp_match_settings + requested_level + the fade push)
  [M the port's mp_confirm_start tail]. The client's copy keeps the settings as published; the
  host's copy has `master = 1` [M +0x180 = 1] and the client's carries what the session gave it.
- Back on the lobby leaves cleanly (close, panels reset, mode page).

## 3. The session trait (`MpSession`, in crates/nf-view/src/menu.rs)

The front end owns **no sockets**. Everything it needs from the network sits behind one trait [S]:

```rust
pub enum MpNetLink { Idle, Listening, Dialing, Connected, Refused, TimedOut, Lost }
pub enum MpNetError { BadAddress, Busy, Denied }
pub struct MpNetAgent { pub slot: usize, pub codename: String, pub ready: bool }
pub struct MpNetChoices { scenario, level, weapon_set, radar, friendly_fire,
                          frag_limit, time_limit, player_mods: [u32; 2], envmods: [u32; 5] }
pub trait MpSession {
    fn poll(&mut self);                        // drive the transport once per menu tick
    fn link(&self) -> MpNetLink;
    fn listen(&mut self, port: u16) -> Result<(), MpNetError>;
    fn connect(&mut self, addr: &str) -> Result<(), MpNetError>;
    fn agents(&self) -> &[MpNetAgent];         // slot 0 = this machine's agent, 1..3 = remote
    fn host_choices(&self) -> Option<MpNetChoices>;   // the host's lobby choices (client side)
    fn publish_choices(&mut self, c: &MpNetChoices);  // the host pushes its choices
    fn start_match(&mut self, s: &nf_game::mp::MpSettings);
    fn match_started(&self) -> Option<nf_game::mp::MpSettings>;
    fn set_codename(&mut self, name: &str);
    fn close(&mut self);
}
```

Why a trait and not a crate: 051 (net-lockstep-core, in flight on `origin/s/net-lockstep-core`)
owns the wire (deterministic lockstep over `std::net::UdpSocket`, no crate downloads). Building the
crate twice in parallel would collide; the front end needs only the surface above. When 051 lands,
its session implements this trait (or a ≤30-line adapter wraps it); `FrontEnd::mp_session` takes
any `Box<dyn MpSession>`. **nf-net was NOT created** — the trait lives beside its only consumer.

The front end mirrors the session into the game's own structures (mp_join slots), so the existing
join-page getters, the start checks (C_SBMPOPTIONS 0x20e818:50-130's team/FFA gates) and the
confirm-page line builder all work over network agents unchanged [M].

Deviations from the couch flow (all [S], all for the report):
- Remote agents arrive joined + ready; the game's per-agent codename/team/skin picks at
  0x40000019/0x40000051 are the LOCAL agent's picks. Clients do not pick team/character in this
  first cut — the host's published `MpSettings.players[slot]` carries the defaults for remote slots
  (team 0, skin 0). A later task can carry per-client picks through the session.
- The client does not run the game's start checks; the host's confirm gate is the single gate.

## 4. Mouse support (the PC-mouse pattern)

`mouse_move` hovers via `list_rows_y()`; the new pages add their hit rows (640×480 space, the draw
contract): MpMode 143/175/207, MpHost 143/175, MpNetJoin 160 [S — port-only pages, the y's are
these getters']. `mouse_click` treats every MpMode row, the MpHost Start row and the MpNetJoin
address row as action rows (a click confirms); the MpHost port row keeps the ◀ ▶ value columns
(x 340..400 left, 400..600 right — the option pages' own columns [M mouse_click's value columns]).

## 5. Tests

`crates/nf-view/tests/mp_net_lobby.rs` — headless (no ISO), numbers/ids only, each with a sister
assertion that can fail. A deterministic `FakeSession` (no threads, no sockets — the test advises
the transitions) implements `MpSession`:
- host side: mode → host page (port default 27420) → step → listen (the fake records the port) →
  the Join page with the host's own panel joined → a remote agent arrives in panel 2 with its
  codename → the whole Scenario → Map → Setup → Options → Confirm walk with the choices published
  to the fake → start_match settings handed over;
- join side: typed address (digits/dots/colon, Backspace, paste) → Connect → Connecting… →
  connected → the lobby mirrors the host's choices → match start hands over;
- refused / timed-out / Back-cancels-cleanly on both sides, page ids of the port-only pages.
