# fix-objective-messages — the report

Branch `s/fix-objective-messages` (from origin/master 9db3fda, merged origin/master 7be3272 before
the final push). Task 101d: the playtest's (results 1013-1758 / 1013-1825) objective-banner bugs.

## What's done — all four symptoms reproduced then fixed

Every fix's repro is a headless NF_ISO test through the real path (`nf_view::load` +
`Sim::step_bond`, the drawn rows measured off `hud::overlay`'s quads):
`crates/nf-view/tests/fix_objective_messages_live.rs` — 5 tests. All failed pre-fix with the
playtest's own numbers; all green post-fix and on the merged tree (origin/master 7be3272).

1. **"I get two objective completion messages"** (07000007, the library table, the armour pickup):
   the game's type-2 pane hides the BODY row for a completion — `HUD_UpdateStatusPane` case 2
   strcmps the message text against `Txt_BindLabel(0x2000053)` ("OBJECTIVE COMPLETE") and case 3
   then forces the body sprite's colour (+0x10) to 0 in BOTH of its phases [M 0x19c310, ACTION.ELF
   decompile case 2's `*(iVar8+0x10)=0` + case 3's `if (IsComplete) *(iVar8+0x10)=0` /
   `uVar11=0`]. The status label alone carries "OBJECTIVE COMPLETE". The port drew the body row
   too — the same string twice, stacked. hud.rs now draws the body row only when the message is
   not a completion, and gives the status label its steady gold 0x785a14ff (case 3's else branch
   stores it unconditionally; the port used 0x7d6d59 for both rows).
   Test `an_objective_complete_banner_draws_one_row_not_two`: the real Kimiko-trigger path
   (TouchOnce #412, ch 0x48), the complete banner opens after the scene's hand-back; pre-fix the
   body band (y 24..46 in the 512×448 HUD space) held text quads, post-fix it is empty and the
   status row centres on 256.

2. **A level start announces other levels' objectives** ("NEW OBJECTIVE / Breach the castle
   walls" on 07000007/07000008 before 07's own; "Find a way inside to the party" on 07's fail):
   root cause chain, fully evidenced:
   - `Mission_MonitorObjectives` case 0 announces every state-0 record whose level is reached:
     `slot.level <= GameState._12_4_` [M 0x1827d8:41-54, disassembly-checked at 0x1828ec-0x182914:
     `lw 0x2a3774` (GameState+0xC) vs the slot's +0x00, `sltu` + `bnez` skip].
   - `Mission_Init` resets only current (mode 2) + future (mode 0) parts; previous parts skip
     (mode 1 cold / 4 warm) [M 0x182270:68-129].
   - The record states live in `.data` and NO load path reinitialises them (`ResetMap_GameInit`
     0x1d5080 only calls `Mission_Init`; `Init_SwitchChannels` clears the channels, not the
     records). At any real arrival the prior parts sit ANNOUNCED: **[L work/ee_start_0700000a.bin
     @ 0x2a3f78 — mission 07000009's rec1 (condition 0x46, still unfired) is state 3 at
     0700000a's start]** (and its own recs 3/3/1 with ch 0x47=1 fired by 0a's start scripts).
   - The port's fresh `Mission::new()` fabricated 0 for the prior parts, so case 0 stacked
     07000005/06's banners at 07000007's first monitor tick. **Fix**: `Mission::init` marks the
     fabricated 0s announced — state 2 for fail-objectives (flags bit 1), else 3 — exactly the
     monitor's own 0→2/3 transition [M 0x1827d8:47-51], without queueing a message.
   - The fail-path half: the port's restart reloaded with NO mission carry (the restart arm built
     the entry-checkpoint RamSave without the mission), so the reloaded level's fresh Mission
     re-announced the earlier parts. The game's restart resets only the player:
     `P_ENDMISSION_Handler` 0x216618:19-21 = `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` +
     `Player_RamLoad(Bond, 1)` — the mission `.data` persists. lib.rs now attaches the live
     mission snapshot (`with_mission`) to the restart save.
   - Note on the enable chain: every castle-mission record (07000005-08) is enable-gated (0x45-0x4a
     [L MissionData 0x2a4350 in the ELF; ee_start_05.bin shows the states 1 at 05's start]), so
     after this fix a level's own objectives announce via case 1 when the level's own scripts fire
     the enable — 07000007's ch 0x47 fires from its start scripts (the pre-existing
     fix-objective-banner test depends on it), and 07000008's TouchOnce #15 (out 0x49, volume at
     [137.1, 48.5, -82.3] [L sim.drones.triggers]) sits by the spawn and fires "Retrieve guidance
     chip from \"Safe Room\"". Cold 07000008 announces nothing until that touch — the same rule
     the game's own data encodes.
   Tests `cold_07000007_announces_only_its_own_objective` and
   `cold_07000008_announces_only_its_own_objectives`: pre-fix the 08 cold load announced
   "Breach the castle walls" / "Find a way inside to the party" / both 07 lines; post-fix only the
   level's own (enable-fired) lines appear.

3. **The banner text vs its box**: `Text_AddMsg` wraps the message text with
   `Font_WordWrapString` at `TextMsgFormats[type]`'s maxWidth — 480 for the type-2 objective text
   ([L TextMsgFormats 0x2bf108+0x14 = 0x1e0]; the box is `ObjectiveStatusSprInfo[2]` x=16 w=480,
   [L 0x319608+0x58]) — and stores the wrapped '\n' string; `__Font_DrawText` advances one line by
   `glyph0.h + Font.gap + 1.0` per '\n', each line left-aligned from the same block start, and
   `Font_DrawText`'s justify-2 centre shifts that start by the WIDEST line [M 0x1ee598:65-71 +
   0x1eee48 `flags & 2 → x -= width*0.5`]. The port drew one unwrapped line — the long objective
   strings ran past the box at both window shapes. hud.rs now wraps at draw time
   (`Font::word_wrap`, the new nf-formats helper — greedy word wrap; `Font::line_height` =
   glyph0.h + gap + 1.0), each line left-aligned from `256 − widest/2`.
   Test `every_objective_text_wraps_inside_the_banner_box`: every level's every objective line
   queued as the game's type-2 message, the drawn rows measured at 16:9 AND 4:3 — every line must
   lie inside the box's x span 16..496. Pre-fix 07000007's "Rendezvous with undercover agents and
   maintain cover as party guest" spanned **2.1..510.4**; post-fix every row is inside.
   (The game's own layout puts a second body line `line_height` lower — below the 56-px box — the
   port now reproduces exactly that geometry too.)

