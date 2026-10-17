## Your task: `bathhouse-armed-girl-02`, a bathhouse girl has a gun and shoots Bond (playtest 1017-1139)

User, 07000002 (F12 at --bond -43.11,6.46,10.37 frame 4206, a bathhouse girl in a towel facing Bond): "one of the
bathhouse girls had a gun and started shooting at me." In the game the bathhouse girls are civilians (they flee/cower;
the civilian no-shoot rule — see civilian_no_shoot_live). Find which placement this is (kind, skin, params, weapon
param [D]), what the game gives her (`NDrone2_DefaultInit` 0x14b300's weapon / civilian flags [M]), and why the port
arms her. Fix, with a test that every 02 civilian is unarmed and never fires (sister: a guard still fires).

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/bathhouse-armed-girl-02`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
