# Script & event census — what the disc ships vs what the port handles

Task script-census (709). The question: "are there scripts and whatnot we haven't found yet?"
This census enumerates every 0x07 archive on the disc (34 level archives incl. the front end,
27 loadable pack archives, 38 per-level anim archives), and counts — per archive — every NIS
script and its command opcodes and event kinds, every anim script (kind 5) and its opcodes,
every trigger placement and action code, every dynamic-object placement type, and every
archive entry type the loader sees. Each item is classified **handled** (the port's handler,
cited) or **unhandled** (the game's handler, cited [M], with what it does).

## Method and provenance

[D] All counts measured from the retail disc's `FILES.BIN` (slim ISO, `golden/files_bin.tsv`
offsets), parsed by `scripts/census_levels.py` (scratch tool, not committed) which mirrors the
port's own parsers byte-for-byte: `LevelBin::parse` (nf-formats/src/levelbin.rs:41-65),
`MapFile::parse` (map.rs:69-88), `placement::parse` (placement.rs:38-57), `Nis::parse` +
`Stream::decode` (nis.rs:80-196), and `anim::Script::parse` (anim.rs:114-126).

Three independent cross-checks say the mirror is exact:

- **5,263 NIS streams / 468 scripts** — matches the stream census quoted in
  nf-formats/src/nis.rs:120-125 ("the 5,263-stream census over all 87 archives").
- **23,329 placement records** — matches placement.rs:3 ("all 23,329 records on the disc").
- **2,230 map entries** (kinds 0/1/2/11: 978+35+551+666) — matches placement.rs:37
  ("true for all 2,230 maps"). Zero decode truncations, zero unknown NIS opcodes, zero
  placement overruns across the whole disc.

[M] Handled/unhandled classifications cite the port source and the decompile
(work/decomp/ACTION.ELF/, addresses inlined). The game's NIS dispatcher `Script_Run`
(0x1d8548) has a 28-entry jump table at 0x3056d0 (index = op−3); its event dispatcher
`Script_EventHandler` (0x1d94c0) a 17-entry table at 0x305740 (index = kind−3) — both verified
against the raw ELF, so the opcode spaces below are complete, not just what the decompiler
happened to show.

## Level archives (all 0x07 .bin levels, front end included)

