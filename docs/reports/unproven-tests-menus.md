# unproven-tests-menus — settle the front end's unproven assertions

Task 754 (branch `s/unproven-tests-menus`). For each of the 16 audit rows: the evidence found,
what changed (tagged / [S] / fixed). Decompile line numbers are the `cat -n` lines of
`work/decomp/ACTION.ELF/<addr>_<name>.c`; disc reads are tagged [D]; "write-only" findings come
from a full-.text scan for the global's address (lui+addiu+load/store pairs).

## The table

| Row (file:asserted behaviour) | Evidence found | What changed |
|---|---|---|
| menus_flow: page order "[L driver 2026-10-13]" not a fresh boot | The chain was already re-derived per step from the handlers; the residual driver-transcript citation removed | Comment only: "re-derived from the page handlers, not the driver's 2026-10-13 transcript" [M 0x216ca8:10-17] |
| menu_movies ~205: ESTHERO_TICKS = 1756x2 only compared with itself | [D] sister added: `the_esthero_budget_matches_the_disc_pictures` (cfg `movies`) counts 0x073B0048's pictures like the INTRO_MOVIES test; skips on the slim ISO's stubbed PSS like its siblings | New ISO-gated test + a not-a-constant sister (INTRO_TICKS != ESTHERO_TICKS) |
| menus_flow: confirm box triangle cancels / cross continues | Decoded [M]: triangle (action 0x22) plays sound 9 and sends msg 0x6d (Scroll_Update 0x221b20:454-458); the box's handler C_LBMSGOPTIONS 0x2022a0:12-17 stores option_return = 2 ("No") and pops on 0x6d; cross (0x1f->0x4b) keeps the selected row's value (:8-11); Menu_UpdateOptionBox 0x202280 only returns the stored value | Comment: [S driver transcript] -> [M addresses]; behaviour already matched |
| menus_flow/parity/e2e: Main Up/Down wraps, cursor 2 on return; 0x100000ed's control type | Main's rows are M_BUTTONs: def 08000002 page 40000002 binds ctrls 10000002/3/4 with kind byte 1 [D]; Menu_Update 0x1f1dc0 dispatches kind 1 to Button_Update — Scroll_Init's wrap flag never applies. 0x100000ed is kind 5 (a Label — only Label_SendMessage handles its msg 0x1a). P_MAIN 0x4c writes DAT_002a3cce=2, but that global is WRITE-ONLY: all 10 refs in the whole .text are stores of 2, no reader | Wrap assertions demoted to [S unverified] and taken out of pass/fail (walks select rows by cursor instead); "Back Main->title" stays [S] (P_MAIN has only 0x4c/0x50 arms [M 0x20a7b0]) |
| menus_flow/e2e: Back targets (Main->title, Dossier->MissionSelect, MissionSelect->Difficulty) | **MissionSelect->Difficulty is WRONG**: P_NFMAP's 99 arm is `Manager_SendMessage(0x44, 0x40000002, 1)` -> MAIN [M 0x20b608:67-68], run by the manager's pop before any stack pop [M 0x21c858:588, :592-600]. Dossier has no back arm [M 0x216de0: only 0x4c/0x50] -> the default stack pop returns to the mission list. Main->title: no 99 arm in P_MAIN -> port's [S] | **TEST FIX**: MissionSelect Back now asserts Page::Main in menus_flow, menu_sp_e2e (each_difficulty + back_walks), menus_mouse_parity (ride-along), menu_audio_flows (round trip); port `menu.rs` Back arm changed Difficulty->Main (cursor 0, [S]) |
| menus_flow/parity/e2e: difficulty +1 offset, 3 tiers on a fresh profile | [D] `difficulty` = .data 0x2df4c0, 72 bytes = 3x0x18 records; +0xc holds **1/2/3** (the port's "+1"), +0x10 = 1 on all three -> the confirm gate [M 0x216ca8:12-15] never refuses a fresh profile | Comments updated to [D 0x2df4c0] + [M 0x216ca8]; the 3-tier walk test restructured around Back->Main, still walks all tiers |
| menus_e2e_mp / mp_menu_flow: Scenario Back -> Main on cursor 1 | [M 0x217348:28-30] the 99 arm opens 0x40000002 unconditionally (no cursor read in the handler); cursor 1 has no evidence — P_MAIN's landing row is unreadable (DAT_002a3cce write-only) | Comment split: Back->Main [M], cursor 1 [S the port's choice] |
| menus_e2e_mp ~244-303: left/right wraps at list ends | [M] Scroll_Update 0x221b20:186-210 wraps BOTH ends while +0x144 stands; the flag defaults on (Scroll_Init 0x22214f0:102-103), only msg 0x69 clears it (0x221810:136-138), and nothing sends 0x69 to the MP pages | **PORT FIX**: `mp_rules_step` clamped — now wraps (rem_euclid); new wrap assertions: Duration 20->...->Unlimited->1->Unlimited, Points 20->...->120->1 |
| menus_e2e_mp / mp_menu_flow: Quick Game always Skyrail | [M C_SBMPSCEN 0x20c600:62-63] the game samples `Rand_Random() % 7` over mp_level rows 0..6 (Ravine, row 7, excluded); the port's row-0 pick is a determinism stand-in | Comments strengthened with the [M] cite; the [S] stand-in label kept |
| mp_debriefing ~109: 0x5d Extra vs Side; action 0x20 | [M] action 0x20 = Square: Button_Update 0x1ef948:80-87,123-127 plays sound 1 and sends msg 0x5d; triangle is action 0x22 -> the manager's pop (0x221b20:454-458). C_MPDBG 0x217860:12-16: 0x5d re-requests the arena (ResetMap_LevelToLoad(MPSettings+0x1A8) + the 80-frame fade + delete) | Comment relabelled (was "~Y arm"); docs/UNKNOWNS U-011 updated (0x20=Square, 0x21=Circle->0x5e, 0x22=Triangle) |
| front_end_chain_live ~171: Results Confirm -> 0x4000001c unconditional | [M 0x208e78:29-52] three gates: the DoneBetter save box (DoneBetter!=0 && DAT_002e1ae0>1) swallows the press; then DAT_002e1c2c==0 AND just_completed==0 for the list; reward->0x40000038, final mission->0x40000053. `just_completed` <=> level 0x0700001b [M 0x1ffa10 tail] | Doc rewritten with the gate chain; the test pins `!results_box && !results_improved && !new_reward && !just_completed` before the press; branch sisters already live in unlock_popups.rs (flag-poked) |
| unlock_popups ~92-122: cheat-medal value and wrap | [M 0x2086b0:19-45] the press reads the row's 0x35 VALUE and switches: 0 = fail-channel poke, 1..5 grant 999/1/2/3/4. **The disc carries no row values**: mission def 08000001's page 0x4000004c binds control 0x1000021c with NO msg-0x11 rows [D — walked from the ISO] | Comments: handler switch [M]; value==index mapping and the wrap marked [S unverified] |
| menu_sp_e2e ~536-548: fail REPLAY = warm restart | [D] the mission def's page 0x40000042 rows: REPLAY value 0 (0x5bc/0), QUIT value 2 (0x16c/2), NO value-1 row -> P_ENDMISSION's BaseMap arm (value 1 [M 0x216618:14-17]) is unreachable; value 0 = the warm checkpoint restart [M :19-21] | Comment updated to the disc values + the dead arm |
| pause_results_flows ~122-130: Esc closes from any tab | [M 0x21d508:27-38] the close is manager-level: close action (+0x1c8 = Create param_5; pause 0x1e [M 0x193600:73-75], fail menu 0x28 [M 0x182c60:78] — excluded) + pressed; manager+0x7b==7 is the CREATE-time sentinel [M 0x21b908:131, the only writer] — vacuous, not a per-tab gate | Comment updated (the old cite was fine; the +0x7b claim settled as vacuous) |
| pause_results_flows ~217-220: after NO, one Confirm re-arms | [M 0x204408:165-171] state_ignore_167 swallows exactly ONE 0x4b in the d1=6 arm, then the rows answer again — the observable rule matches the test | Comment added with the cite |
| pause_results_flows ~508: LOADING_HOLD 5 s | [M 0x1d6360:121-171] no timer/frame minimum anywhere in the load state; the tail pops at once (:166-170) | Already labelled [S port choice]; cite strengthened |
| menu_audio_flows ~269-283 + lib.rs 1 s skip gate | [M 0x1c98c8:145-151] the game's state 6 skips at once (movie finished OR action 0x19 pops the same frame) | Port's 1 s gate kept, tagged [S port choice] with the game-cite in both lib.rs and the harness |

