## Your task: `screen-adjust-model`, AV Options → Screen Adjust

`P_SCREENADJUST_Handler` 0x217a58 / `C_SBSCREEN_Handler` 0x217b08: the page that moves the picture. Port what it
does: the offsets it edits (the GS display registers / the game's screen offset variables, their ranges and steps),
how they are saved in the profile, and what they change on screen — then map that to the port: a picture offset the
renderer can apply (expose the values + a function giving the offset in 640×480 units; the main agent wires the draw).
Also the AV page's Restore Defaults row's full reset list (`P_CNAVOPTIONS_Handler` 0x214818:100-106) and the
Widescreen / Speaker / Split-screen values' runtime effects in the game (document; implement Widescreen's 16:9 view
aspect if it is a simple viewer change — `Camera_CalcViewAngles` 0x1baf70 writes 16/9 when DAT_002a37c4 is set).

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/screen-adjust-model`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