Per archive: NIS scripts in the archive itself plus its loadable packs (every pack carries
exactly one NIS script), anim-script entries (kind 5), script-player placements (0xD9 — the
things that play NIS), SimpleScript placements (0xE2), dynamic-object placements (records in
map block 0x1A that pass the game's spawn gate, i.e. flags bit 0x8000/0x2000 clear), and
trigger placements (the 0xE8/0xEA-0xEF/0xF4 families plus raw 0xDC).

### Per-archive census

| archive | role | NIS (own+pack) | streams | anim scripts | script players | SimpleScripts | dyn placements | triggers |
|---|---|---|---|---|---|---|---|---|
| 07000001.bin | level | 11+1 | 147 | 475 | 7 | 16 | 236 | 12 |
| 07000002.bin | level | 6+2 | 98 | 629 | 6 | 15 | 299 | 37 |
| 07000003.bin | level | 9+0 | 101 | 604 | 8 | 8 | 275 | 27 |
| 07000004.bin | level | 9+0 | 126 | 307 | 5 | 0 | 58 | 2 |
| 07000005.bin | level | 14+0 | 197 | 568 | 8 | 0 | 263 | 52 |
| 07000006.bin | level | 11+0 | 119 | 547 | 8 | 28 | 481 | 57 |
| 07000007.bin | level | 6+2 | 145 | 399 | 6 | 0 | 224 | 19 |
| 07000008.bin | level | 13+2 | 262 | 573 | 9 | 2 | 247 | 20 |
| 07000009.bin | level | 11+1 | 144 | 272 | 5 | 2 | 120 | 7 |
| 0700000a.bin | level | 13+0 | 116 | 272 | 7 | 3 | 313 | 63 |
| 0700000b.bin | level | 20+3 | 237 | 467 | 12 | 1 | 196 | 61 |
| 0700000c.bin | level | 13+1 | 186 | 556 | 5 | 8 | 301 | 35 |
| 0700000d.bin | level | 10+2 | 145 | 573 | 5 | 1 | 316 | 29 |
| 07000011.bin | level | 7+1 | 112 | 491 | 4 | 2 | 190 | 28 |
| 07000012.bin | level | 13+0 | 175 | 483 | 6 | 8 | 344 | 52 |
| 07000013.bin | level | 8+2 | 121 | 487 | 5 | 1 | 156 | 10 |
| 07000014.bin | level | 10+3 | 168 | 535 | 17 | 0 | 777 | 76 |
| 07000015.bin | level | 5+0 | 80 | 532 | 2 | 0 | 283 | 16 |
| 07000016.bin | level | 14+3 | 232 | 577 | 17 | 0 | 247 | 15 |
| 0700001b.bin | level | 77+2 | 852 | 201 | 76 | 0 | 181 | 8 |
| 07000023.bin | level | 14+0 | 130 | 813 | 2 | 0 | 160 | 1 |
| 07000024.bin | level | 15+0 | 135 | 813 | 3 | 0 | 174 | 1 |
| 07000025.bin | level | 17+0 | 160 | 838 | 5 | 0 | 186 | 19 |
| 07000026.bin | level | 14+0 | 130 | 838 | 2 | 0 | 161 | 1 |
| 07000027.bin | level | 14+0 | 130 | 813 | 2 | 0 | 175 | 1 |
| 07000028.bin | level | 14+0 | 130 | 813 | 2 | 0 | 145 | 1 |
| 07000029.bin | level | 14+0 | 130 | 838 | 2 | 0 | 159 | 1 |
| 07000041.bin | level | 7+0 | 97 | 861 | 2 | 0 | 100 | 4 |
| 07000043.bin | level | 0+0 | 0 | 163 | 2 | 0 | 30 | 1 |
| 07000046.bin | level | 14+0 | 150 | 401 | 2 | 10 | 99 | 4 |
| 07000048.bin | **front end** | 0+0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 07000049.bin | level | 0+0 | 0 | 163 | 2 | 0 | 30 | 1 |
| 0700004a.bin | level | 9+2 | 162 | 516 | 7 | 0 | 261 | 14 |
| 0700004b.bin | level | 14+0 | 130 | 813 | 2 | 5 | 185 | 1 |
| 07000500.bin | load-screen art | 14+0 | 15 | 0 | 0 | 0 | 0 | 0 |
| 07000999.bin | special (1 script) | 1+0 | 1 | 0 | 0 | 0 | 0 | 0 |
| **34 archives** | | | 5263 | 18231 | 253 | 110 | 7372 | 676 |

| pack archive | of level | NIS | streams |
|---|---|---|---|
| 07800001.bin | 07000001 | 1 | 15 |
| 07800002.bin | 07000002 | 1 | 9 |
| 07800007.bin | 07000007 | 1 | 15 |
| 07800008.bin | 07000008 | 1 | 30 |
| 07800009.bin | 07000009 | 1 | 12 |
| 0780000b.bin | 0700000b | 1 | 24 |
| 0780000c.bin | 0700000c | 1 | 19 |
| 0780000d.bin | 0700000d | 1 | 28 |
| 07800011.bin | 07000011 | 1 | 19 |
| 07800013.bin | 07000013 | 1 | 10 |
| 07800014.bin | 07000014 | 1 | 25 |
| 07800016.bin | 07000016 | 1 | 25 |
| 0780001b.bin | 0700001b | 1 | 12 |
| 0780004a.bin | 0700004a | 1 | 28 |
| 07900002.bin | 07000002 | 1 | 7 |
| 07900007.bin | 07000007 | 1 | 37 |
| 07900008.bin | 07000008 | 1 | 15 |
| 0790000b.bin | 0700000b | 1 | 16 |
| 0790000d.bin | 0700000d | 1 | 4 |
| 07900013.bin | 07000013 | 1 | 12 |
| 07900014.bin | 07000014 | 1 | 28 |
| 07900016.bin | 07000016 | 1 | 30 |
| 0790001b.bin | 0700001b | 1 | 18 |
| 0790004a.bin | 0700004a | 1 | 27 |
| 07A0000b.bin | 0700000b | 1 | 6 |
| 07A00014.bin | 07000014 | 1 | 14 |
| 07A00016.bin | 07000016 | 1 | 16 |


Placement types the port handles (56 values: 48 simulated + 8 draw-only) and the game handles
(73 values, `parsemap_create_dynamic_objects` 0x1d04f0 — 72 spawn cases plus the 0xDC trigger
raw-action form) overlap except for the rows below: 14 types the game handles and the port
does not act on, plus 4 one-per-level marker types neither side touches. (The game's
`Creature_Create` type 0xE0 never occurs on the disc.)

### Placement types on the disc that the port does not act on

| type | count | port | game spawn fn |
|---|---|---|---|
| 0x27 | 41 | — | GT_Create 0x17ffa0 |
| 0x28 | 62 | — | Sensor_Create 0x1af010 |
| 0x2b | 3 | — | LeafGen_Create 0x1acbd0 |
| 0x2c | 15 | — | Lightning_Create 0x18cde8 |
| 0x2d | 51 | — | Player_AddNewStartPos(,0) 0x195c00 |
| 0x38 | 1 | — | Create_SpaceLaser 0x1aa8b8 |
| 0x39 | 8 | — | Create_SpaceMissile 0x1a9ef0 |
| 0xdd | 33 | — | — none (game ignores) |
| 0xe4 | 64 | — | Hurt_Create 0x17e870 |
| 0xe5 | 584 | — | Drone_CoverCornerNode 0x136ff0 |
| 0xe6 | 384 | — | Drone_CoverLowNode 0x137340 |
| 0xe9 | 162 | — | Drone_AIPoint 0x137550 |
| 0xf3 | 8 | — | Env_Tree_Create 0x17c200 |
| 0xf4 | 1 | — | Trigger_MoviePlayer 0x1b2a08 |
| 0xf6 | 33 | — | — none (game ignores) |
| 0xf7 | 33 | — | — none (game ignores) |
| 0xfb | 15 | — | MusicTrigger_Create 0x1b30e0 |
| 0xff | 39 | — | — none (game ignores) |

### NIS command opcodes (all 20 that occur are handled)

Every stream ends (one End per stream, 5,263). Wait/Marker always pair 1:1 (105,661 each) —
[ D] every Wait is followed by a Marker at the same timestamp in the recorded streams; the game
reads Marker's u16 but discards it [M 0x1d8548 case 6].

| op | name | count | port handler | game handler |
|---|---|---|---|---|
| 0x0a | AnimStart | 129 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x0c | AnimEnd | 127 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x0d | Camera | 45 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x0f | CameraEnd | 41 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x12 | Event | 851 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x13 | Fade | 3 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x14 | Sprite | 18 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x15 | SpriteEnd | 14 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x16 | Sound | 579 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x17 | SoundEnd | 571 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x18 | Light | 108 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x1a | LightEnd | 108 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x1b | SubScript | 133 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x1d | SubScriptEnd | 128 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x1e | Text | 81 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x04 | End | 5263 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x05 | Wait | 105661 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x06 | Marker | 105661 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x07 | Entity | 4985 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |
| 0x09 | EntityEnd | 3038 | nf-game `Script::run` arm | `Script_Run` 0x1d8548 |

### NIS event kinds (851 events across 14 kinds; 12 handled, 2 unhandled; the game's table has 17 slots)

| event kind | count | port | game behavior |
|---|---|---|---|
| 0x03 | 303 | yes | pos sync + delay |
| 0x04 | 36 | yes | Player_Disable |
| 0x05 | 35 | yes | Drone_EnableAll(0) |
| 0x06 | 31 | yes | return-matrix snapshot |
| 0x07 | 29 | yes | flag 0x80 |
| 0x08 | 1 | **NO** | Drone_CoderCreate 0x13a108 |
| 0x09 | 1 | yes (nis-event9-cnis) | Player_Cam2Mode 0x195fc8 — mode 3 = the behind-Bond orbit, else the exit arm; the disc's single firing is mode 1 (the exit) |
| 0x0a | 1 | **NO** | Break_DoBreak 0x12c168 |
| 0x0b | 23 | yes | weapon byte |
| 0x0c | 65 | yes | flag 0x10 |
| 0x0d | 195 | yes | switch channel write |
| 0x0f | 119 | yes | level callback |
| 0x11 | 10 | yes | conditional channel write |
| 0x12 | 2 | yes | RamSave+level load |

Event kinds 0x0e, 0x10 and 0x13 never occur on the disc. The three the port drops — 0x08, 0x09,
0x0a — each occur exactly once, all in script **0600005b** of **07000001.bin** (12 streams):

- stream 9: `Event(0x08, [0x0500001b, 0x060000a6, 0x10, 1])` → `Drone_CoderCreate` 0x13a108 —
  spawns the gadget/coder drone (the Q-claw drone intro of the first mission) at the stream's
  object, with skin 0x0500001b and anim 0x060000a6.
- stream 4: `Event(0x09, [1])` → `Player_Cam2Mode(0, 1)` 0x195fc8 — the camera-mode override's
  exit arm (`SetCamMode(0)` + `ChangeSubState(0)` [M 0x195fc8:11-16]; mode 3 would enter the
  behind-Bond orbit, which no disc script fires [D nis_event9_census]).
- stream 8: `Event(0x0a, [])` → `Break_DoBreak` 0x12c168 at the current object — script-driven
  breakable trigger.

### Trigger action codes (8 the game dispatches; port handles 7)

The port's `triggers::kind` module (nf-game/src/triggers.rs:10-18) covers 1, 2, 3, 4, 5, 10
(0xa), 12 (0xc). The game additionally dispatches **0xd (movie)** in `Trigger_Update` 0x1b1a78
and `Trigger_Activate` 0x1b2318: when its input channel fires it clears the channel, latches
the output channel into `GameState+0x16` and pushes GameFlow states 6/7 (movie + fade)
[M 0x1b1a78 case 0xd]. On disc this is exactly one placement: type **0xF4** in **07000041.bin**
(`Trigger_MoviePlayer` 0x1b2a08 writes action 0xd).

Action codes by occurrence (the placement families plus raw 0xDC with an authored param 0):

| action | count | written by | port | game |
|---|---|---|---|---|
| 1 TouchOnce | 348 | 0xEA | yes | `Trigger_TouchOnce` 0x1b2b88 |
| 2 Touch | 178 | 0xEB ×175 + raw 0xDC ×3 | yes | `Trigger_Touch` 0x1b2aa8 / `Trigger_Create` 0x1b1720 |
| 3 AND | 46 | 0xEC | yes | `Trigger_MultiplexIn` 0x1b2c68 |
| 4 SEQ AND | 21 | 0xED | yes | `Trigger_MultiplexSIn` 0x1b2e08 |
| 5 OUT | 5 | 0xEE | yes | `Trigger_MultiplexOut` 0x1b2ed8 |
| 10 (0xa) load level | 16 | 0xE8 | yes | `Trigger_LoadLevelCreate` 0x1b2950 |
| 12 (0xc) OR | 28 | 0xEF | yes | `Trigger_MultiplexOrIn` 0x1b2d38 |
| 13 (0xd) movie | 1 | 0xF4 (07000041) | **NO** | `Trigger_MoviePlayer` 0x1b2a08 |

Raw 0xDC triggers: 36 placed; only 3 carry an authored action param (value 2 = Touch, counted
above); the other 33 have no param 0, so the game's zeroed param buffer gives them action 0 —
the default arm of `Trigger_Update` — i.e. inert by design [M 0x1b1720 amemcpy of the 0xb0
param buffer].

### Archive entry kinds (13 recognized by the game's `LoaderProcess` 0x1d4880)

Seen on disc: 0 (978), 1 (35), 2 (551), 3 (1217), 4 (18252), 5 (18231), 6 (1485), 7 (468),
8 (34), 11 (666), 12 (26), 15 (1), 16 (33). The port's src handles all except:

- **kind 8 — the menu-def `08000002` entry, exactly one per level archive (34 total)**. The
  runtime front end does not parse it; the menus are transcribed Rust constants
  (nf-view/src/menu.rs), and a tag-stream parser exists only in test helpers
  (nf-view/tests/unlock_popups_live.rs:68).
- **kind 15 — the memcard icon file (`SetUpIconFile` 0x111270), one occurrence, in the front
  end archive 07000048.bin**. ABSENT from the port (cosmetic; nothing reads icons).

Kinds 9, 0xa, 0xd, 0xe have no case in the game either and never occur on the disc — the
opcode space itself is fully covered.

### Anim scripts (kind 5): the biggest script family — 18,231 scripts, 50,279 commands

These are the skeletal-anim sequencer scripts the game runs through `AnimProcessScriptCmds`
(0x1c1b38) and the port parses with `anim::Script::parse` (nf-formats/src/anim.rs:114). Only
three opcodes occur: op 0 ×18,296 (start a sequence window — port: `AnimSeqSet`
nf-game/src/anim.rs:2,172), op 1 ×3,698 (3-D sound — port: drone.rs:784), op 4 ×28,285 (the
kind sub-switch). Ops 2 and 3 never occur (op 2 = per-command fired-bit set/clear on
animobj+0x7c [M 0x1c1b38:461-475]; op 3 has no case in the game either).

Op 4's kind argument (game sub-switch 0x1c1b38:370-411, port replay in nf-game/src/death.rs):

| kind | count | game behavior | port |
|---|---|---|---|
| 0 | 34 | footstep toggle (`ScriptFootstepFoot` flip) | death.rs (footsteps) |
| 1 | 52 | toggle bool at (obj+0xe0)+0x964 [M 0x1c1b38 case 1] | not replayed |
| 2 | 12,947 | call the anim object's `+0x70` callback with the command's word 3 [M 0x1c1b38 case 2] | not replayed |
| 3 | 53 | `Sound_StopAllWithId(word 3)` [M 0x1c1b38 case 3] | not replayed |
| 4 | 6,925 | footstep: `ScriptFootstepFoot = false` | death.rs |
| 5 | 6,976 | footstep: `ScriptFootstepFoot = true` | death.rs |
| 6 | 1,294 | `Effect_Create(1.0, ..., 0xc0)` body effect | death.rs:174 |
| 7 | 4 | weapon bone-world transform via `AnimDatumGetWeaponInfo` [M 0x1c1b38 case 7] | not replayed |

death.rs:126 already documents that kinds 1/2/3/7 "never occur in 0x06000151 and kind 2's
handler (+0x70) is zeroed for player scripts [M 0x1c1600:51]" — the census shows they occur
13,056 times across all scripts; whether any non-player anim object carries a non-zero +0x70
callback is the live check at the end of this document.

## Records the game itself drops

- One **0xDD**, one **0xF6**, one **0xF7** placement per level archive (33 each; hashes
  0x02000f34/0x02000f37/0x02000f38) and 39 dynamic **0xFF** records: no case in
  `parsemap_create_dynamic_objects` — silently ignored [M 0x1d04f0 default].
- **165 placement records with flags = 0xFFFFFFFF**: skipped by the game's walkers (bit 0xa000
  mask) [M 0x1d04f0], skipped by the port identically.
