# Switch ext (the switch instance, 0x1C bytes at obj+0x110; class 0x39)

Proof levels: see [README.md](README.md). Instance size 0x1C from `Switch_Create` 0x1adcc0
(`control_create_object(0x1c,…)` 0x1adcc0:15), confirmed by `Sensor_Init` 0x1b0868:7
`LList_Init(0x2bf0f8, 0x1c, 0)` — the switch list's node size — and by the snapshot's
`SwitchList+0xA = 0x001C` [L]. Class byte `obj+0xFF = 0x39` (0x1adcc0:33).
Level: ee_start_05 (07000005 at start) unless noted.

**No live instances in any snapshot.** Walking all three EE snapshots
(`ee_start_05.bin` 523 objects, `ee_start_07000001.bin` 569, `ee_snapshot.bin` 495) finds
**zero class-0x39 objects** — 07000005/1 place no switches. Per-instance [L-read]s are
therefore impossible today; the [L] evidence below is the global list/table state. Live
verification needs a level with 0x29 placements (the Tower levels 07000009/a/b are the ones
the port models; see Live checks).

Full reader census: the ext is touched only by `Switch_Create` 0x1adcc0, `Switch_Update`
0x1adea8 (`control_funcs[0x39].update`), `Switch_Activate` 0x1b02a8 (via `SP_Activate`
0x1dc338:62), `Switch_GetNearest` 0x1b0398 and `Switch_GetSwitchChannel` 0x1b0430 (both only
via `DroneFunc_SetMissionFailReason` 0x13e560, fail reason 0x10). The instance is also an
LLISTINFO **node**: `LList_Add(0x2bf0f8, inst)` 0x1adcc0:58, so +0x00/+0x04 are list links.
No other decompile file touches these byte offsets on a switch-shaped base.

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | SwitchList prev link (the node's first word; `LList_Add` appends at the tail and zeroes both links of a first node) | [M\*] W `LList_Add` 0x1cca28:12-19; R `LList_Remove` 0x1ccac8 walkers. [L-read] `SwitchList` @0x2BF0F8 = {first 0, last 0} — the list is empty on 07000005 |
| +0x04 | u32 | SwitchList next link | [M\*] W 0x1cca28:12-19; R `Switch_GetNearest`'s walk — `lw $s0, 4($s0)` (ELF disasm 0x1b03f0/0x1b03fc: node→node at +4) |
| +0x08 | u32 | the owning `obj_tag*` (self-pointer) | [M\*] W 0x1adcc0:22 (`*(int *)(iVar2 + 8) = iVar7`); R `Switch_GetNearest` 0x1b0398 — `lw $a1, 8($s0)` (ELF 0x1b03d0) → `Vec_SqDist3D(pos, obj+0x30)`, `lw $s1, 8($s0)` (0x1b03f4) = the obj returned. NOTE: Ghidra prints the walk rotated (`iVar1 = *(int *)(SwitchList + 8)`); the ELF shows the first node is `*(SwitchList+0)` |
| +0x0C | u32 | the switch's script-player handle (`SP_Create`'s return) | [M\*] W 0x1adcc0:44-46; R 0x1adcc0:51-57 (`SP_Update`/`SP_UnPause`/`SP_GetState` drive-to-settled at create), `Switch_Update` 0x1adea8:23/30 (`SP_UnPause`), `Switch_Activate` 0x1b02a8:23 (`SP_GetState(inst+0x0C) == 0` gate — script idle, i.e. the current `SCRIPTINFO+0x9C == 0`, `SP_GetState` 0x1dc900:13-20) |
| +0x10 | u16 | **auto-toggle delay** (placement **param 2**), frames: the watchdog fires once the channel has been off the rest value longer than this. 0 = no watchdog | [M\*] W 0x1adcc0:25 (`*(undefined2 *)(iVar2 + 0x10) = *(undefined2 *)(param_3 + 0x34)` — level_tag+0x34 = param 2); R 0x1adea8:25/28 (`(int)(uint)*(ushort *)(iVar2 + 0x10) < (int)sVar1` — the old `obj+0xF6` counter exceeds it) |
| +0x12 | u16 | **the switch channel** (placement **param 0**) | [M\*] W 0x1adcc0:23 (`= *(undefined2 *)(param_3 + 0x2c)`); R 0x1adea8:11/17/20/26/31-33/38 (every read in Update), 0x1b02a8:20-27 (Activate), `Switch_GetSwitchChannel` 0x1b0430:7 (`*(undefined2 *)(*(int *)((int)param_1 + 0xe0) + 0x12)`), 0x1adcc0:30 (low byte → `SSysItems[param6·8+4]`), 0x1adcc0:39-42 (load beat). [L-read] none (no instance); the channel table `switch_channels` @0x26FC90 it indexes is live (ch 0x45 = 1 at start) |
| +0x14 | u16 | **inhibit channel** (placement **param 5**): while it is ON the switch cannot be hand-activated | [M\*] W 0x1adcc0:28 (`= *(undefined2 *)(param_3 + 0x40)` — param 5); R `Switch_Activate` 0x1b02a8:22 (`*(ushort *)(iVar2 + 0x14) == 0 \|\| switch_channels[*(ushort *)(iVar2 + 0x14)] == '\0'` — allowed only when unset or OFF). Single reader, fully enumerated |
| +0x16 | u16 | **rest value** (placement **param 3**): the value the watchdog enforces the channel back to; also the initial channel value at load and the `obj+0xF4` mirror seed | [M\*] W 0x1adcc0:26 (`= *(undefined2 *)(param_3 + 0x38)` — param 3); R 0x1adcc0:32/34/40 (`obj+0xF4 = +0x16`, `switch_channels[ch] = (u8)+0x16` load beat), 0x1adea8:26 (`(u8)switch_channels[ch] != *(ushort *)(iVar2 + 0x16)` watchdog trigger) |
| +0x18 | u8 | **activation counter** — bumped by `Switch_Update` on every channel-mirror divergence and every watchdog fire; **never cleared anywhere in the decompile** → with +0x19 set it is a one-shot latch | [M\*] W 0x1adea8:22/29 (`= … + '\x01'` — the only two writers); R 0x1adea8:14 (`if (*(char *)(iVar2 + 0x18) != '\0') return;`), 0x1b02a8:15 (same gate in Activate) |
| +0x19 | u8 | **one-shot enable** (placement **param 4**, low byte): when non-zero, the switch goes permanently inert once +0x18 ≠ 0 (both Update and Activate early-return) | [M\*] W 0x1adcc0:27 (`*(undefined1 *)(iVar2 + 0x19) = *(undefined1 *)(param_3 + 0x3c)` — param 4); R 0x1adea8:10, 0x1b02a8:11. Single reader pair, fully enumerated |
| +0x1A..+0x1B | — | padding (`control_create_object` zeroes the whole allocation; nothing else touches it) | [M\*] `control_create_object` 0x1344e0 memsets instance_size+0x110 |

