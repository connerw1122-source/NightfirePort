## Your task: `cutscene-skip-audio` — skipping a cut-scene stops its voice

The user's playtests (1014-1725, 07000002): "Skip the cutscene in the middle of dialogue, and the dialogue still kept
going until it finished." Earlier reports also said the mission music kept playing over cut-scenes, and that the
music only changed late.
- Port what the game does to the cut-scene's streams and voices when a skip fires: the script player's skip path, the
  NIS stream stop/fade, and `SFXStop` on the speech handles [M the skip arm and the stream/sound calls it makes].
  Also port the music handover around a cut-scene (`nis_state`, the NIS pause/quit [M music.md]).
- Headless tests on 07000002's intro and 07000007's Kimiko scene: skip mid-line → no voice item or stream left
  alive after the game's fade window; let it play → the voice ends naturally; the level music resumes the way the
  game does. Tests stay silent (NF_SILENT).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/cutscene-skip-audio`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T05:11Z

---
## Report (done, 2026-10-16T06:26Z)

