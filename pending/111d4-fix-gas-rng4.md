## Your task: `fix-gas-rng4` — land 111d3's deferred impact draws on top of master's HITDATA walk

`s/fix-gas-rng3` (111d3, tip fbf62779) was HELD on 2026-10-15. It does not merge with master: a code conflict in
`crates/nf-game/src/drone.rs` (Bond's bullet world-hit block, about line 6530) and `crates/nf-game/src/effect.rs`
(a test's `bullet_impact` call, about line 385).
- Master has G-041: the player bullet walks the trace's HITDATA LIST inline. Every front-face hit takes the
  penetration roll `Rand_Rand(0x3f)` [M 0x12cfd0..0x12cfe4] and then the full effect stack, and a pierce retraces
  past the surface. `bullet_impact` also gained `&mut self.fx` and an extra trailing argument.
- 111d3 defers the impact effects' shared-RNG draws to a flush after the guard control loop (the collide pass,
  `Collide_Update` 0x1ea398 [M control_movement_object_handler 0x1336f8:238]) through `pending_world_impacts`, and
  moves the explode gate and the ricochet re-aim into that flush.

Both are about RNG draw ORDER, so a mechanical merge is wrong. Start from `origin/master` and port 111d3's
deferral onto the HITDATA walk:
- Decide from the disassembly where the penetration roll falls relative to the effect draws, given that both run
  inside `Bullet_CollisionHandler` during the collide pass.
- Decide whether a pierce's exit-side hits defer too.
- Make the flush keep the per-hit order.
Keep every G-041 test and every 111d3 test green, and re-run the cover/fire-rate AI tests the first gas-rng hold
broke. Report the draw-order table (site → draws → pass) and before/after tallies.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/fix-gas-rng4`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
