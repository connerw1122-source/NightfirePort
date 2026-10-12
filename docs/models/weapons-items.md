# Lane models-weapons-items: weapons, gadgets, pickups, projectiles

Status after `s/models-weapons-items`. Everything [D] (disc scans through the repo's own
`Level::parse`/`weapon_data`, the tests in `crates/nf-view/tests/models_weapons_live.rs`) or [M]
(ACTION.ELF decompile + disassembly) unless tagged [S]. The proof harness is that test file
(4 tests, NF_ISO; each fix was shown failing first by neutralizing it).

## 1. What the lane covers and what resolves

Every weapon's first-person view model (Bond's hands/sleeve + the rigid parts), every third-person
held gun, every gadget's models, every pickup's model, dropped weapons and the projectiles. The
golden pins, per story level, the armed drones, their guns/flash meshes, the placed pickups and the
projectile meshes of every weapon the level can fire (numbers only, `golden_weapon_models_resolve_on_every_story_level`).

- **Third-person held guns** (`weapon_data[w]+0x80` on the skin datum `+0x50`, `AnimObjectDraw`
  0x1c4490 [M]): every armed drone's gun resolves on every level, with two measured exceptions that
  the shipped game shares (§3).
- **Muzzle flashes** (`+0x4C`, e.g. 0x02000261 `Pistol_Muz` [M `AnimDatumGetWeaponInfo`]): resolve
  everywhere — after the session-object-table fix (§2), which is this lane's main defect found.
