# FuseBox ext (the fusebox instance, 0x60 bytes at obj+0x110; class 0x3C)

Proof levels: see [README.md](README.md). Instance size 0x60 from `FuseBox_Create`
0x1b0448 (`control_create_object(0x60,…)` 0x1b0448:12). Class byte `obj+0xFF = 0x3C`
(0x1b0448:23). Level: ee_start_05 (07000005 at start) unless noted.

**No live instances in any snapshot** — the walk of all three EE snapshots (523 / 569 / 495
objects) contains no class-0x3C object (07000005/1 place none; the port knows 07000014's
`OBJECT_FuseBox1` as the canonical user). Per-instance [L-read]s are impossible today; the
[L] items below are the neighbouring global state. See Live checks for the 07000014 pass.

Full reader census: the ext is touched only by `FuseBox_Create` 0x1b0448,
`FuseBox_Register` 0x1b0970 (the SP create-callback, invoked from `Script_EventHandler`
0x1d94c0:172-175 case 0xF via `SCRIPTINFO+0x84/+0x88` — seeded from the SP instance's +0x04/+0x08
by `SP_CreateScriptPlayer` 0x1db9a8:63-65 and copied across by `Script_Load` 0x1d8248:143-146),
and `FuseBox_Update` 0x1adfe8 (`control_funcs[0x3C].update` — no collide/delete handler,
docs/atlas/object.md). No other decompile file touches a 0x60-byte ext at these offsets.

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | the box's script-player handle (`SP_Create`'s return; the same handle also lands in `obj+0xE4`) | [M1] W 0x1b0448:29-31 (`*puVar1 = uVar2`). No reader found in the decompile (the SP system drives the box through `obj+0xE4`) |
| +0x04..+0x2B | u32[10] | **wire group 0**: the objects the box's script creates (entity obj pointers) | [M\*] W `FuseBox_Register` 0x1b0970:22-25 (`*piVar2 = param_1` at `param_3 + 4 + k·4 + group·0x28`, group 0); R `FuseBox_Update` 0x1adfe8:85-97 (`piVar8 = (int *)(iStack_a4 + iVar4 + uVar10 * 0x28)` with `iStack_a4 = inst+4`) |
| +0x2C..+0x53 | u32[10] | **wire group 1** (the same registration, group 1) | [M\*] W 0x1b0970:22-25 (`+ param_2 * 0x28`, param_2 = 1 → base inst+0x2C); R 0x1adfe8:85-97 |
| +0x54 | u16 | group-0 output channel (placement **param 1**) — set to 1 when group 0 is fully destroyed | [M\*] W 0x1b0448:19 (`*(undefined2 *)(puVar1 + 0x15)` = byte 0x54 ← level_tag+0x30 = param 1); R/W 0x1adfe8:111-115 (`switch_channels[*puVar7] = 1` + timestamp + `*puVar7 = 0`, `puVar7 = iStack_ac + 0` = inst+0x54) |
| +0x56 | u16 | group-1 output channel (placement **param 2**) — set when group 1 is fully destroyed; **also the live-wire hurt disable flag** while group-1 objects die | [M\*] W 0x1b0448:20 (`(int)puVar1 + 0x56` ← level_tag+0x34 = param 2); R/W 0x1adfe8:99 (`if (*(short *)(iVar1 + 0x56) == 0) Player_Hurt(0x40a00000, glb_players, obj+0x80)` — hurt **only when unset**), 0x1adfe8:118-122 (the group-1 tail write) |
| +0x58 | u16 | group-0 registered count (bumped by Register, read by Update) | [M\*] W 0x1b0970:24 (`*psVar4 = *psVar4 + 1`, `psVar4 = param_3 + 0x58 + group·2`); R 0x1adfe8:110 (`*(short *)(iStack_b0 + iVar6)` with `iStack_b0 = inst+0x58` — fire only when ≠ 0). [L-read] none (no instance) |
| +0x5A | u16 | group-1 registered count | [M\*] W 0x1b0970:24 (group 1); R 0x1adfe8:110 (`iStack_b0 + 2`) |
| +0x5C | u16 | group-0 **second** output channel (placement **param 3**) — set together with +0x54 when group 0 empties; **cancelled** (written 0) whenever a group-1 object dies first | [M\*] W 0x1b0448:21 (`puVar1 + 0x17` = byte 0x5C ← level_tag+0x38 = param 3); R/W 0x1adfe8:102 (`*(undefined2 *)(iVar1 + 0x5c) = 0` on a group-1 death), 0x1adfe8:118-122 (`iVar1 + 0x5c + iVar6`, group 0) |
| +0x5E | u16 | group-1 **second** output channel (placement **param 4**) | [M\*] W 0x1b0448:22 (`(int)puVar1 + 0x5e` ← level_tag+0x3C = param 4); R/W 0x1adfe8:118-122 (group 1) |

