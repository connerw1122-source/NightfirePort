## Your task: `fix-final-blockers`, the two game bugs walkreal-c reproduced on the final levels

`s/walkreal-c` (merged into master; `docs/reports/walkreal-c.md`) walked 07000015, 07000016, 0700001b and 0700004a
with the real input path and parked each walk test (`#[ignore]`) at its blocker. Two are port bugs:

1. **07000016 — door[7] never opens.** Placement 183 at (4.3, −0.1, −74.2), kind 2 (slide), flags 0x3, locked_flag 0,
   approach 0, ch_lock 0: everything permissive. Bond's pitched E reaches it (use sphere d² 3.36 < 6.25 from head
   (3.5, 2.52, −73.58)) and `Doors::activate`'s guards all pass, yet the leaf never moves: 14 `use_door` calls over
   840 ticks, state stays 0x2. Notable: the door sits 1.0 BELOW the approach floor (the corridor descends at the
   threshold); door[0] on 07000015 (same kind, same-height approach) opens fine. Find why against `Door_Activate`
   0x135be0 / `Door_Update` 0x1363c8 (the swing-zone / side test, the busy marker, the kind-2 slide path).
2. **0700004a — the lift loses its rider mid-shaft.** The platform (script entity 0) descends continuously (−45.8 →
   −63.3 over t1600-2180), but Bond parks at −54.14 from t1620 (capsule_push (0,0,0), no floor within 0.5, grounded
   true: held by something static) exactly as the −52 doors [110]/[111] start their ch-0x40 open. The game's timing
   shows the platform passes them: it reaches −52 at t1560 and ch 0x40 opens the doors at t1610 [D 0x06000571 keys;
   #110/111 ch_lock 0x40]. Fix the door-solid vs rider interaction in the port's moving set (`script_parts` puts the
   platform entity and the door leaves in one dynamic set) against `Collide_Update` and the riding inherit.

Un-ignore `walkreal_07000016_live` / `walkreal_0700004a_live` as each lands and walk them further; if a new blocker
appears, park the test there with the evidence (the same pattern).

Tests (NF_ISO, fail-first): door[7] opens on Bond's E from the corridor; the lift carries Bond from the top lobby to
the bottom without parking; the sisters (door[0] on 15 still opens, a static floor still holds Bond).

Files: `crates/nf-game/src/door.rs`, `crates/nf-game/src/world.rs`, the riding inherit in `player.rs`/`scene.rs`, the walk tests.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  and `cargo build --release -p nf-extract`. On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/fix-final-blockers`.

---
claimed-by: sandbox5 at 2026-10-13T10:16Z
