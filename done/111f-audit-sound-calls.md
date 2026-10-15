## Your task: `audit-sound-calls`, every game sound call vs the port

Several sound gaps have turned up one at a time (the death body thud, the fly-by whiz, the satchel beeps). Make the
full list instead.

Do (non-visual):
1. Enumerate every call site of `Sound_Play3D`, `Sound_Play`, `Sound_PlayAt*`, `Music_Event` and the ambient/loop
   starters in ACTION.ELF (the decomp export: grep the call names). For each: the caller function + address, the sound
   id (constant or computed), and the trigger condition.
2. For each site, find whether the port plays it (grep the id / the caller's port function). Classify each as PORTED
   (cite the port file:line), MISSING, or N/A (driving/MP-only).
3. Write docs/systems/sound-coverage.md as the table, with a summary count. Then port the 5 most-noticeable MISSING ones
   for single-player on-foot (your judgement: weapon/door/pickup/guard voice/explosion classes first), each with an
   NF_ISO test asserting the sound reaches the Sim's sound list (`drones.sounds` / audio) at the game's moment. Tests
   stay silent (NF_SILENT).

Files: docs/systems/sound-coverage.md (new), the ported call sites (small, additive), the new tests.

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


**Branch:** `s/audit-sound-calls`.

---
claimed-by: sandbox6 at 2026-10-15T15:26Z

---
## Report (done, 2026-10-15T17:10Z)

