# The MP match end and the debriefing

mp-debriefing-data2 (2026-10-16, branch `s/mp-debriefing-data2`). This lands 621's
(`s/mp-debriefing-data`, HELD) match-end + debriefing port on top of master's 402
(`s/mp-match-rules`) single state machine. Everything here is first-hand: [M] = decompile/
disassembly read in this task or re-read from 621's task (`work/decomp/ACTION.ELF/<addr>_<name>.c`,
line numbers refer to those files; disassembly addresses are exact), [L] = a byte dump made in
621's task (from `work/elf/ACTION.ELF`, byte-identical to the ISO's /ACTION.ELF; .data file
offset = vaddr − 0x106100), [S] = suggested/inferred. The port side: `crates/nf-game/src/mp/mod.rs`
(the end-condition/state-machine completion, 402's file), `crates/nf-game/src/mp/rules.rs`
(`MP_SortOutWhoWon`'s body, 402's file) and `crates/nf-game/src/mp/debrief.rs` (the debriefing
data builder, 621's file), exposed through `crates/nf-view/src/menu.rs` (`FrontEnd::
open_mp_debriefing` + the `mp_debriefing_*` getters), tests `crates/nf-game/tests/
mp_debriefing.rs` and `crates/nf-view/tests/mp_debriefing_flow.rs`; 402's
`crates/nf-game/tests/mp_match_rules.rs` passes unchanged on the merged machine.

**Offset convention**: the decompiler prints struct offsets in DECIMAL. Bases: MPSettings
0x2a47a0, MPGame 0x2a4980, GameState 0x2a3768 [M symtab + the addiu immediates 0x47a0/0x4980/0x3768
in 0x212934, 0x184e54/0x184e7c, 0x184bec].

## 0. The 621 → 402 reconciliation (what each side got wrong)

Both branches ported `MP_Update` 0x184bd0 / `MP_CheckForEndCondition` 0x184268; where they
disagreed, the decomp decides:

| # | Fact | 402 (master) had | 621 had | The decomp says | Fix |
|---|---|---|---|---|---|
| 1 | Demo/prot time-up: `switch_channels[0xfe]` | latched `time_up = true` | not latched | state 6 is written inside the `0.0 < limit && 0xfe clear` gate; **0xfe is never written there** (decomp 0x184268 lines 57-59) | 621 right; 402 wrong |
| 2 | Demo/prot tail after the state-6 arm | early `return` (no frag latch, no final dispatch) | frag latch inside the else branch (demo/prot skipped it too) | the frag latch (lines 170-172) and the final dispatch (LAB_00184884, lines 173-184) run for **demo/prot AND the general branch**; only 0x10 jumps past the latch (line 168 `goto`) | both wrong; the merged machine shares the tail |
| 3 | The general 0xfe latch's gate | `limit <= clock` | `limit > 0.0 && limit <= clock` | `switch_channels[0xfe] = limit <= clock` with **no limit>0 test** (lines 142-143; the `0.0 < limit` at line 57 is the demo/prot arm's own gate) | 402 right; 621 wrong |
| 4 | State-6 restart gate | `restart_timer >= 5.0` | `> 5.0` | `if (5.0 < (float)MPGame._408_4_)` — STRICTLY greater (decomp 0x184bd0 line 85; the state-3 debrief gate at line 56 is the inclusive `5.0 <=`) | 621 right; 402 wrong |
| 5 | 0x10 fewest-deaths tie | a `tie` flag set on `d == min`, cleared only on a new min | a count that increments per qualifier BEFORE the compare and zeroes on a tie | `iVar10` (count) increments per qualifier first (line 32), a tie at the running min zeroes it (line 39), the draw fires only when the count is 0 at the END (line 49) — a later qualifier re-increments, so **[3,3,1] crowns the 1** (402's flag drew) | 621 right; 402 wrong |
| 6 | FFA qualifier gate | `pos.is_some() && limit <= score` | score only | the FFA loop (lines 56-71) reads only the slot scores — **no `obj != 0` test** in that branch | 621 right; 402 wrong |
| 7 | The 0x10 elimination input | derived: `deaths >= limit` among obj-live slots | a caller-marked `out` flag AND `deaths >= lives` | the game counts slots by the `obj+0xff` state byte (0x12 humans / 0x11, 0x02-with-the-death-check bots) once deaths reach the lives limit (lines 86-119); the port has no obj pipeline | merged: 402's derivation, tied to the death path via 402's `dead` flag (`pos.is_some() && dead && deaths >= limit` — equivalent to 402's counting, since any slot with deaths ≥ 1 is `dead`); 621's `out` field dropped (one stand-in, not two). The 0x02 alive-exemption and the one-per-call 0x02→0x11 flip (lines 124-135) only lift that exemption — unobservable in the port, not modeled (*Deviation*) |
| 8 | The +0x18C top score | `MpGame::high` | `MpGame::top_score` | the same word (Ghidra `_396_4_`) | kept 402's single `high` |
| 9 | `player_killed` | full port (explosive drop preload, assassin block, object drops) + `rng` | simpler port, no rng | the assassin/drop blocks are 0x188f70:223-245/276-303 | kept 402's superset; 621's tests adapted |
| 10 | `update` signature | `update(dt, paused, rng)` (the state-6 restart's site re-pick draws) | `update(dt, paused)` (no rng; the restart deferred to the host) | `MP_RestartScenario` is called INSIDE `MP_Update` case 6 (decomp line 87) | kept 402's signature and in-place restart; added 621's `restart_pending` flag as the host's observable |

Not in conflict: 402's extra fields (`status_bits`, the respawn rules, the mode objects) and 621's
end machine (states 2-6's bodies) slot together without overlap.

## 1. What ends a match — `MP_CheckForEndCondition` 0x184268

Called from `MP_Update` 0x184bd0 state 0 (playing) each tick [M 0x184bd0:37-41]. The clock is the
game's wall clock (`psiGetTimeIn100ths` × 0.01 s, stamped at `MPGame+0x1A4` — the port keeps the
fixed-60 Hz deviation, see the mp/mod.rs module doc); the elapsed word `MPGame+0x19C` always
accumulates when unpaused, the round clock `MPGame+0x190` too [M 0x184268:44-46,55,140]. A dev
switch `switch_MP4EVER != 0` returns early [M :47-49] (not ported — no cheat path).

Dispatch on the scenario word `MPSettings+0x1A4` [M :50-169]:

- **0x20000040 Demolition / 0x20000080 Protection** — inside the gate `0.0 < limit &&
  switch_channels[0xfe] clear`, `limit <= clock` writes **state 6 directly and never touches
  0xfe** [M :57-60; decomp lines 57-59]. 0xfe stays clear, so the final dispatch below cannot
  override state 6 unless the frag latch fires.
- **0x10 Top Agent** — the elimination count: for every slot whose obj pointer
  (`MPGame+s·0x30+0x1C`) is live, a slot is *out* once its deaths (`MPGame+s·0x30+0x08`) reach
  the lives limit (`MPSettings+0x19C`); the game splits the outs by the player-state byte
  (`obj+0xff`: 0x12 vs 0x11, and 0x02 unless it shows the fully-alive signature —
  `(obj+0xe0)+0x4f8 & 0x600 == 0 && & 0x100 != 0 && hp > 0 && obj+0xfe bit0 clear`) into two
  counters [M :86-119] — both feed the same sum. When `totalPlayers−1 <= outs` or all humans are
  out, the state goes to **1** [M :120-123]. If all humans are out while bots remain
  (`botOuts < bots−1`), the first state-0x02 bot is force-flipped to 0x11, one per call
  [M :124-135]. The port carries no state bytes: it counts slots its death path marked (`dead`)
  whose deaths reached the limit — the exemption and the flip only ever lift that exemption, so
  both are unobservable in the port (*Deviation*; the flip's sit-out effect is the bot
  pipeline's concern).
- **Every other scenario** — the time-up latch: when `MPSettings+0x1A0 != -1` and `0xfe` is
  clear, `0xfe = (limit <= clock)` [M :142-143] (state is NOT set here).

The top-score scratch `MPGame+0x18C`: team games max the two team scores (`MPGame+0x180/+0x184`
as ints); FFA (and demo/prot) then max every slot's score (`MPGame+s·0x30+0x18` as ints)
[M :66-79,149-167]. The frag latch (decomp lines 170-172, **shared by demo/prot and the general
branch**): when `MPSettings+0x19C != -1`, `switch_channels[0xfd] = fragLimit <= topScore` —
skipped entirely for 0x10 (its +0x19C is the lives limit; line 168 jumps past).

The final dispatch (LAB_00184884, decomp lines 173-184): `0xfd` set → state **1** (a score/frag
end); else `0xfe` set → state **2** (time up); else the state stays whatever the scenario arms
left it (a demo/prot state 6 or a 0x10 elimination state 1 both survive).

## 2. The end state machine — `MP_Update` 0x184bd0

The switch on `MPGame+0x188` [M 0x184bd0:36-93; offsets confirmed by disasm — `lw $a0,0x188($v0)`
@0x184cac, `sw $v0,0x188($a0)` @0x184e64/0x184e88, `swc1 $f0,0x1a0($v0)` @0x184e50]:

- **0** playing: `MP_Pickup_Process` + `MP_CheckForEndCondition` + `BOT_update` (the first and
  third are phase 2+ in the port).
- **2** (time up): scenario 0x10 falls through to case 1; anything else binds the status text
  "Game Over :" 0x2000024 + "Press START" 0x1c9 into `StatusSpr` and sets state **3** [M :42-50].
- **1** (score end): `MP_SortOutWhoWon` 0x1848d0, which sets state **3** and writes the status
  text [M :51-53; 0x1848d0:20]:
  - 0x10: the winner is the fewest deaths (`+0x08`) among slots with an obj, counting only
    deaths < the lives limit. The survivor count increments BEFORE the min-compare and a tie at
    the running minimum zeroes it — so the verdict is "A Draw" 0x2000023 only when the LAST
    qualifier tied the minimum (nothing followed); otherwise "Player %s Won" 0x2000022 with the
    codename at `MPSettings+slot·0x30` [M 0x1848d0:27-53].
  - FFA (`MPSettings+0x18C == 0`): every slot whose score (`+0x18` f32) ≥ the frag limit
    (`MPSettings+0x19C` as float) qualifies — **no obj gate in this branch** [M :55-71]; exactly
    one qualifier → "Game Over : %s Won" with the codename (0x2000024 + 0x2000027), otherwise
    (0 or ≥2) → "Game Over : A Draw" [M :72-85].
  - Team games: always "Game Over : A Draw" — the team winner is announced on the debriefing
    [M :87-92].
- **3**: scenario 0x10 waits for the debrief timer `MPGame+0x1A0` to reach 5.0 s (the compare is
  the inclusive `5.0 <=`, decomp line 56); then state **4** and `GameState+0x24 = 1` [M :54-61;
  disasm 0x184e6c `sw $v1,0x24($a1)`] — the handover flag.
- **4** — the navigation to the debriefing [M :62-78; disasm 0x184e70-0x184e94]:
  `GameState+0x1C = 0x40000033` (the debriefing page id), state **5**,
  `ResetMap_LevelToLoad(0x7000048, 0, 0)` (the front-end map),
  `GameFlow_PushState(80.0f, 7, 0xff)` (the fade), then for slots 0-3 with a live obj:
  `Player_SetCamMode(obj+0xe0, 1)` + `GS_PausePlayer(1, slot)` (the port leaves the player-side
  calls to the sim; it exposes `request_debriefing`).
- **6** (demo/prot restart): `MPGame+0x198` (restart timer) += per frame; at ≥ 2.5 s the status
  text binds 0x2000049; at **> 5.0 s** (strictly, decomp line 85) state **0** +
  `MP_RestartScenario` + the status text cleared [M :79-92]. The port restarts in place (the
  site re-pick draws the shared RNG) and flags `restart_pending`.

`MP_Update`'s whole body is gated on `GameState+0x0C != 0x7000048` (skip on the front-end map)
[M 0x184bd0:13; disasm 0x184bfc], the MP master enable (`MPSettings+0x384`), GameFlow state 2 and
not-paused; while paused only the wall stamp refreshes [M :14-23,95-98].

The quit path (already documented in mp-menus.md §1): `C_GCPAUSE` 0x204408:146 writes
`GameState+0x1C = 0x40000033` after `ResetMap_LevelToLoad(0x7000048,…)` when
`MPSettings+0x180 != 0` — the same +0x1C word.

## 3. The kill/score bookkeeping — `MP_PlayerKilled` 0x188f70

402's port owns this (`crates/nf-game/src/mp/rules.rs::player_killed`); see
`docs/systems/mp-match-rules.md` for the tagged rules. The debriefing-relevant parts: deaths
`+0x08` += 1 every scenario [M :92-94]; 0x10 also takes the score −1.0 [M :96-99]; the killer's
kills `+0x04` +1 and streak `+0x10` +1 unless a team-game friendly [M :150-165]; suicide: kills
−1, streak 0, `last_killer` 0xffff, score delta −1.0 [M :150-169]; the personal delta lands only
when `MPSettings+0x190 == 0` and the scenario ∈ {0x1, 0x20000002, 0x40000800, 0x60001000}
[M :179-184]; the +2.0 golden-gun arm [M :170-177]; the team scores move only in Team Arena
[M :247-274].

`Menu_GetMPScore` 0x203af8 — the debriefing's TOTAL column and the ranking key [M 0x203af8]:
kills (`MPGame+s·0x30+0x04`) for Arena 0x1 and Team Arena 0x20000002; the slot score (`+0x18`
as int) for 0x10, 0x400, 0x20000004, 0x20000040, 0x20000080, 0x20000100, 0x20000200,
0x40000800, 0x60000008, 0x60001000; **0 for anything else** (including the Quick Game word 0).

## 4. The debriefing page 0x40000033

The front-end menu-def's page (kind-8 entry 08000002, PAGE tag at stream 0xffd9; aux movie
0x07350048, kind 0) [L 621's dump — 43 controls]. Static texts [L the menu-def + USATxt.dat via
the game's fixup algorithm (`index = fixups[id>>24] + (id&0xffffff)`, `Txt_BindLabel` 0x1cad60)]:

| widget | pos | size | static label | text |
|---|---|---|---|---|
| header | 50,40 | 376×17 | 0x151 | "Debriefing" |
| hint bar | 50,419 | 540×21 | 0x2c1 | "~A Continue  ~Y Replay" |
| column titles x=52 | y 183/217/250/284/318 | 120×16 | 0x14e / 0x24c / 0x686 / 0x2be / 0x163 | "Agent" / "Points" / "Victories" / "Deaths" / "TOTAL" |
| result plate | 176,355 | 408×52 | — | box |
| result line 0x1000023d | 178,369 | 405×24 | — | runtime text |
| NightFire logo | 434,38 | 156×43 | tex 0x0300016d | — |

Per-slot widgets (slot index 0-3 in the control's d1 field, x = 176/280/384/488 for the
panels; the portrait 81×81 at y=102, the name 100×16 at y=183, the values 78×16 at
y 217/250/284/318): panel 0x1000015d, portrait 0x10000157 (placeholder texes
0x0300015d/0x0300015c/0x03000156/0x0300016b), rank caption 0x10000156 (72×17 over the
portrait), name 0x10000158, points 0x10000159, victories 0x10000240, deaths 0x1000015a,
TOTAL 0x1000015c. `0x1000015b` is show/hide'd by the handler but **does not exist in the
menu-def** (checked all 56 pages) — a dead send [L+M 0x212198:157,214].

### The populate — `P_MPDEBRIEFING_Handler` 0x212198, msg 0x4c

[M 0x212198:83-250; the pair layout and the winner sprintf pinned in disasm 0x2121d0-0x212a94;
re-read in this task]:

1. If `menu_movie == 0`: remove SFX 0x1d7/0x470, start them, set the flag (the page's music
   stinger) [M :85-91].
2. Build (slot, score) pairs: slots 0..humans (`MPSettings+0x1AC`) then 4..4+bots
   (`MPSettings+0x1B0`), score = `Menu_GetMPScore(slot)` [M decomp :93-111].
3. Bubble-sort descending by score (stable — equal scores keep their order) over
   `MPSettings+0x194` (humans+bots, written by MP_Start 0x183a50:164) entries [M :112-130].
4. Rows 0..`MPSettings+0x194` (the rank walk): the rank index advances only when the score
   differs from the previous row's — tied TOTALs share a rank [M :131-143]. Rank labels
   [L 0x309520 + disasm 0x2121d0-0x2121f0 copying the 16 bytes]: 0x01000297 "1st",
   0x01000298 "2nd", 0x01000299 "3rd", 0x0100029a "4th" (only 4 exist; the row loop caps at
   4 rows — decomp line 219 `while (iVar2 < 4)`, disasm `slti $v0,$s1,4` @0x212904 — so a
   >4-player match shows its top 4).
5. Per row r < 4 with sorted slot s: show the row's widgets (msg 0x2b value 2), or hide them
   (value 1) when r ≥ the player count [M :145-219]. Then:
   - portrait 0x10000157 ← msg 0x24 the preview texture of `mp_characters_small` @0x2df8f8
     looked up by key = `MPSettings+s·0x30+0x24` (the skin/character id); unknown id → no
     send [M :159-166; the 29-row table [L dump, key = +0x0C, tex = +0x00]].
   - rank 0x10000156 ← msg 0x18 the rank label [M :139-141].
   - name 0x10000158 ← msg 0x18: slot < 4 → the codename string `MPSettings+s·0x30+0x00`;
     slot ≥ 4 → `Menu_GetBotShortName(character)` 0x201918 = label 0x10002e2+character
     ("Bond"…"Max Zorin"), literal "Bot" @0x30d290 for ≥ 0x1d [M :167-177; L the label map].
   - points 0x10000159 ← `"%d"` of `(int)(float)MPGame+s·0x30+0x18` [M :178-185;
     format "%d" [L 0x30d2f0]].
   - victories 0x10000240 ← `"%d"` of `MPGame+s·0x30+0x04` (the kills) [M :186-192].
   - deaths 0x1000015a ← `"%d"` of `MPGame+s·0x30+0x08` [M :193-198].
   - TOTAL 0x1000015c ← `"%d"` of `Menu_GetMPScore(s)` [M :199-204].
6. The result line 0x1000023d [M :220-250; disasm 0x212934-0x212a94]:
   - msg 0x66 arg 1 first (mode/color; not further traced).
   - **Non-team** (scenario bit 0x20000000 clear): pair0 score == pair1 score → the bound
     0x010002a0 "Match was a draw"; otherwise `sprintf(mp_result.451 @0x34dc68, "%s wins the
     match." 0x0100029f, MPSettings + winnerSlot·0x30 + 0x00)` — the codename (disasm
     0x212a64-0x212a7c: the arg is the record base, no extra offset). With a single player the
     game compares against an uninitialized stack pair — the port treats a 1-entry board as a
     win (*Deviation*).
   - **Team**: the compares are FLOATS (decomp lines 233/237): `(float)MPGame+0x180 <
     (float)MPGame+0x184` → "%s wins the match." with 0x1c8 "MI6"; `>=` → with 0x1c7 "Phoenix";
     equal → the draw text. [S] Likely an original inversion: P_MPCONFIRM's panels caption MI6
     0x1c8 on the left/blue side (0x1000023f, x=51) and Phoenix 0x1c7 on the right/red
     (0x1000023e, x=324), and the setup lists offer Phoenix first — yet the debrief announces
     MI6 when team0 (the lower index) is BEHIND. Ported literally; a live check can settle
     which side the announce names.

### The return flow — `C_MPDBG_Handler` 0x217860

Exactly two arms (decomp: 0xa8 bytes, only 0x4b/0x5d branches; no init arm; re-read in this
task) [M :8-16]:

- **0x4b (Continue, ~A)**: `Menu_RestoreMPSettings` 0x202848 (MPSettings' first 0x1dc bytes
  back from the backup @0x2dec68 + the four 0x158-byte PlayerSetting records from @0x2de708,
  each preserving its +0x0e byte [M 0x202848:10-19] — the pre-match menu state that
  `Menu_StoreMPSettings` 0x2027d0 saved at the match-summary confirm), then
  `Manager_SendMessage(manager + page·0x1d8, 0x44, 0x4000001a, 1)` — to the scenario page.
- **0x5d (Replay, ~Y)**: `ResetMap_LevelToLoad(MPSettings+0x1A8, 0, 0)` (the arena just
  played), `GameFlow_PushState(85.0f, 7, 0xff)`, `MenuManager_Delete(page)` — the match
  restarts on the same settings without passing the summary page.

## 5. The port

- `crates/nf-game/src/mp/mod.rs` (402's file, the single machine): `MpGame` gained
  `handover_flag` (GameState+0x24's MP arm), `request_debriefing` (the case-4 navigation
  request), `restart_pending` (case 6) and `end_status: MpEndStatus` (`None`/`Won(codename)`/
  `Draw`, from `MP_SortOutWhoWon`); `high` keeps +0x18C (621's `top_score` was the same word).
  `check_for_end_condition` now ports §1 whole (the demo/prot state-6 arm without the 0xfe
  latch, the 0x10 elimination on the `dead` stand-in, the shared frag latch + final dispatch)
  and `update` runs §2's switch (3 → 4 sets the handover flag; 4 → the debriefing request +
  state 5; 6 → the strict-`> 5.0` restart, in place, flagging `restart_pending`). The wall-clock
  fields keep the fixed-tick deviation documented in the module doc. `MpPlayer` gained
  `codename` (+0x00; the boot default "Player N" [M bootup_bootup 0x1c7e78:50-67] — the port
  defaults empty and the menu fills the joined agents').
- `crates/nf-game/src/mp/rules.rs` (402's file): `sort_out_who_won` keeps returning `Winner`
  (the flow result) and now also stores `end_status` (the status text data); the 0x10 branch
  uses the count/tie quirk and the FFA branch drops the obj gate (reconciliation rows 5/6).
- `crates/nf-game/src/mp/debrief.rs` (621's file): `MpMatch::get_mp_score` (§3's table) and
  `MpMatch::debriefing()` building §4's rows (the sort, the rank walk with ties, the cap at 4,
  names, characters) + the result line. `MpDebriefName::Human(codename)` /
  `Bot(character)`; the bot short name is label 0x10002e2+character (None → the literal
  "Bot").
- `crates/nf-view/src/menu.rs`: `Action::Extra` (the pad's ~Y/Triangle; the PC stand-in is Q
  [S]) with arms on every page (all idle except the debriefing's);
  `FrontEnd::mp_debriefing: Option<MpDebriefing>` + `open_mp_debriefing(&MpMatch)` (the host's
  case-4 handover: the page via `enter_page` + the data); `MP_CHARACTERS_SMALL` (the 29
  portrait rows [L]); the page's label consts; the getters `mp_debriefing_rows()` /
  `mp_debriefing_result()` / `mp_debriefing_portrait(character)`; the debriefing page's
  `Action::Extra` → the rematch: the same level re-requested with the same `mp_match_settings`
  (the confirm-start's loader path), mirroring C_MPDBG 0x5d. Confirm returns to the scenario
  page (C_MPDBG 0x4b; the port keeps the menu-side MP state — the game's restore exists because
  the match compacted MPSettings in place, which the port's copy never does). The confirm
  start hands the joined agents' codenames to `MpPlayer`.
- The debriefing page's DRAW is still the main agent's (the menu-def's plate/rows render from
  the getters above; `Page::MpDebriefing` draws nothing yet — reported, with the widget table
  in §4).

## 6. Live checks for the main agent (PCSX2; none runnable here)

1. Watch `GameState+0x1C` @0x2a3784 (w) when a match ends naturally (frag limit in Arena) —
   expect 0x40000033 (disasm 0x184e80); then `GameState+0x24` @0x2a378c — expect 1.
2. Team-announce check: play Team Arena, make team 1 (the second-picked side) win — the
   result line should read per disasm "MI6 wins the match." when team0 < team1 (0x1c8). If
   PCSX2 shows "Phoenix" there instead, the [S] inversion note in §4 is wrong and the port
   flips the two label ids.
3. `MPGame+0x188` @0x2a4b08 (w) at the end: 1 on a frag-limit end, 2 on time-up (Arena),
   6 on Demolition/Protection time-up, then 3 → 4 → 5 across ~5 s.
4. Debriefing rows with a known 3-bot Arena game: the rank captions "1st"/"2nd"/"3rd", the
   bot short names, Points = (int)score, Victories = kills, Deaths, TOTAL = kills.