## Param map (from the creator)

`level_tag` = the 0xB0-byte runtime placement record; param k at `level_tag+0x2C+4k`.
`FuseBox_Create` (0x1b0448) reads:

| level_tag addr | param | width | → instance |
|---|---|---|---|
| +0x2C | 0 | u32 | the box's script id → `SP_Create(pos, rot, p0, 0, 0x40, 0, obj, 0x1b0970)` (0x1b0448:29-30; mode word 0x40 → SPinst+0x26/+0x28, `SP_CreateScriptPlayer` 0x1db9a8:43-44; callback = `FuseBox_Register`, data = the box's instance) — **+0x00**, and the SP system also links it from `obj+0xE4` |
| +0x30 | 1 | u16 | **+0x54** — group-0 main output channel |
| +0x34 | 2 | u16 | **+0x56** — group-1 main output channel + hurt-disable flag |
| +0x38 | 3 | u16 | **+0x5C** — group-0 second output channel |
| +0x3C | 4 | u16 | **+0x5E** — group-1 second output channel |
| +0x40+ | 5+ | — | not read by the creator |

Dispatch: `parsemap_create_dynamic_objects` 0x1d04f0 case 0x31 (main map only, `param_7 != 0`)
0x1d04f0:182-186 — **every** main-map 0x31 placement creates the class-0x3C object, even with
param 0 = 0 (then `SP_CreateScriptPlayer`'s `tag+0x2C/+0x30 == 0` check returns 0, +0x00 = 0,
and the ext keeps only the four channel params).

## Behaviour summaries

**`FuseBox_Register` (0x1b0970, the SP create-callback)** [M]: `(obj*, u16 group, void* inst)`.
Guarded by `inst != 0 && group < 2`. Scans the group's 10 slots (inst+4+group·0x28+4k): a slot
already holding `obj` → return (no duplicate); the first zero slot takes `obj` and
`count(group)++` (inst+0x58/0x5A). Reached from the script's event 0xF
(`Script_EventHandler` 0x1d94c0:172-175: `(*(code **)(SCRIPTINFO+0x84))(createdObj, eventU16,
SCRIPTINFO+0x88-data)`) — i.e. the wires register as the box's script spawns them.

**`FuseBox_Update` (0x1adfe8, per frame)** [M]: for group g = 0 then 1, for each of the 10
slots: if the slot holds an object — `Collide_FilterBullets(obj+0xD0, 0xfffd)` (marks the
hit-list entries made by class-5 hitters whose kind word `*(u16*)(*(ext+0x44)+0x10) & 0xFFFD`
matches, `Collide_FilterBullets` 0x1ece30:24-31); if the hit list is then empty the wire
survived → `alive[g]++`; else it died: `Effect_RemoveSObjs(obj)` when `Control_NextLOD(obj) != 0`
(0x1adfe8:93-96), slot = 0, and for **group 1** additionally: if `+0x56 == 0`,
`Player_Hurt(0x40a00000 = 5.0, glb_players, dying obj+0x80 sphere)` — the live-wire/explosion
hurt, disabled by configuring param 2 — and `+0x5C = 0` (cancelling group 0's second channel
if it had not fired yet; 0x1adfe8:98-103). Tail per group: when `alive[g] == 0` and
`count[g] != 0`, write the group's channels — `switch_channels[+0x54+2g] = 1` + timestamp then
`+0x54+2g = 0`, then the same for `+0x5C+2g` (0x1adfe8:110-123) — both slots are one-shot
(the slot is zeroed, so it cannot refire).