- **Placed pickups** (type 0xF0): every placement resolves its model object on every story level —
  the O2 "armour plate drawn nowhere" class is clean level-wide (fix-player2's instancing held).
- **Projectiles** (`+0x5C`, the flying mesh): `Bullet_update` state 0 sets the weapon's mesh on the
  bullet object (disasm 0x12e8f0-0x12e900: `lw $a1, 0x5c($v0)` → `hashtable_set_object_to_entity_gfx`)
  and `View_SetDrawInAllViews` draws it at the flight matrix (`Mat_Align2Dir(obj+0x90, dir, up
  (0,1,0) [L ee_start_05.bin @0x2d6210], fallback (1,0,0) @0x2d6220)`; the per-tick tail moves
  `obj+0x30..0x38` only). **The port drew no projectile at all** — grenades flew invisible. Fixed:
  the scene pools slots (8 grenades, 32 drone bullets, 8 of Bond's bullets), one Effect instance per
  (slot, mesh) for every distinct projectile mesh the level resolves; the live projectile's mesh
  picks the shown one and the others collapse to the FAR point. Test:
  `a_thrown_grenade_draws_its_weapon_mesh` (throws a frag on 07000005 through `step_bond`; the mesh
  tracks the grenade; when no grenade flies, the slot hides).

## 2. The session object table (the fix behind Tower-2's missing flashes)

`hashtable_delete_hashtype` is called from the loadable-pack path ONLY (`LoadableLoad` 0x1d69e8 /
`LoadableReload` 0x1d6ab0) in all of ACTION.ELF [M decompile grep]: the map-object hashtable
accumulates every archive loaded in the session. Measured consequence [D scan]: Tower-2's guards
(07000011-13, weapons 0x10/0x1c/0x1d/0x1a) and the airbase/evilbase guards (07000014-16, weapon
0x1A) fire weapons whose flash meshes 0x02000261/0x0200044a — and the SMG's held mesh 0x0200025c —
are on NO archive from 07000011 on; the game resolves them through the session table that the
earlier story archives filled. The port drew **64 armed guards with no muzzle flash** across
07000011-16 and left the SMG guard unarmed. Fixed: `nf_view::load_impl` merges every weapon-model
hash the level archive misses (the weapon trio +0x80/+0x4C/+0x5C and the view-model sleeves) from
the latest earlier story archive that holds it (the story order is the case order of
`ResetMap_LevelCode2Img` 0x1d5258 — the level codes ascend in mission order [docs/systems/levels.md
§1]). Appending maps never shifts an existing (map, object) index and `by_hash` is first-wins, so
every local resolution is unchanged. Test: `tower_2_guards_get_their_muzzle_flashes_from_the_session`.

The story arcs 0700000c/14 also get whole-archive skin merges from the characters lane's G-021
(same load path; the weapon-model merge runs after it so no map is taken twice).

## 3. The shipped game's own artifacts (measured, not port defects)

| Where | What | Evidence |
|---|---|---|
| 07000001, 07000003 | The stun-throwing guards (param 11 selects weapon 0x36 [M `NDrone2_DefaultInit` 0x14b300]) throw grenades whose mesh 0x0200071f is on NO archive the session has loaded by those levels [D scan] — `Bullet_update` skips the draw when the hashtable misses (the `iVar14 == 0` arm [M 0x12e7b0]): the stun grenade flies invisible in the shipped game | the golden's proj_miss 1 on those levels; `Bullet_update` decompile |
| 07000004 ×1, 07000016 ×2 | The SMG guards (weapon 0x12, `+0x50` datum 4) carry skin 0x0500001e, which has no datum 4 [D] — `AnimDatumSetEntity` (0x1c7848) walks the skin's datum list and no-ops on a missing id [M]: the game draws that guard unarmed | the golden's tp_miss; the datum list [D 07000004.bin] |

## 4. Render spots (one per level; the main agent renders and eyeballs)

Stand at the spot facing `yaw 0` (north): a placed pickup of the lane's kinds is 3 units ahead.
Pickup kinds: 0 weapon, 1 ammo, 3 armour. Every level has at least one (0700001b's pickups come
from script/drops only — use the dev panel's Frag Grenade there).

| Level | Spot | What is 3 units ahead |
|---|---|---|
| 07000001 | `--bond 17.4,7.2,-6.2,0.0` | armour |
| 07000002 | `--bond -74.2,6.6,21.0,0.0` | ammo (weapon 0x02) |
| 07000003 | `--bond -10.9,6.3,-41.0,0.0` | armour |
| 07000004 | `--bond -10.5,5.4,26.7,0.0` | weapon 0x12 (the SMG) |
| 07000005 | `--bond 59.7,34.6,83.1,0.0` | armour |
| 07000006 | `--bond 34.3,41.0,-27.1,0.0` | ammo (weapon 0x24) |
| 07000007 | `--bond 92.0,44.7,-36.2,0.0` | armour |
| 07000008 | `--bond 159.4,43.6,-116.6,0.0` | armour |
| 07000009 | `--bond -22.6,-367.5,21.9,0.0` | armour |
| 0700000a | `--bond -13.5,1.9,4.6,0.0` | armour |
| 0700000b | `--bond 17.5,48.7,25.5,0.0` | armour |
| 0700000c | `--bond 37.6,1.1,-152.0,0.0` | ammo (weapon 0x1f) |
| 0700000d | `--bond 147.0,22.8,-255.8,0.0` | armour |
| 07000011 | `--bond 14.8,16.1,2.3,0.0` | weapon 0x3b (a dart gun) |
| 07000012 | `--bond -0.0,1.4,-7.1,0.0` | weapon 0x2e |
| 07000013 | `--bond -27.7,-367.5,22.6,0.0` | armour |
| 07000014 | `--bond 32.3,1.9,28.1,0.0` | armour |
| 07000015 | `--bond 12.2,41.3,-2.8,0.0` | armour |
| 07000016 | `--bond -19.0,7.7,-97.5,0.0` | armour |
| 0700001b | `--bond 2.9,1.5,-3.0,0.0` | (no placed pickup; give the frag via the dev panel and throw — the grenade body draws) |

The thrown-grenade check on any level: give the Frag Grenade (dev panel), pull the trigger — the
grenade body (0x02000726) arcs and tumbles to the bounce.

## 5. Dev view models (folded from 190-fix-dev2)

Each level archive ships only its own weapons' first-person skins (+0xDC) — the dev panel showed
"no 1st-person model here" for most weapons. With `--dev` the missing FP skins merge from their
holding archives (the skin-hash → archive index scans every type-3 entry on the disc once), with
their textures/skeletons/sequences and the weapons' kind-5 anim scripts — the same session-table
mechanism as §2. `Scene::build` then builds a view model for every weapon, GPU skinning included
(`skin_layout` covers every character). Test: `dev_gives_every_selectable_weapon_a_view_model_on_07000005`
(every selectable weapon's view model present; the panel catalog reports none missing; the sister
assertion pins that WITHOUT `--dev` nothing changes — the L96A1 stays view-model-less on 07000005).

## 6. Per-level inventory (the golden, numbers only)

Pinned by `golden_weapon_models_resolve_on_every_story_level` (drones = armed-drone placements,
tp = their held guns that resolve, flash = their muzzle-flash meshes, pickups = placed pickup
models, proj = the projectile meshes of every firable weapon, fp = selectable weapons whose
first-person skin is in the level archive):

```
level    drones tp_ok tp_miss flash flash_miss pickups miss proj proj_miss fp_present
07000001   20    19     0      19       0        6      0    0     1          19
07000002   25    21     0      21       0        8      0    1     0          19
07000003   20    17     0      17       0        5      0    0     1          19
07000004    1     0     1       1       0        4      0    1     0          20
07000005   16    16     0      13       0        6      0    0     0          22
07000006   32    31     0      23       0       11      0    0     0          22
07000007   58     8     0       8       0        3      0    0     0          15
07000008   26    26     0      26       0        8      0    1     0          18
07000009    6     0     0       0       0        3      0    0     0          19
0700000a   20     0     0       0       0        5      0    0     0          19
0700000b   10    10     0      10       0        5      0    1     0          25
0700000c   40    40     0      40       0       15      0    1     0          23
0700000d   41    41     0      41       0       16      0    0     0          23
07000011   37    32     0      32       0       13      0    4     0          11
07000012   32    28     0      28       0       18      0    5     0          15
07000013   12    12     0      12       0        7      0    3     0          16
07000014   61    61     0      61       0       32      0    7     0          19
07000015   20    20     0      20       0       12      0    2     0          20
07000016   20    16     2      18       0       17      0    3     0          20
0700001b   20    19     0      18       0        7      0    2     0           2
```

(drones/tp counts: a placement whose param-9 weapon is 0 or ≥ 0x73 counts in drones but not tp;
07000007's party and 07000009/0a's switch-gated guards carry no weapon ids their param-9 slots
resolve — their gun resolution runs when the switch/spawner arms them, with the same tables.)

## 7. Still open in this lane

- **The "torn guns" user report** has no playtest entry to reproduce from; the likeliest visible
  causes in this lane (a held mesh missing its flash or its pickup twin drawing at the wrong slot)
  are covered by the tests above. If it survives the session-merge fix, it needs a fresh screenshot
  with the level and spot named.
- **R1 over-bright (the TWEAK)** on guns: the 0x800 multiplier root cause is fix-render2's
  (characters.md G-042 pinned the source texels); nothing in the gun path adds brightness. Watch
  the §4 spots.
- **The Q-Worm projectile mesh** (0x0200025f) resolves on most levels (the golden counts it) but
  the gadget itself is unported (docs/port-map/gadgets.md — another lane's function work); the mesh
  draws for Bond's own worm once fired.
- **FP flash meshes on the dev-merged view models** use `weapon_data+0x48`; a few weapons (the
  snipers) have 0 there — the game draws no FP flash for them either [L].
- **The grenade body's exact orientation** is `Mat_Align2Dir(dir)` [M 0x12c5a0:185] — the port
  mirrors it; whether the mesh model's long axis is +x or −x is eyeball-only (the §4 throw check).
