# fix-striped-boxes — the red/yellow striped box on 07000005 (and why 0c's are gone)

Task 102k. Branch `s/fix-striped-boxes` (from origin/master b65d6f7; origin/master merged at 30135f8).
Files: `crates/nf-view/src/scene.rs` (the fix), `crates/nf-view/tests/fix_striped_box_census.rs` (new),
`crates/nf-view/tests/probe_striped_box.rs` (kept probe).

## The symptom

`nf-view <iso> 07000005 --bond 16.67,30.04,58.60,1.755,-0.001 --skip-cutscenes --frame 30 --shot a.png`
with `NF_NOPORTAL=1` draws a box textured in bold red and yellow diagonal stripes on the trench's
right, against the castle wall. Stripes like that are an editor/marker texture the game never draws.

## The instance

Probe (`probe_striped_box.rs`, kept) decoded every map texture of 07000005 and 0700000c:

- **The marker texture**: 32×32, red 63.9% / yellow 33.7%, diagonal red/yellow bands — 07000005
  map 50 ("01000099") texture idx 15 (hashcode `ffffffff` — the disc stores no name for map
  textures). 0700000c carries the identical texture at map 9 idx 0 and map 45 idx 31 — **unused by
  any object** — which is exactly why the user's 0c check found no boxes after fix-gadgets2 removed
  the roof markers (those were the 0x3a/0x3f grapple/creep markers; the 0c stripe texture lost its
  last referents then).
- **Every object on 05 referencing it carries glist flags bit 0x10** (`MapObject.flags` =
  `celglist+0x44`): the twelve kind-0x19 cel placements `blocker1`, `Blocking_Right`,
  `Blocking_rear_left`, `Blocking_ramp_side`, `Blockingpolys_Tower_1`, `BlockingPoly_Gatehouse_1`,
  `BlockingPoly_EndStealthroute`, `BlockingPolys_Gate`, `blocker_sk`, `BLOCKINGPOLY_StepRamp`,
  `blocker_sk1`, `blocker_sk2`, `Blockingpolys_Gatehouse_1` (already skipped by the port's static
  loop via `skipped_by_cel_pass`); **`WallHug_Blocker1_Short`** (placement map 50 #324, idx 148,
  kind 0x3b, translation (24.75, 32.17, 65.60), object map 50 #148, hashcode `ffffffff`,
  by-index resolution) and **`WallHug_Blocker2_Windows`** (#325, kind 0x3b, (−35.04, 37.16,
  −31.65)); and **`TouchPlate_GrappleLedge`** (#255, kind 0xfa, object #121 flags 0x50,
  world-baked at (−18.46, 37.43, −72.49)).
- The user's box is **`WallHug_Blocker1_Short`**: drawn at (25.40, 32.70, 65.60) — object centre
  (0.65, 0.53, 0) at the placement — ~11 units from the camera, ~15° right of the view axis:
  the trench's right, against the castle wall.

## The game's rule (why the game never draws them)

A world object reaches the draw lists only through its glist-flags copy:

- **`View_AddObjects` 0x1e62d8:33** — submits a dynamic object only when
  `(obj+0xF0 & 0x8010) == 0` [M; disasm 0x1e6344-4c `bnel` → skip]. `obj+0xF0` is
  `Control_SetGList` 0x134960:12-14's copy of `celglist+0x44` (every creator of the affected kinds
  copies it: `DynamicObject_Create` 0x18dbf8:21-23, `ThirdIcon_Create` 0x197070 —
  `*(glist+0x44) | 0x1000` = 0x1050 live [L ee_start_05.bin 0x016d43e0] —, `Trigger_Create`
  0x1b1720:95, `Grapple_Create` 0x198298:24-30).
