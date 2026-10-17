# scary-link-routing (738) — the AINetwork link flags

Branch `s/scary-link-routing`. The task asked the port's route search to honour the link flags
that `AINetwork_SetLinksFlagInCircle` (0x15b548) sets inside danger radii. The decompile work
changed the shape of that ask: **the flags exist, the port lacked them, and the game's route
search never reads them** — so "honour the flags the game's way" means carrying them faithfully and
NOT inventing an A* avoidance the shipped game does not have. Everything below is cited.

## What the game actually does (decompile census)

**The flag store.** The AI network's link array sits at `*(net+0x3c)` (`net` = the per-cel network
records at 0x2c7964, count at 0x2c7960 [M 0x15b548/0x15b498 walks]); each link is 0x10 bytes:
`+0/+2` ushorts, **`+4` = the flag halfword**, `+6` = the used counter, `+8/+0xa` = the endpoint
node ids, `+0xc` = the length float [M `AINetwork_DoAStarPath` 0x1571b0 — disasm 0x1575e4
`lhu v1, 6($v0)`, 0x1575e8 `lwc1 f1, 0xc($v0)`; `ClearLinkFlags` zeroes `+6` in the same walk,
0x15b498:24-27]. The A* edge cost is `len * (used + 1)` [M 0x1575e0-0x157600].

**Writers of `+4`** (every `sh rt, 4(rs)` site in the binary maps to one of these):
- `AINetwork_ClearLinkFlags(mask)` 0x15b498 — `flags &= ~mask` and `used = 0` for every link of
  every network. Called from `Drone_InitComms` 0x139520:125 with `0x310` (= 0x100 | 0x200), once
  per comms cycle, immediately before `Drone_BuildDynamicAwarePoints` [M 0x139520:125-126].
