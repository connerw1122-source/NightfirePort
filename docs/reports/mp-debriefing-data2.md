# mp-debriefing-data2 — 621's debriefing landed on top of master's 402 match rules

Branch `s/mp-debriefing-data2`, 2026-10-16. Source: 621 (`s/mp-debriefing-data`, tip 20d5bde1,
HELD for double-porting `MP_Update` 0x184bd0 / `MP_CheckForEndCondition` 0x184268) re-landed on
master's 402 (`s/mp-match-rules`) single state machine. Final tree = branch + origin/master
merged (master's driving batch + the online-MP chain; merge commit b8059cd7).

## What's done

- **The single 402 machine kept** (`crates/nf-game/src/mp/mod.rs`, `rules.rs`): `update(dt,
  paused, rng)` signature kept (the state-6 restart's site re-pick draws the shared RNG, the
  game's own call site [M 0x184bd0:87]); `MpGame::high` kept as the only +0x18C (621's
  `top_score` was the same word — not added twice).
- **621's additions ported on top**: `MpGame::handover_flag` (GameState+0x24 = 1 at state 3→4
  [M 0x184bd0:59; disasm 0x184e6c]), `request_debriefing` (case 4's `GameState+0x1C =
  0x40000033` navigation [M :62-78]), `restart_pending` (case 6's host observable),
  `end_status: MpEndStatus` (`None`/`Won(codename)`/`Draw` — 621's unused `Pending` placeholder
  dropped) set by `sort_out_who_won`, which keeps returning 402's `Winner`.
- **The debriefing board**: `crates/nf-game/src/mp/debrief.rs` — `get_mp_score` (`Menu_GetMPScore`
  0x203af8's table), `debriefing()` (`P_MPDEBRIEFING_Handler` 0x212198 msg 0x4c: the (slot,
  score) pairs humans-then-bots, the stable descending bubble sort, the rank walk with ties, the
  cap at 4, codename/bot-short-name cells, the columns, the result line including the
  float-compared team announce), `bot_short_name_label` (0x201918).
- **`MpPlayer::codename`** (+0x00; the debriefing name column and the winner line read it
  [M 0x212198:169-171]); the confirm page hands the joined agents' codenames through.
- **The Top Agent elimination input reconciled with 402's `dead`**: the game counts slots by the
  `obj+0xff` state byte once deaths reach the lives limit [M 0x184268:86-119]; the port counts
  slots its death path marked (`pos.is_some() && dead && deaths >= limit`) — 621's caller-marked
  `out` field dropped (one stand-in, not two). The 0x02 alive-exemption and the one-per-call
  0x02→0x11 flip only ever lift that exemption — unobservable in the port, documented as
  *Deviation* (mp-debriefing.md §1).
- **`Action::Extra`** (the pad's ~Y/Triangle; PC stand-in Q [S]): arms on every page (idle
  except the debriefing's), plus the C_MPDBG 0x5d replay arm on `Page::MpDebriefing`
  (`ResetMap_LevelToLoad(MPSettings+0x1A8)` + the fade push — the same match straight back on;
  the port keeps `mp_match_settings` for the host). Confirm = C_MPDBG 0x4b (back to the scenario
  page). Master's post-merge new pages (Rewards/Gadgets/Weapons) got their idle Extra arms too.
- **Front-end getters for the draw code**: `FrontEnd::mp_debriefing` + `open_mp_debriefing(&MpMatch)`
  (the host's case-4 handover, via master's `enter_page`), `mp_debriefing_rows()` /
  `mp_debriefing_result()` / `mp_debriefing_portrait(character)`, `MP_CHARACTERS_SMALL` (the 29
  portrait rows [L 0x2df8f8]), the page's label/widget consts.
- **Systems doc**: `docs/systems/mp-debriefing.md` (§0 the reconciliation table, §1-4 the tagged
  game facts, §5 the port, §6 the live checks).

## The reconciliation table (disagreements settled from the decomp)

Both branches ported the end machine; the decomp (`work/decomp/ACTION.ELF/`) decides — full
table in `docs/systems/mp-debriefing.md` §0. Summary:

| # | Fact | 402 had | 621 had | The decomp says | Verdict |
|---|---|---|---|---|---|
| 1 | Demo/prot time-up | latched `time_up = true` | state 6, no 0xfe write | state 6 inside the `0.0 < limit && 0xfe clear` gate; 0xfe never written there (0x184268:57-59) | 621 right |
| 2 | Demo/prot tail | early `return` | frag latch in the else branch | the frag latch (lines 170-172) + final dispatch (173-184) run for demo/prot AND the general branch; only 0x10 jumps past (line 168) | both wrong; merged shares the tail |
| 3 | General 0xfe latch gate | `limit <= clock` | `limit > 0.0 && …` | `0xfe = limit <= clock`, no limit>0 test (lines 142-143) | 402 right |
| 4 | State-6 restart gate | `>= 5.0` | `> 5.0` | `5.0 < timer` strictly (0x184bd0:85; state 3's debrief gate is the inclusive `5.0 <=`) | 621 right |
| 5 | 0x10 tie quirk | a `tie` flag cleared only on a new min | a count that re-increments after a tie-zero | count++ per qualifier BEFORE the compare; tie zeroes it; draw only when the count is 0 at the END (0x1848d0:32-49) — [3,3,1] crowns the 1 | 621 right; 402's flag drew |
| 6 | FFA qualifier gate | `pos.is_some() && …` | score only | the FFA loop reads only the slot scores, no obj test (0x1848d0:56-71) | 621 right |
| 7 | Elimination input | derived from deaths | caller-marked `out` + deaths | state-byte split of the same deaths condition | merged on 402's `dead` (row above) |
| 8 | +0x18C | `high` | `top_score` | the same word | one field (`high`) |
| 9 | `player_killed` | full port (drops/assassin/rng) | simpler | 0x188f70:223-245/276-303 exist | 402's superset kept |
| 10 | `update` signature | `(dt, paused, rng)` | `(dt, paused)` | `MP_RestartScenario` runs inside MP_Update | 402's kept; `restart_pending` added |

Suicide scoring was NOT in conflict (both: kills −1, streak 0, delta −1.0, `last_killer` 0xffff
[M 0x188f70:150-169]) — the merged tests pin it.

## How it was verified

- **Tests** (all headless, numbers/IDs only; each with a fail-able sister): adapted from 621 —
  `crates/nf-game/tests/mp_debriefing.rs` (10 tests: the kill/score/team numbers, the frag-limit
  end, the time-up machine walk to the debriefing request, the demo/prot restart with the
  0xfe-not-latched assertion, the 0x10 elimination + the tie-quirk cases [3,3,1]/[3,3,10], the
  board sort/ranks/cap/names/columns, the get_mp_score table, the single-player win, and the new
  no-obj-gate FFA case) and `crates/nf-view/tests/mp_debriefing_flow.rs` (6 tests: the handover
  rows/ranks/portraits, the result line, Confirm/Extra arms, the static labels + two NF_ISO
  disc-byte tests). 402's `mp_match_rules.rs` (14) passes **unchanged** on the merged machine.
- **The full suite** (tools/test/suite.py, release, `--test-threads=1`; this sandbox reaps
  background processes between tool calls, so it ran as foreground slices over the 364 test
  binaries; the log/timeout budget noted below):
  **1565 passed, 7 failed, 66 ignored.** The 7: the 3 `disc_golden` tests (allowed on the slim
  ISO), 3 no-GPU wgpu-adapter failures (`probe_pylon_render` 1, `fix_0c_pylon_live` 2 — the
  sandbox has no GPU; the panic is the adapter request at test start), and 1 TIMEOUT
  (`walkreal_07000001_live` at a 480 s cap — the pre-existing minutes-long full-level walk,
  noted in earlier WORKLOGs).
- **After merging origin/master** (32647326): the tree re-verified on the crates the merge
  touches — nf-game FULL: **452 passed, 1 failed, 2 ignored** (the 1 is
  `driving_ai_follow::real_mis01_track_rngp_walks…`, pre-existing on master's tip: my merge left
  `crates/nf-game/src/driving/` byte-identical to origin/master — `git diff origin/master HEAD`
  over those files is empty — and the test reads this sandbox's `DRIVING/` data); nf-view's
  menu-flow set (mp_debriefing_flow, menus_flow, menu_sfx_flow, fix_scores_handoff_live,
  codename_entry, dev_menu_flow, mp_core_live, async_load_frame_loop, menu_backdrop_still):
  **85 passed, 0 failed**; the MP slice re-run: **87 passed, 0 failed**. `cargo check` clean for
  nf-game + nf-view.
- Determinism: no new RNG draws (the only `rng` use stays the state-6 restart's site re-pick,
  the game's own); no wall-clock time added.

## What's left / reported

- **The debriefing page's draw** is the main agent's (the task ships the data + getters; the
  widget table with positions/sizes is in mp-debriefing.md §4, `Page::MpDebriefing` renders
  nothing yet). Portrait textures come from `MP_CHARACTERS_SMALL`; the plate/rows/hint-bar
  layout is fully specified there.
- 621's assassin +5/+3 arms and the carried-object drops were already in 402's
  `player_killed` (kept); nothing from 621 was dropped except its unused `MpEndStatus::Pending`
  placeholder and its redundant `out` flag (both documented above).
- The suite ran as slices, not one invocation (the sandbox reaps background jobs); every one of
  the 364 binaries ran, `weapon_matrix_live` included (6/0/24).

## Live checks for the main agent (PCSX2; none runnable here)

1. Watch `GameState+0x1C` @0x2a3784 (w) when a match ends naturally (frag limit in Arena) —
   expect 0x40000033 (disasm 0x184e80); then `GameState+0x24` @0x2a378c — expect 1.
2. Team-announce check: play Team Arena, make team 1 (the second-picked side) win — the result
   line should read "MI6 wins the match." when team0 < team1 (0x1c8). If PCSX2 shows "Phoenix"
   there instead, the [S] inversion note in mp-debriefing.md §4 is wrong and the port flips the
   two label ids.
3. `MPGame+0x188` @0x2a4b08 (w) at the end: 1 on a frag-limit end, 2 on time-up (Arena), 6 on
   Demolition/Protection time-up, then 3 → 4 → 5 across ~5 s.
4. Debriefing rows with a known 3-bot Arena game: the rank captions "1st"/"2nd"/"3rd", the bot
   short names, Points = (int)score, Victories = kills, Deaths, TOTAL = kills.
5. The ~Y replay: on the debriefing, Triangle should reload the arena just played without
   passing the summary page (C_MPDBG 0x5d).