- 10,520 records carry flags bit 0x2000 and are skipped by the game's dynamic walker
  [M 0x1d04f0]; 5,602 carry 0x8000 and become static cels. The port matches both gates.

## Unhandled, ranked by occurrences

Items the game handles [M] but the port does not act on, most-occurring first. The anim-script
kind-2 row is ranked last-with-a-star: its handler is an indirect callback that is zeroed for
player scripts, so its real gameplay reach is narrower than its 12,947 count.

| # | item | occurrences | game handler | player-visible effect if ported |
|---|---|---|---|---|
| 1 | anim op-4 kind 2 callback | 12,947* | animobj+0x70 fn-ptr call [M 0x1c1b38 case 2] | scripted anim moments that hang per-object effects |
| 2 | placement 0xE5 cover corner | 584 | `Drone_CoverCornerNode` 0x136ff0 | guards take corner cover in firefights |
| 3 | placement 0xE6 cover low | 384 | `Drone_CoverLowNode` 0x137340 | guards crouch behind low cover |
| 4 | placement 0xE9 AI point | 162 | `Drone_AIPoint` 0x137550 | patrol/waypoint graph points |
| 5 | placement 0xE4 hurt volume | 64 | `Hurt_Create` 0x17e870 | damage volumes (hazards) |
| 6 | placement 0x28 sensor | 62 | `Sensor_Create` 0x1af010 | channel trip volumes drive world logic |
| 7 | placement 0x2D start-pos slot 0 | 51 | `Player_AddNewStartPos(...,0)` 0x195c00 | second start-position set |
| 8 | placement 0x27 GT | 41 | `GT_Create` 0x17ffa0 (+ Update/Activate/TakeControl) | controllable special object |
| 9 | placement 0xFB music trigger | 15 | `MusicTrigger_Create` 0x1b30e0 / `_Update` 0x1b31a0 | `Music_Event(id,2)` countdown music |
| 10 | placement 0x2C lightning | 15 | `Lightning_Create` 0x18cde8 | storm lightning in 07000001/07000041 |
| 11 | anim op-4 kinds 1/3/7 | 109 | flag toggle / `Sound_StopAllWithId` / weapon transform [M 0x1c1b38] | minor audio/anim polish |
| 12 | placement 0x39 space missile | 8 | `Create_SpaceMissile` 0x1a9ef0 | 0700001b set piece |
| 13 | placement 0xF3 tree | 8 | `Env_Tree_Create` 0x17c200 | trees in 07000002/07000004 |
| 14 | placement 0x2B leaf gen | 3 | `LeafGen_Create` 0x1acbd0 | leaf particle emitters |
| 15 | NIS event 0x08 coder drone | 1 | `Drone_CoderCreate` 0x13a108 | 07000001 gadget-drone intro |
| 16 | NIS event 0x09 cam mode | 1 | `Player_Cam2Mode` 0x195fc8 | 07000001 camera mode — **ported (nis-event9-cnis)** |
| 17 | NIS event 0x0a break | 1 | `Break_DoBreak` 0x12c168 | 07000001 script breakable |
| 18 | placement 0x38 space laser | 1 | `Create_SpaceLaser` 0x1aa8b8 | 0700001b set piece |
| 19 | trigger action 0xD movie | 1 | `Trigger_MoviePlayer` 0x1b2a08 / 0x1b1a78 case 0xd | 07000041 movie push |
| 20 | placement 0xF4 movie player | 1 | same as 19 | (same record) |
| 21 | entry kind 8 menu defs | 34 archives | `MenuManager_Load` 0x21de88 | runtime-parsed menus (currently transcribed) |
| 22 | entry kind 15 icon | 1 | `SetUpIconFile` 0x111270 | memcard icon (cosmetic) |