## Param map (from the creator)

`level_tag` = the 0xB0-byte runtime placement record; param k at `level_tag+0x2C+4k`.
`Switch_Create` (0x1adcc0) reads, by address:

| level_tag addr | param | width | → instance / effect |
|---|---|---|---|
| +0x2C | 0 | u16 | **+0x12** — the switch channel |
| +0x30 | 1 | u32 | the switch's script/model id: passed to `SP_Create(..., p1, 0, 8, 0, obj, 0)` (0x1adcc0:24/44-45 — SP entity kind 8). **No ext field.** |
| +0x34 | 2 | u16 | **+0x10** — auto-toggle delay (frames) |
| +0x38 | 3 | u16 | **+0x16** — rest value (initial channel value + `obj+0xF4` seed) |
| +0x3C | 4 | u8 | **+0x19** — one-shot enable |
| +0x40 | 5 | u16 | **+0x14** — inhibit channel |
| +0x48+ | 6+ | — | not read by the creator (param 6's slot +0x44 was checked: it is the SSysItems **index**? no — see next row) |
| +0x44 | 6? | u32 | CAREFUL: the SSysItems line reads `*(int *)(param_3 + 0x44)`. 0x44 = param 6's slot. If non-zero: `SSysItems[p·8+4] = (u8)ch` (0x1adcc0:29-31) — registers the switch's channel as a sound-system item (`SSysItems` @0x2BEFD0, 5 items × 8 B: f32 timer, u8 channel, u8 state; `SSys_Monitor` 0x1ad998 scans items 1-4). **No ext field.** |

Load beat (0x1adcc0:39-43): if ch ≠ 0 → `switch_channels[ch] = (u8)+0x16` (param 3), and when
`(s16)+0x16 != 0` also stamp `switch_channels_time[ch]` (0x1adcc0:42 — a rising-edge stamp at
load only). If `obj+0xF4` (= param 3) == 1 the SP is unpaused and driven to state 0 at create
(0x1adcc0:52-57 — a switch created "on" finishes its animation before first use).

## Behaviour summaries

**`Switch_Update` (0x1adea8, `control_funcs[0x39].update`, per frame)** [M]:
1. +0x19 ≠ 0 && +0x18 ≠ 0 → dead (one-shot spent), return (0x1adea8:10-16).
2. `(u8)switch_channels[ch] != obj+0xF4` (the mirror) → `obj+0xF4 ^= 1`, +0x18 += 1,
   `SP_UnPause(inst+0x0C)` — play the toggle animation (0x1adea8:19-24).
3. Watchdog: while (+0x10 ≠ 0 && `(u8)ch != +0x16`) count `obj+0xF6++`; when the **old**
   counter exceeds +0x10: +0x18 += 1, `SP_UnPause`, then `switch_channels[ch] ^= 1` — the
   channel is forced back toward the rest value — `obj+0xF6 = 0`. The timestamp is written
   **only when the toggle left the channel 0** (falling edge; 0x1adea8:25-41). Note the
   counter is **cumulative since the last fire** — it is NOT reset when the channel returns
   to rest by itself (there is no else-arm).

**`Switch_Activate` (0x1b02a8, from `Player_Activate` 0x1a5170:121-123 → `SP_Activate`
0x1dc338:48-63)** [M]: the E path reaches the switch only through its SP: the use ray hits a
class-0 object with `obj+0xE4 != 0`, `SP_Activate` dereferences the SP instance
(`*(obj+0xE4)`), whose +0x00 is the switch obj (seeded by `SP_CreateScriptPlayer`
0x1db9a8:63 `*puVar2 = param_4`), and dispatches on the owner's class: 0x36 → `GT_Activate`,
**0x39 → facing check then `Switch_Activate`**, else the generic `SPinst+0x2C = 1` arm. The
facing check (0x1dc338:50-61): `dot(dir of the SP entity's obj+0x90 matrix, activator_row3 −
entity_row3) < 0` → refuse (you cannot use a switch from behind).

`Switch_Activate` gates, in order (0x1b02a8:11-24): (a) one-shot (+0x19 && +0x18) → return;
(b) NOT (activator class == 3 && ch == 0x61) — **Bond can never hand-toggle the alarm
channel 0x61**; (c) `obj+0xF4 == (u8)switch_channels[ch]` — the visual mirror is in sync;
(d) +0x14 == 0 || `switch_channels[+0x14]` == 0 — the inhibit channel is off; (e)
`SP_GetState(inst+0x0C) == 0` — the script is idle; (f) ch ≠ 0. Then
`switch_channels[ch] ^= 1`, timestamp **only on the falling edge** (0x1b02a8:25-28). The
mirror itself is not written here — the next `Switch_Update` sees the divergence and flips it
(which is what bumps +0x18 and unpauses the anim).

**`Switch_GetNearest` (0x1b0398)** [M + ELF]: walks `SwitchList` @0x2BF0F8 (first =
`*(head+0)`, next = node+4, payload = node+8) and returns the `obj_tag*` of the switch
nearest the given VECTOR by squared distance (best starts at 9999.9). Sole caller:
`DroneFunc_SetMissionFailReason` 0x13e560, reason 0x10 (a guard raising the alarm):
`Switch_GetNearest(drone+0xC pos)` → `Switch_GetSwitchChannel` (returns +0x12) → if the
channel is 0x61 set FailLabel 0x4000032, then `switch_channels[ch] = 1` + timestamp
(0x13e560:37-44).

Object-side fields this class uses: `obj+0xFF = 0x39` (0x1adcc0:33), `obj+0xF4` = the mirror
(seed param 3; `^= 1` per divergence — assumes 0/1 channels; R 0x1adea8:20-21, 0x1b02a8:21),
`obj+0xF6` = the watchdog frame counter (W/R 0x1adea8:27/35/40), `obj+0xD8 = 0` (0x1adcc0:35),
`obj+0xF0 = *(u32*)(glist+0x44)` (0x1adcc0:37). The switch object is never deleted
(`control_funcs[0x39]` has no delete/collide) and never removed from SwitchList — the list
lives for the level's lifetime.

## Snapshot checks (ee_start_05.bin etc.) [L]

- **No class-0x39 objects** in ee_start_05 (07000005), ee_start_07000001 or ee_snapshot.bin —
  the walked histograms contain classes 0x00–0x52 but no 0x39 (nor 0x3C).
- `SwitchList` @0x2BF0F8 [L-read]: `u32 +0 = 0`, `u32 +4 = 0`, `u32 +8 = 0x001C0000` →
  count16 = 0, **node size16 = 0x001C** (the `Sensor_Init` pre-state) — confirms the instance
  size and that no switch was ever registered on this level.
- `SSysItems` @0x2BEFD0 [L-read]: item 0 = zeros; items 1-4 = {timer −1.0, channel 0xFF,
  state 0} — `SSys_Init` 0x1b01b0's empty state. A switch with param 6 ≠ 0 would overwrite
  one channel byte here at create.
- `switch_channels` @0x26FC90 [L-read]: all 0 except ch 0x45 = 1 — the table the switch
  family reads/writes is live in the snapshot.

## Port mismatches (crates/nf-game/src/useobj.rs; report only, do NOT fix)

The port's switch is `SwitchObj` in useobj.rs. Its **param indices are all correct**
(`ch = p0, auto_delay = p2, rest = p3, busy = p4, gate = p5` — useobj.rs:118-123 and the
66-67 comment match the decompile's +0x12/+0x10/+0x16/+0x19/+0x14). The behavioural
divergences:

1. **Inhibit gate polarity inverted** — game: usable while the param-5 channel (+0x14) is
   **off** (`+0x14 == 0 || switch_channels[+0x14] == '\0'`, 0x1b02a8:22); port requires it
   **on**: `if s.gate != 0 && !ch.on(s.gate as u32) { continue; }` (useobj.rs:206).
2. **Param 4 is a one-shot latch, not a cooldown** — game: with param 4 ≠ 0 the switch is
   permanently inert after its first activation (+0x18 is bumped on the first channel change
   or use and never reset anywhere); port: `if s.busy && s.cooldown > 0 { continue; }` +
   `s.cooldown = 60` (useobj.rs:205/215) — a re-armable cooldown instead of a latch.
3. **The script-idle gate is dropped for non-busy switches** — game: `SP_GetState(inst+0x0C)
   == 0` gates EVERY activation (0x1b02a8:23); port gates only `s.busy && s.cooldown > 0`,
   so a switch with param 4 = 0 can be re-used while its script anim is playing (and the SP
   itself — created from param 1, kind 8 — is unmodelled).
4. **Missing `SP_Activate` facing gate** — the game refuses a use when
   `dot(entity_dir, activator − entity) < 0` (0x1dc338:50-61); the port's `player_activate`
   tests only the radius-1.0 sphere (useobj.rs:187, 211).
5. **Timestamp edge** — the switch family stamps `switch_channels_time` only on **falling**
   edges (0x1adea8:34-38, 0x1b02a8:26-28); the port's `Channels::set` stamps every non-zero
   write (triggers.rs:86), so switch-driven rising edges get spurious stamps (feeds SEQ_AND
   windows downstream).
6. **Mirror semantics** — game XORs `obj+0xF4`, bumps +0x18 and unpauses the script anim on
   ANY external channel change (0x1adea8:19-24); port copies the value
   (`if s.value != cur { s.value = cur; }`, useobj.rs:160-162) — no bump (so the port's
   one-shot could never fire anyway), no anim.
7. **Watchdog counter** — game: `obj+0xF6` is cumulative since the last fire (not reset when
   the channel returns to rest by itself) and fires when the **old** count exceeds param 2
   (delay+2 divergent frames); port: `auto_ct` resets on every matching frame and fires at
   delay+1 (useobj.rs:163-174 vs 0x1adea8:25-41).
8. **`Switch_GetNearest`/`Switch_GetSwitchChannel` not modelled** — the fail-reason 0x10 path
   hardcodes ch 0x61 (ally.rs:36-38, a documented [S] stand-in); the game looks up the
   nearest switch's channel and only gives 0x61 the special label (0x13e560:37-44).
9. **No SSysItems registration** — param 6's `SSysItems[p·8+4] = (u8)ch` write (0x1adcc0:29-31)
   has no port equivalent, so the sound-system alarm items never see switch channels.

## Live checks (main agent)

No switch exists on 07000005, so pick a level with 0x29 placements (the port models the Tower
levels 07000009/a/b). Chain: `DynamicObjList` 0x2705a0 → head+0x14 → next obj+0x14; filter
`obj+0xFF == 0x39`; ext = u32 at obj+0xE0; the ext is also a node of `SwitchList` 0x2BF0F8
(first = `u32 0x2BF0F8`, next = ext+0x04, payload ext+0x08 = the obj).

- **Channel sanity**: `fieldprobe.py watch <ext>+0x12 2` — the u16 channel constant; then
  `watch` `switch_channels[ch]` (0x26FC90+ch) while pressing E in front of the switch: it
  must toggle 0↔1, and **`switch_channels_time[ch]` (0x270190+4·ch) must NOT change when the
  toggle turns the channel ON** (falling-edge-only stamp — the sharpest observable in this
  family, and the one the port gets wrong).
- **One-shot**: on a switch with param 4 ≠ 0 (probe `<ext>+0x19`), use it once, then
  `watch <ext>+0x18 1` (expect 1, never reset) and confirm a second E does nothing.
- **Inhibit channel**: with `<ext>+0x14` = ch X, `poke 0x26FC90+X 1 1` → E must stop
  toggling while X is on (the port's inverted gate does the opposite).
- **Watchdog**: on a switch with param 2 ≠ 0 (`<ext>+0x10`), flip its channel by hand
  (`poke 0x26FC90+ch`), watch `obj+0xF6` (u16) count up and the channel snap back to
  `<ext>+0x16` ~delay+2 frames later.

## Fixed

The inhibit-gate polarity (mismatch 1) and the busy one-shot latch (mismatch 2) are FIXED in
`crates/nf-game/src/useobj.rs`: `player_activate` allows use while the gate channel is OFF and
blocks permanently once the change latch (+0x18) is non-zero on a busy switch; the latch bumps on
every channel change (activate toggle + the tick's mirror sync). Fail-first tests
`switch_inhibit_gate_blocks_only_while_on` / `switch_busy_is_one_shot_never_rearms` (both fail
against the old `!ch.on` + 60-tick cooldown code — shown in the transcript). Mismatches 3/4 (the
SP-idle gate and the `SP_Activate` facing gate) remain open — they need the per-switch SP link
the port does not model yet.
