## Your task: `script-commands`, the script player remainder: fades, sprites, lights, text, letterbox

GAPS G-031: the cut-scene/script player stops these streams today: fade, sprite, light, sub-script and text commands; the
letterbox borders and music events of camera scripts; `SP_Hit`'s bullet and object flags (07000007's urn break, mask 2); AI
volumes / pickups / breakables made by scripts; `Corona_2` glow drawing; drones riding objects.

Port each command from `Script_EventHandler` 0x1d94c0 and `Script_Interp` (docs/systems/script-player.md), in this order:
fades and letterbox (visible in every cut-scene), text, lights, sprites, sub-scripts, `SP_Hit` (the urn), then the rest.
For each: a test that runs a real script containing it (find one per command with nf-extract's script dump) and asserts
the command's effect in the Sim's state.

Files: `crates/nf-game/src/nis.rs`, the cut-scene draw hooks in nf-view, docs/systems/script-player.md.

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

**Branch:** `s/script-commands`.

---
claimed-by: sandbox1 at 2026-10-13T18:37Z
