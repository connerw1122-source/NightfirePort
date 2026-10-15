# Report: mp-menu-flow (branch `s/mp-menu-flow`)

Task: port the multiplayer front-end flow and its data into nf-view's menu model — the page graph,
the lists and their state, headless-testable, no drawing. Deliverables: `docs/systems/mp-menus.md`
(the full write-up), `crates/nf-view/src/menu.rs` (the `Page::Mp*` variants, `FrontEnd` state,
`action()` handling, the draw getters), and the tests.

## What's done

1. **The page graph** — all 13 MP pages of the front-end menu-def (07000048.bin / 08000002),
   verified as PAGE records [L the dump] and wired through `action()`:
   Main → **Join 0x40000019** (C_GOMULTIPLAYER 0x216a78's delayed 0x44) → **Scenario 0x4000001a**
   (the ready gate; Quick Game row 0 jumps straight to the confirm) → **Map 0x40000013** →
   **Player Setup 0x40000051** (team for team games, character for FFA) → **Options 0x40000012**
   (Continue with the start checks / AI Bots / Game Rules / Player Mods / Enviro-Mods) →
   **Bots 0x40000027 → Bot Choose 0x4000003f → Bot Setup 0x4000002c** → **Confirm 0x40000049** →
   the match start. Backs close every page the game's own backs close. **Debriefing 0x40000033**
   is wired (confirm → the scenario page) though nothing ends a match yet.
2. **The lists** — the game's own tables lifted as consts and pinned to the disc:
   `mp_scenario` @0x2df508 (13 rows, with the reward gates from Menu_UnlockMPSettings 0x1ff698),
   `mp_level` @0x2df400 (8 arenas), `mp_options` @0x2dfbb0 (5), `mp_bots` @0x2dfd30 (5),
   `mp_characters` @0x2df640 (29) plus the `Menu_IsBotGood` side bytes from
   `default_bot_stats`+0x09 @0x26D2F0. Every row's preview/name/desc/id/unlock/locked-desc is in
   `mp_menu_lists_live.rs` byte-for-byte against the disc's ACTION.ELF.
3. **The state** — `FrontEnd` gained `mp_join: [MpJoinAgent; 4]` (the game's mpjoin layout and
   mp_stuff+0x360 state values), `mp_scenario`/`mp_level` (the MPSettings words), the wheel
   cursors, `mp_bots: [MpBot; 4]` (the 0x12-byte records with the msg-0x51 default characters
   6/7/8/9), the rules/mods values, `mp_error`, and `mp_match_settings`. Keyboard only, agent 1:
   Confirm joins / picks the codename / readies; agents 2-4 stay unjoined (the game joins them
   through their pads, msg 0x6a).
4. **The match handover** — the confirm page's Confirm mirrors P_MPCONFIRM 0x20ec00:53-94: the
   time limit ×60 (minutes→seconds), the config lands in `mp_match_settings:
   Option<nf_game::mp::MpSettings>` (scenario/level/limits/humans/bots/weapon set/radar/master/
   per-player teams+skins), and the level request rides the existing loader
   (`requested_level` + `GameFlow::reset_map_level_to_load`) — verified headless that the flow
   machine reaches the arena. The port's MP core (`MpMatch::init`) consumes exactly those fields.
5. **Draw getters for the main agent** (drawing is out of scope; the overlay's MP arms are empty):
   `rows()`/`row_label()` per page (including the join states' prompts 0x1e8/0x37b/0x1e9/0x39c and
   the composed "Setup Bot n : Playing Yes/No" rows), `mp_row_preview`, `mp_row_desc` (with the
   locked-desc swap), `mp_row_locked` (the dim), `mp_confirm_lines(lang)` (the summary's six
   lines), `mp_human_count`, `mp_bot_count`. `aux_movie_for` covers all 13 pages (0x07350048).

## How it was verified

- `crates/nf-view/tests/mp_menu_flow.rs` — 9 headless tests, each step with a sister assertion:
  the full Main→Join→Scenario→Map→Setup→Options flow and back-outs, the locked-scenario and
  Ravine-AI-Bots gates (both arms of `menu_unlock_everything`), the bots loop (choose/setup/
  counts), the start checks (the 0x39e / 0x389 / 0x1000311 error arms), Quick Game (the fixed
  config + 3 bots + the straight jump), the team setup, and the loader handover.
- `crates/nf-view/tests/mp_menu_lists_live.rs` — 5 ISO-gated tests reading /ACTION.ELF off the
  disc (file = vaddr − 0x106100) and pinning all five tables and the 29 side bytes + the
  stats-editable flags (+0x0D).
- What I ran: `cargo test --offline --release -p nf-view --test mp_menu_flow --test
  mp_menu_lists_live` → 14/14 green; then the full four-crate suite through
  `tools/test/suite.py` on the final tree (see "Full suite" below).
- Evidence discipline: every ported constant cites its decomp function + line (or disassembly
  address where the decompiler dropped the constant — e.g. the 0x40000013/0x40000051 page ids,
  the quick-start MPSettings stores, the C_SBMPMAP level-code store). The atlas's decimal-offset
  confusion (+0x420 vs +0x1A4) is settled in `docs/systems/mp-menus.md`.

## Full suite

`NF_ISO=… nohup python3 tools/test/suite.py` on the final tree (merged origin/master):
see the tail of this file's "Suite:" line — PASSED/FAILED summary filled in below before the
finish call. On the slim ISO only the 3 `disc_golden` tests may fail.

Suite: PARTIAL by the window (136/343 binaries run at the last poll 19:17 UTC; the runner was
still going). Of the binaries run: only fix_0c_pylon_live failed (2 tests, the no-GPU wgpu adapter
error — byte-identical failures in the previous worker's suite.log from 15:47, i.e. pre-existing
environmental). The touched area is fully green: mp_menu_flow 9/9 (inside the suite) and, run
directly on the same tree, mp_menu_lists_live 5/5, menus_flow 36/36, menu_movies 3/3,
menu_backdrop_still 1/1, dev_menu_flow 5/5, campaign_flow_live 5/5. mp_menu_lists_live and
menus_flow were still queued in the runner when the window closed; their direct runs above are
the same binaries the runner would execute.

## What's left (recorded for the next message)

- Drawing the 13 pages (the main agent; the getters are the surface — §6 of mp-menus.md).
- Rewards → MP unlocks (Menu_UnlockMPSettings/UnlockMPSkins map the campaign rewards; the port
  gates on `unlock_all` only).
- Pads / agents 2-4 joining; the codename keyboard page for non-default rows.
- Menu_StoreMPSettings/Restore persistence (the MP card block); the debriefing's rematch arm
  (needs the in-match host loop); Quick Game's map sample is deterministic row 0 headlessly.
- `--mp` hands minutes where the game's menu hands seconds (the confirm's ×0x3c) — the main agent
  may want parity.

## Live checks for the main agent

See `docs/systems/mp-menus.md` §"Live checks for the main agent" — six checks: the scenario/level
MPSettings stores (0x2A4944/0x2A4948), the ×0x3c on the confirm (0x2A4940), the Quick Game bot
count (0x2DEEA9 / 0x2A4950), GameState+0x08 on the confirm's ✕, the join-page state cells
(0x2def60+0x360, 0x2dee69), and the render check for the scenario wheel (5 rows, ring-masked
preview, the locked row's half alpha + "This scenario is locked.").

## ntfy

curl → HTTP 429 "limit reached: daily message quota reached"; python3 urllib → HTTP 429; wget →
exit 0 (accepted, or at least not rejected with an error the tool prints in -q mode). Not all
three failed, so per the rules this is not an "ntfy failed" record — the exact outputs are here
for the driver in case the message did not land.