- `AINetwork_SetLinksFlagInCircle(pos, net, radius, set, clear)` 0x15b548 — float radius in `$f12`,
  masks in `$a2/$a3` (call sites 0x138d58 `f12 = 12.0, a2 = 0x100, a3 = 0`, 0x138ddc, 0x138e4c;
  the caller's `lui at, 0x4140; mtc1 at, f12` at 0x138d08-0x138d0c pins the convention). For every
  network: if `clear != 0`, clear it from EVERY link [M 0x15b548:31-33]; then per link, gate on the
  height range — `min(zA,zB) <= pos.z + 2.0 && pos.z - 2.0 <= max(zA,zB)` (the game's z is the
  port's y) [M 0x15b548:51-63] — then the closest point on the endpoint segment and
  `Vec_SqDist2D <= radius²` [M 0x15b548:65-77] → `flags |= set`.
- `LinkCreep_ForNodes` 0x159810 (`|= 0x10` — the creep marker on the link being walked) and
  `LinkCreep_Calc` 0x159b10 (`&= ~0x10` on the previous one).

**Who flags what.** `Drone_BuildDynamicAwarePoints` 0x138b10 walks the whole object list and
switches on the weapon-record id of every class-5 object (`Bullet_init` 0x12c5a0:87 sets
`obj+0xFF = 5` — bullets AND grenades are class 5):
- 0x2a, 0x2c, 0x2d, 0x2e, 0x2f (the projectile weapons — Torpedo 0x2c/0x2d, Scorpion 0x2e,
  Unguided 0x2f [L ee_start_05.bin]) → radius 12.0, flag 0x100 [M 0x138b10:36-52];
- 0x34 frag, 0x6c and 0x3b-0x40 satchel → 12.0 / 0x100 [M 0x138b10:100-141], skipped when the
  record's +0xFC float ≥ 180.0;
- 0x36 smoke → 3.0 / 0x200 [M 0x138b10:143-159]; 0x37 mine → 8.0 / 0x100 unless `obj+0xF4 == 3`
  (stuck) [M 0x138b10:150-152];
- **no case for 0x35 (stun) or 0x3a (tripbomb)** — they never flag;
- class 0x2a (`'*'`) objects → 3.0 / 0x200 twice [M 0x138b10:24-45].

The same walk also fills the aware-point table (DAT_00299e20, max 0x40) that
`DroneFunc_HandleExplosives` 0x1435f0 scans — the flee broadcast the port already models
(G-026, drone.rs explosive scan + states 0x86/0x87/0x88).

**The census result: no reader.** No instruction in ACTION.ELF reads `MAPLINK+4`:
- every function that touches `net+0x3c` was read end to end (`DoAStarPath`, `EmitPath`,
  `Emitter_GetNodeAtDistance`, `GetRouteDistance*`, `OptimiseRoute`, `InitAIPath`, `SetupNextNode`,
  `FollowRoute`/`UpdateLinksUsedCount`, `LinkCreep_*`, `NodesForPosition` — only `+6/+8/+0xa/+0xc`
  and the node-info flags at `*(node+4)+4` are ever loaded);
- a full-text scan for `lhu/lh/lw rt, 4(rs)` with any flag mask nearby hits nothing in the AI
  ranges; the only `lhu +4` sites there are the LinkCreep/Clear/Set read-modify-writes plus two
  false friends — `NDrone2_ReachedDestNode` 0x153598 and `NDrone2_ReactToDroneAlertMsg` 0x143824,
  which read `SMI+4` (the current state id) into `drone+0x5A6`/`+0x5A0`, not link flags
  (disasm 0x153588-0x15359c: `lw a0, 0xc($s2)` = the SMI from DCVars+0xc);
- the 64-bit loads in the route range are node-info dwords (0x1565e0), not links.

So the shipped game's A* "honours" the scary flags by not honouring them: they are per-cycle
write-only state, and guard avoidance of grenades runs entirely through the aware-point flee
system (already ported). The audit's premise ("A* ignores the link flags, so guards path straight
through grenade radii") is refuted at the decompile level — the game's A* ignores them too.
Per the task rules the decompile is decisive, so the port must not grow an avoidance the game
lacks; the sister test below pins that.

## What was ported (all of the game's link-flag machinery)

- `nav::LinkFlags` (per-path `Vec<u16>` mirrors of `MAPLINK+4`, living next to the existing `used`
  slices — the game keeps both on the same shared link records) [M layout 0x15b498/0x15b548].
- `Network::clear_link_flags(flags, mask)` [M 0x15b498] and
  `Network::set_links_flag_in_circle(flags, pos, radius, set, clear)` [M 0x15b548:24-77] — the
  exact geometry: clear-all first, the ±2 height gate, closest-point-on-segment, 2D squared
  distance, per-path walk (the port is one cel — the header's standing deviation).
- `Drones.link_flags` / `Drone.link_flags` with the same lend/swap discipline as `used`
  (drone.rs, the two `std::mem::take` sites).
- `hazard_link_flag(weapon_type, stuck)` — the record-id switch above, `None` for 0x35/0x3a and
  for a stuck 0x37 [M 0x138b10:36-160]. [S] unverified: the +0xFC ≥ 180° gate has no port
  counterpart (the port's thrown objects carry no such field); it cannot change routing either
  way because nothing reads the flags.
- The per-cycle wiring at the port's `Drone_InitComms` pass (drone.rs, right after the existing
  `used` rebuild, which was already the `+6` half of `ClearLinkFlags`'s walk): rebuild the flag
  slices zeroed, then flag from every live grenade (`state < 2`) and every live bullet
  (`state < 2` / `!dead`) of the switch's ids.

The A* and the route-follow code are UNCHANGED — deliberately (see the sister test).

## Verification

- `cargo test --offline --release -p nf-game -- --test-threads=1`: 244 lib + 35 + 4 + ... all
  green, including the three new tests:
  - `nav::tests::scary_links_flag_inside_the_radius_and_the_cycle_clears_them` — the flag lands
    only inside the radius, the ±2 height gate rejects a link 3 units above the hazard, the
    closest-point gate rejects a hazard 5 units off the segment, one call with
    `clear=0x100, set=0x200` wipes-then-sets, and `clear_link_flags(0x310)` restores zero — the
    exact 0x15b548/0x15b498 behaviour. This is the regression test that fails without the change
    (the fns did not exist).
  - `nav::tests::astar_still_walks_a_scary_flagged_link` — the sister: with link 0-1 flagged 0x100,
    the A* still returns the short route through it. Cited in-code to the census; a future
    "avoid scary links in A*" change contradicts the decompile and fails here.
  - `drone::tests::hazard_link_flag_matches_the_record_switch` — the record-id table, including
    the None cases and the stuck-mine skip.
- The full fast suite (`tools/test/suite.py`, NF_ISO set) — see the total line below.
- Structure cross-check against runtime data [L ee_snapshot.bin / ee_start_05.bin]: 6 networks,
  node info flags at `*(node+4)+4` (0x40000000 leftovers, 0x1/0x2 disc bits, 0x10000000/0x20000000
  AITarget goal bits), link flags all 0 at rest except one 0x20 — consistent with transient,
  write-only flags.

## What's left

- The class-0x2a (`'*'`) gas-object flagging (3.0 / 0x200): the port has no gas-object walk at the
  InitComms pass yet; when gas entities get a port-side walk they should feed
  `set_links_flag_in_circle(3.0, 0x200)` [M 0x138b10:24-45; the port-side object mapping is [S]].
- The LinkCreep 0x10 creep marker (0x159810/0x159b10): write-only, no reader, no routing effect —
  not ported; noted so a future worker doesn't mistake it for missing scary-link plumbing.
- The +0xFC ≥ 180° gate and the class-0x2a aware points themselves (type 0/1 points) remain with
  the documented direct-scan deviation.

## Live checks for the main agent

1. **The flag write** — throw a frag grenade (weapon 0x34) near a guard route in 0700000a and
   gdbwatch a link flag halfword: links base 0x009d0224 (net[2] of the ee_start_0700000a-era
   snapshot), flags at `base + link*0x10 + 4`, u16 WRITE access. Expected: 0x100 while the grenade
   lives, back to 0 on the next cycle after it despawns (the 0x310 reset).
2. **The no-reader proof** — full-trace the same address for READ (lhu/lh/lw) accesses during the
   throw + flee. Expected: writes only from 0x15b498/0x15b548/0x159810/0x159b10; NO reads outside
   those (this is the census's load-bearing claim).
3. **The A*-ignores-flags confirmation** — with the grenade sitting on the guard's only route to
   Bond, watch the guard: expected behaviour is the flee/alert reaction (msg 0x21 family), and if
   he re-paths, the route still crosses the flagged links (no detour appears from the flags).

## Usage

not visible to me
