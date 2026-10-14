# fix-grapple-hook — the grapple point's pulley-with-hook is drawn

Branch `s/fix-grapple-hook` (from master 69c0104, merged with origin/master 966671a = fix-08-walls).
The playtest (07000005 at `--bond 16.5,29.6,58.6,-2.01,0.24`, main agent 2026-10-13): the real game
shows a **pulley with a hook** hanging from the T-shaped bracket on the castle wall — the grapple
point the Q-Claw latches onto (placement #282 at (13.75, 32.29, 57.3), class 'Q' 0x51 per
fix-qclaw). The port drew the bracket only: the hook/pulley mesh was missing.

## Root cause — fix-gadgets2's "the game discards the referenced object" was wrong for 0x3A

fix-gadgets2 removed the 0x3A marker draw citing `Grapple_Create`'s zero
`hashtable_set_object_to_entity_gfx` calls. Those calls are irrelevant: **the glist comes in as an
argument**, not from a hash attach. The full draw chain:

1. The dynamic-block parser resolves the placement's object and hands it to every dynamic creator
   as the glist argument: `parsemap_block_map_data_dynamic` 0x1d1b68 — `iVar5 =
   hashtable_getitem(hashcode)`, or `pCelList + index*0x50` when the hashcode is −1 (07000005's
   grapple placements carry hashcode 0xFFFFFFFF, so the index branch) [M]. Only a failed
   resolution (index ≥ MapEntityCount, glist 0) skips the create call — never the case here.
2. Case 0x3a passes it straight through: `parsemap_create_dynamic_objects` 0x1d04f0:237-239 →
   `Grapple_Create(pos, quat, level_tag, glist)` [M].
3. `Grapple_Create` 0x198298:24-30 stores it: `obj+0xD8 = glist` (the resolved
   `graple_pulley` entity), `obj+0xF0 = *(u32*)(glist+0x44)` (the draw-flags word,
   docs/fields/glist.md), class 0x51, `build_LinkToRoom(obj, 0, glb_world)` +
   `Controls_StraddleTest` [M].
4. The generic dynamic-object draw renders it: `View_DrawObjList` 0x1e69a0 (entry gates on
   obj+0xFA: 0x8 anim / 0x10 particles / 0x100 emitter — all clear at create) →
   `View_DrawObjectGlist` 0x1e6780: `psiDrawObjectMatrix(obj+0xD8, obj+0x90 matrix, obj+0xEC)` —
   obj+0x90 is the placement transform `control_create_object` 0x1344e0 built [M].
5. `psiDrawObjectMatrix` 0x124708 walks glist+4 → +8 → +4 into `RecurseAndDrawBoxes` [M].

The admission gates all pass for the pulley, live [L ee_start_05.bin, file offset = EE address]:

- Both class-0x51 objects exist at level start: obj 0x16d80a0 at (13.752, 32.292, 57.303) =
  placement #282, obj 0x16d81c0 at (−3.396, 42.940, −71.769) = #281 (walked the dynamic-object
  list, head 0x2705a0, `control_add_object_to_list` 0x134698's +0x14 links).
- `obj+0xD8 = 0x170d6a0` — the `graple_pulley` entity — for both; `obj+0xF0 = 0x4000` (the glist's
  +0x44 word: no 0x10 "not drawn" bit, no 0x8000 forced bit — the point did not straddle cels, so
  `View_AddObjects`' per-cel path draws it); `obj+0xF8 = 0x1f` (bit 0 passes
  `View_CaptureScene` 0x1e6df0's per-frame `object_display_mask = 1 << viewer_LOD`, global
  0x30d87c — the snapshot's stale 0x80 is another viewer's LOD 7); `psiDraw` chain glist+4
  0x16facd0 → +8 0x9b32a0 → boxes **0x9b3260 — DRAWABLE**.
- The same walk shows the **creep walls** (class 0x2B, obj 0x16d69a0 / 0x16e8d90) chaining to
  **boxes 0x0 — EMPTY**: fix-gadgets2's conclusion was correct for 0x3F and stays in force.
- [D] PCSX2 playtest: the pulley is visible at the spot.
- [D] placements 281/282 resolve to `graple_pulley`, gfx ±(0.169, 0.591, 0.183), 212 gfx verts,
  object flags 0x4000 == the live obj+0xF0.