Ignored by the game too (not port debt): 0xDD/0xF6/0xF7/0xFF placements (138), flags 0xFFFFFFFF
records (165), NIS op none, anim ops 2/3 (0), event kinds 0x0e/0x10/0x13 (0), entry kinds
9/0xa/0xd/0xe (0).

## Top 10 ready-to-queue tasks

1. **AI cover nodes (0xE5 + 0xE6, 968 placements, every mission level).** Port
   `Drone_CoverCornerNode` 0x136ff0 and `Drone_CoverLowNode` 0x137340 into the drone AI: nodes
   bind to the cover graph guards query when engaging. Verify: guard in 07000006 (81+35 nodes)
   retreats to a corner node when fired on instead of standing in the open. Highest occurrence
   count of any placement family the game spawns for.
2. **AI waypoints (0xE9, 162).** `Drone_AIPoint` 0x137550: patrol graph points. Port the record
   (params unknown — check `Drone_AIPoint` reads) and wire into patrol pathing. Verify: 07000007
   has 34; guards patrol through them.
3. **Hurt volumes (0xE4, 64).** `Hurt_Create` 0x17e870: damage volumes (params carry damage
   kind/rate — read the decomp). Verify: standing in one drains Bond's armour/health; 07000005
   has 13 (the Alpine incursion? verify level name at render time).