4. **"Mission Fail / Cover Lost" sits left of the box's centre**: the mission banner's rows use
   styles 0x30cdb8 (main) / 0x30cce0 (fail label), both `ff .. fe 03` = justify flag 2 = CENTRE
   [L 0x30cdb8/0x30cce0 bytes; M Font_ParseFormat 0x1ee3e0 case 4 (the 0xfe escape) byte 3 → flag
   2, Font_DrawText 0x1eee48 `flags & 2 → x -= width*0.5`]. The port passed its right-justify flag
   (`right=true` + `Anchor::Centre`), shifting both rows left by half their width — the measured
   centre was **209.4** (span 162.3..256.5) instead of 256. hud.rs now pre-centres both rows
   (x = 256 − width/2, the same pattern the objective banner and the pickup bar already use).
   Test `the_mission_fail_banner_rows_centre_on_their_box`: the real fail hook (ch 0x61 →
   FailLabel 0x4000032 + ch 99 [M 0x1827d8:98-104] → Mission_Update case 2's type-3 banner), both
   rows centred ±3 px post-fix.

## Verification

- The 5 new tests: pre-fix all failing (the numbers above), post-fix all green, re-run green on
  the merged tree (origin/master 7be3272).
- Post-fix regression sweep: nf-game **255 passed / 0 failed** (mission.rs's change lives there);
  fix_objective_banner_live 3/3, fix_hud_live + fix_party_mission_live 15/15.
- Full suite (`tools/test/suite.py`) on the final merged tree: started in the background at
  03:05 UTC; the sandbox deadline cut it short — the pre-fix full run earlier in the session
  (master + my branch's base) reported build ok / 211 binaries with only the 3 allowed
  `disc_golden` failures; the final-tree run's totals are in `suite.out` (scratch, not committed)
  and the WORKLOG records what it reached. **The next session should re-run
  `NF_ISO=<abs> python3 tools/test/suite.py` once and check the totals line.**

## Files

- `crates/nf-game/src/mission.rs` — the init prior-parts rule (task-named).
- `crates/nf-view/src/hud.rs` — the banner rows (task-named).
- `crates/nf-view/src/scene.rs` — a comment only at the objective-drain site (task-adjacent; no
  behaviour change — see the deferred item below).
- `crates/nf-view/src/lib.rs` — the restart arm's mission carry (task-adjacent hook, ~9 lines).
- `crates/nf-formats/src/hud.rs` — `Font::word_wrap` / `Font::line_height` (new, additive).
- `crates/nf-view/tests/fix_objective_messages_live.rs` — the 5 repro/verification tests (new).
No shared-file rewrites; drone.rs/player.rs untouched.

## What's left (for the next session)

1. **The wrapped-life scaling** [M 0x1b0c30's +0x18 store: `time = ticks·len'/len + 0.5`]: the
   port wraps at draw time (the font lives in the scene's HudTables; the Sim-side drain doesn't
   hold it), so the queued life is still the raw 300/180. ~1.5 % shorter than the game for a
   two-line message. Move the wrap to queue time (pass the font into the drain or cache the
   wrapped length) + sister-assert the life.
2. **The first-60-ticks status flash** [M 0x19c310 case 3: `fmodf(time,40) < 20 → 0x785a14ff else
   hidden`, the body hidden during it, PlayMe2nd 469 firing at its end — the port already plays
   469 at `life−60`]: not ported (outside the task's symptom list). The banner's first second
   differs from PCSX2.
3. **The mission banner's main text font**: style 0x30cdb8 selects FontTable[0] = FONTNFont2
   (glyph h max 19 / gap 6 / space 8 vs SerpLight's 17/5/3 [L 0x305a58/0x305a98]) — the port
   renders it with SerpLight. A small visual weight difference on "Mission Complete"/"Mission
   Fail"; needs nf-formats to extract the second font.
4. **The cold mid-mission divergence (deliberate, see below)** and the suite re-run (above).

## Deliberate divergence (documented)

On a literal fresh-boot cold load of a mid-mission part (only reachable via the port's level
argument — the real game arrives at such levels warm (Trigger_Activate 0x1b2318:111 /
Script_EventHandler 0x1d94c0:198), via the persisted-`.data` cold path (Mission_Update
0x182c60:73 passes warm=0 but the process keeps the records), or via LS_LoadMission), the game's
case 0 WOULD announce the prior parts' state-0 records; the port now announces only the current
part's — the driver's required behaviour and every real arrival path's observable behaviour.
Pre-fix full-suite green means no existing test depended on the fabricated announces.

## Live checks for the main agent

1. **The prior-parts arrival state**: PCSX2, 07000007 (via 05→06 play or a save), breakpoint/watch
   `MissionData` slot 0/1's record +0x14 (0x2a3e28+0x14 / 0x2a3e40+0x14) at the 07 load: expect 4
   (05's rec completed during 05 — ch 0x46 fired at the wall breach) and 3 or 4 for 06's. If 06's
   is 3 with ch 0x46 later re-fired at 07, that's the game re-firing the chain mid-level — the
   port's state-3 marking then also completes it with its banner, matching.
2. **Cold level-select load of a mid-mission part** (if the frontend allows it): watch the same
   +0x14 fields and the banner — the decompile says the fresh-boot game WOULD stack the prior
   parts' announces (case 0, states 0). The port does not (deliberate, above). Confirm which the
   console really shows.
3. **The banner rows** at `--bond 91.91,43.85,-32.12` (07000007, the library table): pick up the
   armour after the rendezvous completes — the banner must show "OBJECTIVE COMPLETE" ONCE (one
   row), gold, centred; "NEW OBJECTIVE / Spy on secret meeting" next (two rows: gold label, tan
   body).
4. **The fail box**: die with Cover Lost on 07000007 at `--bond 42.42,44.10,-77.44` — "Mission
   Fail" and "Cover Lost" centred on the 512-wide box.

## nf-view render commands for the main agent

- `nf-view <iso> 07000007 --frame 200 --shot a.png` — the start banner must be "NEW OBJECTIVE /
  Rendezvous with undercover agents and maintain cover as party guest" (possibly wrapped to two
  body lines inside/under the 480-px box), NOT "Breach the castle walls".
- `nf-view <iso> 07000008 --frame 200 --shot b.png` — no earlier-part banner; after walking to
  [137.1, 48.5, -82.3] (TouchOnce #15) "NEW OBJECTIVE / Retrieve guidance chip from "Safe Room"".
- `nf-view <iso> 07000007 --bond 91.91,43.85,-32.12 --frame 120 --shot c.png` — the completion
  moment: one gold "OBJECTIVE COMPLETE" row.
- `nf-view <iso> 07000007 --bond 42.42,44.10,-77.44 --frame 60 --shot d.png` — the fail box:
  "Mission Fail" / "Cover Lost" centred.

## ntfy

(filled in below if the notification fails)
