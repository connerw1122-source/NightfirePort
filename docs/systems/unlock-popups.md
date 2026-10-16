# The unlock pages after a mission's results (007 Bonus / win / cheat-medal)

unlock-popups (2026-10-15, branch `s/unlock-popups`). [M] = decompile read in this task
(`work/decomp/ACTION.ELF/<addr>_<name>.c`, line numbers refer to those files), [L] = a dump made
in this task, [S] = suggested. The port side is `crates/nf-view/src/menu.rs` (the `Page::NfBonus`
/ `WinGame` / `CheatMedal` variants, `PAGE_NF_BONUS` / `PAGE_WIN_GAME` / `PAGE_CHEAT_MEDAL`,
`BonusFields` / `RewardLine`, `FrontEnd::bonus` / `results_level` / `win_movie` / `win_movie_vol`
/ `cheat_medal_sel` / `medal_from_cheat`, the methods `bonus_enter` / `win_enter` /
`win_movie_finished` / `cheat_medal_confirm`) and `crates/nf-game/src/progress.rs`
(`REWARD_TEXT`). Tests: `crates/nf-view/tests/unlock_popups.rs` (12 headless) and
`unlock_popups_live.rs` (the ISO-gated menu-def pins). No drawing — the draw arm is an empty
match leg for the main agent (its data sources are listed in the arm's comment).

## 0. The page ids (the dispatcher)

`Handler_HandleMessage` 0x2029e0 binds [M :458-459, :463-466, :511-513, :569-572]:
0x40000036 → P_NFRESULTS 0x208e78, **0x40000038 → P_NFBONUS 0x209670**, **0x4000004c →
P_CHEATMEDAL 0x2086b0**, **0x40000053 → P_WINGAME 0x215100**. The pages these flow into:
0x4000001c = P_NFMAP (the mission list) [M :359-360], 0x4000002b = P_DOSSIER [M :405-407],
0x40000037 = P_NFSTATS [M :447-449], 0x40000030 = P_CREDITS [M :429-431].

## 1. Who routes into them (the results page's exit)

P_NFRESULTS 0x208e78 computes the medal at its 0x4c enter (the compare chain against
ScoringTable thresholds — progress.md owns that) and its LAST statement records the award:
`just_completed = Menu_SetLevelBonus(GameState+0x72, medal, 0)` [M :169]. On every exit path
(the 0x4b confirm arm :39-52 and the 0x50 save-box arm :204-217) the routing is [M]:

1. `DAT_002e1c2c` (a NEW reward unlocked this level end) → msg 0x44 to **0x40000038** (NFBONUS);
2. else `just_completed` → **0x40000053** (WINGAME) with arg 1;
3. else → **0x4000001c** (NFMAP) with arg 1.

`DAT_002e1c2c` is set inside `Menu_SetLevelBonus` 0x1ffa10 [M :52-81]: the reward loop
(`PlrStarts_ProcessRewardCounter` 0x199210 over tiers `medal..1`, the check call then the grant
call) sets it the first time a tier's reward bit was NOT already standing. `just_completed` is
the function's return: `level == 0x0700001b` [M :84] — the FINAL mission. `Mission_Update`
0x182c60 is what opens the results page: the pass path stores 0x40000036 in GameState+0x28 and
pushes the map load + fade [M case 4 :66-76]; the fail path opens P_ENDMISSION 0x40000042
instead [M :78].

The port: `results_enter` already ran the award (fix-progress); it now also stores
`results_level` (the +0x72 level the bonus page re-reads [M 0x209670:14]). `action()`'s
Results/Complete Confirm implements the three-way route above with the port's `new_reward` /
`just_completed` fields.

## 2. P_NFBONUS 0x40000038 — the "007 Bonus" page (what shows and from where)

Enter (msg 0x4c) [M 0x209670:13-43]:
- `iVar1 = Menu_GetLevelBonuses(GameState+0x72)` 0x1ffe98 — a 0x34-byte static record built per
  level [M :12-33]: find the level in `RewardsTable` (0x2bc920, stride 0x40), then per tier
  1..4: `rec+4·N` byte = `Menu_HasMedal(level, N, 0)` 0x202580 (the tier's bonus BIT in the
  `bonus` u64 array [M 0x202580 → 0x199210's has flag]), `rec`'s label = the entry's u32 at
  +0x18+4·N, and the entry's u32 at +0x2c+4·N. A level outside the table leaves the record
  zeroed [M :11-15]. The table layout is [L ee_start_05.bin 0x2bc920, dumped 2026-10-15]:
  +0x00 hash, +0x04..+0x16 five 4-byte descriptors {u16 bit, u16 kind}, +0x1c/+0x20/+0x24/+0x28
  the four tier labels, +0x30/+0x34/+0x38/+0x3c the four tier texture hashes (0x03xxxxxx). The
  labels are `Txt_BindLabel` ids (`bank<<24|index` [M 0x1cad60:13]) — most are bank 0 (0x2d2
  .. 0x3ab) and some bank 1 (0x010001f9/fb/fc/ff); the port ships them as
  `nf_game::progress::REWARD_TEXT` [L] indexed like REWARDS_TABLE.
- The page's TEXT is gated by `DAT_002e1c2c` [M :16-17]: with the flag down the handler sends
  no text at all (`uVar2 = 0`). With it up it composes [M :19-43]:
  - line 1 (tier 1's bit standing): `Txt_BindLabel(rec+8)` into the page buffer, and the
    texture `rec+0xc` goes to control 0x100001cf via msg 0x24 [M :22-26];
  - then tiers 4, 3, 2 IN THAT ORDER, each appending `Txt_BindLabel(...)` with the separator
    string at 0x34d048 between items [M :27-41];
  - the whole string goes to control 0x100001d0 via msg 0x18 [M :43].
- The layout is the menu-def's PAGE 0x40000038 [L the dump]: header (50,40,376×17) label
  **0x2c5**, hint (50,419,540×21) label **0x2d0**, texture control **0x100001cf**
  (89,112,171×256, its own default texture 0x03000034), text list **0x100001d0**
  (268,94,284×292), the ✕ button 0x10000165 (62,372,61×59), plus two badge sprites
  (0x0300002d top-right, 0x0300004c bottom-right); aux movie 0x07350048 like the other front-end
  pages.

Exit (msg 0x4b = the page's continue) [M 0x209670:45-57]: `just_completed` → 0x40000053 with
arg 1; else 0x4000001c with arg 1. (msg 0x5e → 0x40000037 NFSTATS; msg 0x5d → 0x4000002b
DOSSIER — the stats-page round trip.)

The port: `FrontEnd::bonus_enter(progress, level, has_text)` builds `BonusFields { level,
lines: [RewardLine; 4], texture: Option<u32>, text_labels: Vec<u32> (the 1→4→3→2 order),
has_text }`; the draw arm tells the main agent the control geometry. Tests pin the record
(`unlock_popups.rs::bonus_enter_*`) and the layout (`unlock_popups_live.rs`).

## 3. P_WINGAME 0x40000053 — the end-of-campaign page

The menu-def record is BARE: root control only, no aux movie, no labels [L the dump] — the page
is the handler's own movie. Enter (msg 0x4c) [M 0x215100:12-22]: sets the gate bytes
DAT_002e1c4a = DAT_002e1c49 = 1, DAT_002e1c4c = 0; `SFXRemove(0, 0x1d7)`, `SFXRemove(0, 0x470)`
(the two menu music streams); `menu_movie = 0`; `psiStartBackgroundMovie(0x073f0048, 0,
SFXGetVolume())`; msg 0x68 arg 1 to the manager at 0x2e3cd8.

Every frame (msg 0x50) [M :23-49]: when `psiMovieFinished() != 0` — on the FIRST finish it
restarts the two streams (`SFXStart(0x1d7/0x470, 0)`), sets `menu_movie = 1` and sends msg 0x68
arg 0; on the FIRST finish it then stops the background movie (`psiStopBackgroundMovie`) when
the DAT_002e1c49 gate stands, and sends msg 0x44 → **0x40000030** (the credits page) with arg 1
[M :48]. The credits roll then returns to the MAIN menu because `page_from` = 0x40000053 [M
0x2152b0's exit, credits.md documents the roll].

The port: `Page::WinGame`; `win_enter()` raises `win_movie` (the host plays 0x073f0048 and sets
`win_movie_vol` to the music volume); `win_movie_finished()` — the host's psiMovieFinished
report — lowers it and opens the credits through `open_credits(PAGE_WIN_GAME, vol,
just_completed)` [M :48's arg-1 send]. The audio restart (0x1d7/0x470) is the host's; the model
keeps the flow. Tests: `win_movie_end_opens_credits_from_the_win_page`.

## 4. P_CHEATMEDAL 0x4000004c — the cheat/medal grant page (an open question)

The dispatcher handles the page id [M :511-513] but **no PAGE record exists in the front-end
menu-def** [L the 56-page walk — every other 0x400000xx page is there] and **no static code
route opens it** [M the whole-decompile grep: no `Manager_SendMessage(0x44, 0x4000004c)`, no
`MenuManager_Create` with it, and no control of any page carries a data-driven 0x44 into it
(L the MGRMSG/KMSG scan in `unlock_popups_live.rs`)]. Reachability in the shipped game is
therefore unproven — the handler is ported for completeness.

What the handler does [M 0x2086b0:11-47]:
- msg 0x4c (enter): msg 0x39 then msg 0x17 to control **0x1000021c** — a control id with NO
  handler in the dispatcher [M the 0x2029e0 walk: the C_CHCH* family is 0x10000216/0x100000e7
  etc.; 0x1000021c falls through to the generic list control].
- msg 0x4b (confirm): reads the list's selection (`__Menu_Send(control, 0x35)`), DELETES the
  menu manager, then switches on the row [M :21-45]:
  - row 0: `switch_channels[99] = 1` and the level (GameState+0x52) parked at **0x27031c**;
  - rows 1..5: `medal_from_cheat` (**0x30cc74** [L symtab]) = 999/1/2/3/4, then
    `switch_channels[100] = 1` and the level parked at **0x270320**.
- The two channels are the mission-end triggers `Mission_Update` 0x182c60 case 1 consumes
  (99 = the fail path → P_ENDMISSION, 100 = the pass path → the results page) [M 0x182c60:23-33];
  `Mission_MonitorObjectives` 0x1827d8 sets the same pairs at a real mission end [M :70-71,
  :119-122]. The two 0x27031c/0x270320 slots are `switch_channels_time[99]/[100]` by address
  (0x270190 + 99·4/100·4 [M the symtab]) — the game parks the LEVEL HASH there, and no
  ACTION.ELF code reads them back [M the whole-decompile grep; DRIVING.ELF/SLUS have no
  reference either]. `medal_from_cheat` is likewise write-only in ACTION.ELF [M].

The port: `Page::CheatMedal` with a 6-row list (`rows()` = 6), Up/Down wrap the selection,
Confirm calls `cheat_medal_confirm()` which records `medal_from_cheat` (0 / 999 / 1 / 2 / 3 / 4
per the switch). The caller decides what a simulated mission end does — the port does not fire
the mission-end path from it.

## 5. What is NOT ported / open questions

- The draw pass for all three pages (the main agent's; NFBONUS's geometry + label ids are pinned
  in the live test and the draw-arm comment).
- `medal_from_cheat`'s consumer and the 0x27031c/0x270320 readers — write-only in ACTION.ELF [M];
  the CHEATMEDAL page's opener, likewise. Live checks for the main agent: on a PCSX2 run, watch
  `switch_channels[99]/[100]` (0x26fc90+0x18c/0x190) AND `switch_channels_time[99]/[100]`
  (0x27031c/0x270320) through a mission end — if any code READS 0x27031c/0x270320 during the
  results→bonus→map walk, gdbwatch will catch it; also watch `medal_from_cheat` 0x30cc74 while
  opening the Dossier's cheats screen after a medal run.
- The NFBONUS separator string at 0x34d048 and the format strings at 0x30d350/0x30d358 (the
  composed text's exact punctuation) — ELF .data bytes, not yet read; the port keeps the label
  ids and lets the draw pass bind them.
- The results 0x50 arm's save box (the option box with label 0x1000206) is the existing
  results-page save flow, untouched here.
