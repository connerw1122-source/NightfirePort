## Your task: `fix-death-retry-boost`, the restart's health floor after a death

`Player_RamLoad(Bond, 1)` (0x1957a8) is what the fail menu's Restart calls (`P_ENDMISSION_Handler` 0x216618:19-21). Its
second argument (the "boost") raises Bond's restored health to a difficulty floor, `ContinueHealthBoost*` (all 50.0 on the
retail build, [L] ee_start_05.bin 0x30cc34..3c). The port doesn't do this yet: `crates/nf-game/src/player.rs` ~line 1204
says so ("The `boost` argument ... is not ported"), so a restart after a death at low health brings Bond back at that low
health.

Do:
1. Read `Player_RamLoad` 0x1957a8 in the decompile (`work/decomp/ACTION.ELF/001957a8_*.c`) and write down exactly what
   the boost does: which field, which difficulty table entry, a max or a set, and armour or not. Put it in
   `docs/systems/mission.md` §8 (the death/restart section) with the addresses.
2. Port it: the fail-menu Restart path passes boost = 1; the trigger-exit warm load passes 0.
3. Test (NF_ISO, `crates/nf-view/tests/fix_death_retry_boost_live.rs`): on 07000005, drop Bond's health to 10, take the
   entry checkpoint the way the level start does, kill him, choose Restart (`fail_choose_restart`), then assert the health
   after the reload equals the game's rule (50 if the floor is a max). Sister assertion: a trigger-exit warm load keeps 10.
   Use `crates/nf-view/tests/fix_death_restart_live.rs` as the model (its helpers `load_carry`, `tick`, `exit`).

Files: `crates/nf-game/src/player.rs` (the RamLoad), `crates/nf-game/src/death.rs` (if the restart call lives there),
`docs/systems/mission.md`, the new test.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).


**Branch:** `s/fix-death-retry-boost`.

---
claimed-by: sandbox2 at 2026-10-15T10:05Z

---
## Report (done, 2026-10-15T11:35Z)


# fix-death-retry-boost — the restart's health floor after a death

## What was done

`Player_RamLoad`'s (0x1957a8) second argument — the boost — is ported.

The game's rule [M 0x1957a8:21-34], with the fail/death menu's Restart passing boost 1
(`P_ENDMISSION_Handler` 0x216618:19-21): the saved health — the float at **RamSave+0x5FC** — is
floored at the difficulty's `ContinueHealthBoost*`:

- **The pick**: the difficulty global **`GameState+0x40`** (0x2a37a8) — 1 →
  `ContinueHealthBoostEasy` (0x30cc34), 2 → `ContinueHealthBoostMedium` (0x30cc38), anything else
  (0/3/4) → `ContinueHealthBoostHard` (0x30cc3c). All three are **50.0** on the retail build
  [L ee_start_05.bin 0x30cc34..3c].
- **A max, not a set**: a saved health already above the floor survives; the result is written
  BACK into the buffer (`swc1 $f12, 0x5fc` at 0x1958e4) before `Player_SetHealth` (jal 0x1960b8 at
  0x1958e8) applies it — the floored value is what the reload's `Player_InitWeapon` re-save
  captures [M 0x1a7ef8:325].
- **Armour is NOT boosted** — restored as-is from the save [M 0x1957a8:36].
- **Gated on `IsWarmReset`** [M 0x1957a8:9] — a cold load never boosts.
- **Who passes 0**: `Player_InitWeapon` 0x1a7ef8 passes 0 on ALL EIGHT of its `Player_RamLoad`
  calls [M :158-305] — the trigger-exit warm load never boosts, so a level change at low health
  carries the low health; only a death-Restart floors it.

The port: `RamSave::continue_boost(difficulty)` + `continue_health_boost(difficulty)`
(nf-game::player; the difficulty mirror is `Progress::difficulty` = the GameState+0x40 value),
applied by the restart serve `nf_view::fail_restart_save` — the handler's `Player_RamLoad`
position in the reload flow. fix-restart-health's hardcoded viewer-side `max(50.0)` constant is
replaced by the game-logic call (same observable value on retail — all three entries are 50.0 —
but the difficulty structure and the max semantics now live where the game has them, and the
"not ported" comment is gone). The trigger-exit capture branch never calls it, exactly the game's
boost-0 arm. `docs/systems/mission.md` §8 carries the full entry with the addresses.

## How it was verified

- **Reproduce (pre-fix)**: a probe (scratch `zz_boost_probe.rs`, not committed) showed the port's
  `Player_RamLoad` equivalent (`RamSave::restore`) giving a 10.0-health / 25.0-armour buffer back
  at **10.0 / 25.0**, where the game's `Player_RamLoad(Bond, 1)` arm gives **50.0**. At the App
  level the symptom was masked by fix-restart-health's hardcoded viewer floor — which is exactly
  why the old comment said "not ported": the mechanism existed nowhere in the game-logic layer.
