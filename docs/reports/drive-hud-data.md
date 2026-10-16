# drive-hud-data (646) — the driving HUD's data model as a per-frame snapshot

**Status: done** (the data model + the wiring that exists on master + numbers-only tests). The
main agent draws it: nothing here renders. Branch `s/drive-hud-data` from origin/master
(0f5f07f4).

## Files

- `crates/nf-game/src/driving_hud.rs` (new) — the snapshot: `DriveHud` + `CarReadout`,
  `MissionTimer`, `Objective`/`ObjectiveState`, `Beacon`, `WeaponReadout`.
- `crates/nf-game/src/lib.rs` — one additive line: `pub mod driving_hud;`.
- `crates/nf-game/tests/drive_hud.rs` (new) — 6 numbers-only tests ("the snapshot follows
  the sim"), plus 1 unit test in the module.

## What the HUD reads → what the snapshot carries

| HUD element | Game source (DRIVING.ELF, EE) | Snapshot field | Wired? |
|---|---|---|---|
| Speed | \|velocity\| [M 0x18aa88:26-29; tyre pass 0x190f88:394]; **no mph constant exists** [M-negative byte search] | `CarReadout::speed` (raw units) | yes — `DriveHud::observe(&Car)` per tick |
| Gear | the gear byte actor+0x2d8 [M; auto walk 0x18aa88:36-48] | `CarReadout::gear` | yes |
| RPM | speed·6000/LIMIT[gear] [M 0x18aa88:62; neutral rev throttle·7000 :32] | `CarReadout::rpm` | yes |
| Reverse flag | actor+0x29c [M] | `CarReadout::reversing` | yes (defaults false; the port's step never sets it yet) |
| Minimap viewer | instance+0x10 pos, the class matrix's yaw [M] | `CarReadout::pos`/`yaw` | yes |
| Vehicle health/damage | `EPowerUpHealth`/`ESetVisualDamage` exist [D 0x372300/0x372790]; the PBondCar ctor `FUN_001862c0` is register-mangled — no health field provable [M-negative]; the ESetVisualDamage **command record is {vt 0x370558, p1..p5}** [M 00159638] | `health: Option<f32>` (None until pinned) + `visual_damage: [f32; 5]` verbatim | command level |
| Weapon + ammo | weapon manager `FUN_0020bb00` 400-byte alloc [M]; `EPowerUpAmmo` id 56 [M]; `ESetSpecialWeapon`/`EUnloadWeapon` [D 0x372720/0x372a30] | `weapon: WeaponReadout` | values-as-set [S] |
| Mission timer | command ids **ESetTimer 507/509, EStartTimer 381/383, EStopTimer 357/359, ETimerDrawOn 309/311, ETimerDrawOff 315/317** [M record-table mapping] | `timer: MissionTimer` (remaining/initial/running/draw + `update(DT)`) | command level |
| Objective markers | `EObjectiveAdd/Insert/Pass/Fail` ids **225/227/228/226** [M]; the record is **{vt 0x370e70, p1..p4}** 16 bytes [M 00151510] | `objectives: Vec<Objective>` (params verbatim + state) | command level |
| Target beacons (diamonds) | `ETargetBeaconOnOff` ids **321/323** [M] | `beacons: Vec<Beacon>` + `beacon_toggle` | command level |

Provenance for the id mapping: the dispatch table at **0x36fda0** splits into instant commands
(1-283, handlers bump-allocate the record via `FUN_00175730`) and object commands (285+,
accessor/dtor pairs; per-command record table 173 × 0x18 at **0x370214-0x37124c**); the
executor `FUN_001a35f0` walks 0x40-byte event records (id byte +0xd, data +0xe..) [M]. The
**HUD render object** itself is `RRenderHUDView` [D 0x3856c8] at render-manager+0x28
(`DAT_00333ed8`) [M 001c3db0:22-34]; its only frame reader is `FUN_001c1ec0`:80-81 →
`FUN_001c1b18`:118-144, which copies a **0xa8-byte (21-u32) element block** and passes the
label-table pointer **&DAT_0037d790** [M]. The per-element positions/sheet ids are **not
decoded yet** — that is the main agent's draw pass input (live checks below).

## Verification

- The crates my tests touch, on the final tree: `cargo test --offline --release -p nf-game`
  → **lib 213/213** (212 + the new unit test), **drive_hud 6/6**, **drive_physics 13/13**
  (the snapshot's neighbour, untouched). Tests assert, tick by tick, that the readout IS the
  car's fields through 600 full-throttle ticks; the gear boundary (18.0 == GEAR_LIMIT1 shifts,
  17.9 holds) and the rpm formula bit-for-bit within 1e-3; the timer's 60 Hz tick (60·(1/60)
  accumulates in f32 — tolerance 1e-2), stop-hold, re-arm, floor-at-zero; the objective
  record's 4-param shape + state commands; the beacon toggle in place; the 5-param
  ESetVisualDamage record verbatim; `observe` never touches the mission groups.
- **Full suite (fast runner)** on the branch-point tree (the build finished at 01:32 UTC,
  before my 02:10 code landed): `1577 passed, 7 failed, 66 ignored (62 min)`. The 7: the 3
  allowed `disc_golden` slim-ISO fails; `probe_pylon_render` + 2 × `fix_0c_pylon_live` (the
  documented sandbox Vulkan-ICD adapter fails); `walkreal_07000001_live::walk_from_the_spawn…`
  (the documented pre-existing LEAD_HIDE budget flake, 577 s under my parallel build's CPU
  contention). None touch this module (nothing calls it yet — same phase split as
  mp-bots-data's "data first, wiring later").

## What's left (the next window)

- The mission-**state** structs behind the commands: the timer's value/draw fields (the
  accessors/dtors are trivial; the state lives in the `FUN_0024e7a8`-created event object or
  the mission block), the target/beacon struct, the objective-list consumer, the vehicle
  health field. 647-drive-mission-scripts (pending) is the natural owner; the snapshot's
  setters are its fill points.
- The HUD element walk (FUN_001a4520/44f0/4468 + the 0xa8-byte block + DAT_0037d790) for the
  real positions/label/texture ids.
- Sim wiring: driving levels still refuse to load in scene.rs; when 506's Car joins the Sim,
  `DriveHud::observe` is the per-frame call (right after `Car::step`, the mp
  `MpMatch::update` precedent).

## Live checks for the main agent

1. **The HUD element block**: breakpoint `FUN_001c1b18` 0x1c1b18 (exec) during a drive; dump
   the 0xa8 bytes copied at :118-144 and the table at DAT_0037d790 — expect per-element
   positions/sheet ids; record which u32 lanes move when the speed/gear digits change.
2. **Update order**: after `Leaving StartUp`, dump world 0x3c87b0 (count +0xc, objects
   +0x10.., vtable +0x18, vtable+0x1c target each) — expect the car and any HUD object's
   relative Update order (atlas L-3).
3. **The timer struct**: in snow2a_race, break on the EStartTimer handler (id 381/383 path);
   dump the event object `FUN_0024e7a8` allocated — expect the running/value/draw fields and
   their units (seconds float?).
4. **Vehicle health**: break on the ESetVisualDamage consumer (record vtable 0x370558) after
   a crash; dump the struct the 5 params land in. Also scan 0x1862c0-0x18686c for a
   100.0-style (0x42c8) init — the PBondCar health candidate offset.
5. **Speed units**: drive at a steady known |vel| (watch FUN_0018aa88's return) and read the
   HUD digits — expect raw units or a simple scale (no mph constant exists in the binary).
