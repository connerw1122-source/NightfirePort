## Your task: `blood-puff-render` — the blood puff is in the pool but nothing draws on screen

The playtest report "no red puffs" when shooting guards is NOT fixed. `flesh-hit-effects` gave the mesh 0x0200_08bc
gas draw slots. Its test checks that the slots exist, but nobody rendered the result. The main agent rendered it
(2026-10-16, master a4156da0) and nothing shows.

**Repro (headless, deterministic):**

    nf-view <iso> 07000005 --bond 96.9,17.9,98.8,0.53,-0.24 --pull 236 --frame 240 --shot out.png

- The shot hits guard #6 (health 10 → 4.75), and his hit reaction shows in the render.
- `drones.gas.puffs` holds one 0x0200_08bc puff at pool index 1: pos (99.13, 16.96, 101.65), scale 3.0 → 3.2,
  rgba 255 → 238, life 60, kind 5.
- The gas branch of the scene matrix code DOES run for that slot with those values. A debug print in the
  `if let Some((gi, gmesh)) = a.gas` branch confirmed it.
- Nothing red appears anywhere, even with the matrix scale ×10 AND the position forced to head + look·2, right in
  front of the camera.

So the slot's draw path renders nothing for this mesh. Find out why. Candidates, check each and measure:
- the object `level.by_hash[0x0200_08bc]` resolves to: its geometry (vertex count, bounds), its material and
  texture (alpha, blend mode), and the texture's real pixels [D];
- whether `add_instance` for the slot emitted any vertices into `groups` at all (Layer::Effect; the cel/room
  arguments passed are `&room_cels, u32::MAX, u32::MAX`), and whether a cel/portal or layer filter drops it;
- how `gas_rgba` and the effect layer's blend apply (does the vertex alpha or colour end up 0?);
- whether the other pool meshes draw: SMOKE_QUAD 0x0200_0039 (e.g. a snow hit's floor puff), 0x0200_001f and
  0x0200_026e. If none of them draw either, the bug is shared.

Fix it so the blood puff draws the way the game draws it [M View_DrawObjectGlist 0x1e6780's obj+0xF0 & 8
billboard; the gas object's colour bytes]. Add a test that fails when the puff's draw contributes no vertices or
pixels. Use whatever the scene exposes: the posed vertex list or the draw groups for that slot with non-zero
alpha, at the puff position. Write a render-side assertion, not only a pool check. Include before/after `--shot`
PNG paths in your report (the main agent will look at them). Do not commit images.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/blood-puff-render`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T17:40Z

---
## Report (done, 2026-10-16T18:53Z)

Closed by the driver: the report is on `s/blood-puff-render` (docs/reports/blood-puff-render.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T19:13Z)

