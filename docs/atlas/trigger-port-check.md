# Trigger/door port-check — atlas-3 findings vs `crates/nf-game/src/{triggers,door}.rs`

Verified disagreements between the game's code (docs/atlas/trigger.md, door.md, channels.md) and
the current port. Nothing here was fixed in Rust — this is the worklist. Line numbers refer to
master at the time of writing (triggers.rs 371 lines, door.rs 853 lines).

## triggers.rs

| # | where | game behaviour [M] | port behaviour | impact |
|---|---|---|---|---|
| 1 | triggers.rs:152 (`from_params`, LOAD_LEVEL `mask = q(9)`) | `Trigger_LoadLevelCreate` (0x1b2950) **forces mask = 0xE** (disasm `addiu t0,zero,14` @0x1b299c → tag+0x50) | takes param 9 | inert today (mask unused on the LL path) but the stored data is wrong |
| 2 | triggers.rs:192-201 (`hit`, mask 0x18) | `Trigger_Hit` (0x1b1fc8 decomp :55-123) is an **AND-chain**: every nonzero filter entry must pass (0xB needs a drone, 0xE the player, else a drone skin-hash match); with mask 0x10 and drones present the chain still runs; the touch base is `1`, or `ch[ext+0x2E]` when `mask & 1` | `.any()` over the filters; mask 0x10 short-circuits to `!drones.is_empty()`; no mask&1 gate channel; the fallback arm drops contact bit 0x02 (class-5 objects) and the whole `mask & 2` path | wrong verdicts on filters like [0xB, 0xE] (game: needs BOTH) |
| 3 | triggers.rs:242 (`kind::AND`) | MuxIn accumulator starts **1** — an empty/zero input list writes `ch[out] = 1` (0x1b1cbc `addiu a3,zero,1`, store @0x1b1d2c) | `!inputs.is_empty() && …` → writes 0 | 07000005 has no empty mux, but any future level's `MuxIn` with no inputs latches its output ON in the game |
| 4 | triggers.rs:72-81 (`Channels::set`) | the game stamps `switch_channels_time` on **every nonzero write** (trigger paths), on the **falling** edge in `Switch_Update` (0x1adfc8), never in `Drone_InitComms`; and `Init_SwitchChannels` (0x134310) **pre-seeds all 256 time cells** with the current frame counter instead of 0 | stamps on rising edge only; time starts at 0 | SEQ_AND (type 4) windows evaluate differently right after load and for re-triggered channels |
| 5 | triggers.rs:246-249 (`SEQ_AND`) | window test is `time[prev] − 4 <= time[cur]` (0x1b1e04-0x1b1e1c): the current input's stamp may be up to 4 frames **older** than the previous one's, with no upper bound | `b >= a && b − a <= 4` (current must be 0-4 frames **newer**) | opposite skew; different accept region |
| 6 | triggers.rs:237 (TouchOnce self-delete) | requires `ch[out]` on **and** `obj+0xF0 & 0x10` (0x1b1d78-0x1b1d90) | only the channel check | equivalent on live data (every TouchOnce carries glist 0x10) — model the flag for safety |
| 7 | triggers.rs:259-262 (`update`, LOAD_LEVEL) | case 10 calls the **full** `Trigger_Activate`: cancel channels 0x60-0x63 (0x1b2388-0x1b23bc), key gate + locked message + ext+0x3C cooldown, success `ch[ext+0x2C]=1` / fallback `ch[ext+0x2A]=1` — **the fallback branch still loads the level** (both branches fall through to the load @0x1b24b8) — then `Music_Event(3,1)`, `GS_PauseGame(1)`, `ResetMap_LevelToLoad(ext+0x20,1,0)`, `ext+0x38 := 99`, `Player_RamSave` | latches `gone` only: no cancel-channel check, no key gate, no success/fallback writes on the auto path | the port is more permissive (no alarm block) and skips the channel writes; U-048 noted the cancel-channel gap |
| 8 | triggers.rs:213-218 (`activate_request`, LL) | key-gate blocked → locked message + ext+0x3C cooldown, **no channel write**; gate (ext+0x28) off → **fallback write AND the load still happens** | key-gate blocked writes the fallback channel and does not load; the gate-off path is missing entirely | E-activating a gated LL with the gate off must pulse the fallback and still load |
| 9 | triggers.rs:152 (0x7D) | the game keeps a Touch on 0x7D alive on other levels (per-frame early return @0x1b1bf4-0x1b1c08) | dropped at build | same visible effect; noted for completeness |
| 10 | triggers.rs:70-73 (`Channels` struct) | `switch_channels_prev` (0x26fd90) and `_hold` (0x26fe90) are **global** arrays, snapshotted at a fixed point in the frame (after Trigger_Calc, before object updates — 0x1337cc/0x1337dc) and persisted by RamSave/RamLoad; `Door_Update`'s lock rising edge and `Player_Weapon` read `_prev`; the static cel-swap loop reads `_hold` | `prev_channels` lives ad-hoc in `Doors` (door.rs:203), snapshotted at the port's own point; no hold; nothing persists them | edge detectors can see a different frame than the game's; co-op/trace replay needs the global semantics |
| 11 | triggers.rs `hit` fallback (`m =>`) | `mask & 2` path: a class-5 contact matched against every nonzero filter entry via `*(*(ext+0x44))+2`; contact bit 0x02 | not modeled | only matters for object-touch triggers (none on 07000005) |

