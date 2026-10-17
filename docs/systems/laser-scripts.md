# The level laser-grid beams and their scripts

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp / disassembly read here, [D] disc bytes
(`nf-extract nis <iso> <level.bin> <hash-hex>` / the map-entry dumps), [S] suggested. Runtime:
docs/systems/script-player.md (the ScriptPlayer state machine), docs/fields/switch.md (the switch
ext). Port: `crates/nf-game/src/drone.rs` (the mounts), `crates/nf-game/src/useobj.rs` (the
switch↔player link), `crates/nf-view/src/scene.rs` (the render attachments).

## What the beams are

The laser-grid / tripwire beams are **not** placements of their own: every grid level hides its
beams behind a **switch** (placement type 0x29) whose **param 1 is a NIS script hash**, and the
beams are that script's entity streams. `Switch_Create` 0x1adcc0:44-45 ends with
`SP_Create(pos, rot, param 1, 0, 8, 0, obj, 0)` [M]:

- the embedded ScriptPlayer's script A = param 1 (`SP_Create` 0x1dc7e0:72-74 stores it at the
  tag's script-A slot +0x2C; script B stays 0);
- its mode = **8 = play looping** (param 5 lands in both mode slots +0x34/+0x38;
  `SP_LoadScript` 0x1dbc08:46-49 → `Script_Play(loop=1)`, state 6) — the show runs from level
  load, no trigger needed;
- the SP's mask/channel/gate/scale all stay 0 (`SP_Create` memsets the tag; param 6 = 0) [M].
  Placement param 5 is the switch's own **inhibit channel** (ext+0x14, docs/fields/switch.md) —
  it never gates the player.

`SP_CreateScriptPlayer` 0x1db9a8:18 refuses a both-scripts-zero SP, and a param 1 that is not a
hash (a model id) fails the lib lookup → state 5 — the game's own path [M].

## The show script (0x06000667 — 07000009's lobby grid)

[D `nf-extract nis <iso> 07000009.bin 06000667`] length 985 ticks, loop 0, 3 streams:

| t | stream | command |
|---|---|---|
| 0 | 0 | `Entity` hash **0x020005ed** `SCRIPT_Lasers_ON` (obj_type 0x1F) |
| 2 | 1 | `Event` kind 3, p0 = 0 → `SCRIPTINFO+0x9c = −1` (the hold) |
| 31 | 0 | `Entity` 0x02000a29 `SCRIPT_Lasers_AMBER` |
| 496 | 0 | `Entity` 0x020005ec `SCRIPT_Lasers_OFF` |
| 527 | 0 | `Entity` 0x02000a29 again |
| 985 | 0 | `EntityEnd` → loop restarts, ON again, holds at 2 again |

Stream 2: sound 0x4a1 at t1 (0x1c long), 0x4a0 at t496 (0x20b), 3-D on stream 0's object
(`Script_SoundStart` 0x1d8820). The keys (8, all identity at the origin) never move anything.

The mechanics, each [M]:

- **The art objects are world-space.** All three art glists are placed by 0x2031-flag records at
  the origin with identity rotation and no params (09's map 0100009f placements idx 68/69/143:
  `SCRIPT_Lasers_OFF/ON/AMBER`), like the level-1 rooms — the beams are modelled at their world
  positions, so an identity-matrix entity draws the whole grid. Flags bit 0x2000 has no reader in
  `parsemap_block_map_data_static` 0x1d1998 (only 0x8000 builds a cel) — the static art draws
  nowhere; type 0x31 → `FuseBox_Create` 0x1b0448 makes an invisible wire-box.
- **The entity cycle swaps one object.** `Script_CreateEntity` 0x1d8968:92-101: a same-class
  stream object is RE-POINTED at the new hash's gfx (`hashtable_set_object_to_entity_gfx`) — ON
  becomes AMBER becomes OFF; nothing accumulates. obj_type 0x1F matches no special case →
  `control_create_object(0, …)` + the gfx, then `control_unlink_object` — a draw-only object, no
  collision, no update [M 0x1d8968:170-175].
- **The hold.** Event kind 3 sets the delay to p0 − 1 = −1 [M `Script_EventHandler` 0x1d94c0];
  `Script_Update` 0x1da998:17-18 runs only at delay == 0 and only decrements POSITIVE delays, so
  −1 freezes the script (clock stops, no further commands) until something zeroes the delay.
- **The unpause.** `Switch_Update` 0x1adea8:19-24: when the switch channel's value stops matching
  the ext+0xF4 mirror, the mirror flips and `SP_UnPause(inst+0x0C)` fires — `SP_UnPause` 0x1dca58
  sets the +0x2C activate latch and zeroes `SCRIPTINFO+0x9c`. So EVERY channel change (the E
  toggle, `Switch_Activate` 0x1b02a8:25, or any other writer) replays the whole 983-tick show:
  ON → ON+AMBER → +OFF → AMBER swap → all deleted at 985 → ON, frozen again.
  `Switch_Activate`'s own gate is `SP_GetState(sp) == 0` [M 0x1b02a8:23] — `SP_GetState` 0x1dc900
  returns `SCRIPTINFO+0x9c == 0`, i.e. the toggle is allowed exactly while the show is **held**
  (or its script not yet loaded — the null-SCRIPTINFO arm returns false = allowed), never
  mid-play.
