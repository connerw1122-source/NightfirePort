## Your task: `exact-sine-sites-1`, swap the game-exact trig sites (bullets, pickups, world spheres)

G-046 (docs/GAPS.md; survey in docs/reports/fix-exact-sine-flip.md) lists the port's remaining host-trig call sites
where the game runs `PS2Sinf`/`PS2Sinf2`/`PS2Sinf3` (the bit-exact versions already exist: `nf_formats::anim::sinf`,
`nf_game::trig`). This task takes three groups:

1. `crates/nf-game/src/bullet.rs` ~:57 — the spread (`Bullet_init` 0x12c5a0:171 → `Vec_Spherical_2_Cartesian` 0x1e3948).
2. `crates/nf-game/src/pickup.rs` ~:248 — the pickup spin (`Pickup_Update` 0x18e148:193-201 → `RotMatrix`).
3. The world-sphere builds via `Control_BuildWorldSph` 0x133070 → `View_RotTransMatrix` 0x1e5168: player.rs ~:270 and
   ~:1490, wire.rs ~:60/~:206, ladder.rs ~:44.

For each site: read the decompile and match its exact arithmetic order (which sine variant, the operand order of the
products and sums: the EE FPU is not IEEE-fused). Swap it, and write a fail-first unit or NF_ISO test that compares
the port's value bit-for-bit with the game's formula on inputs where host and exact differ (`exact_vs_host_probe` shows
how to find such inputs). Show each test failing on the old code first.

Line numbers are from the survey; grep, because they drift. Don't touch the movement axes, ladder snap, creep, wire hand,
missile steering or head-track sites: those are other tasks.

Files: `bullet.rs`, `pickup.rs`, `player.rs` (the two sphere sites only), `wire.rs`, `ladder.rs`, the new tests,
`docs/GAPS.md` (G-046: strike the done sites).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).


**Branch:** `s/exact-sine-sites-1`.