4. **Sensor volumes (0x28, 62).** `Sensor_Create` 0x1af010: trip-by-anything volumes writing
   switch channels (not player-only like 0xEA/0xEB). Verify: a drone walking through sets the
   sensor's out channel; drives doors/alarm logic the port currently needs touch triggers for.
5. **Music triggers (0xFB, 15).** `MusicTrigger_Create` 0x1b30e0 + `MusicTrigger_Update`
   0x1b31a0: `Music_Event(id, 2)` while its countdown runs. Verify: entering the volume starts
   the combat music stinger; leaving lets it expire.
6. **Lightning (0x2C, 15; 07000001 ×11, 07000041 ×4).** `Lightning_Create` 0x18cde8: storm
   flashes. Visual — needs the main agent's render pass to confirm the flash/timing model.
7. **NIS event kinds 0x08/0x09/0x0a (script 0600005b, 07000001).** Three single-line handlers:
   `Drone_CoderCreate` 0x13a108 (spawn coder drone: params skin 0x0500001b, anim 0x060000a6,
   0x10, 1), `Player_Cam2Mode(0, p0)` 0x195fc8, `Break_DoBreak` 0x12c168 at the stream object.
   Small, fully specified, unblocks the training level's gadget-drone intro cut-scene.
8. **GT object (0x27, 41 — two per MP map).** `GT_Create` 0x17ffa0, `GT_Update` 0x180498,
   `GT_Activate`/`GT_TakeControl`: a controllable special object [UNCERTAIN semantics — needs
   an EE probe of the object it builds before queuing the port]. Two per MP map suggests
   objective equipment.
