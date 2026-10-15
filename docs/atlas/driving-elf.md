# The DRIVING.ELF map — main loop, subsystem inventory, shared code, the .crp consumer

Status: **phase 3 research** (task `drive-elf-map`), building on docs/atlas/driving.md (§6 phase 3).
Evidence tags: [M] code (function+address, DRIVING.ELF unless another ELF is named), [D] disc,
[L] live/trace, [S] suggested. Nothing from memory. All addresses are EE addresses; for
DRIVING.ELF file offset = EE − 0x107080 (docs/atlas/driving.md build identity).

The whole investigation is reproducible from `work/decomp/DRIVING.ELF/` (9,169 decompiled
functions) and `work/elf/DRIVING.ELF`; the cross-ELF matching below is the committed script
`tools/crosscheck/elf-shape-match.py`.

## 1. main and the frame loop

### 1.1 The boot wrapper (`FUN_00174bf0` 0x174bf0, size 1984) [M]

`entry` 0x24c0d8 calls it (docs/atlas/driving.md §3). Decomposed further here:

- One-shot head: `FUN_0024c2a0()` (crt0 residue), `FUN_00173c60()` (468 B), then the level-key
  switch: `iRam00100984` is a small state machine and `uRam00100974` selects one of 9 keys
  written into the PTP block at 0x1004f4 — cases 1–8 are `paris_mis01` (0x374da8),
  0x374db8 (`uw_mis11`-shaped data), `junglea_mis13a` (0x374dc8), `jungleb_mis13b` (0x374dd8),
  0x374de8, 0x374df8 (pointer cases), `junglec_mis13c` (0x374e08), `snow2a_race` (0x374e18);
  default `paris_map` (0x374e28). So the same menu state that ACTION drives also exists on the
  driving side [M]. (The jungle cases also write the 2-char suffix at 0x100500 — key+0xc..0xe,
  i.e. the `13a` tail is kept separately [M].)
- argv parsing (the `-streams/-ntsc/-pal/-pal60/-t/-T` switches documented in driving.md §3) —
  note this happens **once**, guarded by `cRam001004b4 == 0` [M].
- `FUN_00174bd8()` (20 B), key re-read, then the init chain `FUN_00174198` 0x174198 (progress
  prints; ends by spawning/priming the track object, §4) [M].
- print `Entering GameLoop` (0x375c40) via `FUN_002495e8` (a printf-style progress printer)
  and then, in order: `FUN_00165ed0(stack,mode)` — mission-session open (refcount up; on the
  0→1 edge registers the vblank/semaphore pair `FUN_002611f0(0x3c82b8, 0x165e60, 0x32c540,
  0x2000, 0x61)` + `FUN_00260ca0(0x165eb0)`); `FUN_00175cf8(0,0,0,0,mission,0)` — the mission
  setup (§1.3); `FUN_00166260(stack,2)` — session close/teardown counterpart; `FUN_001749e8()`
  — one-shot engine bring-up [M].
- Tail: exit flags (`uRam001009a0 & 0x80/0x100` → quit), then `FUN_00300058(0x100000, flag)`
  → BASE.ELF (driving.md §2) [M].

**There is no loop in main** — verified by disassembly: zero backward branches in the whole
1984-byte function (also none in `FUN_001749e8`/`FUN_00175cf8`) [M]. The per-frame loop is a
thread, §1.2.

### 1.2 The frame loop: a vblank-driven thread over a scheduler

The world singleton is `FUN_00167d80()` 0x167d80 → object at **0x3c87b0** (lazy init zeroes
0x3c87b0..0x3c8854, string-table setup `FUN_002a71f8(0x167d78)`) [M]. Two thread hooks control it:

- `FUN_00167e10(world, mode)` 0x167e10 (108 B): start/stop. Start = `FUN_00261920(0x167de8, 2, 0)`,
  stop = `FUN_002619e0(0x167de8, 2)`. Called with 1 at the end of the mission setup (inside
  `FUN_00176658`, below) and with 0 from `FUN_001683f8` during engine teardown [M].