## Side findings (reported, not fixed here)

- The MP scenario/map full-page wheels clamp in the port while the game's vertical lists ride the
  wrapped helper [M 0x221b20] — behaviour parity left to a follow-up (no test pinned it).
- Port sfx: the Side action's sound id may need a second look against menu-sfx.md's
  Square/Circle mapping (found while reading Button_Update; no test pinned it here).
- The CHEATMEDAL page exists in the mission archives' kind-8 def 08000001 (9 pages incl.
  0x4000004c) — the docs' "NOT in the menu-def" walked only the front-end def 08000002.
- state_ignore_167's delivery order: the decompile alone can't prove whether the swallow eats the
  NO press itself (same-frame re-delivery) or a fresh player press; the port models the observable
  rule (the next Confirm re-arms). A PCSX2 press-count check would settle it — see Live checks.

## Live checks for the main agent

1. Mission list Back: on a fresh boot, walk START->NightFire->(box)->Difficulty->Confirm->MissionSelect,
   press Triangle — expected: the MAIN menu opens (not the difficulty wheel) [M 0x20b608:67-68].
2. Main menu wrap: on Main (3 rows), press Up from the top row — does the highlight wrap to the
   bottom row, and which rows are listed (order) [S today].
3. Main landing cursor after Back: press Triangle on the mission list, note the highlighted Main
   row [S today; the decompile cannot show it].