9. **Start-position slot 0 (0x2D, 51, one per level incl. every MP map).**
   `Player_AddNewStartPos(...,0)` 0x195c00 vs the port's 0x24-only registration
   (nf-view/src/lib.rs:1141): the port may be spawning Bond at the wrong start where a level
   ships both (07000007 ships 5 of type 0x2D).
10. **Movie trigger (0xF4 / action 0xD, 1, 07000041).** `Trigger_MoviePlayer` 0x1b2a08 +
    `Trigger_Update` case 0xd: input channel clears, `GameState+0x16 = out`, GameFlow push 6/7
    (movie + fade). Wire into the port's `resolve_exit_code` path like LOAD_LEVEL does.

(Watchlist, not tasks: kind-8 menu-def runtime parser — the transcription already covers the
menus; kind-15 icon — cosmetic; anim op-4 kinds 1/3/7 — 109 commands, minor.)

## The pinning test

`crates/nf-formats/tests/script_census.rs` (added by this task) re-runs the census through the
port's own parsers on every 07* archive and pins the handled sets and the headline counts as
constants: 20 NIS opcodes handled, 14 event kinds, 7 trigger actions, 56 placement types; and
the disc totals (468 NIS scripts, 5,263 streams, 851 events with 3 on unhandled kinds, 18,231
anim scripts, 50,279 anim commands, 7,372 dynamic placements, 676 triggers, 1 movie trigger).
A regression that stops handling something, or a newly handled opcode, changes a constant and
fails the test — update it and the tables here together.