- The thread entry `FUN_00167de8` 0x167de8 (40 B) → `FUN_00168060(world)` 0x168060 (92 B), the
  **per-vblank body** [M]:

  1. `FUN_0025a4a8()` 0x25a4a8 (144 B) — **input poll**: two ports × four slots of 0x20-byte
     device blocks at 0x419840, each through `FUN_0025a0d0(port, slot, dev)` [M].
  2. `world->0xa0 = FUN_00261038()` — the vblank/frame counter at **0x3c80d4** (gp-relative
     read; only the reader is in the decompile, the increment is interrupt-side) [M + S for
     the writer].
  3. `FUN_00167e80(world)` 0x167e80 (436 B) — **present pacing**: counts frames, checks
     `FUN_0025a538(0)`, decides the skip flag `world->0x9c` (interlace/30 Hz-style skip on the
     `(count & 3) == 1` branch), polls flip status `FUN_0023c618()`, on flip queues the render
     via `FUN_00165720()/FUN_00165900()` (the event queue), and walks the world object list
     stamping a state (3) into each object's +0x30 cell [M].
  4. If not skipping (`world->0x9c == 0`): `FUN_00168280(world)` 0x168280 (228 B) — the
     **scheduler**: two passes over the registered objects (array at world+0x10, count
     world+0xc; a parallel event-slot array at world+0x50). Pass 1 runs only when the tick gate
     world+0x8 is set, then clears it. Each pass calls the object's **vtable+0x1c** method
     (Update) with the object pointer `obj + *(short*)(vtbl+0x18)`. Pass 2 additionally calls
     `FUN_00168678(world, obj, eventslot)` per object [M].

- `FUN_00168678` 0x168678 (880 B) is the **visibility/LOD event dispatcher**: for each
  sub-object (count from vtable+0x24, stride 0x34) it tests channels 0–9 (case 8/9 are
  edge-triggered "became/went visible" with hysteresis margins at world+0xa8/0xac, comparing
  bounding-sphere floats at obj+0x24/+0x28 against `world->0xa0`) and queues events with
  `FUN_00165720()/FUN_00165850()`; the frame ends by queueing event **0x73** (115) [M].

So the per-frame order is: **input → vblank counter snapshot → present/pacing decision →
(sim pass 1 when gated) object Updates + visibility events (pass 2)** [M]. The subsystem bodies
(car physics, AI, camera, render) are the Update vtable methods of the registered objects —
the objects register through the init/mission-setup calls in §2; pinning each vtable is
phase-4 work (live check L-3).

### 1.3 The timestep

