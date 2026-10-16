# menu-graph-walk — the headless menu-graph explorer

Task 669 · branch `s/menu-graph-walk` · 2026-10-16

## What's done

A headless explorer (`crates/nf-view/tests/menu_graph_walk.rs`) walks the front end's reachable
menu states from `FrontEnd::new()` — every state reached **by actions and ticks only** (`action()`,
`tick()`, and the host-side calls the window makes: `open_pause`, `mission_ended`,
`results_enter`/`stats_enter`/`bonus_enter`, `win_movie_finished`, and the documented host `take()`
of `requested_level` when a requested level finishes loading). `page`/`cursor`/state fields are
never assigned to get anywhere. Movies and the attract are ticked to their end after each action
(bounded by 1100 ticks; the intro budget is 850 [M `INTRO_TICKS`]).

- **Breadth-first walk** over (page, cursor, page sub-state) states; edges are the seven `Action`s
  plus tick bursts and the attract timeout edge (0xa8c frames [M `ATTRACT_FRAMES`]), capped by a
  visited set, a per-(page,cursor) node cap, a per-page cap, and a depth bound (22).
  `FrontEnd` is not `Clone` (it holds `Box<dyn MpSession>`), so the walk stores paths and re-drives
  each node from a fresh `FrontEnd::new()` — deterministic (the headless front end is a pure state
  machine: fixed 60 Hz, no wall clock, no RNG in the menus).
- **Five roots** (all from `FrontEnd::new()`): the raw boot (title/intro/attract), the boot driven
  to Main by actions (the whole front-end graph), the pause root (mission loaded by actions, then
  the host's `open_pause`), the results root (over a loaded mission, so the BFS takes
  `mission_ended(Complete|Failed)` + `results_enter`), and the win root (the final mission
  completed under the host's `--unlock-all` dev flag [M 0x20b8d8:18] so `just_completed`/`new_reward`
  route to the bonus/win pages).
- **Invariants at every step** (per edge result and per tick inside settles): no panic (a panicking
  edge is caught, reported, and the walk continues), `cursor < rows()` on every rowed page,
  `page_id()` is a known page id, Back leaves every non-root page within 8 presses, every reached
  page can get back to Main by actions (a mini-BFS, depth 150 — the credits roll alone is ~8360
  frames ≈ 131 bursts [M `Menu_SetupCredits` 0x1f9a88 stores 0x242 lines]), `requested_level` is
  never set twice for one confirm (tracked across steps; the second-load playtest bug class), and
  no page is a dead end (no action and no tick burst changes anything).
- **Randomized version**: 20 seeded runs × 5000 random actions/ticks over the same invariants,
  re-booting the front end when the host takes over (menu closed / match handed over).

## Result: coverage 38/42 pages, all invariants hold

The walk + probes + randomizer are **green** (`cargo test -p nf-view --test menu_graph_walk` — 3
tests, ~6 s). Coverage per root is printed by the test; the union:

- **38 pages reached and invariant-checked** — every front-end page (SP flow, the codename wheels
  and option pages, the dossier wheel, MP local flow through MpConfirm, the online Host/Join pages)
  plus Pause, Results (both outcomes), Stats, NfBonus, WinGame and Credits (the last four through
  the mission-flow host calls, which is how the real window reaches them).
- **No dead ends**, no cursor-out-of-range states, no double `requested_level`, no page that Back
  cannot leave, and every reached page routes home — the two input-dead pages are the game's own
  design: the win page takes no input, its route is the host's movie-end call [M P_WINGAME 0x215100
  has only the 0x4c/0x50 cases]; the credits take no input at all, the roll cannot be skipped
  [M P_CREDITS_Handler 0x2152b0 has no input cases].

## Fixed: the MP setup page opened with an out-of-range cursor