The box object itself is never deleted (`control_funcs[0x3C]` = update only) and `obj+0xF4`
stays 0 (0x1b0448:24).

## Snapshot checks (ee_start_05.bin) [L]

- **No class-0x3C objects** in ee_start_05, ee_start_07000001 or ee_snapshot.bin (walked
  histograms have no 0x3C).
- Neighbouring globals [L-read]: `switch_channels` @0x26FC90 all 0 except ch 0x45 = 1;
  `switch_channels_time` @0x270190 pre-seeded 0x1A676 in all 256 cells (the
  `Init_SwitchChannels` state the fusebox timestamps would overwrite).
- No per-instance L-read possible; all rows above are [M\*]/[M1] only.

## Port mismatches (crates/nf-game/src/fusebox.rs; report only, do NOT fix)

The port models the box abstractly (one wire per group, cut by E). Divergences from the
decompile:

1. **Live-wire hurt condition inverted** — game: a group-1 death hurts iff the param-2
   channel (+0x56) is **0** (`if (*(short *)(iVar1 + 0x56) == 0) Player_Hurt(…)`,
   0x1adfe8:98-101 — configuring param 2 DISABLES the hurt); port hurts iff `ch[1] != 0`:
   `let hurt = if g == 1 && self.ch[1] != 0 { LIVE_WIRE_HURT } else { 0.0 };` (fusebox.rs:95).
   The port's own doc comment (fusebox.rs:12-13 "hurts Bond 5.0 … unless +0x56 == 0") matches
   the port code but contradicts the decompile. Note the same param doubles as the group-1
   output channel, so "param 2 configured" is the game's no-hurt case.
2. **Group story flipped** — the port calls group 0 the "safe wire first" and group 1 the
   "live wire" (fusebox.rs:19-22, 91-95); in the game the hurt is tied to group-1 deaths
   gated on param 2 == 0 — i.e. a fusebox WITH a param-2 channel (07000014-style sabotage
   box) never hurts, and one WITHOUT param 2 hurts on every group-1 destruction.
3. **`+0x5C` cancellation missing** — a group-1 death clears +0x5C (param 3's channel) before
   it can fire (0x1adfe8:102); the port writes `ch[2]` unconditionally when group 0 empties
   (fusebox.rs:97-101). Latent while the port cuts group 0 first, wrong if group-1 wires die
   first (in the game, e.g. by shooting) or both groups die the same frame.
4. **Create filter narrower than the game** — the port skips 0x31 placements whose param 0 is
   not a script hash (`if script < 0x0600_0000 { continue; }`, fusebox.rs:59); the game
   creates the class-0x3C object for every main-map 0x31 (0x1d04f0:182-186), scriptless ones
   getting an empty ext (all-zero but the channel params, +0x00 = 0).
5. **Wire count and cut mechanic are stand-ins** — **FIXED 2026-10-13 (fix-fusebox-cut)**: the
   port now registers the script's event-0xF entities per group (`FuseBox_Register` 0x1b0970,
   up to 10 slots; the wires are the PREVIOUS stream's object — `Script_EventHandler`'s record
   base +0x80 sits one 0x28 record below `Script_Run`'s +0xa8, the same previous-stream pattern
   as events 6/0xB) and kills them through the bullet path: `Collide_FilterBullets(list, 0xfffd)`
   drops class-5 hitters whose `weapon_definition+0x10` category word ANDs non-zero
   [M 0x1ece30:24-31 — the same drop polarity `Break_Update`'s `ext+4|0x106` mask and MP
   demolition's mask 1 confirm]; a wire with a surviving hit dies. Disc words [L weapon_data
   dump + USATxt.dat labels]: guns 0x0008 (dropped — gunfire never cuts), the **Laser**
   0x4E/0x4F 0x0002 (kept — it cuts), the Stunner 0x0004, Grapple/Micro-Camera/Decryptor/
   Q-Worm 0x0100 (dropped), grenades/Ronin/Shaver 0x0000 (kept). The E stand-in is removed:
   `Player_Activate` 0x1a5170:95-160 has no class-0x3C arm (SP(class 0 + obj+0xE4), 0x1C door,
   0x1D trigger, 0x25 SS, 0x28 car, 0x36 GT, 0x38 monitor, 0x3A lock, 0x47 GunImp, 0x49 PCQWorm).