**Latch state**: nothing to model. Class 0x51 has no `control_funcs` row (all zero [L
0x26fa94..0x26fc68: rows 0x2b..0x50 span 0x26fa94..0x26fc5c; row 0x51 is 0x26fc5c..0x26fc68, docs/atlas/wire-grapple.md]) and no grapple-path writer touches the object's
gfx — `Player_SSGrapple` 0x192860 / `Player_SetGrapplePoint` 0x195da0 / `Bullet_Delete` 0x12e660
only read obj+0x30/+0x80 and store BLData hooks. The pulley is a static mesh at the placement
transform, latched or not.

## What's done

- `crates/nf-view/src/scene.rs` (shared file — additive, listed below): a World-layer sweep over
  the main map's 0x3A placements inside the `cels_only` block, next to the misc sweep — the same
  shape as the wire 0x3E / ladder 0x22 sweeps: instance the resolved object at
  `object_matrix(p.rotation, p.translation)` when its gfx has batches (the `psiDrawObjectMatrix`
  empty-chain no-op) and it is not mirror-only (0x400); `NF_HIDE` honoured. A
  `grapple_hooks_drawn` counter next to `ladders_drawn` (struct field on its own line). The stale
  fix-gadgets2 comment corrected in place, with the full evidence chain and the [L] addresses.
- `crates/nf-view/tests/models_misc_live.rs`: comment/message-only updates where it cited
  "[M 0x198298: no gfx attach] never draw" — the assertions themselves are unchanged and still
  pass (the misc sweep still draws no 0x3a; the pulley draws via the new counter).
- The cels_only gate matters: `instances(false)` includes 0x3A placements, so `--all` (debug view)
  already drew them through the generic loop — the sweep must not double-draw.

## Tests (all through `nf_view::load`, NF_ISO headless)

New `crates/nf-view/tests/grapple_hook_live.rs`:

- `the_pulley_draws_at_both_07000005_grapple_points` — level-data assertions (placements 281/282
  resolve to `graple_pulley`, 212 gfx verts, flags 0x4000 [L-confirmed]) + `grapple_hooks_drawn ==
  2` + the pulley's exact 212 vertices inside its world box at **both** #282 and #281 — the box
  holds nothing else, so the exact count is also the no-double-draw guard.
- `one_hook_per_grapple_point_on_every_level` — per level `grapple_hooks_drawn` equals the disc's
  0x3A placement count [D nf-extract placements] on **all 16 levels that have grapple points**:
  07000005 (2), 0a (1), 0c (5), 0d (3), 11 (1), 12 (1), 14 (3), 23 (2), 24 (7), 25 (4), 26 (4),
  27 (8), 28 (4), 29 (3), 41 (1), 4b (8) — 54 points total. The list: story levels 05/0a/0c/0d/
  11/12/14/41, the MP arena 4b, and the driving-mission archives 23–29 (the sweep is level-generic,
  so they draw wherever the port loads them).
- `the_creep_walls_stay_undrawn` (sister) — 07000005's creep-wall objects parse **0 batches**
  (matching the live empty glists [L]), nothing may draw for them, and the grapple sweep next to
  it still draws 2.

Failing-first: a scratch probe (`probe_grapple_symptom.rs`, deleted) on the pre-fix tree counted
**0** scene vertices in the pulley's box at #282 and failed exactly there; the fixed tree has 212.

## Verification

- `grapple_hook_live` 3/3 green on the branch and again on the master-merged tree;
  `models_misc_live` 10/10, `fix_ladder2_live` 4/4, `fix_objects_live` 6/6 (the neighbours that
  assert scene counts/misc census).
- Baseline full suite on the pre-fix tree: **952 passed, 3 failed (the allowed disc_golden
  slim-ISO set), 29 ignored**. Final suite on the master-merged tree: see the WORKLOG tail line
  (run in the background; result recorded before the finish push).
- No GPU in this sandbox (wgpu: "No suitable graphics adapter found") — the `--shot` renders are
  the main agent's, commands below. The headless drawn-set proof is the 212-vertex box assertion
  plus the instance counter.
