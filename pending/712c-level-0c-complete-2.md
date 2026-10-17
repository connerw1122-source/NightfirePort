## Your task: `level-0c-complete-2`, finish the 0c completion (HELD at merge)

`s/level-0c-complete` (712b, beeac6fc — drone.rs +89, ss.rs +33, world.rs +8) was held: merged onto origin/master (77c7bce6) it
breaks three tests that pass on master:
- `channel_persist_live::the_port_rewrites_only_game_owned_channels` (line 201): "07000002.bin tick 0 frame 49: ch 0x02 = 0
  written by NO game owner" — the branch writes switch channel 2 on 02 from a port-only path (the master channel-persistence
  rule: only game-owned writers may set/clear channels).
- `walkreal_07000012_live::walk_spawn_to_the_shaft_exit` and `walkreal_probe13::probe_13_entrance_warm` (tests/common/tower2.rs:104:
  "E at the console writes ch 0x49 (override #731) and ch 0x59 (lift call #702) [D]") — the 12 console press no longer
  writes its channels.
Your SS_Operate contact change (the mask-0x2 winches) is likely reaching every level's SS/channel path, not just 0c's winches.
Scope it to the game's rule [M 0x1a9df8], make all three pass alongside your own tests. Also (main-agent render check): a shot
winch only advances t = 0.04 and stops — see task 787 for the full-sweep follow-up; you may fold it in.

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

**Branch:** `s/level-0c-complete-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
