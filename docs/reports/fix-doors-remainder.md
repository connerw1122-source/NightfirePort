# fix-doors-remainder — G-035 remainder: the use pick vs the mesh, the door labels, the proxy path

Branch `s/fix-doors-remainder` (the task footer said `s/doors-remainder`; the repeated name won —
no remote branch of either name existed when I cut it from `origin/master` 42309e9).

## Done (committed, compiling)

1. **The use pick is the game's mesh pick** — `Doors::player_use` (door.rs) now sphere-tests the
   use sphere (r 1.0 at head + 1·viewdir [M 0x1a5170:100-108, `Mat_GetDir` of the viewer matrix]) against
   each door's **live leaf mesh**: `CollisionWorld::shape(glist)` × the door's `obj+0x90` matrix,
   closest-point-on-triangle < 1.0 (Ericson §5.1.5; the port's `Collide_Intersect` narrow phase).
   The old stand-in (distance to the placement < 2.5) remains ONLY for doors whose glist hash the
   world never loaded (no mesh to test). Broad phase 6.0. `drone.rs::player_use_doors` passes
   `d.world`; the walkreal/doors tests keep their semantics.
2. **The door labels** — `Doors` grew `label_msgs: Vec<(u32, u16)>` (the port's `Text_AddMsg`
   half; nf-view drains it in `hud_tick` exactly like `script_msgs`), `door_msg_count`
   (`DoorMsg` 0x30cd04), and the GameState gates `no_unlock_label` / `no_aim_label` set at load
   (nf-view `load_impl`: unlock note suppressed on 0x07000014/0x0700004a [M 0x1363c8:101], aim hint
   on 0x0700004a [M 0x1a7810:107]).
   - **0x2000003** (unlock note, 180 frames): pushed on the lock channel's rising edge right where
     `Door_Update` self-activates and clears `DI+0x68` [M 0x1363c8:101-107].
   - **0x200000b** (aim hint): new `Doors::aim_msg` = `Check_Target`'s door arm [M 0x1a7810:29-43:
     the head ray head + 50·viewdir (`0x42480000`), `Collide_RayIntersect`] + `Door_DoMsg` [M
     0x135f18:16-34: latch `DI+0x74` first, label only when the lock channel is not blocking and
     the effective state (group state when grouped, else `+0xF4`) ≠ 4, `DoorMsg < 3`]; ray-vs-tri
     (Möller–Trumbore) over the door meshes, hit < 5.0, a closer static wall (world.segment)
     kills it. Called every tick from the scene's run, use key or not.
3. **The proxy path** — new `Doors::collide_update` (the port's `Collide_Update` door half [M
   0x1ea398:84-214 + dispatch `control_funcs[0x1c]+4` @ 0x26f890]): post-walk (called right after
   `set_dynamic` in `Drones::tick`), each door whose live leaf mesh meets a mover's sphere gets
   that mover as its proxy record (the `obj+0xD0` list, `+0x58` = the mover), and the reworked
   `Door_ReactToHit` [M 0x135980:42-111] consumes the records directly — the old bounding-sphere
   re-scan inside it is gone, `#[allow(dead_code)]` removed. Never called from `Door_Update`
   (A-11b preserved).

Shared-file hooks (small, additive): `drone.rs` (pass `d.world` into `player_use`; the
`collide_update` call + sound playback after `set_dynamic`), `scene.rs` (the per-tick `aim_msg`
call; the `label_msgs` drain in `hud_tick`), `lib.rs` (the two gate flags at load).
`doors_01_live.rs` updated for the new `player_use` signature (empty world → its shapeless doors
keep the stand-in). Master also arrived with `door.rs`'s `use_point` line missing its `[h` bytes
(non-compiling); repaired as the first commit.

## Verified

- `cargo check -p nf-game -p nf-view` clean; the new test binary builds.
- **T1 negative case green**: `the_use_pick_respects_the_leaf_mesh` first half PASSES — E at 2.3
  from the leaf on the through-axis does **not** open (state stays 0x02, `pos01` 0). Pre-fix the
  2.5 placement stand-in opened there — that is the reproduced-before-fixed symptom.
- Baseline: the suite runner was launched twice but the sandbox's full-disk + the mid-edit tree
  killed both runs; **the full four-crate suite has NOT run on this tree.** The next session must
  run it before merging.

## Not verified / left (the next session continues here)

1. **T1 sister fails**: at 1.2 from the live leaf sphere centre the door does NOT open (state 0x2)
   — the sphere at head+1 should overlap the leaf by ~0.8. Suspicion: my through-axis choice
   (`matrix` row 0) is per-door wrong — the probe dump shows door 0's leaf spans z (offset −0.93 z
   from row3) while door 1's spans x; the axis is likely the row whose direction the
   `sph.0 − row3` offset does NOT lie along. Fix the helper to derive the normal that way, then
   the sister should pass. **Do NOT weaken the 2.3 assertion — it is the game-true one.**
2. **T2 blocked on data**: `lang.bind(0x2000_000b)` returns empty in USATxt.dat — the label IDs
   0x2000003/0x200000b do not resolve in the parsed table (event wiring is in; the string
   resolution is open). Probe `bind` over 0x2000000..0x2000020 and check the bank-0x20 fixup.
3. **T3** (closing door reverses off Bond, via the new proxy path) and the **census** (every
   shape-backed unlocked approach-0 door on 07000001..0700001b opens at 1.2) are written in
   `crates/nf-view/tests/doors_remainder_live.rs` but unrun. 07000001's doors are all kind-2
   slides with shapes (`in_shapes` true), `ch_lock` 0, `appr` 0 — the filters are right.
4. Full suite + the census numbers into this report; then the live checks below.

## Live checks for the main agent

- PCSX2, 07000001 door 0 (7.9, 5.4, 2.9): stand ~2.3 units off the leaf on its through-axis, press
  E — expect NO open (the pick's sphere falls 0.3 short); at ~1.2 expect open. Confirms the reach.
- PCSX2, any unlocked door: aim at it from ~2 — expect the aim-hint text (what string does the
  game show for 0x200000b? USATxt.dat gives nothing — is it empty in the game too?).
- PCSX2, a locked door (e.g. 07000005's ch-locked ones): aim — expect NO hint, only the locked
  thud on E; then unlock by script/switch — expect the 0x2000003 note and the door self-opening.
- nf-view `--bond` spots: 07000001 (-4.9,6.5,3.5) looking at door 0 (the hint pane), and the
  doorway of door 0 while it auto-closes with Bond in it (the reversal).