4. Pause RESTART double-press: arm the RESTART confirm, answer NO, then press Cross once — does
   that single press re-arm the RESTART confirm (expected under the same-frame reading) or is it
   dead with the SECOND press re-arming [M 0x204408:165-171 is ambiguous on delivery timing]?
5. Cheat-medal page: unreachable in the shipped code (no opener found); skip unless a debug route
   exists.

## Verification

- `cargo test --offline --release -j2 -p nf-view -- --test-threads=1` on the final tree:
  196 passed, 2 failed, 11 ignored across 64 binaries. The 2 failures (`fix_0c_pylon_live`)
  panic at `request_adapter` — this sandbox has no Vulkan/GL device (environment, not code).
- The fast suite (tools/test/suite.py) was started twice in the background but the sandbox reaps
  background jobs between tool calls; the foreground nf-view run above stands in for the final
  tree (nf-view is the only crate the task touches).
- What ran this session: nf-view full sweep (above), plus the targeted touched-test binaries
  (menus_flow, menus_mouse_parity, menu_sp_e2e, menus_e2e_mp, mp_menu_flow, mp_debriefing_flow,
  front_end_chain_live, unlock_popups, menu_movies, menu_audio_flows, menu_graph_walk, menus_e2e,
  menu_music_flow, memcard_save_flow, mission_unlock_flow, save_progress_roundtrip,
  difficulty_wiring_live, gameflow_stack_sp_run) — all green before the final sweep.

## Suite total

The fast runner produced no total line this session (background reaping; see Verification).

## What's left

- The Main-menu button focus walk (wrap + landing cursor) needs either a decompile pass on the
  manager's focus code or the live checks above.
- The MP wheel Up/Down clamp-vs-wrap parity item.
- state_ignore_167's delivery-timing question (live check 4).

## Usage

Context window: not visible to me (no token/cost figures are exposed for the driver session; the
research subagents' transcripts showed per-agent prompt/completion counts of roughly 105k-212k
prompt tokens each, but my own session counters are not visible to me).
