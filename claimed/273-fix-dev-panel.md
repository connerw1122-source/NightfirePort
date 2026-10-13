## Your task: `dev-panel`, the dev panel: label and spawn exactly what it says

The user: "the dev panel needs work on what it actually spawns. a frag grenade spawns what looks to be a stun grenade that
explodes like a frag grenade. I cant tell whats what. no clue what the rocket launcher is". The dev panel is the user's main
test tool, so it must be exact.

1. Every weapon/gadget/pickup entry in the panel shows the game's own name (USATxt.dat via `Lang`, the weapon's name id)
   and the weapon id, and spawns exactly that weapon_data record and its own mesh (`weapon_data+0x5C` projectile body,
   the held mesh +0x7C). Test: for every entry, the spawned weapon id, mesh hash and projectile hash match the record.
2. Add a "level kit" button that gives exactly the loadout and gadgets the game gives on the current level
   (`docs/systems/levels.md` loadouts) so a playtest starts like the real game.
3. Show the current weapon's fire mode and ammo type in the panel.

Files: `crates/nf-view/src/dev.rs`, the panel draw, nf-game weapon tables (read-only).

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

**Branch:** `s/dev-panel`.

---
claimed-by: sandbox1 at 2026-10-13T17:22Z
