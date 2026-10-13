## Your task: `weapon-modes`, weapon fire modes, the mode switch, and per-weapon sounds

From the playtests: "the scorpion launcher heat seeker mode doesn't work. I don't know how to change weapon modes"; "the
pistol you start with does not sound right"; "I picked up the briefcase gadget and it wouldn't let me switch to it".

1. **Fire modes:** every weapon's alternate modes (weapon_data fields; the mode-cycle action in `psiInput_MapInputs`; the
   HUD fire-mode label already exists). Port the mode switch on the game's own button (keyboard binding: document it in
   the HUD help) and each mode's behaviour: the launcher's heat-seeker lock and homing (`Bullet_update`'s homing branch),
   the burst/semi/auto modes, the silencer toggle.
2. **Fire sounds:** each weapon's first-person fire sound id (the op-1 per-weapon remap, GAPS G-032; G-029's "each weapon's
   first-person fire sound id"). Test: the id table for every weapon against the decompile, and the P2K's real id.
3. **Gadget switching:** every gadget Bond can hold on a level can be selected and used (the briefcase on 07000008, the
   Q-Worm on 0700000a, the camera on 0700000c). A test per gadget: pick up → select → use → the game's effect.

Files: `crates/nf-game/src/{player,bullet}.rs` (weapons), the weapon-data tables, nf-view's HUD/input bindings.

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

**Branch:** `s/weapon-modes`.

---
claimed-by: sandbox1 at 2026-10-13T16:29Z

---
## Report (done, 2026-10-13T17:22Z)

Closed by the driver: the report is on `s/fix-weapon-modes` (docs/reports/fix-weapon-modes.md); the worker didn't run finish.sh.

---
## Report (done, 2026-10-13T17:23Z)

