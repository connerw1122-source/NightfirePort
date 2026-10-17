## Your task: `break-debris-shatter`, breakables shatter into pieces (playtest 1017-1139)

User, 07: "it disappears, but shouldn't it like break and shatter into like smaller pieces?" origin/master removes a killed
0x20 breakable from the draw (`Break_Kill` 0x12b970 → obj+0xFE bit 0). Port what `Break_Kill` spawns: kind 2 (glass) →
`ShatterPolyRecurse` per triangle of `psiGetTriList(obj)` (the fragment split rule, each shard's velocity/spin/gravity/
lifetime/fade [M]); other kinds → `Drone_BrokenObject` + `Debris_Create_Loop` (the debris object set, counts, launch
velocities, lifetimes [M], the debris models [D]). Deliver a sim-side list the renderer can draw each tick: per piece its
source (triangle verts for shards, model hash for debris), its world matrix and alpha, created at the kill and expired on the
game's timer, deterministic (no shared-RNG draws the game doesn't make; cite every Rand_Rand). Tests: a shot vase spawns the
game's debris count with the game's launch numbers; a shot window spawns one shard per source triangle (sister: an unshot
breakable spawns nothing). The main agent draws them and compares with PCSX2.

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

**Branch:** `s/break-debris-shatter`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
