# ladder-02 — report

Task 691, branch `s/ladder-02`. Playtests 1011-1732 / 1014-1725 (level 07000002): "You can't move
your camera while on the ladders"; "When you get on the ladder, it unequips your weapon. And when
you get off the ladder, it doesn't re-equip"; "I'm infinitely latching onto the ladder, and I can't
get off of it" (climbing off into the water).

## What was already on master (verified, not redone)

- **The climb look** — ported by fix-ladder2 [M Player_Climb 0x196310:18-33 (the yaw clamp+ease
  toward `ladder.yaw + π`), Player_Update 0x193600 (the turn channel integrates after the climb),
  Player_Aiming 0x1a1d28:21-30 (runs for every substate ≤ 7), Player_ViewClamping 0x1a92e8:9-16
  (pitch ±1.0)]. Verified live by this task on the bunker ladder (below).
- **The weapon save/redraw** — ported by fix-ladder2 [M Player_WeaponNone 0x1a9858 saves the held
  id to `anim+0x64` and goes to 0x47 immediately; the dismount carry `if (anim+0x62 == 0x47)
  anim+0x63 = anim+0x64` at Player_Climb 0x196310:99-102; Player_SetWeaponAnimObj 0x1a2e38:197-201
  re-runs Player_WeaponSelect]. Verified live by this task on both 02 ladders.
- **The grab/redismount basics** — fix-ladder-water(2): the walk-on grab, the swim grab, the
  mesh-scale swim reach, the InWater regrab. FRAME_RATE_DIV = 1.0 re-verified from the ELF
  [L 0x30d0d4], so the water regrab is `(u8)(1.0·5.0)` = 5 ticks [M Player_InWater 0x195e50:27].

## What this task found and fixed

