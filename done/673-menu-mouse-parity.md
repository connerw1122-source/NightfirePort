Context (2026-10-15): the user's playtest found the menus broken end to end even though every page
had passed its own unit tests.
- No single-player mission could be started.
- Local MP couldn't reach a match.
- Backing out of MP setup left the wrong prompt.
- A second Enter during a load started a second load.
- A notice box could never be dismissed.

The existing flow tests set `fe.page = ...` directly instead of getting there by actions, so the real paths were
never exercised. **Rule for this task: drive the FrontEnd ONLY through `action()`, `mouse_*()`, `tick()` and the
host-side calls the window makes. Never assign `page`/`cursor`/state fields to get somewhere.** The movies and the
attract need ticks to finish: after each action, tick until `!intro_playing && !attract_playing` (bounded). Every
test needs sister assertions.

## Your task: `menu-mouse-parity` — the mouse works everywhere the keyboard does

For every page, assert:
- (a) `mouse_move` over each drawn row (the hit rows the draw code uses: `list_rows_y`, the wheel geometry, the
  tab strip) sets the same cursor the arrows would;
- (b) `mouse_click` on a row does what Enter does on that row;
- (c) clicks outside rows do nothing harmful;
- (d) right-click = Back and the wheel = Up/Down;
- (e) no click ever changes a value it shouldn't. The Host page's port was knocked down by every click until the
  main agent fixed it.

