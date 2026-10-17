# ps2sinf-remaining — GAPS G-046 closed: the last host-trig call sites

Branch `s/ps2sinf-remaining` (from origin/master 170bcf01). Code: `crates/nf-game/src/{trig,bullet,player,
creepwall,wire,ladder,pickup,searchlight,anim,door,drone,world_fx}.rs`. Docs: `docs/GAPS.md` (G-046),
`docs/WORKLOG.md` (appended). No shared file rewritten; no RNG draw added anywhere; no seed changed.

## What the census found, and what was swapped

Every remaining `f32::sin/cos/atan2/acos/asin/tan` in `nf-game` was re-classified against
`work/decomp/ACTION.ELF` (disassembly where the C looked incomplete). The prior surveys (fix-exact-sine-flip
2026-10-15, exact-sine-sites-2, exact-sine-portonly-review) had already swapped the exact-math core, the
missile steering, and the head-track/aim pose; the sites below were still host trig. All [M] facts verified
in the decompile/disassembly this session; constants [L] in the EE snapshot.

**New in `trig.rs`: `acosf_libm`** = `__ieee754_acosf` 0x233228 (fdlibm e_acosf; `acosf` 0x232c48 is its
errno wrapper), ported operation-for-operation on the EE FPU: |x|=1 → 0 / π (the decompile's folded
`pi + 2·pio2_lo` = 0x40490fdb); |x|>1 → (x−x)/(x−x); |x|<2⁻²⁴ → π/2; |x|<0.5 → π/2_hi − (x − (π/2_lo − x·P/Q));
x<−0.5 → π − 2(s + (r·s − π/2_lo)); x>0.5 → 2·(df + r·s + (z−df²)/(s+df)), df = s with the low 12 mantissa
bits cleared. The P/Q Horner order is the game's compiled one (P ends with the outermost ×t; Q's four t-muls
end at +1.0). Constants [L ee_snapshot 0x30b4b0-0x30b4dc] (pio2_hi/lo, pS0-pS5, qS1-qS4). Unit test
`trig.rs::acosf_libm_tracks_host_libm_and_differs_from_it` pins ≤2ulp-vs-host tracking AND real divergence.

**Swapped sites** (each cites its game function):

