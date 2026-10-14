# fix-striped-boxes2 — the 0x10 no-draw gate verified per kind; hide only what the game hides

Task 102o. Branch `s/fix-striped-boxes2` = `s/fix-striped-boxes` + origin/master (merged at 1f8e050,
then again at 8295049 before the final suite). The merge of 102k was HELD because its misc-sweep gate
(`FLAGS_COPYING = 0x30 0x34 0x35 0x3b 0x71 0xe1 0xe3 0xfa 0xfd 0x4a`) hid far more than the striped
markers and `models_misc_live` (its census) did not apply the gate. This task verified the gate's
premise for EVERY kind, resolved the hold, and re-pinned the census.

Files: `crates/nf-view/src/scene.rs` (the gate comment rewritten with the per-kind evidence;
`0xda` added to `FLAGS_COPYING`), `crates/nf-view/tests/models_misc_live.rs` (census applies the
rule; spot checks updated; golden re-pinned), `crates/nf-view/tests/fix_striped_box_census.rs`
(new `probe_gate_verdicts_per_kind`). No other files touched — `drone.rs`/`player.rs`/`lib.rs` were
never needed (the shared-file hook list is empty).

## The verdict: the gate is correct as it stands — every probed object is invisible in the game

`View_AddObjects` 0x1e62d8:33 submits a dynamic object to the viewer draw lists only when
`(obj+0xF0 & 0x8010) == 0` [M, re-read in the decompile C this session]. `obj+0xF0` is the creator's
copy of `celglist+0x44` (= the port's `MapObject.flags`). So an object whose glist carries bit 0x10
never draws **iff its kind's creator makes the copy and nothing clears the bit afterwards**. Both
halves now hold for every kind in the gate:

### 1. Every kind's creator copies `glist+0x44` into `obj+0xF0` (address + the store)

| kind | creator | the copy |
|---|---|---|
| 0x30 | `Monitor_Create` 0x1b08b0:26 | `Control_SetGList` 0x134960:12-14: `obj+0xF0 = *(glist+0x44) \| obj+0xF0 & 0x30008000` |
| 0x34 | `GunImp_Create` 0x131f20:68, :84 | `Control_CreateObjEx` 0x1343c8:29-36 (`obj+0xF0 = *(glist+0x44) \| param_10`), then `\|= 0x1000` at 0x131f20:75/87 |
| 0x35 | `PCQWorm_Create` 0x1b07a8:12 | `Control_CreateObjEx` 0x1343c8:29-36 (same store) |
| 0x3b | `DynamicObject_Create` 0x18dbf8:21-23 | direct `obj+0xF0 = *(glist+0x44)` (already cited by 102k) |
| 0x71 | `rotor_init` 0x18f4b8:25 | direct `obj+0xF0 = *(glist+0x44)` |
| 0xda | `Car_Create` 0x1305b0:24 | `Control_CreateObjEx` with OR-mask 0x44, then `\|= 0x40` at :57 — **added to FLAGS_COPYING this task** (latent only: no 0x10-flagged mesh-ful car glist exists on the disc; the census pins it) |
| 0xe1 | `Destroy_Create` 0x12c318:23 | `Control_SetGList` — and the creator itself honours the bit: 0x12c318:57-59 `if (obj+0xF0 & 0x10) != 0 { *tag = 0; }` zeroes the smash target (`*(glist+0x40)`), i.e. the game expects these boxes never to draw AND never to swap to a broken mesh |
| 0xe3 | `Flicker_Create` 0x1acb00:52 | direct `obj+0xF0 = *(glist+0x44)` |
| 0xfa | `ThirdIcon_Create` 0x197070:34-37 | `obj+0xF0 = *(glist+0x44) \| 0x1000` (already cited by 102k) |
| 0xfd | `CamSubject_Create` 0x1bc5d8:20 | direct `obj+0xF0 = *(glist+0x44)` |
| 0x4a | `Apocalypse_Create` 0x18dcc0:24 | direct `obj+0xF0 = *(glist+0x44)` |

Kind→creator routing verified in the dispatch `parsemap_create_dynamic_objects` 0x1d04f0
(cases 0x30/0x34/0x35/0x3b/0x71/0xda/0xe1/0xe3/0x4a). Non-copying kinds stay ungated (the game
draws their 0x10-flagged meshes): `Lock_Create` 0x1ae1f8 (0x2e), `OneSided_Create` 0x1e7ce8 (0xd5),
`RB_Create` 0x1d6bf0 (0xd8) — as 102k established — plus **0xfe `Mine_Create` 0x17ce28, verified this
session** (plain obj+0xD8 store, no obj+0xF0 write; `Mine_Update` 0x17c930:142's `= 0x30` is the mine
hiding ITSELF when armed, not a glist copy).

### 2. Nothing clears bit 0x10 afterwards for these kinds

Searched every `obj+0xF0` write in the whole decompile (166 files touch the offset). For the gate's
kinds: none of their Update/Activate/ApplyDamage functions writes obj+0xF0 (checked
`Monitor_Update/Activate/ApplyDamage` 0x1afcc8/0x1b0058/0x1b0968, `PCQWorm_Update/Activate`
0x1aee20/0x1b0840, `Destroy_Update/Smash/CollisionHandler` 0x12c4f8/0x12bb60/0x12bd48,
`ThirdIcon_Update` 0x196e80, `Flicker_Update` 0x1ab960, `Apocalypse_Update` 0x18d4f0,
`rotor_update` 0x18f568, `CamSubject_Update` 0x1bc668). The remaining writers are all out of scope:
`DynamicObject_Update` 0x18d3b8:16/34/39 (0x3b only — stores back the word `DynamicObject_Create`
0x18dbf8:27 captured from `*(glist+0x44)`, so a set 0x10 survives the channel restore),
`Drone_PauseNoDraw` 0x13a710 (drone objects only), `SP_HideObj` 0x1dcbf0 → `Script_HideObj`
0x1db808 (clears/sets 0x10 on a script's own stream entities — the copter's streams; never on
placement-created misc objects), and the bullet/player/pickup/car-child paths.

### 3. The probed objects, drawn or not drawn (name / kind / flags → verdict)

Every object in the driver's probe carries bit 0x10 on a FLAGS_COPYING kind ⇒ **not drawn** — they
are invisible gameplay/collision volumes; the visible machines are baked world geometry:

| level | object(s) | kind / flags | verdict |
|---|---|---|---|
| 05 | WallHug_Blocker1_Short, WallHug_Blocker2_Windows | 0x3b / 0x10 | **not drawn** (the striped markers — the user's box) |
| 05 | TouchPlate_GrappleLedge, Valley grapple_touchplate | 0xfa / 0x50 | **not drawn** (gadget-grantor / touch-plate volumes) |
| 08 | SmallExplosionBox | 0xe1 / 0x30 | **not drawn** (Destroy_Create zeroes its smash target) |
| 0d | WallHug_Blocker1_Long at (153.52, 22.29, −258.47) | 0x3b / 0x10 | **not drawn** (0/1 striped — hidden by the flags, not the texture) |
| 11 | 2× 3RD_Person_Icon at (−5.81, 46.38, 24.39) / (5.04, 46.38, 24.35) | 0xfa / 0x50 | **not drawn** |
| 12 | "Computer to hack" at (−16.88, 1.19, −32.15) | 0x35 / 0x50 | **not drawn** — an invisible use volume: its 5-batch mesh (radius 0.5) coincides with baked world geometry (the visible laptop is the baked mesh; shared texture keys 89/163 at the same spot) |
| 12 | ExplosionBox; 3RD_Person_Icon | 0xe1 / 0x30; 0xfa | **not drawn** |
| 14 | 12× SWITCH_Consoles_Evilbase (e.g. (47.45, 2.19, 36.81), (32.5, 1.54, 21.75)…) | 0x30 / 0x50 | **not drawn** — the visible consoles are baked geometry; the 0x30 monitors are the interaction logic |
| 14 | 19× DESTROY_BoxSatellite | 0xe1 / 0x30 | **not drawn** (of 48 destroyables; Destroy_Create also nils their smash target) |
| 14 | 3× 3RD_Person_Icon | 0xfa | **not drawn** |
| 15 | 3× DESTROY_BoxSatellite | 0xe1 / 0x30 | **not drawn** (8 of 11 destroyables draw) |
| 1b | 8× Apocalypse (a line at y −13.19, x −44.65…−105.87) | 0x4a / 0x30 | **not drawn** (the rumble/shake remains — `Apocalypse_Update` 0x18d4f0) |
| 1b | DESTROY_BoxMissile/BoxSatellite/BoxSmall | 0xe1 / 0x30 | **not drawn** (3 of 6 destroyables draw) |

Sister proof that the gate is per-object and not kind-blind (all in the new test): 0700000a's
monitor (0x30) and 3 PCQ worms (0x35), 07000041's two third icons (0xfa), 07000012's own monitor,
and 0b/46's PCQ worms carry **no** 0x10 and keep drawing — a kind-level hide would fail them.

## The census now applies the same rule (`models_misc_live`)

`expected_misc` skips `obj.flags & 0x10 != 0` objects of the FLAGS_COPYING kinds (cited inline), and
`misc_census_golden` is re-pinned `bb66b783…` → **`c2dc182372e5d06d1eada9041c439489e2da3cb7`**.
Per-level line diff (old → new; the numbers are exactly the probe's hidden objects):

- 07000005: `0x3b:2 0xfa:2` → *(empty — all four hidden)*
- 07000008: `0xe1:4` → `0xe1:3`
- 0700000d: `0x3b:4` → `0x3b:3` (WallHug_Blocker1_Long)
- 07000011: `…0xfa:2` → `0x2e:1 0x35:1 0x71:3`
- 07000012: `0x2e:2 0x30:1 0x35:1 0x71:1 0xe1:1 0xfa:1` → `0x2e:2 0x30:1 0x71:1`
- 07000014: `0x2e:3 0x30:12 0x3b:22 0xe1:48 0xfa:3` → `0x2e:3 0x3b:22 0xe1:29`
- 07000015: `0x2e:3 0xe1:11 0xfe:9` → `0x2e:3 0xe1:8 0xfe:9`
- 0700001b: `0x3b:1 0x4a:8 0xe1:6` → `0x3b:1 0xe1:3`
- unchanged elsewhere, including 07000041's `0xfa:2` and 0700000a's `0x35:3` (no bit on those).

The pre-existing `fix_striped_box_census` equality (`no_draw_bit == game_drawn_0x10_exceptions`,
the 0xe8 LOAD_LEVEL precedent) still passes on every level — the gate hides nothing the game draws.

## Renders (nf-view --shot, NF_NOPORTAL=1, lavapipe in this sandbox; before = origin/master 1f8e050, after = this branch)

1. **05 trench** `--bond 16.67,30.04,58.60,1.755,-0.001 --frame 30`: before shows the striped box
   (70,153 differing pixels in the right-centre region, dominated by pure (255,0,0)/(255,255,0) —
   the marker texture); after: **box gone**, the pulley still there. ✓
2. **14 consoles** `--bond 35.5,1.84,21.75,4.712,-0.12 --frame 30` (3 units off the
   (32.5,1.54,21.75) console): before shows a console mesh (4,629 differing px, bbox
   (585,398)-(700,459)); after: **gone**. ✓
3. **12 "Computer to hack"** `--bond -16.88,1.69,-30.95,3.142,-0.1 --frame 30`: before and after are
   pixel-identical — measured honestly, not skipped: 22 camera spots (8-yaw grids at r = 2.5/1.2/0.9,
   two overhead, four from inside the mesh) all render zero difference pre-gate, because the 0x35
   mesh coincides with the baked laptop geometry (see the table above). The draw-list change is
   pinned by the census (`0x35:1` → absent) instead of pixels.

## Tests (all NF_ISO, through `nf_view::load`)

- **Fail-first, shown**: on the merged pre-task tree the fast runner failed exactly
  `models_misc_live::misc_census_golden` + `misc_static_entities_match_the_census_on_every_level`
  (suite.out, 14:47 UTC); after the census re-pin both pass (10/10 in the file).
- **New** `fix_striped_box_census::probe_gate_verdicts_per_kind`: for each probed (level, kind) pair,
  `drawn == unflagged-count` with the probe's flagged count asserted (teeth against scope drift);
  sisters pin the same kinds' unflagged peers keep drawing. **Shown failing with the gate disabled**
  (`FLAGS_COPYING: [u8; 0]`, one-line sed): 07000008 drew 4 vs 3 unflagged — FAILED; restored, 4/4
  green.
- `fix_striped_box_census` full file 4/4; `models_misc_live` 10/10 on the final tree.

## Live checks for the main agent

PCSX2 (the port now matches the game; these confirm the game side of the verdicts):

1. **07000014 consoles**: stand at any SWITCH_Consoles_Evilbase (e.g. (47.5, 2.2, 36.81) — the room
   with the satellite boxes): **expected** no console monitor mesh floating in the room (the visible
   consoles are the baked desks/panels).
2. **07000014 satellite boxes**: the DESTROY_BoxSatellite brushes (e.g. (40.41, 1.51, 32.61)):
   **expected** nothing drawn; shooting the spot hits invisible collision.
3. **0700001b Apocalypses**: the line at y −13.19 (`--bond -55.4,-12.2,0,0,0` is the port spot):
   **expected** no meshes, only the quake rumble when they trigger.
4. **07000012 computer**: at (−16.88, 1.19, −32.15): **expected** the laptop visible (baked
   geometry) and the use prompt working — the hidden 0x35 object is the hack target logic.
5. **Poke** (`fieldprobe.py`): any 0x30 console control object on 14, obj+0xF0 should read
   0x50 (`*(glist+0x44) | 0x1000` is 0xfa only); clearing bit 0x10 should make the monitor mesh
   APPEAR (the gate is what hides it). Same for a DESTROY_BoxSatellite (0x30 → appears).

nf-view `--bond` spots per kind (kinds 0x34/0x71/0xda/0xe3/0xfd have no 0x10-flagged mesh-ful
object on the disc — latent gate, census-pinned; no spot needed):

- 0x3b: 05 `16.67,30.04,58.60,1.755,-0.001` (no box, pulley present); 0d `152.0,23.5,-256.0,1.2,0.1`
  near (153.52, 22.29, −258.47) — no blocker mesh.
- 0x30: 14 `35.5,1.84,21.75,4.712,-0.12` — no console mesh; 0a `--bond` near the 0a monitor —
  it still draws (sister).
- 0x35: 12 `-16.88,1.69,-30.95,3.142,-0.1` — the laptop area renders identically with/without the
  gate (baked geometry) — PCSX2 comparison is the use prompt, not pixels.
- 0xe1: 14 `44.2,3.3,39.8,2.32,-0.2` and 15 `35.6,45.2,17.5,0,0` (the mines' level; the remaining
  0xe1 draw) — hidden boxes invisible, unflagged ones drawn.
- 0xfa: 05 `-35.3,38.5,-31.4,0,0` (both icons hidden) vs 41 (its two icons still draw — sister).
- 0x4a: 1b `-55.4,-12.2,0,0,0` — no Apocalypse meshes.

## What's left

- Nothing blocking on this task. Note for a future docs pass: `docs/models/misc.md`'s
  "drawn by the game?" column predates the gate — 0x30/0x35/0xe1/0x4a rows should read "except when
  the glist carries the no-draw bit 0x10 (View_AddObjects 0x1e62d8:33)" and the 07000014/1b --bond
  spots there expect consoles/Apocalypses that the game (and now the port) never draw. Left
  unedited here: the file is outside this task's file list.
- 0xda joined the gate on creator evidence alone (no disc instance today); if a future level mod
  flags a car, the census equality will flag it either way.

## Suite

Fast runner (`tools/test/suite.py`) on the final tree (origin/master 8295049 merged, commit 3478c61):
see the total line below (updated at finish).

<!-- suite-total -->
