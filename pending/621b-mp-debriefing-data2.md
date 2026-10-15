## Your task: `mp-debriefing-data2` — land 621's debriefing on top of master's 402 match rules

`s/mp-debriefing-data` (621, tip 20d5bde1) was HELD on 2026-10-15 because it ports `MP_Update` 0x184bd0 /
`MP_CheckForEndCondition` 0x184268 a SECOND time. Master already has 402 (`s/mp-match-rules`):
`crates/nf-game/src/mp/rules.rs` and the state machine in `mp/mod.rs`, with these names:
- `MpSlot::status_bits` / `dead`
- `MpMatch::high` (the +0x18C running high score)
- `update(dt, paused, rng: &mut Rng)` (the state-6 restart's site re-pick draws)

621 has `MpSlot::out`, `top_score`, `handover_flag`, `request_debriefing`, `restart_pending`, `MpEndStatus` and
`update(dt, paused)`. Conflicts are in `mp/mod.rs` and in `menu.rs`; the menu.rs ones are trivial: master's pages
call `self.enter_page(..)` / `self.focus_moved()`, and 621 added `Action::Extra` arms plus the C_MPDBG replay.

Start from `origin/master`, keep 402's single state machine, and port only what 621 adds on top:
- the debriefing board (`mp/debrief.rs`: `get_mp_score`, `debriefing()`, ranks/ties, names, the result line);
- the end status and the handover/debriefing/restart requests, as fields on master's machine where 402 lacks
  them (reuse `high` for +0x18C, don't add a second one);
- the Top Agent elimination input, reconciled with 402's `dead`;
- the `Action::Extra` arms.
Where the two ports disagree on a fact (state transitions, the frag latch compare, suicide scoring), decide from
the disassembly and say which side was wrong. All 402 and 621 tests must pass on the merged design; adapt 621's
tests to master's API. Report the reconciliation table.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-debriefing-data2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
