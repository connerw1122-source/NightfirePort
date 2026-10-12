# fix-truck — the 07000005 truck level end, against the real game's trace

Branch: **s/fix-truck** (from origin/master 5056ab4).

## What's done

**The truck ride now ends the level like the game's.** The chain fires in the trace's order — 0x72 (the gate
guard at his post, trace rel −317) → **0x70+0x65 the same tick** (the truck's parking plate, rel −300) → 0x73
(rel −273) → 0x1D (rel 0) → the LOAD_LEVEL trigger loads 07000006 — and Bond parks at **y 37.929** against the
trace's **37.887** (within the task's ±0.05 bar; x/z within 0.5 — the port parks at z 52.14 vs the game's 51.66,
both on the plate).

**Root cause — Bond's capsule, not the truck's collision.** I measured the bed through the port's own data path
before touching anything: the bed's only resting surface is part 0x020003be's hull with its floor top at 37.28
(the visual mesh and the block-0x2E collision are the same hull, AABB 36.248..39.496; no surface exists at ~36.8
under the park spot). The game's standing pos-to-floor is 1.086 (trace level start: obj+0x30 24.963 over floor
23.877, the floor measured through the port) and the port matches it at 1.096 — the convention is right. But at
the park the trace's pos−floor is **0.607** — 0.48 below the standing offset, where no collision surface exists.
The game's capsule height is **dynamic**: `obj+0xCC = obj+0x3C` every tick ([M] `Player_Update` 0x193600:449,
`Player_CollisionHandler` 0x1949a8:182) — the stance's root height above the feet, written by the stance/anim
(`Player_StandAtNewPosition` 0x190d88 takes it from the placement vec4's W; `Player_Enable` 0x195498 copies the
free script's matrix translation row, W included, onto obj+0x30..0x3C). The trace's ride is **crouched**: park
pos−floor 0.607 minus the 0.051 standing rest offset ⇒ cc ≈ 0.556 ≈ the capsule radius. The hand-off bounce in
the trace (f33800–33900, y 19.7..20.95) is the stance transition. The port had a FIXED standing capsule
(ROOT_HEIGHT 1.0347), so even the crouched ride parked at floor + 1.135 = 38.41, and the ch 0x70 touch (a flat
quad at y 37.7418 [D script stream 6 key 8]; the touch capsule runs from pos+0.15 up, radius 0.55) missed it by
0.234.

**The fix** (crates/nf-game/src/player.rs, riding/capsule only): `Bond::cc` — the stance-driven capsule height,
standing 1.0347183 [L], crouched 0.55 [L trace-derived ≈ the radius]; the capsule ends, both floor rays and the
grounded test read it; the crouch toggle swaps it. (Simplification tagged [S] in the code: the game's cc follows
the anim pose per tick, the port per stance — both land the same capsule in the bed.)

## What's verified and how

- `crates/nf-view/tests/fix_truck_live.rs` (new, NF_ISO): `truck_ride_parks_bond_at_the_game_height_and_ends_the_level`
  drives the real path (`nf_view::load`, the viewer's `--bond` setup, the crouch through the real input toggle)
  and asserts the park height ±0.05 of 37.887, the plate-touch geometry, the full chain ORDER against the trace
  (0x22 → 0x72 → 0x70+0x65 same tick → 0x73 → 0x1D → level_exit 0x07000006). **Fail-before shown**: on the
  pre-fix player.rs the same test parks Bond at 38.414 and panics.
- Sister: `standing_ride_never_touches_the_parking_plate` — the standing ride never fires 0x70, and the
  0.23 touch-capsule miss is asserted numerically (proves the plate assertion can fail).
- The harness gap is closed: `level_reload_live::truck_ride_to_the_castle_ends_the_level` is **un-ignored and
  passes**. The gap was the setup path — the test assigned `sim.bond.pos` on the level-start Bond, while the
  viewer's `--bond` (which does trigger the gatehouse cut-scene) replaces the whole Bond with `Bond::new`; the
  test now uses the same `Bond::new` setup plus the crouch, like the trace's play.
- Full suite on the slim ISO (`NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p
  nf-formats -p nf-audio -p nf-view -- --test-threads=1`): everything green except the 3 allowed slim-ISO
  `disc_golden` failures (bigf_header_fields / bigf_tables / directory_tree).
- Docs: `docs/systems/player.md` (the capsule section now documents the dynamic cc), `docs/WORKLOG.md`
  (the fix-truck entry, including what failed).

## What's left

- **The other playtest items in the brief were not started** — the sandbox went into the truck park and the
  capsule mechanism, which consumed it. Each needs its own session: the 07000002 bathhouse C2 guard (fix-drone4's
  evidence pass recorded what it ruled out; the next step there is walking Bond over the real ch-21 plates and
  re-dumping), the 07000005 patrol spine-flip re-check, the 07000006 stationary guards, the searchlight alarm
  sound/music event, the 07000007 guest-walk retest (fix-drone4's CivilianMission port is on master — retest in
  play first), and the grenade work (explosion sound/visual per type, the flashbang stun states) — the grenade
  items live outside this task's file list (weapon.rs, searchlight.rs).
- **Who crouches Bond for the ride is unproven** — the trace has no button columns (pad axes only), so the
  crouch could be a scripted anim on the player or the player's own toggle; both land the same capsule, and the
  port models the stance. Tagged [S] in the code and the WORKLOG.
- The park xz differs by ~0.4 (the port parks at z 52.14, the game at 51.66) — the hand-off xz and the
  inherit-velocity slide; both are on the plate, and the task's bar was the height. Left as is.
- If the main agent merges s/fix-cutscenes, note its level-start intros now play on 01/02 — that branch's
  fix_cutscenes_live tests are independent of this branch (no shared files).
---

## Supplementary session (merged into the same branch)

A second fix-truck session worked the same trace in parallel and landed these pieces on top of the fix
above; both are in the branch now.

**The hand-off enable is game-exact ([M 0x195498]).** `Player_Enable` (0x195498:28-46) writes obj+0x30
from the stored matrix **directly** (a 16-byte copy MATRIX+0x30→obj+0x30..0x3C, W included), sets the
spherical angles from `Mat_GetDir` + `vecutil_cartesian_to_spherical_acc`, zeroes the velocities
(`Vec_Zero` BL+0x10/+0x20/+0x30/+0x40), and its only collision pass is the `Collide_SphereIntersect`
wire/creep-wall dispatch (',' → `Player_CollWire`, '+' → `Player_CollCreepWall`) — no floor snap.
`crates/nf-view/src/scene.rs`'s `PlayerEnable` handler now places Bond with `Bond::new` at the matrix
(the old `Bond::stand_at` re-snap with a ray from matrix y + 1.0 was port-invented). Event 6's storage
(Script_EventHandler 0x1d94c0:80-84, 123-127 — the PREVIOUS stream's object, `glb_players+0x90` when
there is none) and `Script_Free` (0x1da880) were re-verified and already matched the port.

**Independent audit (read-only, line-cited): all five decompile claims VERIFIED.** (1) Player_Enable's
direct placement and SphereIntersect-only collision; (2) Player_Disable's two arms; (3) Script_Free's
0x80/0x100 flag arms (note: the kill counter is zeroed at +0xab3 first); (4) event 6's previous-stream
(idx−1) storage, triple-checked against Script_Free's +0xA8 stream base, Script_AnimStart's +0x10/+0x24
writes and Script_KillStream's layout; (5) Script_AnimStart's new-object root placement with no player
special case (AnimGetFirstTrans writes only with sAnimScript+0x78 bit 0x800000). Caveats: the dispatched
'+' (creep-wall) handler does its own ray Y snap — irrelevant to the truck bed.

**Probes and the superseded diagnosis (crates/nf-view/tests/level_reload_live.rs).** The second session
first root-caused the park height as a bed-rails stand reached because the game's plate-to-truck walk (SP
#298, 0x06000994 length 1400, stream 14 animating Bond's skin 0x05000007 with 0x06000995) never runs in
the port, and Bond is freed on the road mid-route at the SP #38 matrix (92.25, 19.42, 103.58). That
diagnosis is SUPERSEDED by the stance-capsule fix above (the trace's ride is crouched; the walk mechanism
is real but the crouch is what moves the park height). The raw evidence dumps stay in the test file
(rest-triangle, hand-in, park surfaces, script/stream dumps) with a note; the un-ignored
`truck_ride_to_the_castle_ends_the_level` and `fix_truck_live.rs` carry the passing assertions.

**Suite:** the merged branch runs green on the slim ISO — nf-game 129 passed / 0 failed, nf-formats
72 passed / 3 failed (the 3 allowed slim-ISO disc_golden), nf-audio 7/0, nf-view all test files green
including both fix-truck files.

