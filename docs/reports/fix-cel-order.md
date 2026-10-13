# fix-cel-order — the static draw order is the game's cel-chain order

## What was wrong

models-world keyed list 0 with `Material::chain` = the placement's parse ordinal, on the assumption
that the game's cel chain is "≈ placement-parse order" (`View_AddCels` 0x1e6ba0's −90000 + 10·k). On
0700000c the baked `pylonshadow` decal (parse ordinal 68) drew before the ground room under it
(ordinal 88): the decal darkened the fog clear colour and its depth write rejected the ground — the
user's pale lattice. The real chain order is neither parse order nor its reverse.

## The law (evidence)

- **[M] `View_AddCels` 0x1e6ba0, disassembled** (the decompile's field split was flagged unreliable
  by docs/atlas/cel.md — the disassembly settles it): the **−90000 + 10·k** keys go to the **+0x20
  chain** (`lw $s3, 0x20($s3)` at 0x1e6dac), entry object = the cel, k advancing only for pushed
  cels (record != 0 and `record+0x44 & 0x10 == 0`). The **`cel+0x18` inner chain** gets **±dist²**
  keys (0x1e6d74: `Vec_SqDist3D` against the eye at 0x34b390), frustum-tested per node, list chosen
  by `cel+0x94` bit 0x1 (list 1 = the alpha list, −dist²) and bit 0x2000 (list 2), with the
  `+0x3c == 0x8018 && viewer-cel == 0xc023` skip.
- **[M] `Vision_Portal_Recurse` 0x1e86e8**: the +0x20 chain is threaded **per frame** from
  `VisCelList` (0x34b3b0): the viewer's cel inserted first (depth 0), `Vision_Recurse` 0x1e82e0
  inserts each portal-reached cel at **first reach** with depth = parent+1
  (`*(short *)(&DAT_0034b3b4 + n*8) = depth+1`), then `QuickSort` 0x1d30a8, and the sorted array is
  threaded root-first through `cel+0x20`.
- **[M] `QuickSort` 0x1d30a8 comparator id 3** — jumptable 0x304fc0[3] → 0x1d3430, disassembled:
  `lhu $a0, 4($a0); lhu $v1, 4($a1)` — the ascending **u16 portal-recursion depth** at element+4.
  Unstable: Hoare partition, pivot = element `count>>1` swapped to the front, smaller side pushed to
  the stack. (Comparator ids 0/2 are 0x1d3320/0x1d33c0 = CompLight/CompHitData; id 1 → 0x1d3350,
  matching WORKLOG:5666.) The tie order feeds the −90000 + 10·k ordinals, so the port sorts
  exactly like the game (`quicksort_viscel`, verbatim, unit-tested with a hand-traced golden).
- **[M] `parseentity_fixup_entity` 0x1d00c0**: the runtime cel's `+0x3c` marker comes from the
  placement's u16 flag word — rooms 0xc023/0xc047 keep it (+0x804c023/0x804c047), the dynamic
  classes rewrite to 0x8019, 0x8018/0x8017/0x803d stay. So the top-level chain cels are exactly the
  **room-class placements**; every non-room 0x8000 placement is an inner node.
- **[L] `work/ee_start_05.bin`** (file offset = EE address): 137 runtime cels walked via
  `glb_world`+0 → +0x14 = **19 room cels (0xc047) + 113 inner 0x8019 cels**; every inner cel hangs
  on a room's +0x18 chain (8 chain roots, 58 + 41 on the two big rooms; child class histogram:
  113× 0x8019, 0 rooms as children). `GfxList+4` (0x2d6274) = the eye's own cel (no glist record →
  never pushed), `VisCelCount` = 1 at that instant. The port's 07000005 creates_cel set is exactly
  **19 rooms + 113 non-rooms** — a 1:1 match with the snapshot's cel split.
- **[D] disc probe** (0700000c/07/05 placement flags): every baked shadow decal is a non-room
  0x8019 placement — 0c's `pylonshadow` (cel rank 69, the driver's chain 68), `obj_turb*_floorshadow`,
  `shadow-turbine`, `skylight-shadows`, `tableshadows`…; 07's `hall_shadows1-4`,
  `OBJECT_banistershadows1-2`, `OBJECT_Tableshadow01`, `hall_shadow_banister01`.

## The fix

