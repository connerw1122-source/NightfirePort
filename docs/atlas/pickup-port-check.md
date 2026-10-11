# Pickup / script-player port-check

Every place `crates/nf-game/src/{pickup,nis}.rs` (and their two call sites, `drone.rs`'s take loop and
`scene.rs`'s placement parse — the take bug lives in the call) whose **behaviour disagrees** with
[pickup.md](pickup.md) / [script-player.md](script-player.md). Agreements are not listed.

Tags: `[M]` measured in the decompile, `[L]` snapshot-verified, `[S]` suggested. No Rust code was changed.

---

## Confirmed disagreements (real bugs)

### 1. The take test's LOS origin — `player.pos` twice, game uses the sphere centre `[M+L]`

**The game**: `Pickup_Update@0x18e148:203-214` — squared distance from the pickup's world sphere (`obj+0x80`) to
the player's **position** (`obj+0x30`), but `Collide_LineOfSight` runs from the player's **world-sphere centre**
(`obj+0x80`) to the pickup's `obj+0x80`, mask 0x721. On 07000005 the two points differ: sphere centre =
position − (0.12, 0.12, 0.25) (player obj 0x16cefe0: pos (104.95, 24.96, 108.83), sphere (104.83, 24.84,
108.58)) `[L]` — the port's LOS ray starts at a different point than the game's, by construction.

**The port**: `drone.rs:3662` — `p.touches(d.world, player.pos, player.pos)`: both the distance point and the
LOS origin are `player.pos` (feet). `pickup.rs:157-162` runs `world.line_of_sight(body, c)` from that point.

A floor-level pickup's centre is only ~|glist centre y| above the floor, so the take ray is shallow; whether it
clears the floor mesh depends on the exact origin. The game's origin is a different, fixed point (the build-world
sphere centre), so its accept/reject geometry differs from the port's wherever the floor is uneven — the kind of
difference that flips on approach direction, i.e. "works after walking away and back". That the reported
second-Raptor symptom is *this* effect is a hypothesis to confirm by replay, not a measured cause; the two-point
difference itself is measured. (The game has no other latch: two overlapping drops are tested independently every
tick, falling drops included — pickup.md "Untouchable-pickup conditions".)

| file:line | game | port | severity |
|---|---|---|---|
| `drone.rs:3662` (+ `pickup.rs:157`) | LOS from player `obj+0x80` | LOS from `player.pos` | **bug** — take-ray origin differs from the game's; suspected cause of the playtest symptom |

**Fix**: pass the player's sphere centre (what `obj+0x80` holds: the capsule's `Control_BuildWorldSph` centre)
as the third argument.

### 2. Kind 2 (key) is never taken — no `Player_EquipKey` equivalent `[M]`

**The game**: `Pickup_Handler@0x18e650:217-219` → `Player_EquipKey(BLData, id)` sets `BLData+0x70C+id = 1`
(`Player_EquipKey@0x1a97a0:7`); the key never refuses, so the pickup is always consumed (deleted/respawned at
`:338-348`), the 0x2000010 message shows, and the switch channel `+0x28` flips.

**The port**: `pickup.rs:245-267` `Inventory::take` has no `kind::KEY` arm — it falls to `_ => None`, so a key
pickup is never consumed and no key flag exists.

| file:line | game | port | severity |
|---|---|---|---|
| `pickup.rs:266` | key always taken, `BLData+0x70C+id = 1` | `None` — stays forever | **gap** (no kind-2 pickups on 07000005/01 `[L]`; blocks key-gated levels) |

### 3. Placement params read by position, game scatters them by index `[M]`

**The game**: dynamic-object params land at `level_tag+0x2C+4k` where k is the param's **index**; case 0xF0 reads
kind/id/rounds/switch from +0x2C/+0x30/+0x34/+0x38 (`parsemap_create_dynamic_objects@0x1d04f0:457-462`), and
`SP_CreateScriptPlayer` reads p0–p10 the same way. An absent index reads 0.