- Independent audit (subagent, 2026-10-14): PASS. It re-verified the decompile claims and the [L]
  walk (both 0x51 objects, glist 0x170d6a0, boxes 0x9b3260 vs the creep walls' 0x0), confirmed
  `add_instance` is RNG-free and order-deterministic, that the scene.rs diff removes only the stale
  comment, and that no other test asserts total instance/vertex counts. It strengthened the
  per-level claim: **every 0x3A placement on all 16 levels** carries neither 0x8000/0x2000 (the
  dynamic/static skip bits) nor glist bits 0x8/0x4 (the anim/jitter matrix paths), so the
  static-at-placement draw is the game's draw everywhere; and every grapple object's flags lack the
  0x800 dim bit (0700000a's SkyBridge point carries 0x84000 — bit 0x80000 is only the
  viewer-proximity tweak gate in `View_SetupRenderModes_Tweak` 0x1e5888, active within 4 units
  behind a global gate, which the port's other drawn objects treat the same way).
- Citation fix from the audit: the control_funcs zero-row proof for class 0x51 is
  [L 0x26fa94..0x26fc68] — rows 0x2b..0x50 span 0x26fa94..0x26fc5c and row 0x51 is
  0x26fc5c..0x26fc68 (the shorter range, inherited from docs/atlas/wire-grapple.md:7, was one row
  short; that doc still carries the old range).

## Shared-file hooks (the driver's list: drone.rs, scene.rs, player.rs, lib.rs)

- `scene.rs`: the 0x3A sweep (13 lines) inside the `cels_only` block, the `grapple_hooks_drawn`
  struct field + init + constructor entry, the corrected fix-gadgets2 comment block. Nothing
  removed; no existing arm/constant renamed.
- `models_misc_live.rs` (test file): comment/message text only.
- `drone.rs` / `player.rs` / `lib.rs` / `nf-formats/*`: untouched.

## What's left

- Nothing for the mesh. Open observations from the walk (not this task's): the 07000005 wire
  (class 0x2C) object's glist is also DRAWABLE live and the port already draws 0x3E placements
  (scene.rs's wire sweep) — consistent; the rope/flying-claw visuals of the fired Q-Claw remain
  the fix-qclaw report's open item.

## Live checks for the main agent

PCSX2 (07000005):

1. Spot `--bond 16.5,29.6,58.6,-2.01,0.24` (the playtest spot): the pulley with the hook hangs
   from the T-bracket on the castle wall — the port's render must now show it. Expected: a small
   pulley (~0.34 × 1.18 × 0.37 units) at (13.75, 32.29, 57.30), drawn once, no double mesh.
2. Poke/verify no hidden state: fire the Q-Claw (`--give 0x50` draws it; 05 owns it at start) at
   #282 and latch — the pulley must NOT move or change mesh (class 0x51 has no update function
   [L 0x26fa94..0x26fc68: rows 0x2b..0x50 span 0x26fa94..0x26fc5c; row 0x51 is 0x26fc5c..0x26fc68]); the rope is the known fix-qclaw open item, out of scope here.
3. Second point on 05 at `--bond -3.4,41.5,-69.0,-2.6,-0.3` (near placement #281 at
   (−3.40, 42.94, −71.77), yaw π): the same pulley visible on its wall bracket.
4. The creep walls must still be invisible at the wall-shimmy spots (fix-gadgets2's behaviour
   unchanged): no red/yellow boxes at 07000005's shimmy walls.

nf-view render checks (`--shot`, on this branch — before/after = master vs branch):

```
nf-view Nightfire-onfoot.iso 07000005 --bond 16.5,29.6,58.6,-2.01,0.24 --skip-cutscenes --frame 30 --shot after_hook_282.png
nf-view Nightfire-onfoot.iso 07000005 --bond -3.4,41.5,-69.0,-2.6,-0.3 --skip-cutscenes --frame 30 --shot after_hook_281.png
nf-view Nightfire-onfoot.iso 0700000c --bond 63.0,13.5,-141.0,2.6,-0.1 --skip-cutscenes --frame 30 --shot after_hook_0c.png
```

- after_hook_282/281: the pulley visible on the bracket (master's renders show the bracket only).
- after_hook_0c: 0700000c carries 5 points [D]; this spot faces placement #308 at
  (61.13, 12.28, −143.48) [D placements] from ~3.5 units — one pulley on its bracket.
