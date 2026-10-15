# fix-progress-remainder — the mission-progress layer's open items (G-045)

Branch `s/fix-progress-remainder`. All three G-045 remainder items landed. Every fix cites the doc
entry it implements and the game address; every fix has an NF_ISO test that was recorded failing
against the pre-fix source (`progress_remainder_failfirst.rs`, 3/3 FAILED on HEAD~1, green on the
fix) plus a sister assertion.

## 1. The "detected" counter's live hook

**The game's rule** [M `DroneVision_LogSeenOpponent` 0x1780a8, called from
`DroneVision_HaveOpponentSight` 0x175ef0:20-25 on every LOS tick past the reaction time — the call
sits in `DroneVision_ProcessDroneSight` 0x176a28:216-231]:

- the drone's opponent is the player (`+0x170 == glb_players`);
- the `Drone+0x42` latch is clear — set once, no clearer in ACTION.ELF [M grep across all files];
- team `+0x44 != 2`; team 3 (civilians/hostages/allies) only on 0x07000009/0x0700000A
  (`GameState+0x12 − 0x07000009 < 2`);
- on 0x07000007 only while Bond's current weapon class (obj+0xdc → +0x62 — the same byte
  `Player_AutoAim` 0x1a7c98:0x1a7d04 reads) is outside the excluded band: skip iff
  `b ∉ [0x6a..0x71] && b > 1 && b >= 0x47` (signed-char compares [M :25-26]; weapon ids < 0x80);
- then `+0x42 = 1` and `PlrStat_LogEnemyDetectedPlayer(0)` 0x199960 (SP gate: slot < 8, non-MP,
  Mission_Status ∈ {0,1}, `+0x00 += 1`).

**The port**: the rule runs at the `Drones::tick` sight ack (where `seen_ticks >= reaction_time`
already mirrors HaveOpponentSight's gate), outboxed through the new additive `Drone.detected_logged`
and folded into `Drones.progress.log_detected(ms)` with the other Log counters. The weapon class is
the port's existing `Drones.bond_weapon` (scene.rs maintains it with `Player_WeaponSelect`
0x1a2cc0's mid-switch semantics) — no new plumbing. (An early attempt added a `Player.bond_weapon`
field; reverted before commit — the field already existed.)

**Tests** (`progress_remainder_live.rs`): a 07000006 idle gate guard spotting Bond counts exactly
once and the invariant `stats[0].detected == #drones with detected_logged` holds through 600 more
exposed ticks; on 07000007 the hands class 0x47 (excluded band) counts nothing while the guard
holds Bond in sight (seen_ticks 414) and the P2K counts; a team-3 party guest seeing Bond never
latches. Sister: the fail-first probe (`detected >= 1`) fails on the pre-fix source.

## 2. The autoaim distances

**`Player_InitWeapon` 0x1a7ef8:24-39** writes MAX_AUTO_AIM_DIST_MIDL/NEAR per difficulty: d2 →
14.0/8.0, d<3 or d>3 → 18.0/12.0, d3 → 0/0, MP → 16.0/10.0 [M disasm 0x1a7f38-0x1a7fb4; L
ee_start_05: 12.0/18.0 with difficulty 1 — the writes are live in memory].

