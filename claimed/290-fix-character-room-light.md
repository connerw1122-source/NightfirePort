## Your task: `character-room-light`, characters lit by the room they stand in

The user's playtest on 07000007 ("characters lit by the room"): **no**. The port multiplies 0x800 objects by tint x cel
ambient >> 8 (render-port-check.md 1.1), but the user still sees the party's characters lit wrongly. Also G-042: the
07000002 bathhouse civilian's skin texels. Find which cel ambient a character uses in the game (the anim object's cel,
`obj+0x20`, set by `control_link_object_to_cel` 0x1346b8, vs the port's lookup), whether dynamic lights (`Light_Create`, the
muzzle light) add to characters, and the per-character ambient multiplier. Write render spots (07000007 ballroom guests,
07000002 bathhouse) and a precise live-check request: the main agent captures PCSX2 at those spots with
`tools/trace/pcsx2_shot.py --pos ... --pitch ... --god`.

Files: the character lighting in `crates/nf-view/src/scene.rs`, `docs/systems/render.md`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/character-room-light`.

---
claimed-by: sandbox7 at 2026-10-14T19:40Z