`enter_mp_setup` never reset `self.cursor`, so the map wheel's cursor rode in — MpSetup drives one
pick row (`rows() == 1`) and the walk tripped `cursor 1 out of range on page MpSetup (rows 1)` on
every root. Fix: the page cursor starts on the pick row [M C_RBMPSETUP 0x20de58's 0x4c re-selects
the per-agent list's row on every state step; the port keeps the pick in `mp_setup_sel`, so the row
cursor is 0]. This is exactly the class of bug the playtest hit ("backing out of MP setup left the
wrong prompt" — state riding between pages), caught now by the invariant instead of a playtest.

## Unreached pages (4) — each with its reason

| page | id | why the walk can't reach it |
|---|---|---|
| Options | 0x40000068 | No opener exists anywhere in the decompile (a whole-decompile grep for the id finds nothing) — the port's three-row stub page appears to be unreachable in the game itself. |
| MpDebriefing | 0x40000033 | Only entered by quitting during a running MP match [M C_GCPAUSE 0x204408:146] — the walk has no MP match sim; the page's own Confirm route is ported (back to the scenario wheel). |
| MpLobby | 0x50000004-ish | Needs a live (or fake) `MpSession` behind the join flow; the walk runs session-less, so Dialing always ends Refused/TimedOut. The lobby page's graph is covered by `mp_net_lobby` tests with a fake session. |
| CheatMedal | 0x4000004c | The opener is an open question from the unlock-popups task: nothing in the decompile opens it and nothing reads `medal_from_cheat` 0x30cc74 (docs/reports/unlock-popups2.md §"open"). The page's handler is ported (6-row list). |

## Live checks for the main agent

1. **MpSetup cursor reset** [L]: open MP → Local → join agent 1 → scenario → map; on the setup
   page the agent-1 list's selection should be the stored team/character (not a leftover row), and
   the page's row cursor effectively row 0. Address: the per-agent list control's selection byte
   (mp_stuff+0x360 slot state); expect state steps 2→3→4→5 with the list re-selected each step
   [M C_RBMPSETUP 0x20de58's 0x4c].
2. **The Options page opener** [D]: is page 0x40000068 ever opened on the disc (any handler sending
   msg 0x44 with it, or a menu-def link)? If not, the port's `Page::Options` stub is dead code and
   can be deleted in a later cleanup.
3. **The CheatMedal opener** [D/L]: play a cheat code that grants a medal (the Secret Unlocks
   grid) and watch whether 0x4000004c opens over the front end; gdbwatch on `medal_from_cheat`
   0x30cc74 for any reader after the write.

## What I ran

- `cargo test --offline -p nf-view --test menu_graph_walk` — the walk (5 roots + probes), the
  randomizer (20×5000), green; coverage 38/42 printed.
- The full fast suite on the final tree (`NF_ISO=… nohup python3 tools/test/suite.py`) — it builds
  and runs all four crates' binaries including nf-view's (menus_flow, mp_menu_flow, menu_sfx_flow,
  codename_entry, unlock_popups, memcard_save_flow, … and the walk's own 3), which is the touched
  crate's run; see the total line below. On the slim ISO only the 3 `disc_golden` tests may fail.

## What's left

- The 4 unreached pages' routes (needs the live/decomp answers above); a fake-`MpSession` root
  would pull MpLobby into the walk cheaply.
- The per-(page,cursor) caps (24/400) skip the deepest option-value combinations (weapon-set ×
  radar × …); the randomized runs sample them, but a value-aware walk (step each wheel once per
  cursor instead of BFS-ing the lattice) would cover every wheel value exhaustively.
- The walk asserts `page_id()`/`flow.page` are known ids but not that `fe.page` and `flow.page`
  agree — the game keeps two page variables (the port splits them deliberately in places); a
  follow-up could pin the exact agreement points from the handlers.

## Suite total

The fast suite on the final tree was still in its test phase when the sandbox deadline hit: the
build line is `build ok: 387 test binaries in 340 s`, the run was cut short before the total line
could print (the runner buffers its output). What was verified directly: the walk's own 3 tests
green (`cargo test -p nf-view --test menu_graph_walk`), and the branch-point tree's suite earlier in
the session (`build ok` + the nf-view flow binaries green on their own runs during development:
menus_flow, mp_menu_flow, menu_sfx_flow, unlock_popups, codename_entry, credits_data,
memcard_save_flow). The next session should re-run `NF_ISO=… python3 tools/test/suite.py` on this
branch and paste the total line here.
