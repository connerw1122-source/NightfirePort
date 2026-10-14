## Your task: `escort-01`, 07000001: the contact reaches the extraction plate, the level chains

docs/reports/campaign-flow.md, blocker 1 (and 5): 01's exit is the gate ch16 TOUCH zone, which needs BOTH Bond and the
contact (skin 0x4F) inside it, and the contact's Lead/AllyLead root motion stalls before he gets there
(docs/reports/walkreal-a.md). Fix the stall from the game's code (`NDrone2_DSTATE_AllyLead` 0x15e698, the lead route,
the root-motion path) so the contact walks to the extraction plate, ch16 holds, and 01 chains to 02 through the real
trigger. Also: the exit LOAD_LEVEL trigger has no collision volume (lo/hi = inf); find how the game tests a meshless
trigger (the touch filter, `Trigger_Calc`) and port it, since 0a's real exit (blocker 5, -> 0700000b, key ch74) has the
same shape. Tests (NF_ISO): the 01 walk from the spawn reaches the plate with the contact and loads 07000002; 0a's exit
fires with ch74 set and loads 0700000b; sister assertions.

Files: `crates/nf-game/src/{drone,triggers}.rs`, `crates/nf-view/tests/campaign_flow_live.rs`.

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

**Branch:** `s/escort-01`.

---
claimed-by: sandbox1 at 2026-10-14T15:47Z

---
## Report (done, 2026-10-14T16:58Z)

