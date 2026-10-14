## Your task: `pause-audio`, pausing the game pauses the sound

The user's playtest (2026-10-13): with the pause menu open, the ambient sound keeps playing, including the truck driving
loop on 07000005. The port stops ticking the sim while a menu is up, but nothing pauses the sound output, so looped and
3D voices keep running.

Find the game's pause path for sound: `GS_PauseGame` 0x17d178 sets `GameState[0x50]` and the MP pause words; find who
reads `GameState+0x50` on the sound side (`Sound_Update*`, the EE->IOP batch, `SFXPause`/`SFXUnPause` RPCs in SFX.IRX,
`docs/systems/sound.md` section 8: the port notes it has no global `SFXPause`) and what the music does while paused (the
streamed music may keep playing in the game: check). Port the global pause in `nf-audio` (voices hold their position and
resume; no new key-ons while paused) and drive it from the pause page opening/closing (`menu.rs`/`lib.rs`), with the menu's
own sounds still playing. Tests: a looping ambient voice's sample position doesn't advance while paused and resumes after;
the menu click still plays; sister: unpaused it advances.

Files: `crates/nf-audio/src/{sfx,lib}.rs`, `crates/nf-view/src/{lib,menu}.rs` (the hook), `docs/systems/sound.md`.

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

**Branch:** `s/pause-audio`.

---
claimed-by: sandbox6 at 2026-10-14T00:20Z

---
## Report (done, 2026-10-14T02:02Z)

