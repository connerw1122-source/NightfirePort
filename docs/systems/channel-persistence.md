# The switch-channel table and its writers (channel-persistence)

The game's switch channels — `switch_channels` (ACTION.ELF 0x26fc90) and their stamps
`switch_channels_time` — are a 256-byte array the whole level communicates through: buttons,
switches, lift/door controllers, objectives, spawners, scripts. The port carries them as
`Channels` (`crates/nf-game/src/triggers.rs`: `v`, `time`, plus the census probes `writes` and
`script_channels`). **There is no per-tick recompute in the game**: a channel holds its value until
one of the game's own writers rewrites it, and every writer owns specific channels. This doc is the
writer inventory, with evidence, and the census method that checks the port against it.

## The game's writer families [M — every one checked in the decompile]

| Writer | Channels it writes | Evidence |
|---|---|---|
| Touch clear + hit | each type-2 (TOUCH) trigger's own OUT — cleared every tick, then `\|= hit` | `Trigger_Calc` 0x1b1900's clear loop (`switch_channels[out] = 0` per type-2 object) + `Trigger_Update` 0x1b1a78 case 2 (`\|=` + stamp on hit) |
| TOUCH_ONCE fall-through | each type-1 trigger's own OUT — `\|= hit` **every tick** while its channel is off (a no-op 0-store when unhit) | 0x1b1a78 case 1 falls through into case 2 [M]; the port's `update` writes only on the hit (same values, no no-op store — a documented equivalence, not a divergence) |
| AND / OR / SEQ_AND muxes | their own OUT — recomputed every tick, 0 or 1 (the AND writes 1 for an empty/zero prefix) | 0x1b1a78 cases 3 / 0xC / 4 (LAB_001b1e60) |
| OUT copies | each nonzero INPUT slot — `switch_channels[input] = switch_channels[out]` every tick | 0x1b1a78 case 5 (8-slot loop, zero slots skipped, stamp on nonzero) |
| LOAD_LEVEL | the success/fallback channel on fire | 0x1b1a78 case 10 → `Trigger_Activate` 0x1b2318 |
| MOVIE | its input channel, cleared when it fires | 0x1b1a78 case 0xd (0x1b1f48) |
| Sensors | their OUT — cleared every tick in states 0 and 2, written 1 while the sight holds past 2 s | `Sensor_Update` 0x1af618:169-171 (state 0), 152-157 (the trip), 225-243 (state 2) |
| Flickers | their signal channel every tick (1 when the map light's +0x48 is 0, else 0) | `Flicker_Update` 0x1ab960 (both arms write) |
| Emitters | their ch_out every tick — cleared, re-set 1 (+stamp) while any particle lives | `Emitter_Update` 0x17b080 (0x17b080:127 clear, :163-164 set) |
| Doors | their ch_open — mirrors the OLD state's motion bits (0xc) on every motion transition (not per tick), stamped when nonzero | `Door_SetState` 0x135618:26; the start-open doors stamp at create (`Door_Create`'s tail) |
| Switches | their channel — at load (`switch_channels[param0] = param3`), on activation, on auto-return | `Switch_Create` 0x1adcc0:39-43, `Switch_Update` 0x1adea8, `Switch_Activate` 0x1b02a8 |
| Objectives (init) | each objective's channel once at level load — `switch_channels[ch] = init` | `Mission_Init` 0x182270:106-107 (+ the stamp at :112) |
| Level latches | 05: ch 0xa4 on → 0x6f = 1; 0b: ch 0x34 on → 0x2d = 1 | `Trigger_Calc` 0x1b1900:21-26 |
| Drone comms | ch 0x97 = 1 when an enabled living drone at alert ≥ 1 is within 30 units (NO time stamp) | `Drone_InitComms` 0x139520:198-201 |
| Script events | the op's target channel — event 0xD (`ch = param1`), event 0x11 (conditional pick of two channels) | `Script_EventHandler` 0x1d94c0 case 0xD / 0x11 (the stamp at 0x1d94c0:112) |
| Script players | the SP's own OUT while its trigger bits hold | `SP_Update` 0x1dc1c0's out write (the port's nis.rs `Player::update` tail) |
| Script-created triggers | `Script_CreateEntity` 0xEA/0xEB build type-1/type-2 trigger objects — they write like the placed families | 0x1d8968; the port rebuilds them per tick from live entities (drone.rs, `script: Some`) |
| Bond-moment sweep | READS 0x65-0x6E (no write) → `PlrStat_DoneBondMoment` | `Trigger_Calc` 0x1b1900:12-20 |

Known UNPORTED writer: the SimpleScript operate arm — `SS_Update` 0x1a9948:20-23 writes the
operate result into a 0-valued channel (documented in `crates/nf-game/src/ss.rs`; needs the
presence result). A channel only that arm writes never turns on in the port.

## The 750q "open port bug" was the disc's own OUT copy

750q's repro (`docs/reports/cutscene-guard-models.md`, "Open port bug"): on 07000001,
`Channels::set(0x33, 1)` then one `step_bond` read 0 — read as "some port system clears channels
within one tick". The census traced the writer: **the disc's OUT placement #296 (out 0x32, input
51 = 0x33) [D 07000001 placements]** — `Trigger_Update` case 5 copies `switch_channels[0x32]`
(0) into `switch_channels[0x33]` EVERY TICK [M 0x1b1a78:147-161]. The game does exactly the same
to an externally written 0x33. The channel was owned all along; the port is faithful. Pinned by
`the_0x33_clear_is_the_discs_out_copy_on_07000001` (channel_persist_live.rs).

## The port's per-tick writer census

`Channels::writes` (set `Some` by the test) records every write as `(channel, value, frame,
source)` — source 0 = an object writer, 1 = the script path (`Channels::set_script` at the event
0xD / 0x11 sites and the SP out). The direct `v[..]` writers (the door's transition mirror and the
0x97 comms byte) record too — the door now routes through `set` (identical stamp semantics).

The census test (`crates/nf-view/tests/channel_persist_live.rs`,
`the_port_rewrites_only_game_owned_channels`) loads every on-foot level (23), idles 120 ticks
through the real input path, and requires every recorded write's channel to be owned by a live
object of the families above (or the documented stand-ins below). Script-owned channels
(`Channels::script_channels`) are exempt as their own class — the port's script path writes what
the disc's scripts say [D], same as the game's 0x1d94c0 path.

Census result (2026-10-17, this branch): **all 23 levels pass** — no port-only per-tick writer
exists. Owner sets per level are printed by the run (e.g. 07000001: 0x04, 0x05, 0x0b, 0x10, 0x33,
0x97, 0x99 at enable).

## The [S] stand-ins (port-only writers, kept out of the game-owner set)

- 0700004a ch 0x22 and ch 0x4a — the Tower-2 lift's resume/lobby channels; the scans found no disc
  writer (docs/levels/0700004a.md §6). The census prints them as stand-ins when they fire.
- The flicker's write VALUE: the game picks 1/0 from the map light's +0x48 state [M 0x1ab960]; the
  port's stubbed `FindMapLight` always takes the 0 arm (world_fx.rs). Ownership is faithful; the
  value is a documented stub.

## Deviations (documented, none behavioural)

- The port skips the game's no-op `|= 0` stores for TOUCH_ONCE outs (the channel values and stamps
  agree; only the redundant store is gone).
- The census itself is test-only (`writes`/`script_channels` are `None`/empty outside it).
