## Your task: `character-room-light`, characters lit by the room they stand in

The user's playtest on 07000007 ("characters lit by the room"): **no**. The port multiplies 0x800 objects by tint x cel
ambient >> 8 (render-port-check.md 1.1), but the user still sees the party's characters lit wrongly. Also G-042: the
07000002 bathhouse civilian's skin texels. Find which cel ambient a character uses in the game (the anim object's cel,
`obj+0x20`, set by `control_link_object_to_cel` 0x1346b8, vs the port's lookup), whether dynamic lights (`Light_Create`, the
muzzle light) add to characters, and the per-character ambient multiplier. Write render spots (07000007 ballroom guests,
07000002 bathhouse) and a precise live-check request: the main agent captures PCSX2 at those spots with
`tools/trace/pcsx2_shot.py --pos ... --pitch ... --god`.

Files: the character lighting in `crates/nf-view/src/scene.rs`, `docs/systems/render.md`.

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

**Branch:** `s/character-room-light`.
