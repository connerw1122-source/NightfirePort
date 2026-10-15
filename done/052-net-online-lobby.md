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


---
## Report from the worker (done, 2026-10-15T23:55Z)

# net-online-lobby — report

Branch `s/net-online-lobby`, task 052. The online Host / Join front end on top of 611's MP menu
flow. **No drawing** — the main agent draws every page; this task delivers the pages, the state,
the flow, the getters and the tests. Design + evidence: `docs/systems/net-online-lobby.md`.

## What's done

1. **The mode choice** [S — the PS2 game has no online mode; the menu-def holds no network page
   (08000002 of 07000048.bin, parsed 2026-10-15)]. `Main → Multiplayer` now lands on the port-only
   `PAGE_MP_MODE 0x5000_0001` (Local / Host Online / Join Online); row 0 Local keeps the game's own
   `0x40000019` Join Game page byte-for-byte (the couch flow is unchanged — `mp_menu_flow` walks it
   through the new page). Port-only page ids live in a spare `0x5000_0000` block.

2. **Host** — `PAGE_MP_HOST 0x5000_0002`: the Port row (default **27420**, ◀ ▶ step ±1 clamped
   1024..65535, the mouse's value columns step it too) and Start Listening. Listen arms host mode:
   the session binds the port, the host's own panel (slot 0) joins with the lobby codename (the
   active profile's, else "Player 1" — the game's template [M C_RBMPCNAME 0x20d448]), and the page
   becomes the game's own 0x40000019 as the host lobby. Remote agents arrive into **panels 2-4**
   (slots 1-3) with their codenames — they mirror into the game's own `mp_join`, so the existing
   panel getters, the start checks [M C_SBMPOPTIONS 0x20e818:50-130] and the confirm-page lines all
   work over network agents unchanged. The host then runs the usual Scenario → Map → Setup →
   Options flow; every choice change is **published** to the session on the next tick (a diff
   against the last publish); Confirm at `P_MPCONFIRM 0x20ec00`'s summary calls
   `MpSession::start_match(&settings)` alongside the port's existing handover [M
   `mp_confirm_start`'s tail]. A busy port reports "Port already in use"; Back tears the session
   down and resets the panels.