- **`View_AddCels` 0x1e6ba0 L29/L46** — the cel pass refuses the same bit straight off the glist
  (`*(glist+0x44) & 0x10`), twice: the world glist and the per-room glists. Already implemented in
  the port as `MapObject::skipped_by_cel_pass` (the doc's "blocking polygons, and skies").
- **Bit 0x10 is collision-only**: `Collide_Pick` tests only `+0xF0 & 0x30000000`, so the brush
  keeps blocking while nothing ever draws it. The other 0x10 readers agree (radar exclusion
  `MP_GetRadarObjects` 0x189840, no turret targeting `GT_Track` 0x17f9c8, hurt-only triggers
  `Trigger_Activate` 0x1b2318).
- Kind notes: **0xfa** = `ThirdIcon_Create` (the grapple-ledge gadget grantor — writes
  BLData+0x95F) and **0xeb** = `Trigger_Touch` 0x1b2aa8 → `Trigger_Create` (TouchPlate forces
  type 2; only type 10 clears the 0x10 bit) — both never drawable. **0x3f creep walls** "look"
  no-gfx only because their 0x50 glist fails this same gate — a correction to fix-gadgets2's
  no-attach read (the attach is identical to `Grapple_Create`).

## The fix

Additive gates in `crates/nf-view/src/scene.rs` — skip when the object's glist flags carry 0x10,
but only where the kind's creator actually copies `glist+0x44` into `obj+0xF0` (otherwise the
game's gate reads a clear word and the mesh draws):

1. the **models-misc sweep** (`MISC_KINDS`) — drew `WallHug_Blocker*` (0x3b, param 0 == 0) and
   `TouchPlate_GrappleLedge` (0xfa): the user's box. The gate applies to the flag-copying kinds
   (0x30, 0x34 via `Control_CreateObjEx` 0x1343c8:36, 0x35, 0x3b via `DynamicObject_Create`
   0x18dbf8:21-23, 0x71, 0xe1, 0xe3, 0xfa via `ThirdIcon_Create` 0x197070, 0xfd, 0x4a). NOT to
   0x2e `Lock_Create` 0x1ae1f8, 0xd5 `OneSided_Create` 0x1e7ce8, 0xd8 `RB_Create` 0x1d6bf0 (plain
   `control_create_object` + direct obj+0xD8 store, no obj+0xF0 write — the game draws even their
   0x10-flagged meshes); 0xda/0xfe unverified, left ungated;
2. the **0x3a grapple sweep** — `Grapple_Create` 0x198298:24-30 copies the flags; the pulley's
   glist carries 0x4000 [L ee_start_05.bin 0x16d80a0/0x16d81c0] and passes;
3. the **corona (0xfc) and wire (0x3e) draws** — `Corona_Create` 0x18da40-48 and `Wire_Create`
   0x198254-5c copy `*(glist+0x44)` to obj+0xF0, so `View_AddObjects`' gate applies (no 0x10 hits
   on the disc today; the census pins it).

**Ladders (0x22) stay ungated**: `Ladder_Create` 0x1966c0 stores only obj+0xD8 (0x19671c), never
obj+0xF0 — the game draws even a 0x10-flagged ladder glist. The port matches.

**LOAD_LEVEL triggers (0xe8) are the game-drawn exception — an earlier revision of this fix gated
them and was WRONG**: `Trigger_LoadLevelCreate` 0x1b2950 forces the trigger type to 10
(0x1b2998/a0 store 10 to placement+0x2c) and `Trigger_Create` 0x1b1720 clears the no-draw bit for
exactly that type (0x1b18c8-d4 `obj+0xF0 &= 0xffffffef`) — the game draws these meshes whatever
their glist flags (the four "Load Point"-family meshes on 01/03/12/13, and 07000005's castle gate
`Door_Gate8`). The gate was reverted after the audit caught it.

## Verification

- **Fail-first** (`fix_striped_box_census.rs::striped_marker_brushes_not_drawn_on_07000005` against
  the pre-fix tree): 4 World materials with the no-draw bit, 3 keyed on the marker texture —
  (25.40, 32.70, 65.60), (−18.46, 37.49, −72.49), (−35.04, 37.16, −31.65). The test asserts 0/0 and
  FAILED exactly there before the fix, passes after.
