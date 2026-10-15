# fix-restart-health — the restart gives the game's health: FULL after a cold level, max(saved, 50) after a hurt checkpoint

## What was wrong

The user: "if I die and it takes you to the beginning of the level, I should be at full health.
This is ridiculous." and (07): "I'm not really sure I believe you when you say you start with
half health at a checkpoint." The port's fail/death-menu Restart served health **50.0 ("half
health") on every restart of a cold-loaded level**, and the merged test suite pinned that wrong
model (`fix_death_restart_live.rs::restart_of_a_cold_loaded_level_restores_the_zero_buffer_with_the_health_floor`
asserted `health == 50.0` — it passed on the shipped tree; shown again in this session).

## The cause

The viewer kept `entry_save` = the save the level was **LOADED with** — `None` after a cold
load — and the restart branch then served a zeroed buffer (`z.health = 0.0` → the 50.0 floor →
**50.0 always**). The game's buffer is not the zero buffer at death time:

- **Every** SP level load ends in an unconditional `Player_RamSave(BLData)` [M
  `Player_InitWeapon` 0x1a7ef8:325 — the MP arm jumps past it to `LAB_001a8a18`]. The RamSave
  buffer 0x2b9440 therefore holds the **POST-load** state.
- On a cold load nothing restores first: `Player_RamLoad`'s whole body is gated on
  `IsWarmReset` [M 0x1957a8:9], so `Player_Init`'s **health 100 / armour 0** [M 0x1910f8:128-138
  — `fVar11 = 100.0`, `+0x8b0 = 0`, `Player_SetHealth` gated only by `PlrStat_OkToUpdate`
  [M 0x1993e0: Mission_Status < 2, true during the load]] and the level arm's loadout ARE what
  the :325 re-save captures. The BSS-zeros buffer exists only before the first load ever
  completes — unreachable in play.
- The fail-menu Restart is `ResetMap_LevelToLoad(current, 1, 1)` + `Player_RamLoad(Bond, 1)`
  [M `P_ENDMISSION_Handler` 0x216618:19-21]. `Player_RamLoad` floors the health at the
  difficulty's `ContinueHealthBoost*` — **50.0 on all three** [L ACTION.ELF 0x30cc34-3c, read
  this session: `00004842` ×3] — and the disasm shows the max written **back into the buffer**
  (`lwc1 $f12, 0x5fc($s1)` … `swc1 $f12, 0x5fc($v1)` at 0x1958a0-0x1958e4) before
  `Player_SetHealth` (jal 0x1960b8 at 0x1958e8, `$f12` = the floored value, `$a0` = BLData;
  armour `+0x8b0 ← RamSave+0x600` at 0x1958f4-0x1958f8, the weapon request `+0x934 ←
  RamSave+0xA18` at 0x1958fc-0x195900). The reload's `Player_InitWeapon` re-runs the level's
  entry arm over that buffer (for 05 the fresh arm ignores it and `Player_Init`'s 100 stands;
  for the RamLoad arms it restores it) and re-saves at :325.

**The game's model** (what the port must give):

| case | health | armour |
|---|---|---|
| cold load / pause-menu cold Restart | 100.0 (`Player_Init`) | 0 |
| fail-menu Restart after a cold level | **100.0 — FULL** (the buffer holds the :325 re-save) | 0 |
| fail-menu Restart, last save = H | **max(H, 50.0)** | the save's armour |
| warm level carry (the RamLoad-arm levels) | H (the exit health) | the save's armour |

So (07)'s disbelief is right in the common case: the 50.0 is a **floor**, not the rule — "half
health" only when the last save was under 50 (a hurt entry or a hurt checkpoint). And a level
entered at full health restarts at FULL health — the user's report.

A second, smaller deviation surfaced while testing: the serve carried the **dying** mission
snapshot, whose current part's monitor states (2/3 on fired condition channels, the fail dirt)
came back — the game's `Mission_Init(level, 1)` resets the current + future parts in the
`.data` [M 0x182270:68-129] and the RamLoad never touches the states, so the reset is the last
writer. The port applies its carried snapshot AFTER its `Mission_Init` latch (drone.rs), so the
serve must pre-apply the reset.

## The fix

- `crates/nf-view/src/lib.rs` — the two viewer hooks, both cited inline:
  - `entry_checkpoint_save(sim)` — the 0x1a7ef8:325 mirror; the reload loop captures it from the
    fresh sim after **every** completed load (`self.entry_save = Some(entry_checkpoint_save(...))`),
    replacing the load-time save.
  - `fail_restart_save(entry, sim)` — the `level_exit_restart` branch's serve: the entry
    checkpoint with the 50.0 floor [L 0x30cc34-3c] + the mission payload in the
    `Mission_Init(level, 1)` shape (previous parts kept, current + future reset to
    `enable → 1 else 0` [M 0x182270:98-105]) + the live mission re-snap (fix-objective-messages).
  - The `entry_save` field doc and the branch comments rewritten to the corrected model.
- `crates/nf-game/src/player.rs` — `RamSave::with_mission_snapshot` (additive, own lines): the
  pre-built payload setter the serve uses.
