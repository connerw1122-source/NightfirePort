**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `mp-screens`, the missing multiplayer screens

The user, 2026-10-16 evening: "missing mp screens". List every multiplayer front-end page in the game (the page
table in ACTION.ELF .data, the P_MP* handlers, the menu-def pages) and which the port lacks or leaves blank
(character select, team setup, scenario options, map select, bot setup, the per-player confirm/ready screens,
the results/debriefing pages...). Port each missing page's data/state/flow in menu.rs with [M] citations; the
main agent draws them. Also: an F12 spot shot taken on a front-end page saves an all-dark image (the user's
nf-shots/001-07000048.png) — make `spot_shot` capture what the window shows on menu pages. Report the page list
with ported/missing so the main agent can draw and render-check each.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-screens`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-17T03:46Z

---
## Report (done, 2026-10-17T04:56Z)

# mp-screens — the multiplayer front-end pages

The user (2026-10-16 evening): "missing mp screens". Two deliverables: the page census with
ported/missing status (so the main agent can draw and render-check each), and the F12 spot-shot
bug that saved an all-dark image on a front-end page (nf-shots/001-07000048.png).

## What was done

**The census** (`docs/systems/mp-screens.md`): the MP front end is **13 pages** bound by
`Handler_HandleMessage` 0x2029e0's dispatch switch (there is no static page table — the
research agent scanned every word of ACTION.ELF; each handler address occurs once, in the inert
runtime symtab copy at 0x318770). The walk: P_MAIN 0x20a7b0 → P_MPJOIN 0x20ca00 (per-pad join +
codenames) → P_MPSCENARIO 0x217348 (the wheel; **row 0 = Quick Game** — random map, frags 10,
time 10, 3 bots → straight to the confirm) → P_MPMAP 0x217468 → P_MPSETUP 0x20d7a8 (per-agent
team/character/handicap) → P_MPOPTIONS 0x217678 (the hub: start/bots/rules/mods) → P_MPRULES
0x20f790, P_MPPLAYERMODS 0x2104d0, P_MPENVIROMODS 0x210030, P_MPBOTS 0x217790 → P_MPBOTCHOOSE
0x210cd8 → P_MPBOTSETUP 0x211270 → P_MPCONFIRM 0x20ec00 (the launch: MPSettings+0x180 = 1, the
frag limit ×60, `ResetMap_LevelToLoad` + the 84-frame fade + `MenuManager_Delete`) → in-match
pause quit routes to P_MPDEBRIEFING 0x212198 (the results: the sorted score rows, the winner
line, the team comparison MPGame+0x384/+0x388, the rematch `C_MPDBG` 0x217860). Every page's
handler, purpose, label ids and port state is in the doc with [M] citations, plus the
MPSettings (0x2a47a0) field map and the companion structs (mpjoin, mp_stuff, mpbots, mp_level,
mp_scenario, mp_characters, mp_options, mp_bots).

**Port status**: a prior task (mp-menu-flow) already gave the port all 13 page ids, the flow
(`action()` arms) and the MP state mirrors — and the overlay DRAWS 11 of the 13 (wheels, join
panels, rules/mods lists, the debriefing). **Exactly two pages drew nothing**:
`menu_overlay`'s arm was literally `Page::MpSetup | Page::MpConfirm => {}` (menu.rs) — the
per-agent setup and the match summary, the two screens a player sits on longest. This task
delivers their draw-side data (no drawing — the main agent draws):

- `FrontEnd::mp_setup_panels(lang)` — the 4 panels' joined/state (0/2/3/4/5), the 0x37c/0x37d
  prompt labels, the pick text (the bound 0x1c7/0x1c8 team pair, the character name, or the
  handicap's raw .data strings [D 0x30d390..0x30d3c8]) and the handicap value (the page-enter
  defaults 1/3/2/0 [M P_MPSETUP 0x20d7a8:20-23]) [M 0x20d7a8:15-89].
- `FrontEnd::mp_confirm_summary(lang)` — the six summary rows with the exact ids (MAP 0x3d4 +
  the arena's mp_level name, SCENARIO 0x3d5 + the scenario name, WEAPON SET 0x24d + the
  per-value switch 0→0x1a0 … 10→0x1b2, TIME 0x3d8 / LIVES 0x3d7 for scenario 0x10 / NONE 0x3ca
  at −1 with the 0x10001de variants, POINTS 0x3d6, friendly fire 0x3da + OFF 0x1b6/ON 0x1b7),
  the blue/red human panels (the name, the "%s%d" handicap line, the sides) and the bots'
  portrait columns (blue-only in a free-for-all) [M P_MPCONFIRM 0x20ec00:143-389].
- `MP_CONFIRM_WEAPON_LABELS` — the switch as a table.

**The spot-shot bug**: the window draws every front-end page over its aux FMV backdrop through
the ONE movie plane, and the overlay skips its dark plate while `fe.backdrop_live` stands.
`spot_shot` built the overlay but passed **no movie frame** — the plane drew nothing, the plate
was skipped, and the front-end level's void rendered black: the all-dark png. The fix mirrors
the redraw's `movie_up` composition (`spot_movie_gates`, pure and unit-tested): the state-6
briefing always blanks the overlay, the intro/attract FMVs blank it once a frame exists, the
looping page backdrop draws under the overlay, and the shot now passes the live player frame
to the plane.

## How it was verified

`crates/nf-view/tests/mp_screens.rs` — 5 tests, **5/5 pass**:

- `spot_movie_gates_mirror_the_redraw` — the six gate cases (aux under overlay, the
  decode-ahead hold, intro/attract/briefing own the screen, plain gameplay) + the sister
  (the intro must never be Blank::Never — the bug class).
- `mp_confirm_summary_ffa_ids_and_values` / `_team_split` — the FFA and team ids/values with
  sisters (the weapon-set switch's last case; the −1 → NONE rows; the bot columns). **The tests
  caught a real getter bug**: the bots' columns counted disabled bots (bots_red 3 where the
  game would show 0) — fixed to count enabled bots only, per [M 0x20ec00:320-389].
- `mp_setup_panels_ffa_and_team` / `mp_setup_handicap_state_shows_the_data_string` — the FFA
  vs team prompts and picks, the unjoined panels, the handicap .data strings, with sisters.

Full run of the crates the task touches, foreground (`cargo test --offline --release -j2 -p
nf-game -p nf-view -- --test-threads=1`): **94 test binaries, 622 passed, 0 failed**. The
fast-suite runner (`tools/test/suite.py`) was started in the background per the task but was
killed between tool calls (this sandbox reaps background jobs, as the pause-music-1b and
bonus-007 entries recorded); the foreground run stands in. No ISO-gated disc reads changed, so
the slim-ISO `disc_golden` caveat does not apply here (and none failed).

The render side is the main agent's: the doc's last section lists the `--page` spots
(mpscenario/mpmap/mpscenopts/mprules/mpbots/mpbotchoose/mpbotsetup + the walk into MpSetup and
MpConfirm) and the two string ids (0x295/0x1c8 headers, the 0x30d3e0/0x30d2d8 handicap
prefixes) whose English the PCSX2 compare settles. The F12 fix itself needs a GPU to exercise
(this sandbox's wgpu has no adapter — `fix_0c_pylon_live` proved that in the bonus-007 session);
the composition parity is pinned by the unit tests instead.

## What's left

- The draws themselves for MpSetup/MpConfirm (the data getters are ready; the geometry lives in
  the menu-def parse's controls 0x100001a0-0x100001a2/0x1000022a and 0x1000022d-0x1000023f).
- The main agent's per-page render compare against PCSX2 (the doc lists the spots).
- The Quick Game map sample uses row 0 [S — the game samples Rand%7; the shared RNG is not
  wired into the front end].
- The 0x1000023f header pair (0x295/0x1c8) is recorded from the handler — the strings' English
  is unverified here (no USATxt dump consulted).

## Usage

not visible to me
