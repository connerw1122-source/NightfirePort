## Your task: `fix-input`, gamepad support, the 8 controller styles, and the aim ramps

The user plays with keyboard and mouse, and doesn't like tasks that need a controller. But the game is a pad game, and the port should support one. Open items: GAPS G-003 and G-005, and `docs/systems/input.md`.

1. **The 8 controller styles** (G-005): generate the full style × action table mechanically from `psiInput_MapInputs`, and store it as data in nf-game, with a test that compares it against the decompile's table in the EE snapshot ([L]).
2. **The aim and turn ramps** (G-003): `Player_Aiming` state +0x8f4/+0x8f8/+0x900, how the stick's turn speed ramps up and the aim-mode slowdown, and the dead zone (G-012: the game overwrites the raw analog bytes in place, so read the code between `scePadRead` and the dead-zone loop).
3. **A gamepad backend in nf-view:** map an XInput/DualShock-style pad onto the PS2 pad bits and analog bytes, then run the game's own mapping and curves on them, so pad play is the game's. If this needs a crate (e.g. `gilrs`), name it, its licence and size in your report, and put it behind a cargo feature `gamepad` so master builds without it until the user approves. Keyboard and mouse stay the default and must not change.
4. **Mouse look stays as it is.** Don't touch its feel; the user tuned it.

Tests: style tables vs the snapshot; a synthetic stick input through the ramp gives the decompile's turn speeds tick by tick, with a sister assertion; keyboard input still produces the exact same `Input` as before (a regression test over a recorded trace).

Files: `crates/nf-game/src/player.rs` and `input` modules, `crates/nf-view/src/lib.rs` (the backend, minimal), `docs/systems/input.md`; update G-003, G-005 and G-012.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-input`.

---
claimed-by: sandbox6 at 2026-10-13T13:41Z