- Re-pointed `fix_death_restart_live.rs` test 3 to the corrected model
  (`restart_of_a_cold_loaded_level_restores_the_post_load_entry_checkpoint`: health **100.0**,
  armour 0, clean channels/mission/loadout) — the same re-pointing fix-restart-target did to
  `walk_0700000a_live` when the model under it was corrected.

Shared files touched: **none** beyond the task's own (`lib.rs` is the viewer's reload loop —
the fix lives in the named task files `player.rs` + the viewer's reload branch; `drone.rs`,
`scene.rs` untouched).

## Verification (NF_ISO, slim ISO; real input path throughout)

- **Failing first**: the new test binary does not compile against the pre-fix src
  (`error[E0425]: cannot find function entry_checkpoint_save / fail_restart_save in crate
  nf_view` — shown by checking out 2987e078's lib.rs/player.rs). The shipped symptom is pinned
  by the OLD test 3, which **passes on the pre-fix tree asserting health == 50.0** (shown this
  session: `test restart_of_a_cold_loaded_level_restores_the_zero_buffer_with_the_health_floor ... ok`
  at 2987e078) — that 50.0 IS the playtest's "half health".
- `fix_restart_health_live.rs` (new, 2 tests, green on the merged tree d28d6f07):
  - `restart_after_death_on_a_cold_level_gives_full_health` — cold 07000005, dirty the run
    (damage + ch 0x45), die through the real health path, ✕ (action 25), the menu auto-restarts
    → the serve = 100.0, the warm reload spawns at **100.0 full / armour 0**, channels + mission
    states + loadout == a clean load's, no re-fail over 360 ticks.
  - `restart_health_is_the_entry_health_floored_at_the_50_boost` — on 07000007: a full-health
    entry dies → restarts at **100.0**; a sub-50 entry (the hurt warm carry) dies → restarts at
    **50.0** (the game's floor, [L 0x30cc34-3c]); armour 0 both [M 0x1a7ef8:180]; no re-fail.
- Re-run of the neighbours on the merged tree: `fix_death_restart_live` 8/8,
  `fix_restart_target_live` 2/2, `level_carry_live` 5/5, `level_reload_live` 5/5,
  `fix_level_chain_live` 3/3, `campaign_flow_live` 4/4, `fix_progress_live` 8/8,
  `fix_save_profile_live` 9/9, `turn_stutter_live` 2/2, `fps_determinism_live` 4/4,
  `fix_interp_ghosting_live` 3/3 (the turn-stutter merge shares lib.rs), `nf-game --lib` 160/160.
  **Total this session: 215 passed, 0 failed.**
- Full fast suite: the `nohup python3 tools/test/suite.py` run died at once (this sandbox reaps
  background processes between tool calls — the same failure fix-remote-mine-play and
  fix-aims20-modes-lights recorded), and the 2-hour window closed before a foreground full run
  could fit. The foreground chunks above are the covered set. **Next session: run the full
  fast suite on this tree and put the total line here.**

## Exact repro (the user's symptom, pre-fix)

1. Load any level cold (the mission's first entry, or after the pause menu's Restart) — e.g.
   `nf-view <iso> 07000005`.
2. Die (fall, gunfire), press ✕, pick Restart on the 0x40000042 menu.
3. Pre-fix: Bond comes back at **50.0** ("half health"). Post-fix: **100.0** — full health,
   armour 0, the fresh loadout, the clean channels/objectives. On 07 entered hurt (< 50) the
   game itself gives 50.0 — the floor, not a bug.

## Live checks for the main agent

PCSX2 (`gdbwatch.py` / `fieldprobe.py`), retail or slim ISO:
- **spot**: Bond's BL+0x894 (health) right after `Player_Start` returns on a COLD load of
  07000005 — **expect 100.0** [M 0x1910f8:138].
- **spot**: the RamSave buffer 0x2b9440+0x5FC after that load settles — **expect 100.0** (the
  0x1a7ef8:325 re-save), not 0.
- **action**: die (any path), pick Restart on the fail menu; **watch** BL+0x894 through the
  reload — **expect 100.0 after `Player_Start`** (the 50.0 `ContinueHealthBoost` write at
  0x1958e4 lands in the BUFFER, then `Player_SetHealth(100)` from `Player_Init` runs mid-load;
  for a RamLoad-arm level the final write is the buffer's floored 100.0).
- **action**: on 07000007 entered warm from a < 50-health exit, die and Restart — **expect
  BL+0x894 = 50.0** (the floor), armour BL+0x8b0 = 0.

nf-view `--bond` spots (render checks for the reload):
- `--bond <start of 07000005>` after a die→Restart: Bond stands at the level start, the HUD
  health bar FULL, the fists/P2K loadout per the 05 case, no "MISSION FAILED" banner, no
  re-announced objectives.
- Same on 07000007 (the party level): the disguise loadout (fists request), armour 0.

## What's left

- The full fast-suite total on this tree (the background run was reaped; the covered chunks are
  all green — see above).
- The `pending_weapon_assets` branch (the dev give's warm reload) still sets `entry_save` from
  the PRE-reload capture — a dev-only affordance with no game path; left untouched.
- The fail-menu row 1 (BaseMap cold) and the pause-menu Restart were already correct (cold →
  `Player_Init`'s 100) and are pinned by the clean-load comparisons.

## ntfy failed

(none — the notification is sent after this report)
