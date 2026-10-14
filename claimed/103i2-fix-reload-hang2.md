## Your task: `fix-reload-hang2`, the shared-magazine fix silences the gadget draw sound

Base: `s/fix-reload-hang` (merge origin/master into it). HELD at merge: with this branch,
`fix_weapon_sounds_live::every_weapon_on_07000005_draws_fires_and_reloads_with_sounds` fails (passes on master and on
master + fix-striped-boxes2): "0x4c / 0x4f / 0x51 / 0x52 / 0x53 / 0x56 / 0x57 / 0x58 / 0x59: draw sound 0x281 never
played" — the gadget (and twin) records' draw sound. The change is `Sim::clip_slot` (the base-unification predicate,
[M `Player_ReloadAmmoType` 0x1a1988:18-22]) in scene.rs. Find why it stops the draw script's sound cue for these
records (the twin/base mapping applied to records that aren't fire-mode twins, gadgets without ammo, the draw going
through a different slot) and narrow it to what the game unifies. Keep `twin_hop_keeps_the_magazine_shared_with_the_base`
green. Run the full `tools/test/suite.py`; total in the report.

Files: `crates/nf-view/src/scene.rs` (small).

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

**Branch:** `s/fix-reload-hang2`.

---
claimed-by: sandbox5 at 2026-10-14T18:08Z