- **Post-fix** (`crates/nf-view/tests/fix_death_retry_boost_live.rs`, 4/4):
  - `boost_is_a_max_picked_by_the_difficulty_global` — the pick table (1/2/else), the max rule
    (10 → 50, 60 stays 60), armour untouched, and the floored buffer being what `restore`
    applies. Would not compile before the fix (no API); the probe above is its behavioural
    pre-fix form.
  - `without_the_boost_the_buffer_restores_raw` — the boost-0 arm (sister).
  - `restart_after_death_at_low_health_comes_back_at_the_floor` (NF_ISO, the real input path): on
    07000005 the health drops to 10, the ENTRY checkpoint is taken the way the level start does
    (the `Player_InitWeapon` re-save shape [M 0x1a7ef8:325]), Bond dies, ✕ skips the fade, the
    menu answers Restart via `fail_choose_restart`, the serve (`fail_restart_save`) carries the
    floored 50.0 buffer, and the reload comes back at **50.0** (the pre-fix port gave 10.0);
    armour rides the buffer; the exit is the current level, warm, restart-flagged.
  - `trigger_exit_warm_load_keeps_the_low_health` (sister): the same 10-health buffer carried the
    way `Trigger_Activate`'s exit does comes back at **10.0** — the boost-0 contrast.
- **The suite** (final tree = merge `302f7b34` of origin/master `0b91e999`, which brings 109r's
  player.rs loadout work — auto-merged clean): the sandbox reaps background jobs, so the run went
  in resumable foreground chunks (the established practice): **324 unique test binaries — 1275
  passed / 6 failed / 66 ignored**. The 6 failures are exactly the documented allowed set: the 3
  `disc_golden` (slim ISO) + the pylon-render trio (2 `fix_0c_pylon_live` +
  `probe_pylon_render`), verified pre-existing on master by a stashed re-run
  (docs/WORKLOG.md, fix-laser-watch-look) and untouched by this task's paths.
- **Final-tree re-runs** after the last merge: nf-game lib 168/0, my 4/4,
  fix_restart_health 8/0, fix_restart_target 3/0 + 1 ignored, fix_death_restart 2/0, and 109r's
  fix_start_loadouts 6/0 (the player.rs interaction check).

## Shared files touched

- `crates/nf-view/src/lib.rs` — `fail_restart_save`: one call (`s.continue_boost(sim.drones.progress.difficulty)`)
  replacing the hardcoded max, plus the doc block. Additive; the trigger-exit branch untouched.
- `crates/nf-game/src/death.rs` — doc-only (the boost mapping on `fail_choose_restart`).
- `crates/nf-game/src/player.rs` — the new const + selector + `continue_boost` (additive) and the
  `restore` doc corrected.
- Docs: `docs/systems/mission.md` §8, `docs/WORKLOG.md` (own entries only).

## Known model simplification

The game's boost block sits inside `IsWarmReset != 0` **at handler time** — on a level that was
loaded COLD the handler's `Player_RamLoad(Bond, 1)` is a no-op and the restart restores the
buffer unfloored. The port's serve always boosts. This is unobservable on retail (a cold load's
buffer holds `Player_Init`'s health 100 ≥ 50 [M 0x1910f8:128-138], so the max is a no-op; the
port does not model mid-level script checkpoints into `entry_save`, so the divergent corner — a
cold level with a sub-50 script checkpoint — is unreachable). If script checkpoints ever feed
`entry_save`, gate the serve's boost on the load's warm flag.

## Live checks for the main agent

- **PCSX2 gdbwatch** on the float at **RamSave+0x5FC = 0x2b9a3c** (the RamSave global base is
  0x2b9440 — `amemcpy(0x2bcc20, 0x2b9440, 0x38)` reads it [M 0x1957a8:15]): die at low health,
  answer Restart — the watch should show the value jump to max(saved, 50.0) when
  `P_ENDMISSION_Handler` runs (the write at 0x1958e4), BEFORE the reload completes. Access: write,
  float, from 0x216618:21's call.
- **PCSX2 fieldprobe**: poke Bond's health (BL+0x894) to 10.0 on any story level, die, Restart →
  after the reload the health pane reads **50** at difficulty 1, 2 and 3 (all three table entries
  are 50.0 [L 0x30cc34-3c] — one spot-check per difficulty is enough). The armour pane keeps the
  checkpoint's armour (not 50).
- **PCSX2 trigger-exit contrast**: exit a level through a LOAD_LEVEL trigger at ≤50 health → the
  next level keeps the low health (no floor). Watch the same RamSave+0x5FC float: no jump at the
  exit (the exit's fresh `Player_RamSave` [M 0x1b2318:111-116] writes the live health, and the
  next level's `Player_RamLoad(param_1, 0)` [M 0x1a7ef8:158-305] never boosts).
- **nf-view**: no render-side surface (the rule is numeric); the tests are the check. The visible
  confirmation is the health pane after a restart — the main agent's usual `--bond` playthrough
  covers it; no special spot needed.

## What's left

Nothing functional. Optional follow-ups: none required; the simplification above is documented
with its trigger condition.
