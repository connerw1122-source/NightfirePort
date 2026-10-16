## Your task: `death-loop-07` — no replay loop after dying on 07000007

Playtest 1012-1723 (07000007): "once I died, we went into a replay loop where it kept replaying at the start of the
mission over and over again. I had to quickly close out." Reproduce headless: die on 07 (a guest-shot fail and a
health-zero death), take the fail path (the port's auto-restart and the fail menu's REPLAY/RESTART), and run 600 ticks
after the reload. Find what re-fails at once:
- a carried fail channel;
- the guest-shot flag not reset;
- the weapon-lock edge firing a guard reaction;
- `level_exit_restart` loops.

Fix it to the game's restart semantics [M Mission_Update's fail cases, the RamLoad restore, the 07 party script's
init]. Test: two consecutive deaths and restarts each land in a playable 07 that stays alive for 600 ticks.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/death-loop-07`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T14:11Z

---
## Report (done, 2026-10-16T16:01Z)


---
## Report from the worker (done, 2026-10-16T16:02Z)

# death-loop-07 — no replay loop after dying on 07000007

Playtest 1012-1723 (07000007): "once I died, we went into a replay loop where it kept replaying at
the start of the mission over and over again. I had to quickly close out."

## What was done

**The diagnosis.** The sim-side fail path was already loop-free on this tree — fix-restart-target
(the fail-menu Restart is `P_ENDMISSION_Handler` 0x216618:19-21: a WARM reload of the CURRENT level
served off the ENTRY checkpoint, `Player_RamLoad(Bond, 1)` 0x1957a8), fix-guest-reactions
(Mission_Update's case-5 wait, not the catch-all re-arm) and campaign-flow (`Mission::restore`
restores the objective states only) had closed the carried-capture and the re-arm loops. What was
still live was the **front end's answer to the same menu**: the Results page's REPLAY row (menu.rs
arm 0, the disc's item value 0 [L page 0x40000042]) requested the level warm
(`reset_map_level_to_load(level, true, true)`) but never carried the row's `Player_RamLoad` half,
so the host's `start_async_load` fell into the **live-capture arm** and served the DYING sim. Its
carried fail channel re-fails the mission the tick the reload comes back (the 0x60/0x99 monitor
hooks for the guest-shot fail), the auto-restart answers again, and the loop turns — exactly the
playtest symptom. The four task hypotheses:

- **a carried fail channel — the mechanism, via the front-end serve.** Reproduced headless: the
  dying capture (ch 99/0x60 up) re-fails the mission with no input after the reload, and the
  auto-restart re-queues (`the_fail_menu_replay_row_serves_the_entry_checkpoint_not_the_dying_state`'s
  sister). The ENTRY-checkpoint serve cannot loop (the buffer predates the fail).
- **the guest-shot flag not reset — closed.** Every guest latch (`screamed`, `scare_shouted`,
  `pending_shout`, `alert`, `hit_log`, `party_loud`) is a per-`Drones` field, fresh at every load;
  nothing guest-related rides any RamSave (the only cross-load carriers are the channel arrays and
  the mission states, both taken from the entry checkpoint / reset snapshot on a restart serve).
- **the weapon-lock edge firing a guard reaction — closed.** The lock's auto-draw
  (`weapon_lock_auto_draw`, player.rs) fires only on 07000007 with ch 0x48 rising; the viewer's
  edge memory `prev_lock_ch` resets to false on every load (poll_async_load) and the reloaded sim's
  ch 0x48 is 0 (the entry checkpoint predates the hall trigger), so no spurious draw at spawn — the
  bouncers stay calm (they attack only an armed, seen Bond).
- **`level_exit_restart` loops — closed.** The flag is fresh per `Drones`, set only by
  `fail_choose_*` (and now the front-end pick), read once per `start_async_load`; `level_exit` is
  consumed with `.take()`.

**The fix** (two files, viewer-side only):

- `crates/nf-view/src/menu.rs` — `FrontEnd::requested_restart` (the field, the `new()` init, the
  action-top clear, the `mission_ended` clear, and the Results arm 0 set): the REPLAY row now
  carries the row's `Player_RamLoad(Bond, 1)` half [M P_ENDMISSION_Handler 0x216618:19-21].
- `crates/nf-view/src/lib.rs` — the host's `requested_level` take consumes the marker (with the
  request, one pick) and marks `sim.drones.level_exit_restart`, so the reload serves the ENTRY
  checkpoint exactly as the sim's own auto answer (`fail_choose_restart`, death.rs) does.

## How it was verified

- **New test** `crates/nf-view/tests/fix_death_loop_07_live.rs` (all through the real input path —
  `nf_view::load` + `Sim::step_bond`, the viewer's own serve/load functions):
  - `two_health_zero_deaths_each_restart_into_a_playable_07` — two consecutive deaths
    (1000-damage pain → ✕ skip → the menu → the auto-restart), each reloaded through
    `fail_restart_save(entry, dying)` + `load_with_carry`, each landing in a playable 07 that
    stays alive 600 ticks (health > 0, MissionState 1, no ch 99/0x60/0x62, no menu, no re-queued
    restart).
  - `two_guest_shot_fails_each_restart_into_a_playable_07` — the same twice over the guest-shot
    fail through the REAL punch path (`NDrone2_PunchImpact` 0x145f28:45-56 → reason 5 → ch 99
    0x13e560 → the 300-tick banner → the menu → the auto-restart).
  - `the_fail_menu_replay_row_serves_the_entry_checkpoint_not_the_dying_state` — the front end's
    Results REPLAY row now exposes `requested_restart`; the host wiring marks the restart serve;
    the fixed reload plays 600 clean ticks. Sister: the pre-fix live-capture serve reproduces the
    loop (the carried ch 99 re-fails with no input and the auto-restart re-queues).
- **Crate tests run before pushing** (`cargo test --offline --release -p nf-view -- --test-threads=1`):
  - the new file: 3 passed;
  - the restart/menu suites: fix_death_restart_live 8, fix_restart_target_live 2,
    fix_restart_health_live 2, fix_death_retry_boost_live 4, fix_guest_reactions_live 3 (1 ignored,
    pre-existing), pause_results_flows_live 9, fail_replay_level 1, menu_graph_walk 2, menus_flow
    42, menu_sp_e2e 9, front_end_chain_live 2, memcard_save_flow 7, menu_audio_flows 9,
    menu_movies 3, menu_music_flow 14 — all green (nf-game is untouched by the fix).
- **Full suite** (fast runner, `tools/test/suite.py`): the baseline run on the branch base
  (origin/master + the WORKLOG entry) read **1835 passed, 6 failed, 72 ignored (46 min)**; the
  final tree (merged with origin/master — master had not moved) was re-run; see the tail line
  below. The 3 `disc_golden` failures are the allowed slim-ISO set. The 3 pylon failures
  (`fix_0c_pylon_live` ×2, `probe_pylon_render`) are pre-existing on the base tree (render tests
  for 0C, untouched by this task).

Final suite total line: `1838 passed, 6 failed, 72 ignored  (44 min)` — the 6 are the baseline's 6
(3 × `disc_golden`, the allowed slim-ISO set; 3 × pylon, pre-existing on the base tree); the +3
over the baseline's 1835 are this task's new tests.

## What's left

- Nothing on the task. Two latent holes noted for later tasks (not reachable in the shipped app,
  where the sim's `auto_restart` answers the menu before any page exists): the BaseMap REPLAY arm
  (menu.rs value-1 arm) is dead code with no sim-side `fail_choose_replay`; a failed async load
  leaves the mission in internal 5 forever (no menu, no timeout).
- Follow-on fix worth considering: `MissionSave` still carries `internal`/`state` (mission.rs) —
  `restore` ignores them today (campaign-flow); any future caller that restores the globals
  re-introduces the freeze/loop.

## Live checks for the main agent

None required (no visual work). Optional confirmation of the row semantics the front-end path now
mirrors: on 07000007, die, let the 0x40000042 fail menu come up, pick the FIRST row (REPLAY) — the
game should reload **07000007 itself** (the CURRENT level, warm, full health at the party entrance),
not 07000005 (the mission's first level) [M 0x216618:19-21 `GameState._12_4_` vs
`Mission_BaseMapHCode`]. A second die→REPLAY should behave identically (no auto re-fail, no rapid
loop).