**The port**: `scene.rs:1424` (the pickup placement closure: `p.params.get(idx).map(|(_, v)| *v)` — the pair's
index is discarded, the value taken by **position**; used :1425-1430). `nis.rs:491`'s `g(k)` is already
index-based (`params.iter().find(|x| x.0 == k)`) and is fine. `nf-formats/src/placement.rs`'s own test record
carries params `[(0,7),(5,9)]` (:98) — indices are not guaranteed contiguous, so any pickup placement with a
skipped index shifts every later field (e.g. a switch channel landing in the rounds slot).

| file:line | game | port | severity |
|---|---|---|---|
| `scene.rs:1424-1430` | value of param **index** k at base+0x2C+4k | value of the k-th pair | **risk** — wrong whenever a pickup placement's indices are sparse; the two snapshot levels' pickups happen to parse correctly |

**Fix**: look up the pair by its index, as `nis.rs:491` already does.

### 4. `Upgrade_Weapon` remap missing — a drop can hand over the wrong weapon id `[M]`

**The game**: every human-path pickup id goes through `Upgrade_Weapon` (`Pickup_Handler@0x18e650:142-144`):
6 → `UpgradedHandguns[Upgrades]`, 0x1E/0x24/0x43/0x4A/0x4E via their tables, and **10 → 0xC whenever
`GameState[0x52]` is non-zero**; the remapped id decides the owned-check (`+0x1BA`), the equip and the message.

**The port**: `pickup.rs:245-253` uses `p.weapon` directly. With no upgrade shop `Upgrades` is 0 (the tables'
slot 0 entries presumably hold the base ids — unverified here), but the unconditional 10→0xC case is a visible
divergence for any level placing weapon 10.

| file:line | game | port | severity |
|---|---|---|---|
| `pickup.rs:246` | id → `Upgrade_Weapon(id)` | raw id | **gap** (low until upgrades exist; id 10 case is live) |

---

## Divergences in `nis.rs` (script players / NIS streams)

### 5. Eight stream opcodes halt the stream; the game has handlers for all of them `[M]`

`nis.rs:261-266` treats ops 0x13, 0x14, 0x15, 0x18, 0x1A, 0x1B, 0x1D, 0x1E as "handlers not read yet" — pushes a
placeholder event and sets `pc = None` (the stream never runs again). The game dispatches every one of them
(`Script_Run@0x1d8548:80-118`) to real handlers that consume their arguments and continue: 0x13
`Script_FadeStart` (0 args), 0x14/0x15 sprite start/end, 0x18/0x1A light start/end, 0x1B/0x1D nested-subscript
start/end, 0x1E text start. Any script containing one of these commands freezes that stream in the port from the
opcode onward (fades, sprites, lights, subtitles, nested scripts — all missing as a side effect).

### 6. Event 3 (delay) skips the per-stream position sync `[M]`

`Script_EventHandler@0x1d94c0:98-117`: after writing the delay, every stream's object gets
`Vec_Copy2(obj+0xC0 → obj+0x30, obj+0x40)` — objects' positions snap to their current matrices before the pause.
`nis.rs:346` only sets `self.delay`.

### 7. Events 5, 8, 9, 0xA, 0xF are not carried out `[M]`

The game runs them inline: 5 `Drone_EnableAll(0, hash)` (the cut-scene drone freeze — `nis.rs` relies on its own
`Drones::paused` stand-in, see core-loop §7), 8 `Drone_CoderCreate` (MP), 9 `Player_Cam2Mode`, 0xA
`Break_DoBreak` on the previous stream's object (destructibles smashed by scripts), 0xF the `SCRIPTINFO+0x84`
callback (null from parsemap levels). `nis.rs:377` lumps them into `Request::Event`.

### 8. No create-time `SP_LoadScript` for modes other than 0/1 — the load-tick beat is missing `[M+L]`