- **Post-fix**: 3/3 green —
  05: 0 no-draw-bit draws, 0 marker-texture draws, `grapple_hooks_drawn == 2` (the pulleys keep
  drawing — the user's playtest saw them), and the castle gate `Door_Gate8` still has a World
  material at its extent (the over-hide sister); 0c: 0/0 and `laptops` — a flags-0 object with a
  16×16 hazard texture that merely looks striped — still resolves through the static cel path and
  draws; **census**: all 33 on-foot FILES.BIN archives (driving missions 07000023-29 and the
  shared packs excluded, per docs/systems/levels.md §7) fully loaded through `nf_view::load`, and
  every level's no-draw-bit draw count EQUALS its game-drawn 0xe8 LOAD_LEVEL exception count — an
  equality that fails both on over-draw (extra 0x10 draws) and on over-hide (a gate eating the
  game-drawn meshes; it read 1/1 on 01/03/12/13 with the reverted tree, 0/1 with the wrong 0xe8
  gate) — and **0 marker-texture draws disc-wide** (2 archives skipped: they carry neither the
  texture nor a 0x10-flagged meshed object). Marker-texture identity is per-level and rule-based
  (32×32 red+yellow AND every referencing object carries 0x10), so legit red/yellow world textures
  (07000015's club decor) are not flagged.
- Full suite on the final merged tree: see `suite.out` (fast runner, `tools/test/suite.py`);
  total line at push time: **(see below — updated at finish)**.
- No GPU here: the census observes the built draw list (`Material.lflags` = `obj.flags & 0xffff`,
  `Material.key` = the batch's `(map, texture)`), which is the exact render input; the pixel-level
  confirmation is the main agent's render (spots below).

## Independent audit

A second agent re-verified every claim against the decompile C + capstone disasm of
`work/elf/ACTION.ELF` and caught one real regression in the first revision, which this branch
fixes:

- **REFUTED → fixed**: the 0xE8 gate over-hid four game-drawn meshes ("Load Point"
  07000001 map53#18, 07000003 map50#451, "New LevelLoadTrigger" 07000012 map38#669,
  "oTrigger Load level" 07000013 map41#72 — all gfx-true) because the type-10 bit-clear
  (0x1b18c8-d4) makes the game draw them. Reverted; the census equality now pins it both ways.
- **REFUTED → fixed**: my first fix cited an incomplete copiers list — `Wire_Create` 0x198254-5c
  and `Corona_Create` 0x18da40-48 DO copy `glist+0x44` to obj+0xF0, so those two sites are now
  gated (latent only: no 0x10-flagged wire/corona on the disc; the census pins 0).
- **Over-strict → narrowed**: the MISC gate excluded the non-copying kinds (0x2e/0xd5/0xd8 —
  `Lock_Create`/`OneSided_Create`/`RB_Create` never write obj+0xF0).
- **Confirmed**: `View_AddObjects` 0x1e62d8:33's `andi 0x8010` (0x1e6344-4c), `View_AddCels`
  0x1e6ba0 L29/L46's two `glist+0x44 & 0x10` refusals, `Control_SetGList` 0x134960:12-14's copy
  (`| obj+0xf0 & 0x30008000`), the pulley's 0x4000 passing, `Ladder_Create`'s no-copy, the
  fail-first arithmetic (exactly four flags-0x10 gfx-true sweep-reachable objects on 05), and the
  05/0c test runs green.
- Census hole noted by the audit (over-hide blindness) is closed by the equality assertion + the
  `Door_Gate8` material sister.

## Live checks for the main agent

PCSX2 (real game — the box should never be there; the point is confirming the port now matches):

1. **Spot** 07000005, walk the trench to ~(24.75, 32.17, 65.60) facing the castle wall
   (`--bond 16.67,30.04,58.60,1.755,-0.001` is the standing view): **expected** no striped box in
   the real game; Bond is still **blocked** by the invisible brush (collision survives —
   `Collide_Pick` has no 0x10 test [M]).
2. **Poke** (`tools/trace/fieldprobe.py`): find the 0x3b WallHug control object at that spot
   (class 0x2B/0x28 family from `DynamicObject_Create` 0x18dbf8), **clear bit 0x10 of obj+0xF0**
   → **expected** the striped box APPEARS in PCSX2 (proves the gate is what hides it). Restore.
3. **TouchPlate** obj 0x016d43e0 on 05 [L ee_start_05.bin]: obj+0xF0 == 0x1050 at level start;
   a `gdbwatch.py` watch on `View_AddObjects` 0x1e62d8's `0x8010` test should record the rejection
   for this object when its room renders.

nf-view renders (before/after on the fixed branch):

1. `NF_NOPORTAL=1 nf-view <iso> 07000005 --bond 16.67,30.04,58.60,1.755,-0.001 --skip-cutscenes --frame 30 --shot a.png`
   — the red/yellow striped box right of centre must be **gone**; the grapple **pulley** at
   placement #282 (13.75, 32.29, 57.30) must **still** be there (same view, slightly left-below).
2. `NF_NOPORTAL=1 nf-view <iso> 07000005 --bond -35.04,37.16,-31.65,2.6,0.05 --frame 30 --shot b.png`
   — second WallHug blocker's spot: no striped box.
3. `NF_NOPORTAL=1 nf-view <iso> 07000005 --bond -18.46,39.0,-72.49,0.0,0.3 --frame 30 --shot c.png`
   — the TouchPlate's world-baked spot: no striped box; the ledge/grapple geometry normal.
4. `NF_NOPORTAL=1 nf-view <iso> 0700000c --bond 14.22,16.28,-139.54,0.986,-0.05 --frame 30 --shot d.png`
   — 0c pylon area regression: pylon intact, no striped boxes anywhere on 0c.
5. 07000001/12 gate spot (`Door_Gate8`, the castle gate on 05: `--bond 104.95,25.12,108.83` looking
   at the gate): the gate door still draws (the 0xE8 gate must not over-hide).

## What's left

- Nothing blocking. The audit's findings are all resolved in the final tree (see the audit
  section); the census equality keeps both directions pinned for future edits.
- 0xda/0xfe creators were not disassembled (no 0x10-flagged meshed objects of those kinds on the
  disc — the census would flag a wrong draw either way); a later task can verify their copier
  status and widen the FLAGS_COPYING list if one copies.
- The full-suite total line is appended below once the fast runner finishes on the final tree.