- `crates/nf-view/src/scene.rs`: `Material::chain` is now the **room's** parse rank (the fallback
  ordinal when the per-frame walk cannot run) and a new `Material::node` carries the room placement's
  `CelGraph::cells` index for the per-frame lookup. Every **non-room** creates_cel placement — the
  baked decals, walkways, blockers — is now distance-keyed (`chain = node = u32::MAX`), which also
  restores the game's list choice by glist flag bit 0x1 (the pylonshadow decal moves to the alpha
  list, pass 3 — the game's own list-1 push for `+0x94 & 1`). `portal_cel_chain` (new) runs the
  game's DFS recording first-reach order + depth and applies the verbatim `quicksort_viscel`;
  `portal_visible_cels` is now derived from it (same visibility mask as before — the cull path is
  unchanged). The room rank counts resolved main-map room-class placements before any skip, aligned
  with `parse_cel_graph`'s `rooms`/`room_to_cell`.
- `crates/nf-view/src/gpu.rs` (outside the task's named files — the per-frame sort key lives at the
  render site; minimal, additive: `DrawCall.node`, one `chain_ord` computation, the pass-1 key
  branch): each frame the walk from the eye's room produces the depth-sorted chain, each room cel's
  position k gives the key −90000 + 10·k; rooms missing from the chain (or a failed walk) fall back
  to the baked parse rank. Ordering only — the cull (including the NF_PORTAL gate) is untouched.
- `crates/nf-formats/src/level.rs` needed no change.

## Verification

- **Failing first**: the new `fix_cel_order_live.rs` run against unmodified master fails at
  `the pylonshadow decal is an inner node, not a chain entry: left: 68, right: 4294967295` — the
  driver's measured chain 68.
- `the_pylonshadow_decal_draws_after_the_ground_room` (0700000c, pinned): the decal's draws are
  distance-keyed and draw after every overlapping room-chain ground static (material #3380 is now
  pass 3 = the game's list 1; a pass-1 decal would sort by dist² ≥ 0 > any room key). Sisters: the
  rooms' fallback ordinals ascend, and the whole −90000 block sorts before every distance key.
- `statics_draw_in_parse_order_and_decals_after_their_walls` (07000007, reworked pin): rooms first
  (parse-rank order, pass 1), the 0x8019 shadow decals distance-keyed after their walls — the
  user-verified R2 outcome preserved under the real law.
- `the_quick_sort_orders_viscel_by_depth_like_the_game`: hand-traced golden for depths [1,0,1,0]
  (game order [1,3,0,2]) + permutation/sortedness/determinism sisters.
- **Full suite** (`NF_ISO=$NF_ISO cargo test --offline --release -j2 --no-fail-fast -p nf-game
  -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`): **617 passed, 3 failed, 13 ignored** —
  the 3 failures are the allowed slim-ISO `disc_golden` trio (`bigf_header_fields_behave_as_documented`,
  `bigf_tables_match_the_golden_manifest`, `directory_tree_matches_the_golden_manifest`).

## Live checks for the main agent

1. **EE snapshot on 0700000c** (the one this task asked for): save an EE snapshot mid-game at/near
   the pinned spot (`0700000c --bond 14.22,16.29,-147.02,0.973,-0.300`). Expected [L]: `GfxList+4`
   (word at 0x2d6274) walks a +0x20 chain of room cels (0x3c words 0x0804c023/0x0804c047) in
   ascending `VisCelList`-depth order; the `pylonshadow` cel (record pCelList+81·0x50) must NOT be in
   that chain but hang on some room's +0x18. That confirms the per-frame law on the bug level itself.
2. **Render checks** (no GPU here):
   - `0700000c --bond 14.22,16.29,-147.02,0.973,-0.300` — the pylon base: the lattice must now be
     DARK (decal over ground), not pale fog-coloured.
   - The 07000007 R2 spots from the existing decal test's view (`hall_shadows`/`banistershadows`
     corridors): the shadow decals must still paint over their walls/floors (no re-opened holes).

## What's left

- The per-frame chain is computed every frame from the eye; the game also re-sorts when doors close
  portals (`Door_IsClosed` 0x1367b0) — the port's walk has no per-portal door link yet (the
  pre-existing deviation, noted in `portal_visible_cels`), so a closed door cannot change the order.
- `View_AddCels`'s list-2 path (cel+0x94 bit 0x2000) is not separated in the port (rare; nothing on
  the surveyed levels pinned it).
- A mid-game 0c snapshot (check 1 above) would pin the per-frame chain on the bug level directly.

## ntfy

sent (see the transcript; fallbacks not needed).
