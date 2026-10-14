## Your task: `weapon-matrix`, every weapon on every level: the check and the report

CONTEXT (the user, 2026-10-13): "need heavy work on getting weapons to work no matter what level, with sounds and
proper animations and particles." The dev tools (the dev window / F2 menu, `--give id,id`) hand Bond ANY weapon on ANY
level. Today `scene.rs` builds a first-person view model only when the weapon's first-person skin (`weapon_data+0xDC`),
its sleeve and its parts are in the CURRENT level's archive (`level.skins.get(&fp)`, scene.rs "The first-person gun
models"), and the same holds for its view-model animation scripts, its muzzle-flash mesh (`weapon+0x48`), its casing,
its projectile and its sounds (the level's own sound bank). So a weapon the level doesn't natively carry is invisible,
silent, unanimated or fires the wrong effects (the user saw the grapple fire a bullet impact, the laser and stunner fire
"bullets", no Q-Claw model). The real game only ever loads a level's own set; the port goes further on purpose, so that
every weapon behaves exactly as it does on a level that carries it.

The four 100x tasks split the work: 100a the assets and animations (the foundation), 100b the sounds, 100c the fire
behaviour and effects, 100d the all-weapons x all-levels check. They run in parallel: pull `master` often, and if
100a's branch (`s/weapon-assets`) isn't merged yet, build on it (`git merge origin/s/weapon-assets`) and say so in
the report. Keep the per-level behaviour bit-identical for weapons the level carries (the existing tests stay green).

YOUR PART: the matrix that proves it, and the fixes it turns up that the other three don't own. A headless test binary
(and an `nf-extract weapon-matrix <iso>` command that prints the table) that, for every on-foot level x every weapon id:
loads the level, gives the weapon (the `--give` path), draws it, fires it 3 times (holding for hold-to-use gadgets),
reloads, switches away and back, and records: view model present / its sequences played / muzzle flash / sound ids
requested and started / effects spawned / errors or panics. Write a compact table (levels as rows, weapon groups as
columns, a mark per cell) to `docs/reports/weapon-matrix.md`, numbers only. Run it on master at the start (the
baseline) and again at the end with the 100a-c branches merged; every red cell at the end gets a reason or a fix. Any
panic is a bug to fix. Also add a "Test weapon" row to the dev menu's Weapons page (dev.rs; it shows in the dev window)
that runs the same sequence live for the selected weapon (draw, 3 shots, reload) so the user can watch it.

Files: a new `crates/nf-view/tests/weapon_matrix_live.rs`, `crates/nf-extract/src/main.rs` (the command),
`docs/reports/weapon-matrix.md`, `crates/nf-view/src/dev.rs` (the row).

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

**Branch:** `s/weapon-matrix`.
