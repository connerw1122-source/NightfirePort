## Your task: `gadget-carry-map`, which levels carry the tripbomb, satchel and other thrown gadgets

The main agent compares the port's visuals with PCSX2 and may only give itself a weapon in PCSX2 on a level that
already carries it (the level's own models, textures and sounds must be loaded). It needs to know, for every on-foot
level, which of these the level carries: Laser Tripbomb 0x3a, Satchel Charge 0x3b, Remote Mine 0x37, Frag 0x35,
Smoke/Flash grenades, the Micro-Camera 0x54, Q-Decryptor, Q-Worm, and the Scorpion 0x2e.

Do (non-visual, data only):
1. From the decompile, list where a level gets a weapon: `Player_InitWeapon` 0x1a7ef8's per-level cases, the
   pickups placed in each level archive (pickup placements, their weapon ids), drone drop weapons, and anything
   the level scripts give. Write the sources and addresses in `docs/playtest/gadget-carry.md`.
2. Produce the table: level id → the ids above it carries (start loadout / pickup at x,y,z / drop), plus whether
   the weapon's view-model and projectile meshes resolve in that level archive (the port's `level.by_hash`).
3. For the tripbomb and the satchel: on the best level, a `--bond x,y,z,yaw,pitch` spot standing 1–2 m from a flat
   wall with clear space in front (checked headlessly: grounded after 60 ticks, a wall hit within 2 m ahead).
4. Test (NF_ISO, `crates/nf-view/tests/gadget_carry_live.rs`): the table's rows for at least 3 levels (loadout
   equals the port's `Player_InitWeapon` result; the pickup ids exist in the archive); the wall spots hold. Sister
   assertion: a level that doesn't carry the tripbomb reports it absent.

Files: `docs/playtest/gadget-carry.md` (new), `crates/nf-view/tests/gadget_carry_live.rs` (new). No game code changes.

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


**Branch:** `s/gadget-carry-map`.

---
claimed-by: sandbox6 at 2026-10-15T10:52Z

---
## Report (done, 2026-10-15T12:28Z)

