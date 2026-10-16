# The driving missions' AI cars — road network, path following, speed, spawn

Status: **ported** (task `drive-ai-traffic`, branch `s/drive-ai-traffic`, 2026-10-15). The port lives in
`crates/nf-game/src/driving/` (`road.rs` the RNgp/Wmap-side data model + `ai.rs` the AI car step) with the
RNgp container decode on top of `nf_formats::crp`. Evidence tags: [M] code (function + address,
DRIVING.ELF — EE address = file offset + 0x107080), [D] disc bytes, [L] EE snapshot, [S] suggested.
Everything below was read out of `work/decomp/DRIVING.ELF/` for this task; nothing from memory. The
container/carrier facts (CARP records, RNgp as the road-network article) come from
docs/formats/crp.md (task drive-crp-format).

## 1. The road network: the `RNgp` article and its five arrays [M]

The track `.crp`'s `RNgp` article ("<<RoadNetwork>>" per the namer FUN_002dc1b0 0x2dc1b0,
docs/formats/crp.md §3) is read by **`FUN_00220c28` 0x220c28** (1,744 B, "Init RoadNetwork", called
lazily through FUN_00220c18/DAT_0033a5f8 — also in the mission-setup list as "Init AI Controllers"
FUN_0013e688's neighbour FUN_00220c28, docs/atlas/driving-elf.md §1.4):

1. `FUN_002d51f0(track->0x14 /* the crp */, 0x524e6770 /* "RNgp" */)` locates the article [M 0x220c28:71].
2. Inside the RNgp window it reads **`RNhd`** (header; payload = five u16 counts) then the indexed
   short-tag records **`rn  `** (nodes), **`rs  `** (segments), **`ri  `**, **`rr  `** (routes),
   **`rj  `** [M 0x220c28:75-201]. The counts map 1:1 to the tags in the order rn/rs/ri/rr/rj
   ([M 0x220c28:82-86] reads them in that order).
3. Each array is stored as a malloc'd table of **pointers** at DAT_0033a5fc (rn), DAT_0033a600 (rs),
   DAT_0033a604 (ri), DAT_0033a608 (rj), DAT_0033a60c (rr) [M 0x220c28:91-96]. Every consumer indexes
   them as `*(int*)(idx*4 + DAT_0033a5xx)` — e.g. FUN_002247b0 0x2247b0:125,173, FUN_0012e808
   0x12e808:165 — so the port keeps `Vec<*const Rec>`-shaped `Vec<&Rec>` tables.

### 1.1 `rn  ` — road node [M]

| offset | type | meaning | proof |
|---|---|---|---|
| +0x00..+0x0c | 3× f32 | position x,y,z | FUN_00225c48 0x225c48:179-186 reads the triple; FUN_00126c98 0x126c98:178 copies 12 B |
| +0x0e | u8 | link count (outgoing segment count) | FUN_002247b0 0x2247b0:99,139-150 (`(char)(uVar4+0xe)`, loop bound) |
| +0x10 | u16[] | the linked segment indices | FUN_002270e8 0x2270e8:12-22; FUN_002247b0 0x2247b0:153,173 (`*(ushort*)(iVar16 + iVar7*2 + 0x10)`) |

### 1.2 `rs  ` — road segment (the AI spline piece) [M]

A segment joins node A to node B; a car drives it A→B or B→A. Measured fields:

| offset | type | meaning | proof |
|---|---|---|---|
| +0x0c | u16 | node A index | FUN_002215f8 0x2215f8:7-8 |
| +0x0e | u16 | node B index | same |
| +0x10 | 0x2c B | curve control data, only meaningful when flags&0x4000; FUN_00222d40 0x222d40:54 passes `rs+0x10` as the third control point into the 4-point spline init FUN_001a8380 0x1a8380 (P0=rs nodeA pos via param_3, P1=param_5=rs, P2=`rs+0x10`, P3=param_4) — a cubic 4-point path [M init + S exact interpolation order] |
| +0x3c | s16 | route (rr) index | FUN_00126c98 0x126c98:137; FUN_0013d3f0 0x13d3f0:49 |
| +0x3e | s16 | the segment's own id | compared as an id in FUN_002247b0 0x2247b0:105, FUN_002270e8 0x2270e8:16 |
| +0x40..+0x4c | 3× f32 | unit direction A→B | FUN_002216b0 0x2216b0:8-10 |
| +0x4c | f32 | **speed limit** — the AI's per-segment target speed | FUN_0012e808 0x12e808:165 (`*(float *)(*(int *)(nav->0x3e * 4 + DAT_0033a600) + 0x4c)`) |
| +0x50 | u16 | flags | 0x1 blocked-variant [M 0x2270e8:17 `(f^1)&1`]; 0x2 [M 0x1377c8:117]; 0x8 type-restricted [M 0x225490:129]; 0x10/0x800 road class → lane table [M 0x221e98]; 0x100 skip + trigger id at rs+0x59 [M 0x225490:127, 0x126c98:148]; 0x200/0x400 turn restrictions [M 0x2247b0:115-118]; 0x4000 curved [M 0x222d40/0x222468]; 0x8000 skip-type5 [M 0x225490:140] |
| +0x52 | i8 | lanes, negative side | FUN_00221f20 0x221f20:28-29 |
| +0x53 | i8 | lanes, positive side | same |
| +0x59 | u8 | trigger id (with flags&0x100) | 0x126c98:148 |
| +0x5a | i8 | car-group id (spawn filter) | FUN_00225490 0x225490:133 |
| +0x60 | u64 | runtime visit stamp (nearest-query dedup) | 0x225490:121-126 |

### 1.3 `rr  ` — route [M]

`+0x2e`/`+0x2f` u8 occupancy counters, incremented/decremented as cars enter/leave
([M 0x220c28:201-202 init, FUN_002213d8 0x2213d8:25-26, 0x126c98:134/142]). The traffic spawner weighs
routes by occupancy (FUN_0013d3f0 0x13d3f0). Rest of the record: unproven.

### 1.4 `ri  `/`rj  ` [M-negative]

Arrays are built and freed (0x220c28) but no field reader is in the decompile (`ri` is read once at
FUN_001377c8 0x1377c8:108, `rj` has none). Port keeps raw payloads.

## 2. How a car follows the road [M]

The car's path state lives at car+0x1d0 (first arg of FUN_00225c48/002257c0/002247b0):

| offset | type | meaning |
|---|---|---|
| +0x00 | u32 | kind: 1/2/4 — 4 = traffic [M 0x225c48] |
| +0x04 / +0x20 | 3× f32 | previous / current position |
| +0x30 | 3× f32 | direction |
| +0x3c | u8 | end: 0 = driving toward A, 1 = toward B [M 0x2257c0 join: `dot(dir, rs->0x40)` sign picks the end] |
| +0x3e | s16 | current segment index |
| +0x50 / +0x60 | 3× f32 | node A / node B position cache |
| +0x70 / +0x80 | 3× f32 | curve control (from rs+0x10 when flags&0x4000) [M 0x2257c0:174] |
| +0x90 | ptr | curve object (0x4191e0 template) |
| +0x95 | i8 | lane |
| +0x98/+0x9c/+0xa0 | f32 | progress along the segment |

The per-frame position: straight segments evaluate `nodeA + (nodeB − nodeA)·t`
(**FUN_00222468 0x222468:12-20**, VU `_vsub/_vmulbc/_vadd`), curved segments go through
**FUN_00222d40 0x222d40** → the 4-point path **FUN_001a8380 0x1a8380** + **FUN_001a85d0 0x1a85d0**
into the curve object at 0x4191e0 (lazy-init once, DAT_0033a958 [M 0x222d40:49-53]).
Joining a segment: **FUN_00225c48 0x225c48** (2,288 B) — `dot(car dir, rs->0x40 dir)` picks the
direction (end A/B), the lane offset comes from **FUN_00221f20 0x221f20** (lane-offset table at
DAT_00427000, 6 entries built by FUN_00220c28:24-61, selected from rs flags by FUN_00221e98,
clamped by rs+0x52/+0x53) [M].
At the segment end: **FUN_002247b0 0x2247b0** (1,616 B) picks the next segment — walks the exit
node's u16 list (rn+0x10, count rn+0x0e), filters rs flags 0x2/0x80(trigger)/0x200/0x400, XORs the
candidate's node indices against the entry node to detect direction reversal, and returns the next
segment + lane [M 0x2247b0:99-255].
The nearest-segment query (used to find the segment under a world position) is
**FUN_00225490 0x225490** (816 B): world→cell through the CDat `CGrd` grid (DAT_0033a5c8; cell
payload `cn  ` +0x7 count, +0xe u16 offset into a s16[] of segment indices — the road net reuses the
collision grid [M 0x225490:104-120, FUN_0021c1f8 0x21c1f8:48-50]), dedup via rs+0x60 stamp vs
DAT_0033a610, closest point via FUN_00222968→FUN_00222a30 (returns t and position), curve distance
via FUN_002223b0 when flags&0x4000 [M].

## 3. The AI car object and its per-frame step [M]

`AIVehicle` (RTTI name [D]; 0x2a0 B, ctor FUN_0012dd80 → FUN_0013de68 + FUN_0012ded8):

| offset | type | meaning | proof |
|---|---|---|---|
| +0x04 | ptr | the actor (vtbl at actor+0x68: +0xd4 SetSteer f32, +0xf4 SetThrottle f32, +0xfc SetBrake f32, +0x104 SetHandbrake i32, +0x9c mark-updated) | FUN_00133ab0 0x133ab0:136-143 call sites |
| +0x54 | 0xc0 B | WRoadNav "current" (road-node/segment id at nav+0x3e; nav+0x90 holds a 0x70 spline) | 0x133ab0; nav+0x3e read at 0x12e808:165 |
| +0x58/+0x5c | u32 | enable flags (Update guard) | FUN_0012e350 0x12e350 |
| +0x60 | u32 | slot id (pool slot from FUN_0013e688) | 0x13e688 |
| +0x70 | u32 | state 1..5 (1 standby, 2 ?, 3 teleport-to-marker, 4 traffic, 5 chase) | FUN_0012e350 switch; FUN_00131620 = the state-3 teleport |
| +0x78 | u32 | drive pattern 1..5 | 0x12e808 switch on +0x84's sibling |
| +0x84 | u32 | profile (1, 0xb..0xf; 9/10 = fire modes) | 0x12e808:56 (`iVar2 = *(int *)(param_1 + 0x84)`) |
| +0x8c | f32 | **target speed** (written by FUN_0012e808, read by FUN_00130508→FUN_00133ab0) | 0x12e808 stores; 0x133ab0:100 |
| +0x134 / +0x138 | f32 | speed multiplier (init 1.0) / max-speed cap | 0x12ded8 init |
| +0x164 | s16 | spawn-delay countdown | 0x131018 |
| +0x19c | u32 | forced-stop flag (≠0 → throttle 0, brake 0, handbrake 1) | 0x133ab0:144-179 |
| +0x1a4 | ptr | chase target (an AIEl element) | 0x131770 |
| +0x226/+0x22c | s16 | gun cooldowns; +0x228 f32 fire rate | 0x12ded8/0x130050 |

Per-frame: the world scheduler (FUN_00168280 0x168280, docs/atlas/driving-elf.md §1.2) calls the
vtable+0x1c Update = **FUN_0012e350 0x12e350** (280 B): guards +0x58/+0x5c, honors the +0x1a0 reset,
then switches on state +0x70:

- **1 standby**: FUN_00131018 (activate the sim object when the spawn delay +0x164 counts out).
- **2**: FUN_00133850 + FUN_00131080 (1,160 B; re-targets on a 4 Hz slice, +0x174).
- **3**: FUN_00131620 (336 B) — teleport: FUN_001f7880 sets the sim transform from the AIEl marker.
- **4 traffic**: FUN_00132d80 (508 B) node-occupancy bookkeeping (init FUN_00132808 924 B).
- **5 chase**: FUN_00133850 + FUN_00131770 (4,240 B target management) + FUN_0012e808 (target speed)
  + FUN_00130050 (236 B weapons) + FUN_00130508 (2,432 B lead-aim FUN_00132f80, then the drive arms
  FUN_00133f60/FUN_00135f70/FUN_00137310/FUN_001377c8).
- States 2/4/5 share **FUN_0012e808 0x12e808** (2,408 B, target speed → +0x8c) and
  **FUN_00130508 0x130508** → **FUN_00133ab0 0x133ab0** (1,196 B, the drive-control applier).
- Tail: actor vtbl+0x9c(1) (mark-updated) [M].

### 3.1 Target speed (FUN_0012e808 0x12e808) [M]

- State 5 requires FUN_00195a30(actor) > 0 else target = 0 (dead/despawned → stop);
  state 4 requires ≥ 0.5 [M 0x12e808:9-29].
- vtbl+0x144 query ≠ 0 → 0 (some per-actor stop request); byte +0x170 ≠ 0 → 0 [M 0x12e808:41-52].
- Then per profile +0x84 / pattern: the **current segment's speed limit rs+0x4c** is the base
  [M 0x12e808:165]; observed constants in the selector: 10.0 (a minimum), 25.0, 30.0 (chase case),
  7.5 (a follow distance: physics param +7.5, FUN_00132f80), multipliers 0.35/0.5/0.75/0.8/0.85/1.1/1.25
  (the 1.25× case scales a gap by `phys0x4c8` of self+target) [M — per-case mapping not fully
  disambiguated; the profile→case table is [S]].

### 3.2 Drive control (FUN_00133ab0 0x133ab0) [M]

Inputs: target speed (param_1, after the caller multiplies by the +0x134 profile factor), the AI
object, the aim/heading vector (*param_3). Steps, in order:

1. `dir = normalize(aim − carPos)` (sim obj +0x10/0x20 positions; `_vsub/_vmul/_vrsqrt`) and three
   dot products against the car's facing vectors (sim +0x5c rows) → alignment factors [M 0x133ab0:40-96].
2. `steer = clamp(alignment, ±1)` (FUN_00210510) → **actor vtbl+0xd4 SetSteer** [M 0x133ab0:135-137].
3. **SetThrottle(0), SetBrake(0), SetHandbrake(0)** reset [M 0x133ab0:138-143].
4. `speed = |car velocity|` (sim +0x20, `_vsqrt`) [M 0x133ab0:103-113].
5. `target' = max(target · |alignment|, 0.5)` [M 0x133ab0:100-102].
6. Arrival: `radius = max(speed · 0.5, 5.0)`; if `dist(aim, carPos) ≤ radius` → `target' = 0`
   [M 0x133ab0:114-130].
7. If +0x19c (forced stop): throttle 0, brake 0, handbrake 1 [M 0x133ab0:173-180].
8. Else `ramp = min(|target' − speed|, 7.5) / 7.5`, floored at 0.05; `target' > speed` →
   **SetThrottle(ramp)**; otherwise brake-side early-outs (speed ≤ 0 → return; alignment ≥ threshold
   and target' ≥ 2.5 → return) then **SetBrake(ramp)** [M 0x133ab0:144-172].

### 3.3 Weapons (the chase states' fire) [M]

Profile +0x84 == 9 → FUN_0012f730, == 10 → FUN_0012fa28, else FUN_0012fd58 (dispatched through
FUN_00130050, gated by FUN_00195a30(actor) > 0): line-of-sight FUN_0012f530, aim-dot thresholds
0.9/0.99, then FUN_001a27a0(animObj, channel {9 | 5/6}, frames 8→10→0xb, …) — animation-channel fire
commands on the actor's +0x48 object; cooldowns +0x226/+0x22c tick down in frames × rate +0x228
[M 0x130050/0x12f730/0x12fa28/0x12fd58 call shapes].

## 4. Spawn/despawn per mission [M]

- **The AIEl database**: the `Map `→`AIEl` chunk (mission setup FUN_00176658's "Init AI Elements"
  calls FUN_001c4840 then FUN_0012be48) is parsed into the 0x58-byte database at DAT_00327c30 by
  **FUN_0012b8b0 0x12b8b0** (1,216 B): slot availability u32s at +0x00/+0x04/+0x08 (henchman/vehicle/
  heli; defaults 32/6/16 from attributes 0x36b120/0x36b138/0x36b150), the 0x100-byte record array at
  +0x0c, per-slot runtime records (0x50 B, init FUN_0012af48: activation distances 200.0/220.0).
- **AIEl record** (0x100 B): +0x44 s32 type (0 = henchman, 1 = vehicle); +0x48 s32 must equal 1 to
  activate; +0x50 ptr to the linked target element (the chase target); +0x4c f32 scale (× the global
  FUN_002084a8); +0x54 char[11] model name ("rendername" attr → `data\car\model\`); +0x7c sub-count,
  subs 0x20 B at +0x80 [M FUN_0012b8b0 + FUN_0012cb08 attr loader].
- **The pool**: "Init AI Controllers" **FUN_0013e688 0x13e688** (436 B) preallocates the AIVehicle
  objects: VEHICLE_SLOTS_AVAIL (6) heap + **22 static** (2 at 0x327d90 + 20 at 0x3282d0, reset by
  FUN_0013ff98) + 16 AIHelicopter (0x130 B) [M].
- **Activate**: a trigger/script (FUN_0014ae60 through the manager at *DAT_00339f5c, vtbl+0x5c
  GetFree) → **vtbl+0x24 FUN_00131508** (sets the +0x58/+0x5c enables and state 3 = teleport to the
  AIEl marker; config from record+0x50's linked target) [M].
- **Deactivate**: FUN_0014c110 → vtbl+0x2c **FUN_00131590** → state 1 standby [M].
- The traffic director **FUN_00126c98 0x126c98** (2,136 B) is the per-frame traffic spawner: picks a
  spawn segment by route occupancy (FUN_0013d3f0), updates rr+0x2e/0x2f, reads rs+0x59's trigger, and
  teleports the car to `nodeB + rs dir·0.75` [M 0x126c98:134-208].

## 5. The port

`crates/nf-game/src/driving/road.rs`: `RoadNet::parse_rngp(&[u8])` — the RNhd counts + the rn/rs/ri/rr/rj
record tables exactly as FUN_00220c28 builds them (records kept as byte slices with typed accessors,
every offset from §1); `RoadNode`/`RoadSeg` accessors (`node_pos`, `seg_a`, `seg_b`, `seg_dir`,
`seg_speed_limit`, `seg_flags`, `seg_lanes`, `links`), the straight-segment position evaluation
(FUN_00222468), the join direction rule (FUN_00225c48's dot test), and the next-segment choice
(FUN_002247b0's filter chain: flags 0x2/0x200/0x400, self-id skip, direction reversal by node XOR).
`crates/nf-game/src/driving/ai.rs`: `AiCar` — the §3 subset a headless sim needs: state, target speed
(FUN_0012e808's traffic path: the segment limit rs+0x4c, the alive gates), and the FUN_00133ab0 drive
control (steer clamp, throttle ramp min(|Δ|,7.5)/7.5 floor 0.05, arrival radius max(speed·0.5, 5.0)),
stepping position along the segment at the game's 60 Hz. No drawing, no RNG draws.

Tests (`crates/nf-game/tests/driving_ai_follow.rs`): a synthetic but format-exact RNgp fixture
(straight segments + a curved one), an AI car entering at a node, stepping at 1/60 s:
- it converges onto the segment and reaches the segment's speed limit (rs+0x4c) via the game's ramp;
- it crosses to the next segment at the shared node and keeps the direction rule (A→B vs B→A);
- the arrival rule stops it inside `max(speed·0.5, 5.0)` of the aim;
- the 0x200/0x400 flag filters and the car-group check exclude segments exactly like FUN_002247b0;
- a sister assertion per test (a wrong-format fixture must fail).
ISO/DRIVING-gated (`NF_DRIVING` set, like driving_golden.rs): parse the real `MIS01.VIV` track RNgp,
walk an AI car along a real segment chain, pin the real record counts (numbers only).

## 6. Live checks for the main agent

- **L-1 — rs+0x4c is the speed limit**: breakpoint FUN_0012e808 0x12e808 during a traffic mission;
  confirm the float read at `*(rs)+0x4c` lands in +0x8c and matches the mission's traffic speed
  (paris_mis01, state-4 cars) [M expectation].
- **L-2 — lane offsets**: dump the 6-entry lane-offset table at DAT_00427000 after "Init AI
  Controllers"; expect lane-center offsets in game units [M expectation; the port uses the table's
  existence, not its values].
- **L-3 — curve eval order**: breakpoint FUN_001a8380 0x1a8380 on a curved segment (flags&0x4000);
  record the four control points passed (param_2/3/5/4) to confirm P0/P1/P2/P3 ordering vs rs+0x10
  [S expectation].
- **L-4 — real RNgp golden**: run `NF_DRIVING=<extracted DRIVING folder> cargo test -p nf-game
  --release driving_ai` — the gated tests parse the real paris_mis01 RNgp; confirm the counts against
  a PCSX2 dump of DAT_0033a5fc..'s array lengths after "Init RoadNetwork" [M expectation].

## 7. What's left / not done

- The per-case mapping of profile +0x84 → FUN_0012e808's constants (10.0/25.0/30.0 and the 0.35..1.25
  multipliers) — the selector is 2,408 B with a nested pattern switch; only the traffic base rule
  (rs+0x4c) is pinned [M]; the rest is [S].
- `ri`/`rj`/`rr` non-occupancy fields (§1.4).
- The curve interpolation of FUN_001a85d0 0x1a85d0 (the port carries straight segments + the control
  data; exact curve eval is a live check, L-3).
- Weapons fire (§3.3) is documented, not ported (needs the actor/animation layer).
- The AIEl chunk binary layout inside the `.crp` (record +0x44.. is proven from the parsed runtime
  records; the on-disc AIEl chunk record layout itself is [S] — the port reads AIEl only through the
  documented slots, real-data AIEl parsing is a live check).
- 506's physics (s/drive-physics) landed beside this: the seam is wired as of `drive-ai-traffic2`
  (2026-10-15) — see §8 (`SimCar::of_car` + `AiDriver`, no trait needed).

## 8. The physics wiring (`drive-ai-traffic2`, 2026-10-15)

507 landed the AI beside 506's physics with `SimCar` as the unwired seam; 506 then merged as the
single file `driving.rs`, and `drive-ai-traffic2` moved it to `driving/physics.rs` (507's
`driving/` directory had made the two layouts collide, rustc E0761) with a top-level re-export
keeping every public path (`nf_game::driving::{Car, CarInput, CarParams, DT, FlatWorld, Rigid,
Tuning, Friction, SURFACES}` — the module test pins them). The wiring:

- `ai::SimCar::of_car(&Car)` [M the field sources]: pos = instance+0x10, vel = instance+0x20,
  forward = the yaw's `[sin, 0, cos]` row (the row `Car::step` drives along [M physics `step`'s
  `fwd`]); `alive` = 1.0 — FUN_00195a30's health gate is the actor layer, out of headless scope
  [S].
- `ai::AiDriver` { `brain: AiCar`, `car: Car`, `world: FlatWorld`, `rigid: Rigid` }: one 60 Hz
  step = the brain's FUN_0012e808 → FUN_00133ab0 command mapped 1:1 onto 506's `CarInput`
  (SetSteer/SetThrottle/SetBrake/SetHandbrake [M 0x133ab0:135-180]), `Car::step` integrates it
  [M 0x1ff3d8's order], the **pre-integration** sim speed drives the path progress (the AI reads
  |vel| before the vehicle sim integrates [M 0x133ab0:103-113]; `AiCar::advance_path` is the
  progress walk moved verbatim out of `AiCar::step`, which keeps its exact rail behavior), then
  the car lands back on the spline [M 0x222468] facing the travel direction.
- The brake rest clamp [S]: 506's brake integrates through zero into a negative vel (reverse is
  the R-gear throttle path 506's gearbox work owns), which would back the rail car up; the driver
  zeroes a brake-induced backwards velocity, and at rest the AI's own speed ≤ 0 early-out
  [M 0x133ab0:153] keeps it parked.
- Known gap: `Car::step` does not read `handbrake` yet (506's report lists "handbrake in the
  step") — the forced-stop command is exact [M 0x133ab0:173-180] but holds speed by no force;
  the test `the_forced_stop_handbrakes_and_the_speed_holds_until_506s_handbrake_lands` fails the
  day 506's handbrake lands, on purpose.
- Tests: `crates/nf-game/tests/driving_ai_physics.rs` (8) — the layout paths, the sim view, the
  acceleration numbers with a bit-exact bare-`Car` replay (the driver adds no physics of its
  own), the steering-angle falloff, the arrival rest, the forced-stop record, the clean-link
  crossing at the physics speed, standby.