- The switch object itself sits at its placement transform — for 09's #71 that is the origin
  (`Switch_GetNearest` 0x1b0398 walks the SwitchList reading obj+0x30 = the placement pos [M]),
  so the E-use sphere at the origin is as unreachable in the game as it is in the port.

## The inventory (every script-driven laser on the disc)

Scanned all 07* archives' type-7 NIS entries + every object named `*aser*` and their placements
[D]. The SCRIPT-driven laser shows:

| level | driver (switch, ch, param 1) | script (length, streams) | art (hash → name) | touch triggers |
|---|---|---|---|---|
| 07000009 | #71 `SCRIPTPLAYER_Lasers` (0x29, ch 0x37, p1 = **0x06000667**, inhibit 0) | 985 t, 3 streams, holds t2 | 0x020005ed ON / 0x020005ec OFF / 0x02000a29 AMBER (map 0100009f) | none on 09 — the grid is presentation |
| 0700000a | #643 `SCRIPTPLAYER_KeyboardLasers` (0x29, ch 0x3B, p1 = **0x06000664**, inhibit ch 0x3C = 60) | 985 t, 2 streams, holds t2 | 0x020005e0 ON / 0x020005df OFF / 0x02000a2d DURING (map 010000a0) | the office web's ~30 `laser`/`laserbeam` 0xEBs are SEPARATE (below) |

The 0EB "laser"/"laserbeam" TOUCH triggers (0a's office web #606-#662, 0b's dome fence
"laserbeam-large") are alarm volumes, not beams: their glist flags word 0x50 has bit 0x10 = not
drawn [M docs/fields/glist.md, docs/fields/trigger.md], their beams ride the room's static
geometry. `SCRIPTPLAYER_Lasers`' ch 0x37 is their **lock** channel on 0a [D the 0xEB params;
0700000a.md's laser-web section] — toggling the 09 switch disarms the next level's grid.

Everything else laser-named on the disc is the gadget family, not the grids: `LaserSight` /
`Laser_gun_Bolt` / `Laser_Blast_frame_1/2` (the weapon), `Lock - Laser` (the burn target),
`taser`/`taser_muzzle`, 07000014's `Beamlaser*` (the prototype's mounted cut-scene laser,
docs/levels/0700000d.md), and the fusebox-wire scripts (0x06000041/42 — event kind 0xF
WireRegister, already ported as the fusebox players).

`nf-extract nis`' hashes 0x06000052/0x06000027 (37/24 entity streams) are the **explosion**
scripts (`Explode_Create`'s per-source scripts [M 0x12bc50-0x12bd20], entities cloud_1/smoke/
Blast_frame_*) — they are already handled as the blast scripts and are not lasers.

## The port (before → after)

The port's `SwitchObj` never read param 1, so neither show existed: no laser player, no beam
entities, nothing to render (fail-first: `crates/nf-view/tests/laser_scripts_live.rs` panicked
"no mode-8 embedded ScriptPlayer for 0x06000667 — the laser show never mounts" on the pre-fix
tree). After (all [M]-cited):

- `Drones::attach_level` (crates/nf-game/src/drone.rs) mounts one `nis::Player` per main-map 0x29
  placement with a hash param 1 — scripts [p1, 0], modes [8, 0], mask 1 (the port's prime() skips
  mask-0 SPs; `SP_Create`'s own mask word is 0 — the fusebox player carries mask 1 for the same
  load-beat reason), out/gate 0, scale 1 — mirroring `Switch_Create`'s `SP_Create` exactly.
- `SwitchObj` carries the ext+0x0C handle (`sp`) and its main-map placement index;
  `Drones::attach_level` links them. `Switch_Update`'s mirror arm, the watchdog arm and the
  E-toggle all unpause the linked player (`SCRIPTINFO+0x9c = 0` + the activate latch —
  `SP_UnPause` 0x1dca58:10-14), and `Switch_Activate`'s script-idle gate [M 0x1b02a8:23] blocks
  the toggle while the show is mid-play (delay == 0).
- `nf-view`'s script-entity attachment sweep (scene.rs) includes the switches' scripts, so the
  beam art gets attachment slots and resolves through `Sim::script_entity` like the truck's.
- `Scripts::entities()` (crates/nf-game/src/nis.rs) is the per-beam state getter: `(script hash,
  stream, art hash, world matrix, alpha)` per live entity — which variant (ON/OFF/AMBER) is up,
  where, at what alpha, under which stream.

## Deviations / open items

- The port runs the switch's unpause on the SAME tick as the channel change; the game's
  `Switch_Update` runs after the SP in the object walk, so the show resumes one tick later there.
  With a 983-tick show this is invisible; noted for completeness. [*deviation*]
- The E-use of the origin-placed laser switch is as unreachable here as in the game (both read
  the placement/origin transform [M]) — the channel changes only through other writers in play.
- The show's sound (0x4a1/0x4a0) and the AMBER/OFF swap cycle only run after an unpause; a fresh
  level shows the static ON art, matching the game's load beat [D the t2 hold; M the delay rules].
- Live checks for the main agent (PCSX2): on 07000009, EE-watch `switch_channels[0x37]`
  (@0x26FC90+0x37) while using the lobby-area laser switch — expect 0→1 with the beam art still
  drawn (the beams never toggle off; only the NEXT level's touch triggers disarm); and confirm
  the lobby grid draws the ON art from level start (art object 0x020005ed alive at
  SCRIPTINFO+0x9c = −1).
