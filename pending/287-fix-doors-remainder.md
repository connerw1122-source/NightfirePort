## Your task: `doors-remainder`, doors: the use test against the mesh, the labels, the proxy path

GAPS G-035's remainder: the use sphere vs the collision mesh (the port uses a bounding-sphere stand-in for
`Player_Activate`'s pick against the door), the door text labels 0x2000003/0x200000b (the "locked" messages), and the exact
proxy-list path of `Door_ReactToHit` (from `Collide_Update`, not `Door_Update`). Census first: for every door on the 27
levels, can a player standing at the door's approach open it with E (the game's pick), and does a locked door show its
label? Then port what fails. Don't touch the door->portal links (the portal work owns those).

Files: `crates/nf-game/src/door.rs`, `crates/nf-view/src/scene.rs` (the pick, minimal), the HUD label hook.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/doors-remainder`.