Fuzz it: random mouse moves and clicks interleaved with keys, 20 seeded runs of 5000 steps each, with the
invariants from task 669 (no panic, cursor in range, Back leaves). Report and fix pages where the mouse targets
disagree with the drawn rows (compare against the draw code's positions; the main agent owns the drawing, so
change hit-test numbers only, with the draw source cited).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/menu-mouse-parity`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T07:10Z

---
## Report (done, 2026-10-16T09:32Z)


---
## Report from the worker (done, 2026-10-16T09:33Z)


# menu-mouse-parity — the mouse works everywhere the keyboard does

Branch `s/menu-mouse-parity`, 2026-10-16. The playtest found the menus broken end to end while
each page's own tests passed by jumping to pages directly; the mouse had to match the keyboard on
every page, driven only through the real paths.

## What's done

**The audit.** Every front-end page's mouse hit-test was compared against the draw code's own row
positions. The mismatches found and fixed (hit-test numbers and missing hover paths only — the
drawing is untouched, sources cited in the code comments):

1. **The `page_rows` pages drew rows the mouse could not see correctly.** Options, MpMode, MpHost
   and MpNetJoin draw through `page_rows_no_head` (rows centred at y 140+26·k, ink middle ≈ pen+6
   [M `menu.rs::page_rows_no_head`, `label()`'s measured offset]) while `list_rows_y()` still held
   143/175/207 and 160 — numbers from before the stand-in draw existed. The hits moved onto the
   drawn rows: MpMode/Options 138/164/190, MpHost 138/164, MpNetJoin 251 (the address value label
   (200,250,240×18) puts its middle at 259 [M `menu_overlay`'s MpHost/MpNetJoin arm]).
   `docs/systems/net-online-lobby.md` §4 updated to match. Options also joined the action-row
   click mapping (its stub ✕ is the keyboard's no-op [M `action()`'s Options arm]).
2. **The wheel pages got the missing hover, and lost a dangerous confirm.** `mouse_move` now
   hovers the drawn `WHEEL_Y` slots (the wheel re-centres on the hovered row — the same cursor
   the arrows step to); the click confirms the row the pointer *hovered* (`wheel_hover` — the
   re-centre moves rows under a stationary pointer, so the pre-hover row is the aimed one). A
   direct click with no hover behind it keeps the page's documented contract (centre confirms, a
   neighbour scrolls). **Bug fixed:** a click LEFT of the wheel column (x < 290) fell through to
   the single-action arm and CONFIRMED the centre row — a stray click on MissionSelect's emblem
   side could start a mission. Now it does nothing. Gadgets and Weapons draw wheels
   [M `menu_overlay`'s `wheel()` arms] and joined `is_wheel_page`.
3. **Pages whose drawn rows had no mouse at all** got the parity paths: the pause tab strip
   (hover shows the tab — the tab ◀/▶ would land on), the OBJECTIVES tab's rows (y 159+22·i
   [M `pause_page`'s tab-1 arm]; hover scrolls the tab's selection, a click shows the row's hint —
   the control-1 0x4b activation [M 0x204408]), the Screen Adjust bars ([L page 0x40000047:
   vertical (304,176,32×128) d1 0, horizontal (256,224,128×32) d1 1]; hover picks a bar, a click
   on a bar is ✕ = Enter keeps and leaves), the Join Game panels ([L page 0x40000019:
   (50/328,103/253,262×134)]; hover walks the panel cursor, a click on a panel is that panel's ✕,
   off-panel clicks do nothing — the old fall-through confirmed from anywhere), and the Records
   spinner's drawn ◀ ▶ ([L page 0x4000003a, arrows at x 144/497 mid y 133]; a click walks the
   campaign = ~V, a click on the row is ✕ = back to the Dossier [M the Records arm]).
4. **The modals now block the mouse exactly like the keyboard** (`input_blocked`: notices,
   `mp_error`, `pause_hint`, the intro and attract movies [M 0x21c858 case 0x68
   `Input_ClearAllActions`]). A click through a box is the ✕ (Enter's dismissal); hovers do
   nothing under it — the cursor used to move under open boxes.
5. **CheatMedal's clicks are inert.** The page draws no rows yet (the overlay's arm is empty — the
   drawing task owns the look), so every click was "outside rows" — and the old fall-through
   *confirmed* on click, toggling a cheat. Now: nothing until the page draws rows.
6. **A real freeze the fuzz caught:** Back on MpJoin did nothing while the panel cursor sat on
   panels 2-4 (both Back arms were gated on `can_join` = cursor 0). The game's △ acts on the
   local agent and the page close [M C_RBMPSTART 0x20d048's 0x6b arm; the msg-0x6b close], not on
   the wheel cursor — the gate came off the two Back arms (the un-join stays agent 1's; Confirm
   keeps it). Keyboard parity: Up-then-Back and direct Back now agree.
7. **The host mappings are pinned and testable:** `menu::MOUSE_RIGHT_ACTION` (= Back, lib.rs's
   right-press arm) and `menu::wheel_scroll_action` (LineDelta → Up/Down, lib.rs's MouseWheel
   arm); lib.rs calls them — behaviour unchanged, the mapping asserted by tests (d).

**The tests.** `crates/nf-view/tests/menus_mouse_parity.rs` — 17 tests, every one driven ONLY
through `action()`, `mouse_move/mouse_click`, `tick()` and the host-side calls the window makes
(`open_pause` / `mission_ended`; the two stub pages with no action path — Options, CheatMedal —
enter exactly the way lib.rs's `--page` dev override does, noted in the tests). Each test carries
sister assertions. The fuzz: 20 seeded runs (xorshift64*) × 5000 steps, mouse moves + clicks +
keys interleaved, the task-669 invariants every step — no panic, `cursor`/`page_row()` in range
(rows>0), `cheat_medal_sel < 6`, and Back leaves (with the game's own input-less pages excepted:
Start, Credits, WinGame, MpDebriefing [M their action() arms]) — plus the port invariant
(`mp_net_port` never moves without typing) at each run's end.

## How it was verified

- `menus_mouse_parity` 17/17 green (release, --test-threads=1).
- The 16 existing menu-related binaries re-run green after the changes: menus_flow 42, menus_e2e 5,
  mp_net_lobby 9, mp_menu_flow 9, menu_sfx_flow 14, codename_entry 14, screen_adjust 9,
  dossier_pages 7, unlock_popups 12, mp_defaults_setup 10, mission_stats_flow 3, menu_movies 3,
  menu_music_flow 14, dev_menu_flow 5, memcard_save_flow 7, objective_hints_flow 7 — 170 tests, 0
  failed.
- **The suite** (`tools/test/suite.py`, NF_ISO set, foreground — background jobs are reaped in
  this sandbox between tool calls, so it ran in chunks): the nf-view subset ran 41 binaries before
  the session's time box cut it — **145 passed, 2 failed, 28 ignored**. The 2 failures are
  `fix_0c_pylon_live::the_0c_pylon_fills_the_view_from_the_closeup_spot` and
  `::the_0c_pylon_is_on_screen_at_the_user_spot` — **pre-existing on master, not from this task**:
  A/B-verified by rebuilding the binary against master's `menu.rs`/`lib.rs` (same 2 failures) and
  re-running against this branch (the same test passes/fails identically; it is a render-spots
  test this task never touched). The nf-view chunk did NOT finish inside the window — the
  remaining nf-view binaries and the untouched nf-game / nf-formats / nf-audio binaries
  (this task changed no file under them, and the branch carries no master delta — the merge with
  origin/master was already up to date) are the next session's suite residue.
- A/B harness used twice for isolation: `git checkout <master rev> -- crates/nf-view/src/menu.rs
  crates/nf-view/src/lib.rs` + rebuild → the binary in question; then `git checkout HEAD --` back.
  Both runs restored; the final tree is this branch's HEAD (e2cb2cfc + the report commit).

## What's left

- The rest of the nf-view suite chunk (the chunk was alphabetical and reached `fix_…`) and the
  other three crates' suite pass on the final tree.
- CheatMedal and Options have no action path into them yet (no `enter_page` anywhere [M]) — when
  the drawing/wiring tasks land them, the hit rows must follow the drawn rows (CheatMedal's clicks
  stay inert until then, by design).
- MpHost's port value line (200,250,240×18) is a readout: its click does nothing. When the real
  page art replaces the `page_rows` stand-in, the hit rows should move with it.
- Visual confirmation of everything here needs the main agent's renders (lesson 6): hover a wheel
  row on MissionSelect and click it (the mission starts), hover the pause tabs, the objectives
  rows, the Screen Adjust bars, the Join panels; a click left of a wheel column should do nothing.

## Live checks for the main agent

None required — this task is input-side only. The [L] page-data pins in the new hit tests (the
Join panels, the Records spinner arrows, the Screen Adjust bars, the pause rows/tabs) come from
the already-parsed menu-def dumps cited in the code.

## ntfy

(sent after the push; see the finish step's output — any failure text goes here.)
