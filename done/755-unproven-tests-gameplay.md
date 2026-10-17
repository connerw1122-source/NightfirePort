**Base:** merge `origin/master` first and check it contains `crates/nf-view/tests/briefing_plays_live.rs` (the
front-end audit + test fixes landed with it, 2026-10-16 evening). If it does not, fetch again before you start: the old
master still carried 28 tests that contradicted the decompile.

## Your task: `unproven-tests-gameplay`, settle the gameplay tests' unproven numbers

Every row gets evidence (a decompile address that says it, disc bytes, or a documented PCSX2 measurement) and a tag,
or `// [S] unverified` and out of pass/fail. A contradiction fixes the test.

Rows (2026-10-16 audit; find them by behaviour if lines moved):
- fix_hud_live.rs ~89-97: bar fade = last 15, 16 px/tick close; the pane field piVar4[3] in HUD_UpdateStatusPane 0x19c310:37.
- hud_playtest_live.rs ~108: crouch icon centred at (48,48); the crouch sprite's flags (0x800 = centred) at 0x19b9c8:310.
- hud_playtest_live.rs ~127-154: 16:9 pickup-bar centring is a port choice; tag [S].
- loading.rs / async_load_frame_loop.rs `dots_lit`: a step every 11 vblanks [M VBlankInt 0x124088], but VBlankInt is not in the decomp export; export it (tools/ghidra) and cite the line.
- models_misc_live.rs ~338-345, fix_weapon_effects_live.rs ~196/205: muzzle-smoke life 8 / 7 ticks; Gas_Init's ushort argument in the 0x1a4298 disassembly.
- fix_laser_watch_live.rs ~149-174: drain 31+-2 per 60 ticks, idle refill; the charge writer in 0x1a5460 state 0xb.
- fix_context_hints_live.rs ~177/196: prompt window 6.0; the distance gate in the ThirdIcon_Update 0x196e80 chain.
- fix_satchel_tripbomb_live.rs ~235-237: the 7th bomb removes the oldest; the meaning of obj+0xEC in 0x12ce80:347-368.
- drive_hud.rs ~99, driving_ai_physics.rs ~309-310: countdown floors at 0, no reverse runaway; DRIVING.ELF's timer clamp and reverse logic.
- walk_07000005_live.rs ~66: channel 0x45 set by hand; find 07000005's mission-init writer and drive it instead.
- fps_determinism_live.rs ~76: the 4-tick catch-up cap; the game runs at most 1 tick per loop at MUL=2 (0x109080:76-78), so label it a port choice.
- subtitles_live.rs ~218-223: subtitle starts at 1 step per tick; Script_Update 0x1da998:33 adds FRAME_RATE_MUL (2 under FMV), so check parity and fix if the port steps 1.

Report a table: row, evidence, change.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/unproven-tests-gameplay`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T06:26Z

---
## Report (done, 2026-10-17T08:22Z)