`SP_CreateScriptPlayer@0x1db9a8:81-86`: mode A ∉ {0, 1} loads the script immediately and (mode dispatch,
`SP_LoadScript@0x1dbc08:41-55`) leaves it **primed in state 1 after having run once** — mode 64's arm is
play+update+stop. Live: the 07000005 EndLevel SP (0x16d3650, script 0x6000A23, mode 64) is in state 1 with its
SCRIPTINFO clock already at 2.0 at level start `[L]` — its load-time streams/events have already fired. The port
(`nis.rs:495-500`) leaves state 0 and nothing loaded; everything the script does before its trigger happens at
trigger time instead of load time.

### 9. The disabled byte (`SCRIPTPLAYER_tag+0x36`) is not modelled `[M+L]`

`SP_Update@0x1dbda8:57-59` returns while `+0x36` is set — the SP and its script freeze. Live: SP 0x16ddb40
(script pair 0x6000046/0x6000047, state 6 looping) has `+0x36 = 1` at level start on 07000005 `[L]`. The port has
no such flag (`Player.deleted` only), so a disabled-at-start looper would run in the port.

### 10. Trigger mask bits 2 / 8 / 0x10 (bullets, player touch, drone touch) are not modelled `[M]`

`SP_Hit@0x1dc470:17-85` walks every stream object's proxy list (`obj+0xD0`) and returns bit 2 for class-5
bullets (filtered to weapon `+0x37` when set), bit 8 for class-3 player neighbours, bit 0x10 for class-2 drone
neighbours. The port's `Player::update` comment (`nis.rs:526-527`) already records this; only mask bits 1 and 4
exist. Also `SP_Activate`'s gate — with mask & 1 an activation only lands while the out channel is ON
(`0x1dc338:72-81`) — is absent (`nis.rs:535`).

### 11. Streamed-script loading beat and cut-scene skip `[M]`

State 0's trigger path: `isLoadable(script A)` → `GameFlow_PushState(12.0, 0xD, 0xFF)` + flag +0x38, the load
happens the next tick (`0x1dbda8:99-105`) — a visible loading beat for streamed NIS scripts. The port loads
synchronously. State 2: `Input_Action(-1, 0x19)` (the skip action) fast-forwards the script via `Script_FFwd`
unless it is a death NIS (`0x1dbda8:129-141`) — the port has no skip.

### 12. Camera start/end side effects `[M]`

`Script_CameraStart@0x1d9140` and `Script_KillStream` case 4 (`0x1d9860:66-81`) also: `Music_Event(0xB/0xC,
hash)`, `Camera_PushStates`/`PopStates`/`Camera_Enable(4,…)`, the letterbox border sprites, +0xAB5 bit 0x10.
The port models the camera object, the FOV and the pop request only. `ScriptCam` is the **hashcode** of the
camera script (0 = none; `Script_Play@0x1db588:26`, `Script_EventHandler:201`, cleared by `Script_Free:49-51`) —
the port's `script_cam: bool` is equivalent for gating but loses the hash the music events need.

---

## Checked and agreeing (sample)

State machine numbering/transitions and the out-channel write rule (`SP_Update` ↔ `nis.rs:528-596`); p0–p10
mapping incl. p10 ‰ scale and mode semantics 1/8/other (`SP_CreateScriptPlayer`/`SP_LoadScript` ↔
`nis.rs:490-524`); mask bits 1 and 4; countdown 5; state-6 switch frees slot A; `SP_Delete` frees both;
SCRIPTINFO +0x9C/+0xA0/+0xAA8/+0xAB2 and SSTREAM +0x08/+0x0C/+0x10/+0x14/+0x1C/+0x1E/+0x20/+0x24/+0x25 field
semantics; the previous-stream rule for events 6/0xB (stream index − 1, kind 1/2); event ids
3/4/6/7/0xB/0xC/0xD/0xE/0x10/0x11/0x12/0x13 effects incl. the conditional-channel 0x11 arithmetic; loop restart
clocks (script = loop_start, stream = loop_start − 1); drop rounds formula and the single `Rand_FRand` per drop
(two for 0x1A); `Pickup_CreateSimple` id rules; kind 0 owned→ammo branch; kind 6 bonus giving nothing; armour
50.0 hard-set; take-time channel set; respawn/lifetime semantics; pickups takeable while falling; lifetime not
decremented on a touch tick.