- `FUN_002d6b90()` 0x2d6b90 (32 B) returns `((float*)0x3c1a38)[DAT_00355c74]`; the table at
  0x3c1a38 is **{60.0, 60.0, 50.0, 60.0}** [M], indexed by the video mode (argv `-ntsc`=0,
  default=1, `-pal`=2, `-pal60`=3 per main's switch handling) [M].
- `FUN_00176658` calls `FUN_00110900(1.0 / FUN_002d6b90())` under the `Init Actors` progress
  print — the clock/scheduler tick is set to 1/60 s (NTSC) or 1/50 s (PAL) [M].
- Therefore the fixed timestep is **1/60 s on NTSC, 1/50 s on PAL** — 60 Hz confirmed from code
  (driving.md §3's "60 Hz assumed [S]" is now [M]); no wall-clock dependency found in the loop.

### 1.4 Mission setup and teardown (`FUN_00175cf8` 0x175cf8 → `FUN_00176658` 0x176658)

`FUN_00175cf8(mission)` stores the mission index at DAT_0032f150, copies the `mis0X` string
(table at 0x32e770), then runs `FUN_00176658` (2,108 B) — the annotated load sequence
(progress strings [D] at the addresses shown, all [M] call sites):

| print | call(s) |
|---|---|
| `Reset Ray Shells` 0x376b30 | `FUN_00199780` |
| `Reset Sim` 0x376b48 | `FUN_001feda0(0x339190)` |
| `Init Clock` 0x376b58 | `FUN_001799d8` |
| `Init Events` 0x376b68 | `FUN_001755f0` |
| `Init Scheduler` 0x376b78 | `FUN_00179c48`, `FUN_001968f8`, `FUN_00185668` |
| `Init Game Render` 0x376b88 | `FUN_001c4008` |
| `Init Sound` 0x376ba0 | speech `.spe` resolve `driving\%s.spe` (0x376c58), music/effects volume prints 0x376cb0/0x376ce0, audio mode 0x376d10 |
| `Init Actors` 0x376d28 | `FUN_0010b228`; `FUN_00110900(1/FUN_002d6b90())` (the timestep, §1.3) |
| `Init Weapon Manager` 0x376d38 | `FUN_0020bb00` on a 400-byte alloc |
| `Init World` 0x376d50 | `FUN_0022faf8(DAT_0033aac8)` — the track resolve (§4) |
| `Init AI Elements` 0x376d60 | `FUN_001c4840`; `Map `→`AIEl` → `FUN_0012be48` |
| `Init AI Controllers` 0x376d78 | `FUN_0013e688`, `FUN_00140110`, `FUN_00220c28` |
| `Init Roads` 0x376d90 | `FUN_002051f8`, `FUN_002151b8` |
| `Preload common models` 0x376dc0 | `FUN_00196740`…, attribute lookup `SECONDARY_TYPE`/`pvehicle` (0x376e18/0x376e28), car models dir `data\car\model\` (0x376e08) |
| `Spawn player car` 0x376e38 | `FUN_001ff850(0x339190, 0, …)` |
| `Init AI Characters` 0x376e50 | `FUN_001186b0` (flag DAT_00326000) |
| — | `FUN_001bcc70`, `FUN_00167e10(world, 1)` ← **the frame thread starts here** |
| `Leaving StartUp` 0x376e68 | end |

Teardown mirror (`FUN_00176e98` 0x176e98, 600 B; shutdown prints at 0x376e78 "Shutting Down
Game", `Kill World Camera` 0x376eb0, `Kill Player Camera` 0x376ec8, `Kill PIP Camera` 0x376ee0,
`Shutdown AI char` 0x376ef0): unregisters the track object (`FUN_0022fa40(obj,3)`,
`DAT_0033aac8 = 0`) [M].

## 2. Subsystem inventory

Method: the decompile clusters cleanly by address range (the linker kept object files in
order); anchors are the mission-setup prints (§1.4), the RTTI/symbol-pool class-name strings
[D] at 0x376010–0x3766f0 and 0x39b500–0x39b730 (`PVehicle`, `PBondCar`, `PhysicsObject`,
`COLLISION_INFO`, `Smackable`, `RayShell`, `WRoadNav`, `AIVehicle`, `AIHelicopter`,
`AICharacter`, `AnimationController`, `ActActor`, `WTrigger`, `RCamera`, `RRenderer`,
`RPlayerCamera`, `RWorldCamera`, `RDirectorQueue`, `RSkeletalObj`, `RVehicle`,
`RVehicleParticle`, `RShadowVolume`, `DeviceScalar`, `InputDevice`, `InputTable`,
`InputToAction`, `ActionQueue`, `IFeedback`, `AStream`, `AVoice`, `ASystem`, `SMissionRule`,
`SMissionTarget`, `SMissionTimer`, `WGrid`, `WWorldPos`, `WSoundGroup`, `AWorldSound`,
`WSound`, `UCarpNamespace`), and the shared-function matches of §3. Names with `Act`/`W`/`R`/`A`/`S`/`P` prefixes are [S] (class names from the binary, not yet tied to specific functions except where [M] is shown).

| Range (approx) | Subsystem | Anchors |
|---|---|---|
| 0x107100–0x10b000 | engine init, attribute system entry | `FUN_0010b228` (Actors init) [M] |
| 0x10b000–0x12b000 | actors / physics-object plumbing | track-object refs (§4 consumers), `FUN_0012be48` = `AIEl` consumer [M] |
| 0x12b000–0x145000 | AI elements | `FUN_0013e688`, `FUN_00140110` [M]; class names `AIVehicle`, `AIHelicopter`, `AICharacter` [D] |
| 0x145000–0x159000 | roads/nav consumers (heaviest `DAT_0033aac8` users: 14 functions) | `FUN_00145d00`…`FUN_00153238` [M]; `WRoadNav`, `WGrid` [D] |
| 0x159000–0x165000 | actor/prop updates (contains the 8.8 KB state machine `FUN_001a49e0`'s neighbor cluster) | [S] |
| 0x165000–0x16a000 | world/session/scheduler/visibility (§1.2) | `FUN_00165720/15850` (event queue), `FUN_00165ed0`, `FUN_00166260`, `FUN_00167d80..00168678` [M] |
| 0x16a000–0x17a000 | level/session management, mission setup/teardown | `FUN_0016e140` (attributes), `FUN_00175cf8/176658/176e98` [M] |
| 0x17a000–0x1a0000 | clocks, events, scheduler support | `FUN_001799d8` (clock), `FUN_00179c48` (scheduler) [M] |
| 0x1a0000–0x1b0000 | ray shells + big per-object state machines | `FUN_00199780` (ray shells reset) [M]; `RayShell` [D] |
| 0x1b0000–0x1d0000 | render glue, mission rules | `FUN_001c4008` (game render), `FUN_001c7f48` (rendername attr), `FUN_001bcc70` [M]; `SMissionRule` [D] |
| 0x1d0000–0x200000 | sim reset, cameras, HUD glue | `FUN_001feda0` (sim reset), `FUN_001ff850` (spawn player car) [M]; `RCamera`, `RPlayerCamera`, `RWorldCamera` [D] |
| 0x200000–0x213000 | weapons, roads | `FUN_0020bb00` (weapon manager), `FUN_002051f8`/`FUN_002151b8` [M] |
| 0x213000–0x234000 | track/chunk consumers (§4), AI controllers | `FUN_00212ce8`, `FUN_0021eae8`, `FUN_00220c28`, `FUN_0022c730`, `FUN_0022cd30`, `FUN_0022faf8`, `FUN_00230730`, `FUN_00230ab0` [M] |
| 0x234000–0x249000 | printf/scanf/malloc core | `FUN_002495e8` (progress printf) [M]; matches `__sbprintf`, `_d2b`… (§3) |
| 0x249000–0x259000 | devices: pads, memcards, file progress | `FUN_0024b020`, `FUN_0024b9c8`–`FUN_0024d848` cluster, `FUN_0024e440` [M] |
| 0x259000–0x262000 | pad poll + threads/semaphores/vblank | `FUN_0025a0d0`/`FUN_0025a4a8` (pads), `FUN_00261038` (frame counter), `FUN_002611f0/261920/2619e0/261a40` (thread/sem) [M] |
| 0x262000–0x2a7000 | EAGL render library (GeoPrim/Draw*/TAR/DMA) | strings `EAGL::…` [D]; `FUN_002c8cf0` (context clears) [M] |
| 0x2a7000–0x2c0000 | libc/libm/libgcc (matched to ACTION, §3) | [M] exact matches |
| 0x2c0000–0x2d6000 | render contexts, art/chunk consumers | `FUN_002cd8c0` (`Arti`/`Name`/`Shar`) [M] |
| 0x2d6000–0x2e0000 | file system: RefPack/chunk container, `.viv` open, malloc | `FUN_002d0910` (alloc), `FUN_002d51f0`/`FUN_002d5a10` (chunk find), `FUN_002d5a10` layout §4, `FUN_002d6b90` (fps table), `FUN_002d6ed8` (viv open) [M] |
| 0x2e0000–0x310000 | audio streams (ASF) + SIF RPC + mpeg/IPU decode | `FUN_002e96b8`, `FUN_002ec408`, `FUN_002f9740` [M]; `AStream`, `AVoice`, `ASystem` [D]; libmpeg matches §3 |
| 0x310000–0x326000 | libmpeg/IPU movie decode (matched to ACTION, §3) | `_updateRefImage` etc. [M] |

Sym-hint status: `DRIVING.SYM` could **not** be re-extracted in this sandbox — the rebuilt
on-foot ISO zeroes the DRIVING directory records' extent/size fields (checked by raw ISO9660
parse: /DRIVING records carry LBA 0/size 0 while the root records are intact) [D], which also
explains `nf-extract drivingsym` failing with an empty buffer. The 12,719-name parse quoted in
docs/formats/driving-sym.md remains the hint source, and it describes a different build
(SYM `main` 0x190590 does not exist here) [M-negative]. Anchor-based hints for this build:
`PS2PadDevice::PollDevice` ≈ `FUN_0025a0d0` (called per device per frame, §1.2) [S];
`InputToAction`/`ActionQueue` machinery should live next to the 0x419840 device table [S].

## 3. Functions shared with ACTION.ELF (reuse list)

Method: `tools/crosscheck/elf-shape-match.py` (committed). Two passes — exact byte equality,
then a masked shape signature (j/jal targets, lui his and all I-type immediates dropped;
relative branch encodings kept). Result: **329 matches** = 193 byte-exact + 136 shape-equal
(3,038 ACTION functions vs 9,169 DRIVING functions; full list in the script's output).

Headline: **every game-side function differs** — the matches are libc, libm, libgcc, the
PS2 SDK stubs, VSync, and the whole libmpeg/IPU movie decoder. No BIGF/archive match: the
DRIVING-side container reader (0x2d6000 cluster) is a different build of that code and must be
re-derived, not reused (matches ACTION's unnamed archive code in shape only where noted [S]).

Curated, strongest first (all [M] — exact byte matches verified against both ELFs):

| What | ACTION.ELF | DRIVING.ELF | size | evidence |
|---|---|---|---|---|
| memcpy | 0x236388 (`memcpy`) | 0x2a7e84 | 176 | exact bytes; decompiles identical |
| memmove | 0x236438 | 0x2a7f34 | 260 | exact |
| memset | 0x23653c | 0x2a8038 | 192 | exact |
| memchr | 0x23ab04 | 0x2b06d4 | 224 | exact |
| strcpy | 0x236b38 | 0x2a964c | 276 | exact |
| strncpy | 0x236d88 | 0x2a9a50 | 444 | exact |
| strcat | 0x236738 | 0x2a924c | 300 | shape |
| strcmp | 0x2369f4 | 0x2a9508 | 324 | exact |
| strchr | 0x236864 | 0x2a9378 | 400 | exact |
| strlen | 0x236c4c | 0x2a9760 | 312 | exact |
| floorf | 0x235600 | 0x2bfb50 | 228 | exact |
| fabsf | 0x2355b8 | 0x2bfb30 | 28 | exact |
| tanf | 0x232bc0 | 0x2bfd28 | 136 | shape |
| scalbnf | 0x235710 | 0x2c6880 | 344 | shape |
| isinf | 0x23b788 | 0x2b1970 | 72 | exact |
| soft-float (dp) | 0x224ff0 (`__fpcmp_parts_d`), 0x224840 (`__unpack_d`), 0x225158 (`litodp`), 0x2252a8 (`dptoul`), 0x224b20 (`dpadd`) | 0x24d488, 0x24d8a0, 0x24d5f0, 0x24d740, 0x24cfb8 | 276/152/184/160/88 | exact ×4, shape ×1 |
| soft-float (bi) | 0x23af58 (`_multiply`), 0x23acc0 (`_multadd`), 0x23b288 (`_lshift`), 0x23b600 (`_d2b`), 0x238a40 (`quorem`), 0x23b468 (`__mdiff`), 0x23ae60 (`_lo0bits`), 0x238850 (`exponent`) | 0x2b0c70, 0x2b0890, 0x2b0fa0, 0x2b1528, 0x2ae2f0, 0x2b1180, 0x2b0b78, 0x2ad498 | 556…224 | shape |
| VSync primitives | 0x23c058 (`VSync`), 0x23c0e8 (`VSync2`), 0x23bf90 (`SetVSyncFlag`) | 0x2b71f0, 0x2b7280, 0x2b70d0 | 144/164/16 | shape, exact, exact |
| SIF/RPC | 0x23dc18 (`sceSifWriteBackDCache`), 0x23de70 (`_sceRpcGetPacket`), 0x23df68 (`_request_end`), 0x23e0d0 (`_request_bind`) | 0x2b91b0, 0x2b9428, 0x2b9560, 0x2b9820 | 172/168/180/176 | exact, shape ×3 |
| GS | 0x225430 (`sceGsResetGraph`) | 0x2b6380 | 400 | shape |
| libmpeg/IPU | 0x22a140 (`sceMpegInit`), 0x232768 (`sceIpuRestartDMA`), 0x22f590 (`_updateRefImage`), 0x22d4c0 (`_dualPrimeVector`), 0x22e0e8 (`_decode_motion_vector`), 0x22b160 (`_initRefImages`) + the `_ri0_*/_rix_*` macroblock family | 0x31c3e0, 0x324380, 0x321a90, 0x31f9c0, 0x3205e8, 0x31d660, 0x31eae8…0x31f9c0 | 156…692 | exact (the whole `_ri*` family is byte-identical) |

So for phase 4+ the port can keep its ported libc/libm behavior and its ASF/mpeg decoders as
the reference implementations; **the CARP physics, AI, camera, HUD code shares nothing with
ACTION.ELF at byte level** — it is the same codebase's driving half, compiled separately
(more below in the matches: zero ACTION game functions like `Drone_*`, `SFX*` matched beyond
8–28-byte stub collisions, which the script's ≥32-byte rule filters).

## 4. The `.crp` track consumer (§7 open question 5 of driving.md — answered from code)

**The consumer chain, all [M]:**

1. **Filename** — `FUN_0022f898` 0x22f898 (404 B), called once from the init chain
   (`FUN_00174198`, line 328 of its decompile) with the global `DAT_0033aac8` object and
   `"data\track\"` (0x375b78): it formats `%s.crp` (string 0x39b620, ref at 0x22f8cc) with the
   level key buffer, stores `dir` at obj+4, the full track filename at obj+8, and the key at
   obj+0xc. The track object itself is 0x3c bytes, allocated in the init chain via
   `FUN_002d0910(0x3c)` + ctor `FUN_0022f820`, with the work buffer 0x424010 [M].
2. **Load + decompress** — same function: `FUN_001a9cc8` (request init, 72 B) →
   `FUN_001a9d70(dir, filename, 0, 0)` (the async read; 1,220 B) → handle at obj+0x18,
   **decompressed file buffer pointer at obj+0x14** [M].
3. **Chunk walk** — the `.crp` body is a chunk tree read by two finders:
   `FUN_002d51f0` 0x2d51f0 (top-level; chunk entries are 0x10-byte directory records at the
   buffer base, FourCC in +0, flags|count<<5 in +4, first-child index in +8; flag 0x10 = binary
   search via `FUN_002d4288`) and `FUN_002d5a10` 0x2d5a10 (sub-chunks; flag 0x8 = binary search
   via `FUN_002d4268`; flag 0x2 on a child = its pointer field is buffer-relative) [M].
4. **Who reads what** (all [M]; FourCCs from the call sites):

| Consumer | Chunks read | Meaning [S until parsed] |
|---|---|---|
| `FUN_0022f898` (loader itself) | `DBN ` | the track database header |
| `FUN_0022faf8` 0x22faf8 (2,896 B, "Init World") | `Map ` → `Wmap`, `Rule`, `MSet`, `Audi`, `Trgr`, `Envi`; print `Resolving misc world elements` 0x39b628 | waypoint/road map (Wmap — `FUN_0022c730` init on it, pointer kept at obj+0x1c), mission rules/sets, audio placements, triggers, environment |
| `FUN_00176658` ("Init AI Elements" block) | `Map ` → `AIEl` → `FUN_0012be48` | AI element placement (cars/characters) |
| `FUN_00230730` (896 B) | `Map `, `Wmap`, `AIEl`, `Rule`, `MSet` | the roads/AI join (road graph + AI lanes) |
| `FUN_0021eae8` (988 B) | `Map `, `Shad`, `VisC`, `in  `, `ps  ` | shadows + visibility culling |
| `FUN_00212ce8` (188 B) | `Map `, `iN  ` | instances |
| `FUN_0022cd30` (1,296 B) | `Map `, `Rule` | mission rules |
| `FUN_002cd8c0` (440 B) / `FUN_002dc1b0` (608 B) | `Map `, `Arti`, `Name`, `Shar` / + `CDat`, `RNgp`, `sr  ` | render/geometry side (art instances, names, shared data) |
| `FUN_00230648` (called from teardown `FUN_00176e98` with the track object) | (release path) | track unload |

So driving.md §7 Q5 is answered: **the `.crp` consumer is the track object at `DAT_0033aac8`
(ctor `FUN_0022f820`, loader `FUN_0022f898`, resolver `FUN_0022faf8`), the chunk readers
`FUN_002d51f0`/`FUN_002d5a10`, with the physics-side consumer being the Wmap init `FUN_0022c730`
(pointing at obj+0x1c) and the roads/AI join `FUN_00230730`** [M]. The per-frame reader of the
Wmap data (CARP physics) is phase-4 work; the heaviest `DAT_0033aac8`-referencing clusters are
0x145000–0x159000 (14 functions) and 0x21xxxx–0x23xxxx (§2).

## 5. Phase 4 task description (vehicle physics) — for the driver's next dispatch

**Task `drive-carp-physics` (suggested name), branch `s/drive-carp-physics`** — port the
vehicle physics core, keeping docs/atlas/driving.md §6's phase-4 row as the scope header.

1. **The car object**: from the class names [D], `PVehicle`/`PBondCar` (0x376180/0x376190) and
   `PhysicsObject`/`COLLISION_INFO` (0x376170/0x3761a8) are the structs. Entry points: the
   spawn path `FUN_001ff850` (mission setup, "Spawn player car"), the attribute link
   `SECONDARY_TYPE`/`pvehicle` (0x376e18/0x376e28), and the model dir `data\car\model\`
   (0x376e08). Find the per-frame Update (the vtable+0x1c dispatch of §1.2) for the player car
   object and walk its callees; the `UCarpNamespace` strings (0x376160, 0x39b720) bracket the
   CARP code.
2. **The road world**: parse `Wmap` out of the `.crp` (§4) — chunk finder layout is proven
   [M]; the record formats inside `Wmap`/`RNgp` are the first format work
   (`FUN_00230730` is the join to read). `docs/formats/driving-viv.md` + RefPack decompressor
   (phase 0 of the plan) are prerequisites.
3. **Integration**: `FUN_00110900(1/60)` clock (§1.3), the tuning files
   `data\tuning\Physics\Rigid\default.tun` / `Physical\default.tun` [D] as the numeric source,
   deterministic fixed 60 Hz steps, the shared bit-exact RNG rules.
4. **Tests**: golden numbers only (counts, ids, hashes), each with a sister assertion; the
   `.crp` of a small mission (e.g. snow2a_race) as the fixture source via the existing
   `unviv_bigf.py` path; live checks below for the main agent's PCSX2 confirmations.
5. **Out of scope for phase 4**: actors in cars (`Act*`), traffic AI (`AIVehicle`), camera,
   HUD, audio placement (phases 5–6).

## 6. Live checks for the main agent

- **L-1 — frame counter**: watch EE 0x3c80d4 (word) during a driving mission with gdbwatch:
  expect +1 per vblank (60/s NTSC) [S from the reader FUN_00261038 0x261038; the writer is
  interrupt-side and not in the decompile].
- **L-2 — the track open**: breakpoint `FUN_0022f898` 0x22f898 (exec) and dump a1/a2:
  expect a2 = pointer to `"data\track\"`, and the formatted `%s.crp` result (obj+8) to be
  `data\track\<mission key>.crp`. Also dump 0x424010 (the key buffer) [M expectation].
- **L-3 — the world object list**: after `Leaving StartUp` (0x376e68 print), dump the world at
  0x3c87b0: count at +0xc, object pointers at +0x10.., each object's vtable at +0x18 — the
  vtable list is the per-frame subsystem order (physics/AI/camera/render Update order) that
  §1.2 leaves open. Record each vtable's +0x1c target address [M expectation].
- **L-4 — thread start**: breakpoint `FUN_00261920` 0x261920 (exec) once during mission load:
  expect a1 = 0x167de8 (the thread entry) and a2 = 2; and `FUN_00167e10` called with mode 1
  right after `Leaving StartUp` [M expectation].
- **L-5 — skip flag**: watch world+0x9c (0x3c884c) across frames: expect it nonzero on the
  skipped (30 Hz-style) frames only, and `FUN_00168280` (0x168280) not entered on those frames
  [M expectation].

## 7. What's left / not done

- The `Map `/`Wmap` record formats (the .crp contents) — phase 4.
- The per-subsystem vtable pinning (L-3) — phase 4.
- The SYM name hints stay [S]: the SYM is a different build and is not extractable from this
  ISO (§2 note); the pad/deadzone recipe of driving.md §4 remains the input-path entry.
- The second pass of `FUN_00168280` reads a parallel array at world+0x50 whose meaning
  (event slot per object, read by `FUN_00168678`'s third arg) is [S].
- `FUN_001749e8`'s callee-level purpose split (render context creation vs input registration)
  is only partially decomposed here.
