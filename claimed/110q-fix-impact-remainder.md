## Your task: `fix-impact-remainder`, the decal penetration roll and the emitter arm (G-041)

G-041 (docs/GAPS.md; docs/atlas/effect.md) lists what the impact-effect port still leaves out: the `Rand_Rand(0x3f)`
penetration roll and the emitter arm of `Effect_Bullet` 0x1ce8a8 / the collision handler. Both consume the game's RNG,
so leaving them out moves every later AI roll after a wall hit.

Do:
1. Read the decompile around `Effect_Bullet` 0x1ce8a8 and its caller in the bullet collision handler. List the
   penetration roll (when it runs, what it decides: a pass-through? a second decal?) and the emitter arm (what it
   spawns, for which materials). Document both in docs/atlas/effect.md with addresses.
2. Port both in `crates/nf-game/src/effect.rs` and the caller, consuming the RNG in the game's order and at the game's
   point in the frame.
3. Test (NF_ISO): a P2K shot into a 07000005 wall advances the sim RNG by the game's count, and the roll's outcome
   matches the decompile's rule for a fixed seed. Sister: a shot into the sky (no hit) draws nothing. The full suite must
   stay green; RNG-order changes move AI fights, so investigate any flip.

Files: `crates/nf-game/src/effect.rs`, the bullet collision caller, docs/atlas/effect.md, the new test.

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


**Branch:** `s/fix-impact-remainder`.

---
claimed-by: sandbox6 at 2026-10-15T12:29Z