**The climb-down over the pond dismounted on the deep pond bed mid-column** — the root cause left
in the "infinite latch" family. The climb's hit-below flag (the dismount trigger `hit below &&
forward < 0` [M Player_Climb 0x196310:63-78]) was read through the FLAT world ray
(`floor_below(pos, 1.2)`), which sees the pond bed under the hut ladder — the bed's solid core is
**material 10 at y≈2.09** (the water planes above it are mat 0xE and already dropped). The
climb-down therefore dismounted at root ≈3.27–4.47, 0.44–1.6 ABOVE the ladder's bottom threshold
(`centre.y − radius + 1.0` = 2.83 [M Player_CollLadder 0x196118]), dropping Bond into the grab
window; the swimmer then settled inside the grab column (the grab snap is 0.5 off the axis [M
0x196118:36-44], the authored mesh reaches the settle capsule — probed directly, see below), and
the 5-tick InWater regrab could not cover it: re-latch → climb down → dismount → re-latch, the
playtest's loop.

The fix (crates/nf-game/src/player.rs `climb_tick`): the hit-below read is now the **cel-scoped
straddle ray** `segment_hit_scoped(self.cel, …)` — the game's own player-collision mechanism [M
Player_FeetOnPoint 0x194188 passes `obj+0x20`; the scan tests only the cels `Collide_StraddleCels`
0x1eac98 keeps]. With Bond in the garden cel, the water cel's bed is outside the straddle set, the
climb-down runs UNDER the pond surface to the bottom threshold, and the dismount lands at root
**2.811 < bottom 2.830** (asserted) — below the grab window's strict lower bound [M 0x196118:28-34].

## The lock (tests, through the real viewer path)

`crates/nf-view/tests/ladder_02_live.rs` (NF_ISO, `nf_view::load` + `Sim::step_bond`):

- `bunker_ladder_look_dismount_and_weapon` — the bunker exit ladder (≈(−26.9, 2.3, 42.2) [D
  atlas/ladder.md]): the walk-on grab snaps Bond to `centre + fwd(yaw)·0.5` [M 0x196118:36-44] and
  holsters to 0x47 with the P2K (weapon 2, the loadout) saved; the turn channel moves the yaw
  inside the eased ±90° window (deviation > 0.1 under a held turn, never past π/2) and the pitch
  takes the stick, clamped at ±1.0; the climb to the top fires the +0.6 lunge dismount [M
  0x196310:54-62]; 240 ticks on the roof without a re-latch; the P2K comes back through the draw
  cycle (`switch_state` 0).
- `water_ladder_bottom_dismount_does_not_relatch` — the pond hut ladder (≈(−74.8, 4.0, 22.2),
  bottom 2.83 under the surface): the swim grab holsters and saves; the climb DOWN dismounts at the
  bottom threshold (≤ 2.83, asserted) under the surface; the swim entry carries the regrab [M
  0x195e50:27]; the push-off held from the dismount tick swims free for 300 ticks with NO re-latch
  and ends > 0.9 off the ladder axis (out of the grab column); the weapon returns after the bottom
  dismount.
- Sister `swimming_back_into_the_hut_ladder_still_grabs` — swimming INTO the hut ladder still
  attaches and climbs (the grab is not broken by the escape work).

## Known divergence / live checks for the main agent

1. **The neutral settle at the hut ladder.** With the stick NEUTRAL after the dismount, the
   swimmer does not stay at the dismount depth: the solid pond bed (material 10, y≈2.09) pushes
   the swim capsule up to its equilibrium at root ≈3.20 — inside the grab window AND inside the
   ladder's authored mesh (probed: the hut ladder's celglist resolves through the synthetic
   `(map,object)` key 0x8033012b — 100 triangles — and its capsules HIT the settle spot, so the
   port's analytic swim reach is faithful there). Once the 5-tick regrab expires, the port
   re-latches. The game's own collision would settle him the same way; whether the REAL game
   re-latches a neutral settle turns on its cel assignment and feel. **Check**: PCSX2, 07000002
   pond hut ladder — climb down into the pond and release the stick; watch Bond's position
   (`fieldprobe.py` on obj+0x30) for ~2 s. If the game re-latches (substate 1, obj+0xF6), the port
   matches the game and the divergence note stands; if the game stays free, the port's swim-grab
   guard needs the game's extra condition (candidates: the climber's cel during the dismount, or a
   longer regrab path I did not find).
2. **The climb-down's dismount depth in the real game** — the fix predicts the climb continues
   under the surface to the bottom threshold (root 2.81, fully submerged) before the dismount.
   **Check**: same scene, hold back on the ladder; the drawn root should submerge before the climb
   releases. If the game dismounts at the surface instead, the cel scope does not hide the bed
   from the climb scan and the fix's basis needs the live cel list (`gdbwatch.py` on the scan's
   straddle set).

## What I ran

- `cargo test --offline --release -p nf-view --test ladder_02_live` — 3/3 passed.
- `cargo test --offline --release -j2 -p nf-game -- --test-threads=1` — 49 test binaries, all ok.
- The ladder/swim regression set (fix_ladder2_live, fix_ladder_water_live, fix_ladder_live,
  fix_swim_live, fix_swim_entry_live, fix_swim_vent_live, fix_swim_vent5_live,
  walkreal_07000005_live) — all green.
- The full suite (fast runner `tools/test/suite.py`) on the pre-fix tree: **`1805 passed,
  6 failed, 72 ignored (37 min)`** — the 6 are the 3 `disc_golden` (allowed on the slim ISO) +
  `probe_pylon_render`/2 `fix_0c_pylon_live` render probes from the 07000005 pylon task
  (pre-existing, GPU-render, untouched here). The final-tree rerun (merged tree, this branch): **`1808 passed, 6 failed, 72 ignored
  (42 min)`** — the same 6 pre-existing failures, no regressions.

## What's left

- The two live checks above (the neutral settle; the dismount depth).
- The swim pick's stage 2 is still the analytic column (the game's `Intersect_CylGeom` mesh
  refine, blocked per the G-046 review on the data model — though this task proved 02's ladder
  meshes ARE reachable through the synthetic `(map,object)` shape keys, so a follow-up can
  implement the true refine for the ladder picks using that path).
