## Your task: `cutscene-faces` — facial animation in the cut-scenes

Playtest 1015-1738: "I'm actually a little bit disappointed, though, how we don't have facial movement", and earlier:
"Mouths aren't moving in these cutscenes". fix-cutscenes2 ported the 22-channel face track (script_pose_together).
Find why no face moves:
- does the face track's window start (the opcode-0 windows, the face-track hash per actor);
- do its channels reach the face bones in the skin (the bone mapping);
- does the game also use morph targets (108c gpu-facial-morphs was HELD earlier: read its branch
  `s/fix-gpu-facial-morphs` and the reasons in docs/merge-log.md)?

Deliver the sim/data side: per actor, per tick, the face bones' pose (and the morph weights if the game uses them),
plus a getter the renderer reads, with tests on 07000002's intro and 07000007's Kimiko scene showing the jaw/lip
channels change over the dialogue. Sister: an actor with no face track keeps a neutral face. The main agent wires
the draw.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/cutscene-faces`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