6. **Hurt has no position** — the game hurts from the dying object's world sphere
   (`obj+0x80`, 0x1adfe8:100); the port applies a flat 5.0 (fusebox.rs:95).
7. **`gone`** — the port sets `gone` when both groups are empty (fusebox.rs:107); the game
   keeps the object alive (inert) forever.

## Live checks (main agent)

Needs 07000014 (the port's fusebox level). Chain: `DynamicObjList` 0x2705a0 → head+0x14 →
next obj+0x14; filter `obj+0xFF == 0x3C`; ext = u32 at obj+0xE0.

- **Channels**: `fieldprobe.py watch <ext>+0x54 2` (and +0x56/+0x5C/+0x5E) — each holds its
  placement param until its group empties, then flips to 0 the frame after
  `switch_channels[ch]` (0x26FC90+ch) turns 1. Watch both together: the channel byte and the
  slot clear on the same frame.
- **Registration**: `watch <ext>+0x58 2` / `<ext>+0x5A 2` while the box's lid script plays —
  the counts must step up as the script's event-0xF entities spawn (the port has no
  equivalent; its wires are pre-set).
- **The hurt polarity** (mismatch #1): stand next to the box and shoot a group-1 wire with
  `+0x56 == 0` → the game must hurt Bond 5.0; then `poke <ext>+0x56 2 <nonzero ch>` and
  repeat → no hurt (the port does the opposite on both).
- **Group-1-first cancellation**: `watch <ext>+0x5C 2`, destroy a group-1 wire before
  finishing group 0 → +0x5C must snap to 0 and `switch_channels[param3]` must never turn on.

## Fixed

**The hit-driven cut is ported (fix-fusebox-cut, 2026-10-13)**: the wires register through the
box script's event 0xF (07000008's 0x060008AD stream 2 → group 0; 07000014's 0x06000A1B streams
2/6 → groups 0/1 [L NF_SCRIPTLOG]), the bullet walk marks kept-kind contacts on the wire's
sphere at the script object's world position [S radius 0.5, unmeasured], and `FuseBox_Update`'s
filter/death/channel logic runs per tick (the E stand-in removed). Tests:
`crates/nf-view/tests/fix_fusebox_cut_live.rs` (the laser cuts 07000008's box → ch 137; the E
and gunfire sisters do nothing) + the re-driven `the_07000008_safe_chain_works_in_play` and
`walk_07000014_live::the_facility_chain_runs_to_the_delta_sector_exit` stage 3.

The two gameplay-relevant rows (+0x56 hurt polarity, +0x5C clear) are FIXED in
`crates/nf-game/src/fusebox.rs` (commit `fields-world: the fusebox live-wire polarity...`):
`player_use` now hurts iff `ch[1] == 0` and clears `ch[2]` on a group-1 cut, with the fail-first
tests `live_wire_hurts_only_when_param2_unset` / `group1_death_cancels_group0_second_word`
(they fail against the old `!= 0` reading — shown in the session transcript) and the NF_ISO test
`fusebox_07000014_param2_decides_the_live_wire_hurt` (crates/nf-game/tests/fields_live.rs):
all five of 07000014's real fuseboxes carry param 2 non-zero, so the old port hurt the player on
every live-wire cut and the fixed one never does — matching the game.
