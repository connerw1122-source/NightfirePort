## Your task: `fix-wire`, crossing the wire and the zipline (07000005's left route)

Source of truth: `docs/atlas/wire-grapple.md`.

What it established:
- The wire is placement type 0x3E → class 0x2C, an 8-byte instance.
- The player substate is `obj+0xF6`: 6 = `Player_Wire` (hand over hand), 0xF = `Player_Zipline`, 2 = `Player_SSGrapple`.
- `crates/nf-game/src/wire.rs` has real field bugs: misnamed params, endpoints from the world sphere, no zipline, and the wrong substate (it says "creep"; wires enter 6, creep-walls 7).

User report: at the cable left of the castle gate, pressing E plays the falling-to-death animation instead of the hand-over-hand crossing.

Do this:
1. Fix `wire.rs` fields per the atlas.
2. Port `Player_Wire` (substate 6) whole from the decompile: attach on use, hand-over-hand movement with forward/back, the hanging pose and camera, dismount at both ends, and no fall or death check while attached.
3. Then port `Player_Zipline` (0xF) if the time allows.
4. NF_ISO test: Bond uses the 07000005 wire left of the gate, moves forward N ticks, travels along it, stays alive and dismounts at the far end.

Files: `crates/nf-game/src/{wire,player}.rs` and the player substate dispatch. Session `fix-player` also edits `player.rs`, so pull often and keep your changes in separate functions.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Also: 07000002 has a climbable wire too**, like 07000005's. Make sure your wire port works on both levels, and add an NF_ISO test for the 07000002 wire.
**Branch:** `s/fix-wire`.

---
claimed-by: sandbox2 at 2026-10-12T04:04Z

---
## Report (done, 2026-10-12T05:47Z)