**They are never read.** Evidence: (a) a whole-ELF scan of every gp-relative load/store (`$gp` =
0x314670, derived from Check_AutoAim's own `Autoaim_*` reads) — the only touching instructions are
Player_InitWeapon's six stores; (b) a symbolic lui/addiu scan over the whole text for absolute
references — no load/store resolves to 0x30ccf0/0x30ccf4; (c) the full disassembly of
`Check_AutoAim` 0x1a73f8 (1044 B) and `Player_AutoAim` 0x1a7c98 (604 B) — every global they touch
is accounted for (Autoaim_Range/Angle_H/Angle_V/Easy/Normal/Hard/LockOnMul), none is the MAX pair.
Two whole-ELF scan hits that looked like readers (`lb` in HUD_UpdateHealthPane 0x19ba04 and
HUD_UpdateNightSightPane 0x19d230) are register-relative (`lb 0x1c($s0)`), not these globals.
**The port does NOT thread them** — that would add behavior the shipped game does not have.

**The real gap found on the way, fixed**: `Player_AutoAim` 0x1a7c98 seeds the scan cap from
`Autoaim_Range` (25.0 [L], `ReadTuningVars` 0x1d3a80:20, store 0x1d3bcc) — except weapon 0x50/0x51
(the Q-Claw), whose cap is its own `weapon_data[type]+0x1c` [M disasm 0x1a7d54-0x1a7d68; L
ee_start_05: 50.0 / 100.0]. The port capped everything at 25. Fixed in `Drones::auto_aim`.

**Tests**: the Q-Claw's auto-aim snaps onto a live guard at 30 units on 07000005; the P2K sister
does not (the 25 cap). Fail-first probe: the snap assert fails pre-fix.

## 3. The NewScoresRef hand-off

**What the game does** [M]: `PlrStat_GetScore` 0x1984b0:44-48 aborts (returns NULL) for the maps
whose entry +0x38 flag is 0 — the 0x09 (driving) maps — when `NewScoresRef` (0x30cc6c) is NULL.
But bootup 0x1c7e78:86 sets `NewScoresRef = &DAT_0031ca38` unconditionally, so **the abort is dead
code in the shipped game**. The buffer 0x31ca38 is the zeroed tail of the PTPDATA region (PTPDATA
0x31c090 — the ACTION↔DRIVING hand-off block [M symtab; docs/atlas/driving.md §2]) and no
ACTION.ELF writer exists (whole-decompile grep for the addresses/name + a whole-ELF symbolic
lui/addiu scan: only bootup's pointer store and GetScore's reads). [L ee_start_05: the pointer is
0x31ca38; rows 0..8 all zero.] So on retail the 0x09 maps copy zero rows → every row's count is 0
→ nothing scores (base 0, last 0).

**The port** was `new_scores: None` (= the NULL abort) — wrong on both counts. Fixed:
`Progress::default()` carries `Some([[0; 3]; 9])`, and the copy adds the missing row-7 ×100
[M 0x1984b0:52-54 — the driving module's par seconds become the 1/100-s unit the row carries; the
row-7 step's /100 turns it back, so the score math nets out but the stored row value differs].

**Doc erratum corrected**: docs/systems/progress.md's GetScore paragraph (and
docs/fields/player-stats.md step 2, untouched — shared verified doc) carried the 0x07/0x09 labels
swapped vs the decompile. The decompile settles it: the flag is 0 for 0x09 maps and the NewScoresRef
copy runs in the flag-0 branch; the accuracy formula (shots/hits — an on-foot stat) lives in the
flag-1 branch. The port's CODE was always correct; only the prose was swapped.

**Tests**: a real-load 07000005 Sim scores 0x09000001 to `Some` with last 0 (pre-fix: `None`); with
a hand-off-shaped buffer, the entry's rows copy value/count/target = [0]/[1]/[2] and row 7's value
lands ×100 (ratio math reads it back through /100). Unit sister updated in progress.rs (the 0x09
assert is now the zeroed-buffer score, not the abort).

## Verification

- Fail-first recorded: `progress_remainder_failfirst.rs` 3/3 FAILED against the pre-fix source
  (HEAD~1 checked out for the run), 3/3 green on the fix.
- Final tree (merged with origin/master at ca75b8fc), foreground chunks — **this sandbox reaps
  background jobs** (nohup and setsid both die between tool calls; the exact-sine-flip worker's
  WORKLOG note holds), so `tools/test/suite.py` could not run to completion in the background:
  - nf-game lib: **166 passed / 0 failed**;
  - touched cluster, all 0 failed: progress_remainder_live 4, progress_remainder_failfirst 3,
    ai_live 35, ai_traces_live 4, fix_ai_remainder_live 3, nav_kick_live 5 (+1 ignored),
    progress_live 1, fix_progress_live 3, enemies_07000012_live 3, anim_events_live 4 (+1 ignored),
    drone_portcheck_live 5 — 73 tests over 11 binaries;
  - **Total this session: 239 passed / 0 failed** (166 lib + 73 live), 2 ignored.
- Not re-run: the long-walk binaries (walkreal_07000001_live et al., 120-270 s each). My drone.rs
  changes are additive (a new outbox field, a gated block inside the existing reaction-ack branch,
  a two-line auto-aim cap change) and the whole AI cluster around them is green; the main agent's
  pre-merge suite run covers the rest.

## What's left

- Nothing from G-045's three items. G-045's remaining bullet (the menus' screens) is 180-fix-menus.
- The driving hand-off itself (who fills the PTP rows) has no ACTION.ELF writer; if the main
  agent's live checks below show writes to 0x31ca38 during/after a driving section, a follow-up
  task can model the PTP block copy — the port's `Progress::new_scores` is the ready landing spot.

## Live checks for the main agent

PCSX2, on the real game:

1. **The detection counter** — 07000005 (or any SP level): `gdbwatch 0x2bcc20 4 readwrite
   --seconds 30` while a guard first spots Bond. Expected: exactly one +4 write per seeing guard
   from `PlrStat_LogEnemyDetectedPlayer` 0x199960 (the caller chain 0x175ef0 → 0x1780a8), then
   nothing more from that guard (the +0x42 latch); a second guard adds one more.
2. **The detection latch** — `gdbwatch` on the spotting guard's `Drone+0x42` (drone base + 0x42):
   one byte write of 1 at the detection tick, no clear for the drone's life.
3. **The MAX_AUTO_AIM distances** — `gdbwatch 0x30ccf0 8 readwrite` across a whole level (load,
   fight, the Q-Claw out if available): expected writes only at the level load (Player_InitWeapon
   0x1a7f58-0x1a7fb4) and **no reads**. If a read appears outside Player_InitWeapon, the port must
   thread the per-difficulty distances after all — that would contradict this report's scans.
4. **The NewScores buffer** — `gdbwatch 0x31ca38 0x6c write` on an on-foot level, then (if
   reachable) right after a driving section hand-off: expected no writes on-foot (the rows stay
   zero [L]); any write would identify the PTP copy for a follow-up task.
5. **nf-view** (`--bond x,y,z,yaw,pitch`):
   - 07000005 at Bond's start facing the first courtyard guard until it reacts — the pause stats
     screen (if 180-fix-menus lands) should show 1 detection; `--bond` trace: the guard's
     `detected_logged` stands after its first sight ack.
   - 07000005 at (truck yard) holding the Q-Claw ~30 units from a guard, crosshair on it: the
     crosshair should pull onto the guard (the 50-unit cap); with the P2K it should not.

## Files

- `crates/nf-game/src/drone.rs` — the LogSeenOpponent hook + `Drone.detected_logged` + the tick
  fold + the Q-Claw auto-aim cap (task files).
- `crates/nf-game/src/progress.rs` — `new_scores: Some(zeroed)` + the row-7 ×100 + the updated
  unit sister (task file).
- `crates/nf-view/tests/progress_remainder_live.rs` (new), `progress_remainder_failfirst.rs` (new).
- `docs/systems/progress.md` — the detected hook row, the autoaim rows, the corrected NewScores
  paragraph.
- Shared files: **none** (the `Player.bond_weapon` attempt was reverted before commit).
