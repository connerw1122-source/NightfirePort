## Your task: `qclaw`, the Q-Claw: no hook model, a bullet impact instead of the grapple

The user's playtest (results 1013-1758, 07000005 at `--bond 16.5,29.6,58.6,0,0.25 --give 0x50`, beside grapple point
#282 (13.75, 32.29, 57.3)): "the grapple hook isn't there; using the hook within range makes a bullet impact". Find
the Q-Claw's real weapon id (the port's dev panel and `--give` used 0x50; another note says 0x4a: settle it from the
weapon table and the game's equip code), its first-person model, and its fire path (the grapple: latch on a grapple
point within range, reel in, hang while held; no bullet). Fix so `--give <qclaw id>` draws the claw and firing at #282
from that spot latches and reels. Tests: through the real input path at that spot: fire held -> Bond travels to the point
and hangs; 0 bullet decals; the view model is the claw's mesh; sister assertions.

Files: `crates/nf-game/src/{gadget,gun,player}.rs`, `crates/nf-view/src/lib.rs` (`--give` only if the id mapping is
wrong).

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

**Branch:** `s/qclaw`.

---
claimed-by: sandbox3 at 2026-10-14T02:03Z
