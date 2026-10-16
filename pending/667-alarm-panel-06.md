## Your task: `alarm-panel-06` — the alarm panel on 07000006 turns the alarm off

The user's playtest 1015-2142 (07000006): "no panel to turn off the alarm". When the searchlights or guards raise the
alarm on 07000006, the game has a wall panel/switch Bond can use to silence it.
- Find the alarm system's objects and channels [M the alarm sound/state owner, the script or switch object that ends
  it, the `Player_Activate` 0x1a5170 arm that uses it].
- Find where the panel is placed on 07000006 (placement type/hash, its glist/prompt).
- Find why the port has no usable panel: not created, no use-prompt, or the channel write is missing.
- Port the missing part so pressing E at the panel stops the alarm exactly as the game does: the channel, the sound
  stop, and the guards' state, if the game changes it.
Headless tests: raise the alarm on 07000006, walk Bond to the panel, use it, and assert the alarm state, sound and
channel clear. Sister: using it with no alarm does what the game does. Data/sim only; if the panel's mesh doesn't
draw, report that (the main agent draws).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/alarm-panel-06`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