3. **Join** — `PAGE_MP_NETJOIN 0x5000_0003`: a typed address (digits, dots, colon — an IPv4
   IP:PORT needs nothing else; 22-char cap; Backspace deletes; Ctrl+C/Ctrl+V work the page's own
   clipboard — **the system clipboard needs a crate, see "Needs"**). Connect gates the address
   (`mp_net_addr_ok`: 1-4 dot-separated digit parts ≤ 255, port 1..65535) and starts the handshake:
   **"Connecting..."** → on the session's link: Connected → `PAGE_MP_LOBBY 0x5000_0004`, **"Connection
   refused"**, or the front end's own 600-tick (10 s at the fixed 60 Hz) budget → **"Connection
   timed out"**. The lobby mirrors the host's published choices **read-only** through
   `mp_lobby_lines()` (the confirm page's own label pairs 0x3d4 Map / 0x3d5 Scenario / 0x24d Weapon
   Set / 0x3d8-0x3d7 limit / 0x3d6 Duration / 0x3da FF [M P_MPCONFIRM's enter]) plus the agent
   panels; it updates live as the host changes things. When the host starts, the client runs the
   same handover the host's confirm page runs (mp_match_settings + requested_level + the fade push
   [M `mp_confirm_start`'s tail]) from the session's `match_started()`. Back leaves cleanly.

4. **The session trait** — `MpSession` in `crates/nf-view/src/menu.rs` (`MpNetLink`, `MpNetError`,
   `MpNetAgent`, `MpNetChoices`, `poll/link/listen/connect/agents/host_choices/publish_choices/
   start_match/match_started/set_codename/close`). **`crates/nf-net` was NOT created** and
   `nf_game::net` was not touched: 051 (net-lockstep-core) owns the wire; building the crate twice
   in parallel would collide, and the front end needs only the surface above, which now lives
   beside its only consumer. When 051 lands, its session implements `MpSession` (or a ≤30-line
   adapter wraps it); `FrontEnd::mp_session` takes any `Box<dyn MpSession>`. **No socket is opened
   anywhere** — the front end holds the trait object and nothing else; `FrontEnd::new()` has
   `mp_session: None` (both online pages then report "No network session is wired").

5. **Keyboard wiring** (`lib.rs`): on `MpNetJoin` the raw keys type first (digits/dot/colon via
   `net_char_key` — physical keys, so numpad digits and numpad-decimal count; Semicolon is the
   colon's physical key), Backspace deletes, Enter confirms, Escape backs out, **every other key is
   swallowed** so the menu's W/A/S/D rows never fire mid-address. Ctrl+V/Ctrl+C are the page
   clipboard.

6. **Back cancels cleanly everywhere**: MpMode → Main; MpHost/MpNetJoin/MpLobby → the mode page
   (session closed, host mode off, panels + mirror reset, state Idle); hosting Back on the Join
   page → the mode page; the local flow's previously-stuck Back with nothing joined now pops to the
   mode page [M P_MPJOIN 0x20ca00's msg-0x6b arm sends the delayed Manager 0x5f pop, so the
   under-page shows — in the port's stack that is the mode choice (S; the game's own under-page is
   Main)]. Two Backs leave the couch page (one un-joins the agent first, as before [M C_RBMPSTART's
   0x6b]).

7. **Mouse** (the PC-mouse pattern): `list_rows_y()` gains the new pages' hit rows (640 space:
   MpMode 143/175/207, MpHost 143/175, MpNetJoin 160 — the draw contract's y's, mirrored in the
   doc §4); MpMode's rows, MpHost's Start row and MpNetJoin's address row are action rows (a click
   confirms); MpHost's port row keeps the ◀ ▶ value columns.

8. **Draw stand-ins** (`menu_overlay`): the port-only pages list their own getters (rows via
   `page_rows`, the port/address text, the lobby's agents + mirror lines, the status line) so
   nothing is blank — the real layouts are the main agent's (no drawing in this task's scope).

## Deviations from the couch flow (all deliberate, all [S])

- Remote agents arrive joined + ready; the game's per-agent codename/team/skin picks are the LOCAL
  agent's. Clients do not pick team/character in this cut — the host's published
  `MpSettings.players[slot]` carries the defaults (team 0, skin 0) for remote slots. A later task
  can carry per-client picks through the session.
- The client does not run the start checks; the host's confirm gate is the single gate.
- While hosting, Confirm on the host's own (ready) panel walks to Select Scenario (the game's
  ready gate [M P_MPJOIN's Menu_MPAreWeReady arm] stays for the couch flow).

## Verification

- **Tests** (each with a sister assertion that can fail; numbers/ids only, no drawing):
  `crates/nf-view/tests/mp_net_lobby.rs` — 9 tests over a deterministic `FakeSession` (no threads,
  no sockets — the test advises the transitions; the front end cannot tell it from the real one):
  the host flow (mode → port default 27420 → step → listen → the host's own panel → a remote agent
  lands in panel 2 with its codename → the whole Scenario → Map → Setup → Options → Confirm walk
  with the published choices → `start_match` settings with human_count 2 / master 1); port-busy
  report + the host's Back teardown; the join flow (typing incl. letter rejection and Backspace,
  paste's simple-set filter, Connect → Connecting… → the lobby with the mirrored choices → the
  match handover with requested_level); refused (+retry, +Back); the 600-tick timeout (sister:
  599 ticks still Connecting); the address gate (9 cases, and the bad address reported through the
  page); no-session reports on both pages; Local/Back routing; the mouse rows.
  `mp_menu_flow.rs` — the couch flow re-verified through the new mode page (9/9); `menus_flow.rs`
  37/37; `mp_menu_lists_live` 5/5; `mp_bots_live` 2/2.
- **Suite** (`tools/test/suite.py`, release, `--test-threads=1`, on the final tree **merged with
  origin/master** — the merge took async-level-load; lib.rs auto-merged, WORKLOG both kept): the
  detached background run dies between tool calls in this sandbox (a known constraint, same as
  mp-match-rules and async-level-load reported), so foreground `--only` chunks ran:
  - chunk A (mp/menu family + the four lib tests): 15 binaries, **415 passed / 0 failed**;
  - chunk B (rng/fields/trig/tuning/level/diff/spawner/pickup/progress/inputs + async-load +
    objective_state): 14 binaries, **40 passed / 0 failed**;
  - the two full-suite attempts together covered another 43 binaries (0 FAILED / 0 TIMEOUT
    observed) before the 10-minute tool window killed them — the ~360-binary pass does not fit the
    sandbox window; the three disc_golden tests and the no-GPU render tests were not re-verified
    here (they are the suite's known skip/fail set on this sandbox).

## What's left / for the next session

1. **Wire a real session** (051's lockstep): construct `Box<dyn MpSession>` where the host builds
   the front end (`mp_session = None` today) and merge the trait if 051's API differs.
2. **The main agent draws** the four pages (getters: `rows/row_label`, `mp_net_addr`,
   `mp_net_port`, `mp_net_status`, `mp_lobby_lines(lang)`, `mp_join` panels, `mp_net_state`;
   hit-row y's in §4) and renders them against the couch pages' look.
3. Per-client team/character picks through the session (the deviation above).
4. The system clipboard (needs a crate — see Needs).
5. A second-machine playtest (loopback is what the tests cover).

## Needs (asking per the no-new-crate rule)

- **The system clipboard** for Ctrl+V (paste is implemented against the page's own buffer): reading
  the Wayland/X clipboard needs a crate (`arboard` or `wl-clipboard-rs`). The in-app clipboard
  stands in; a crate would make Ctrl+V paste from other apps.
- Nothing else — the session trait is std-only by design (`std::net::UdpSocket` inside 051's
  implementation, no crate).

## Live checks for the main agent (no GPU in the sandbox — nothing rendered here)

1. `--bond` is not needed (menu pages): open Main → Multiplayer and confirm the mode page shows
   the three rows (Local / Host Online / Join Online) with the couch flow intact under Local.
2. Host Online: the port row shows 27420, ◀ ▶ step it; Start Listening shows "Listening" in the
   status line once a session is wired (with none, "No network session is wired").
3. Join Online: typing an address shows the digits as typed; a refused/timeout shows its message
   on the address page; a connected client's lobby shows the host's choices as they change.