## Not censused (follow-ups)

- Per-SimpleScript state/paths (110 placements; port `SsList` covers the state machine —
  nf-game/src/ss.rs cites SS_Create 0x1a9c58 / SS_Activate 0x1a9ea8 / SS_Update 0x1a9948).
- DRIVING.ELF-side script surfaces (driving levels 080000xx live outside the on-foot engine).
- EE-side script invocations that bypass level data (GT objects loading NIS via
  `GT_Create` 0x17ffa0 [M 0x1d94c0 callers], Mine_Update 0x17c930, FMV handlers).
- Text label resolution for the 81 Text commands (needs the USATxt.dat banks).

## Live checks for the main agent

1. **animobj+0x70 during op-4 kind-2 replay.** In PCSX2 with gdbwatch, watch a drone's anim
   object +0x70 (word) during a walk anim in any level, access type R, expect: 0 for player
   scripts [M 0x1c1600:51], non-zero for at least some drone anim objects — this decides
   whether the 12,947 kind-2 commands are live callbacks or dead data.
2. **07000041 movie trigger.** Render `--bond` at the 0xF4 placement's volume after arming its
   input channel: the real game pushes movie states 6/7. Expect the port currently does
   nothing there (action 0xD unhandled).
3. **07000001 coder-drone intro.** Play the 0600005b cut-scene (the script with the three
   unhandled events): the real game spawns the gadget drone (event 0x08) and overrides the
   camera (event 0x09). The port drops both — confirm the visual gap against the render.