| Site (port) | Game [M] | What changed |
|---|---|---|
| `bullet::spread` (bullet.rs) | Bullet_init 0x12c5a0:171-180 → Vec_Spherical_2_Cartesian 0x1e3948 | **Cone rebuilt, not just re-rounded**: the old form had θ/φ roles swapped (`[r·sinφ·cosθ, r·cosφ, r·sinφ·sinθ]`). The game's off = `((r·sinθ)·cosφ, r·sinφ, (r·cosθ)·cosφ)` [M disasm 0x1e39a4-0x1e39d4; the y-offset is r·sinφ — biased upward]. Products are EE mul.s; the r·0.0014 also a mul.s. RNG draw order unchanged (FRand(2s), FRand(TAU), FRand(PI)) [M 0x12c5a0:166-169]. |
| `Bond::look` (player.rs) | RotMatrixX 0x1e46a0:16-24 + Mat_GetDir (Player_Move 0x191dd0) | PS2Sinf2 lanes via `trig::sinf/cosf`; the pitch arg `pitch·π/2` is a mul.s (ps2float::mul). |
| movement axes (player.rs tick) | View_RotTransMatrix 0x1e5168:33-36 / RotMatrixX 0x1e46a0 | `(sinf(yaw), cosf(yaw))` — downstream speed products stay host (documented residual). |
| ladder grab snap (player.rs) | Player_CollLadder 0x196118:31-38 | The game's own arguments: `a = yaw+π` (add.s); x = centre − sinf(a)·0.5, z = centre − sinf(a+1.5707964)·0.5, EE mul/sub. Old form used sin/cos of the raw yaw (math-equal, bits-different). |
| creep offset (player.rs creep_tick) | Player_Creep 0x1969d0:118-125 | `crawl_yaw = ee_add(w.yaw, π)` (stored to obj+0x54 [M]); pos = closest + sinf(yaw)·0.275 / sinf(yaw+1.5707964)·0.275, EE ops. |
| wire sway (player.rs wire_anim_add) | Player_Wire 0x197658:194-204, Player_CollWire 0x197130:228 | The lanes are `PS2Sinf2(−yaw+1.5707964, −yaw)` → pos −= lane·(−sway_sign·0.0542) (EE mul/sub). Old form used ±cos/sin of +yaw (math-equal, bits-different). |
| `Bond::sphere_centre` (player.rs) | Control_BuildWorldSph 0x133070 | Yaw rows `sinf/cosf` (the stored matrix is PS2Sinf-built [M]); the VU0 `_vmulabc/_vmaddabc` MAC chain stays host (residual noted). |
| world spheres (wire.rs create/icons, ladder.rs create, player.rs hint gates) | the placement object matrix (0x133070's source; nf-formats::placement::object_matrix already runs anim::sinf) | `sinf/cosf` yaw rows. |
| creepwall endpoints (creepwall.rs) | Player_CollCreepWall 0x196758:80-90 → Vec_Spherical_2_Cartesian | θ = ee_add(yaw, π/2); **the φ=0 lane still multiplies by cos(0) = sinf(1.5707964) = 0x3f800001** — a real factor the port dropped; ± sums EE. |
| pickup spin (pickup.rs) | Pickup_Update 0x18e148:193-201 → RotMatrix 0x1e2368:28-31 | REC_FRAME_RATE = 0x3c888889 [L ee_snapshot 0x30d0dc] (was `1.0/60.0` — same value, now cited); PS2Sinf3 lanes; the row products/sums EE mul/add. |
| searchlight steer (searchlight.rs state 1) | Searchlight_Update 0x1cb900:150,166 | `atan2f_libm(dx, dz)` and `atan2f_libm(dy−1.0, √(EE(dx²)+EE(dz²)))` (sqrt.s) — the comment already knew it was libm atan2f, the code didn't. |
| `anim::slerp` (nf-game) | Spline_Interp 0x1e1858:285-300 | `acosf_libm`; wt = mul(angle, t); weights = sinf(±)·(1/sinf(angle)) — the game's div.s reciprocal then mul.s (was per-weight host division); the angle−wt sub is EE. |
| `door.rs slerp` | Spline_Interp 0x1e1858:285-300 | Same (the two port copies had even diverged from each other — door.rs divided, anim.rs recip-mul'd). |
| `anim::root_yaw` | Vec_DirToRot 0x1e3218:8 | `atan2f_libm`. |
| `anim::unyaw` | Quat_AxisAngleToQuat 0x1e3f20's PS2Sinf2 lanes | w = sinf(−yaw/2 + 1.5707964), y = sinf(−yaw/2) [the θ=−yaw mapping [S] — the helper's game counterpart is otherwise unresolved, G-046's UNCLEAR note]. |
| drone head-track measurement (drone.rs) | AnimObjectHeadTrack 0x1bf618:189-281 | The two acos blocks run the game's `acosf_libm` (was host acos). |
| world_fx emitter cone | Emitter_Update 0x17b080 (disasm 0x17b4d0-0x17b53c) | **Cone rebuilt**: the stored dir is `(sin p·cos t, cos p, sin p·sin t)` — polar p from +Y, azimuth t from +X — then Vec_MulR32(speed). The old form (elevation from horizon, azimuth from +Z) pointed the cone differently. PS2Sinf2 lanes via trig::sinf/cosf; RNG draw order unchanged (yaw, pitch, speed, life) [M :208-230]. |

**Listed, NOT swapped** (no game trig at the site, unresolved counterpart, or other program):

- PORT_ONLY stand-ins already reviewed (exact-sine-portonly-review): ladder.rs mesh columns, player.rs crawl
  advance (:911), casing.rs `eject`'s [S] bone-rotation build (:149-150 — the game's path reads the
  AnimGetBoneWorldTrans matrix; the [S] fallback stays).
- player.rs:686/:710 wire root-motion composition: `AnimSeqTick` 0x1c28d0 / `AnimFrameResolve` 0x1c35f8
  contain **zero** trig calls [M — the motion rides stored matrices]; the port's yaw-rotation stand-in stays.
- `world.rs` GroundMotion heading (`m[2][0].atan2(m[2][2])`): the port's own derivation for movers; no game
  counterpart identified.
- `drone.rs` AI helpers (:5649 POLE gate [S]; :6574/:7133 spherical aim, :7163/:7178, :8634, :8986): port-side
  AI stand-ins — the spherical ones use `asin` where the game's vecutil uses atan2f; swapping them changes
  AI aim inputs and belongs to an AI-focused task with its own fail-first pins.
- **DRIVING.ELF** (`driving/physics.rs:496` tan, `driving/ai.rs:323/402` atan2): DRIVING.ELF's decompile has
  no trig library files (no `*sin*`/`*atan*`/`*tan*` decomp — the `[S]` bicycle-model yaw-rate is a
  documented port model); DRIVING-side trig is its own follow-up.
- **nf-view**: every remaining host-trig site is presentation-side (camera basis, HUD projection, fx polish,
  the panel's yaw) — none touches sim state. Render-side swaps are a renderer task (each would need the
  main agent's render verification anyway).

## How it was verified

Before/after with identical command lines, NF_ISO, `--test-threads=1`:

- **Before** (master's tree + this branch's docs, pre-swap): `trig_live` 4, `diff_calls` 6, `doors_live` 2,
  `pickup_live` 1, `searchlight_live` 3, `grenade_live` 2, `explode_live` 2 (nf-view) `fix_ladder_live` 3+1ign,
  `wire_live` 2+2ign, `ai_traces_live` 4, `walkreal_07000001_live` 2, `ai_live` 35, `fix_portonly_review_live` 3
  — all green.
- **After** (all swaps in): the same binaries — all green, same counts (no pinned trace changed state:
  walkreal 2/2, ai_traces 4/4, ai_live 35/35, drone_portcheck_live 5/5, fix_portonly_review_live 3/3,
  doors/searchlight/pickup/grenade/explode/wire/ladder/facial/anim_events/rng/impact_remainder/ledge/
  weapon_switch/stun_grenade/grenade_rattle green), plus `nf-game --lib` 242/242 (includes the new acosf
  unit test).
- **Fast runner on the final tree (merged with origin/master — already up to date, 170bcf01):**
  `NF_ISO=… python3 tools/test/suite.py . target --limit 600` two chunks:
  chunk A (diff_calls, trig_live, rng_live, ai_live, ai_traces_live, walkreal_07000001_live,
  drone_portcheck_live, fix_portonly_review_live): **60 passed, 0 failed, 0 ignored (8 min)**;
  chunk B (doors_01_live, doors_live, searchlight_live, pickup_live, grenade_live, grenade_rattle_live,
  stun_grenade_live, explode_live, wire_live, fix_ladder_live, facial_live, anim_events_live,
  impact_remainder_live, weapon_switch_live, ledge_live): **36 passed, 0 failed, 4 ignored**.
  **Final-tree total: 96 passed / 0 failed / 4 ignored over 23 binaries.** (The 4 ignored are the standard
  skips; the disc_golden trio was not in these chunks — slim-ISO exceptions documented in the task.)
- What ran: `cargo build/test --offline --release` for nf-game, nf-view, nf-formats (the suite builds all
  four crates' test binaries — 449 binaries, build ok). Crates touched by the changes: **nf-game** (all of
  it), transitively nf-view's live tests (they drive the sim), nf-formats built but unchanged.

## Trace changes, before/after

No seed-pinned trace changed state (every family green before and after — the tables above are the per-site
records; the swaps are last-ulp-exact or cone-shape corrections whose pinned observables agree). The two
sites whose *behaviour* (not rounding) changed — `bullet::spread`'s cone shape and the emitter cone
direction — are covered by `grenade_live`/`explode_live`/`impact_remainder_live`/`ai_live` (the guard-miss
debris/emitter counts), all green.

## What's left

1. The drone AI spherical helpers (`asin` → the game's atan2f form) — needs AI-aim fail-first pins; G-046
   keeps a pointer.
2. DRIVING.ELF's trig (no decomp files for its maths yet).
3. nf-view's presentation-side trig (renderer task; needs the main agent's render).
4. The VU0 MAC rounding in Control_BuildWorldSph's world-sphere sums (host f32 today; needs a VU0-MAC model
   in ps2float — same class as the interpreter's pinned `div`).

## Live checks for the main agent

Visual/behavioural deltas worth one render each (both are cone-shape changes, not rounding):

1. `--bond` at a 07000001 spawn, fire the pistol at a wall ~10 m away: the impact scatter should now sit in
   the game's cone (the y-offsets biased by r·sinφ, previously cosφ). Old vs new: bullet paths near the
   muzzle differ by ≲ r = spread·0.0014·2 — sub-pixel at 10 m; the check is that shots still hit where the
   crosshair points and tracers fan naturally.
2. Any impact emitter (07000005 wall impacts with debris/gas): the puff cone should now be polar-from-+Y
   around the def's azimuth (previously elevation-from-horizon) — gas puffs rise/edge outward per the def's
   polar angle; confirm nothing sprays downward that should climb.
3. Searchlight heads on 07000006 (state 1): tracking still converges and clamps (atan2f_libm inputs are the
   same shape; only last-ulp targets changed).

## Usage

not visible to me — this environment exposes no token, credit, context-window or cost figures for the
session (no tokenizer/usage panel is available to copy numbers from).