## door.rs

| # | where | game behaviour [M] | port behaviour | impact |
|---|---|---|---|---|
| 1 | door.rs:216-224 (`blocked`) | the auto-close and immediate-close scans use `Vec_SqDist3D(obj+0x80, other+0x80)` vs `2·(r_door+r_other)²` with **obj+0x8C** radii (0x1366ac-0x1366c8: `addiu a0,s1,128` = obj+0x80; `lwc1 f1,140(s1)` = obj+0x8C) — door.rs's own `sph.0`/`sph.1` are exactly those values | uses the door's **placement position** (`d.pos`) and a 1.0 stand-in; the comment claims the game uses obj+0x30 | wrong centre (hinge vs sphere centre) and wrong radius; zone size/shape differs per door |
| 2 | door.rs:397-410 (auto-close scan) | the scan **skips every candidate with `obj+0xF0 & 0x10`** (0x136678-0x136684); the player (obj+0xF0 = 0x890) and drones (0x8B0) have it SET on 07000001 [L], the 07000005 player has it CLEAR (0x880) [L] | `actors` has no visibility filter | on 07000001 the game's auto-close is effectively unconditional (only `Door_ReactToHit`'s reversal protects an actor in the closing path); the port under-closes there, and over-delays on 07000005 whenever Bond lingers |
| 3 | door.rs:513 (`react_to_hit`) | `Door_ReactToHit` (0x135980) is the collide function: `control_funcs[0x1C]` = {0x1363c8, **0x135980**, 0} @0x26f9e0 [ELF], called from `Collide_Update` (0x1ea398 :59-63) whenever the door has contacts, guarded only by ext+0x6C == 0 | defined but **never called** (the A-11b removal was correct — the bug was calling it from `update`), so the anti-crush reversal is currently unreachable | a sliding door closing on an actor never re-opens (kind 2 reversal), a swing door never pushes back; needs the collide-stage hook, not a call from `update` |
| 4 | door.rs:513-545 (`react_to_hit` filter) | the contact filter picks the first class-3 contact, else the first class-2 drone passing `(droneExt+0x4F8 & 0x600) == 0 && (& 0x100) != 0 && droneExt+0xAC > 0 && !(obj+0xFE & 1)` | any `blocked()` actor | corpses/passive drones trigger reversals in the port that the game would ignore (stand-in, U-041 family) |
| 5 | door.rs:158 (flags comment) | ext+0x70 bits: 0x01 interp mode, **0x04 "starts open"** (+ open channel forced on at create, Door_Create :96-99), 0x10 busy, 0x20 keep-open; 0x02 has no reader (U-043) | comment lists 0/4/5 but omits 0x04's meaning; the code implements it (`state: if flags & 4 { OPEN }` :263, `create_channels` :293-297 forces the channel on) | doc-only; align the comment with docs/atlas/door.md |
| 6 | door.rs:203 + 390-395 (`prev_channels`) | the lock rising edge reads the **global** `switch_channels_prev`, snapshotted after Trigger_Calc each frame (0x1337cc); RamSave persists it | per-Doors snapshot taken at the port's own frame point | off-by-one-frame risk on the unlock edge when a trigger sets the lock channel in the same frame's update pass |
| 7 | door.rs:428-495 (`activate`) | the swing anti-crush lives in `Door_Activate` too (RotPreTransVec + obj+0x8C radius, 0x135d90-0x135e30) and the one-sided direction memory (ext+0x76) can refuse activation with the locked sound | no swing-plane crush check on the activate path; `locked_flag` covers only ext+0x77 (no-E-use) | a swing door can close into an actor via channel/auto-close in the port; ext+0x76 one-sided doors behave two-sided |
| 8 | door.rs:340 (`interp`) | swing closing rests at t = 0.5 within `ext+0x54 · FRAME_RATE_MUL` of the midpoint; sliding clamps t < 0 → SetState(2) (door can end CLOSED with dir still −1) | same structure (kind 0 midpoint snap present; verify the sliding t < 0 branch on the next edit) | minor; both end states match |

## Channels (cross-cutting, affects both files)

- Writers the port does not model yet [M]: `Drone_InitComms` 0x97/0x98 (no timestamp),
  `DroneFunc_SetMissionFailReason` 0x60/99, `Mission_MonitorObjectives` 99/100,
  `Player_CheckForDeath` 0x5E, `Player_HandleDeath` 0x62, `Player_CollisionHandler` 0x7C,
  creep/wire/zipline param channels, `Copter_Update`, `Sensor_Update`, `Flicker_Update`,
  `FuseBox_Update`, `Pickup_Handler`, `Searchlight_Update`, `Monitor_Update`, `Emitter_Update`,
  `Destroy_*`, `PCQWorm_Update`, `CamSubject_Update`, `SS_Activate`, `Switch_Create/Update/
  Activate` (toggle + falling-edge timestamp). docs/atlas/channels.md has the full table.
- `Init_SwitchChannels` pre-seeds `switch_channels_time` with the frame counter and zeroes
  MusicVars; the port's `Channels::default()` zero-fills time [M vs triggers.rs:70-73].
- The alarm counters (`switch_channels_MusicVars` u16[256], `DroneFunc_CheckAlarmRaised`
  0x1453a8) are not represented at all [M].
