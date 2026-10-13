# The driving atlas — DRIVING.ELF and the driving missions (research phase)

Status: **first map, research-only** (task `research-driving`). The port covers ACTION.ELF; this
file is the starting map for a later driving phase. Evidence tags: [M] code (function+address),
[D] disc bytes, [L] EE snapshot, [S] suggested. Nothing here is committed from memory.

Build identity of the driving executable (docs/build-identity.md, *measured*): `DRIVING.ELF`
LBA 1153, 2,895,244 bytes, SHA-1 `d7b8a57f3b8e17e980a6d9228b2ad02ed8eebc5c`, entry `0x24c0d8`,
one LOAD segment file `0x80` → vaddr `0x107100` (so **EE address = file offset + 0x107080**),
`$gp` 0x3cfdf0, **stripped**. Decompile: `work/decomp/DRIVING.ELF/` (9,170 functions).

## 1. Which levels (archive → mission)

The level keys are stored in DRIVING.ELF as string tables in four copies (file offsets 0x25c948,
0x25ce30, 0x2602b8, 0x2622f0 → EE 0x3639c8, 0x363eb0, 0x367338, 0x369370) [D]:

| Key | Archive | Driving mission (ACTION's menu strings) |
|---|---|---|
| `paris_mis01` | `MIS01.VIV` | `HT_Level_Driving_Paris_ST` [D, ACTION.ELF 0x22255f] |
| `uw_mis11` | `MIS11.VIV` | `HT_Level_Driving_Underwater_ST` [D, 0x222625] |
| `junglea_mis13a` | `MIS13A.VIV` | `HT_Level_Driving_JungleA_ST` [D, 0x222644] |
| `jungleb_mis13b` | `MIS13B.VIV` | (Jungle B; no `_ST` string found in the grep) |
| `junglec_mis13c` | `MIS13C.VIV` | (Jungle C) |
| `snow1a_mis3` | `MIS3.VIV` | `HT_Level_Driving_Alps_ST` [D, 0x2225b3] |
| `snow2a_mis4` | `MIS4.VIV` | `HT_Level_Driving_SnowMobile_ST` [D, 0x222594] |
| `snow2a_race` | `RACE.VIV` | the race on the snow2a map (same level key prefix) |
| — | `MISC.VIV` | shared data, loaded at boot (below) |

The archive→key mapping above is from the name strings only [D]; the archive files live in
`DRIVING/` on the disc (docs/formats/disc.md), content census in docs/formats/driving-viv.md.
Which menu entry triggers which key is [S] until the level-select code is read.

## 2. The handover (measured end to end)

**ACTION → DRIVING** [M]:

- `GameFlow_Main` 0x1c98c8, state 9: `Boot_GetPTPData(&buf, &size)` then
  `psiLaunchDriving(buf, size)`.
- `Boot_GetPTPData` 0x1c8198 (968 B) fills the **PTP block** at 0x31c090 (symbol `PTPDATA`,
  size **0xa50**): GameState map id, mission/cheat flags (`CheatInfo`, `menu_unlock_everything`),
  MP settings fields, `SFXGetLanguage()`, medal state (`Menu_GetBestMedal`,
  `Menu_HasMedal(0x9000001,3,0)`), and the sound volumes **divided by 2** for the driving engine
  (`DAT_0031c9f4 = SFXGetVolume()/2`, `DAT_0031c9f8 = SFXMusicGetVolume()/2`) [M].
- `psiLaunchDriving(void*, uint)` 0x120af0 (148 B): `PS2SafeCleanup()`, `DisableIntc(2/5)`,
  removes the `VBlank_HID`/`VIF_HID` handlers, `amemcpy(0x100000, data, size)` — the PTP block
  goes to **EE 0x100000** — then `uRam0010097c = 1` (a "came from ACTION" flag inside the block,
  at PTP+0x97c) and `RunNextModule("cdrom0:\\DRIVING.ELF;1")` (string at 0x2f3b00), then parks in
  an infinite loop [M].
- `bootup_bootup` 0x1c7e78 memsets `PTPDATA` (0xa50) at boot [M].

**DRIVING side consumes it** [M]:

- `FUN_00175a28` 0x175a28 reads the **level key string at EE 0x1004f4** (PTP+0x4f4), takes the
  suffix after the last `_` (`snow2a_mis4` → `mis4`), prints `Opening %s.viv` (string 0x376830),
  opens it through `FUN_002d6ed8("driving", suffix, …)` (path strings `driving\%s.spe` 0x26fbd8,
  `Opening %s.viv` 0x26f7b0, `*** %s.viv found, and being used! ***` 0x26f7c8) [M].
- `FUN_00174198` 0x174198 is the **boot/init chain**: a sequence of progress prints
  (`FUN_002495e8`) — including `Opening misc.viv` (0x375a78) / `*** misc.viv found, and being
  used! ***` (0x375ac0) — and calls `FUN_00175a28` (line 318 of its decompile) [M].

**DRIVING → ACTION** [M]: `psiGetDrivingData(char*)` 0x120a88 (36 B) copies **0x1000 bytes from
0x100000** back out — the same transfer window, so DRIVING.ELF leaves its results there
(the DRIVING-side writer is not yet identified — open). `PlrStats_SetDrivingData(int*, uint*)`
0x199b18 walks a 12-entry table rooted at the `HT_Level_Driving_*` string pointers (stride 0x3c;
per entry: id into +0x18 of the table node, value into the pointed record +8) and
`PlrStats_GetDrivingData` 0x199ac8 reads it — 12 driving stat slots carry across [M].

**The boot loader** [D]: `SLUS_205.79` (= `BASE.ELF` byte-identical, build-identity) is a custom
ELF loader: strings `Now parsing and loading %s`, `Now loading segment %d at %08x, size %08x`,
`Fatal Error: ELF Magic number not present in header`, plus `host0:BondPS2_Release.elf`
(= ACTION), `host0:BasePS2_Release.elf`, `host0:TestPS2_Release.elf` and
`cdrom0:\ACTION.ELF;1` / `cdrom0:\DRIVING.ELF;1` / `cdrom0:\BASE.ELF;1` [D, file offsets 0x1c138–0x1c1d8].
Which branch selects DRIVING (a save/flag? the PTP? argv?) is **open** — the loader decompile is
in `work/decomp/SLUS_205.79/`; a live check (below) settles it cheaply.

## 3. The main loop and the subsystems

**Coverage warning:** the DRIVING.ELF decompile has **not** been function-mapped yet. What
follows is the measured skeleton + the symbol inventory; everything deeper is next phase.

- Entry 0x24c0d8 [D]. The SYM file's `main` 0x190590 does **not** exist in our build (no
  decompile file at that address; the SYM is from another build — docs/formats/driving-sym.md),
  so `main` is **not yet located** [M-negative]. Likely candidates: the function that calls the
  init chain `FUN_00174198` — its caller list is the first thing to read next.
- The init chain `FUN_00174198` 0x174198 sequences the subsystem startups with progress strings
  (the strings between 0x375ae8 and 0x375b60 in its decompile name the steps) [M].
- The per-frame loop, the timestep (60 Hz assumed [S] — unverified), and the vehicle physics are
  **unmapped**. The physics tuning inputs exist on disc: `data\tuning\Physics\Rigid\default.tun`
  (298 B) and `Physical\default.tun` (365 B) per mission, identical everywhere, with visible keys
  `GRAVITY=-34`, `SLEEP_VEL=5`, `GROUND_FRICT…` [D]. The track data is `data\track\<level>.crp`
  (2.1–2.9 MB compressed, announced decompressed size up to 6.8 MB [D]), format unread.
- Collision: unknown; the mission .VIVs contain **no** on-foot-style level.bin — the collision
  world must come out of the track `.crp` and/or car `.crp` files [S].

**Symbol inventory** (from `DRIVING/DRIVING.SYM`, 12,719 entries — a different build, hints only;
docs/formats/driving-sym.md) [S for roles, D for the names]:

- The engine namespaces: `EAGL::*` (renderer/loader: `DynamicLoader`, `EAGL_VD_PS2_SINGLE_BUFFERD`),
  `RView*` (camera: `RViewCamera`), `Act*` (the actor/character layer used inside the car:
  `ActActor`, `ActCharacter`, `ActPoser`, `ActIKSolver`, `ActWeapon`, `ActModel*`, `ActManager`),
  `CARP::*` (the car-physics middleware: `CARP::AIHenchInfo`), plus `IOModule`, `InputToAction`,
  `InputDevice`, `PS2PadDevice`, `BankInfo`/`PrivateData::LoadBank` (audio banks).
- `ActCharacter::LoadCharacter`, `ActCharacter::InheritWeaponLightingFromCar`,
  `ActWeapon::SetHenchInfo(CARP::AIHenchInfo*)` — in-car henchmen use the same character/weapon
  machinery as the on-foot actors [S, from names+parameters].
- `IOModule::SendGameMessages(InputDevice*, InputToAction*)` — the input → game-message bridge
  (the on-foot side has the equivalent `psiInput_*` layer) [S].

**Shared code with ACTION.ELF** — not yet measured per function. Method for the next phase: pick
the known ACTION functions (docs/atlas/*.md) and shape-match into our DRIVING decompile
(same constants/structure), using the SYM names as search seeds only. The identical per-mission
files (skeletons `0skel.o`/`1skel.o`, `ALookup.bin`, `driving.dat/.rel`, `common.ssh`,
`hudNTSC.gal`, `attrib.dir` [D]) strongly suggest the same loaders/formats on both sides [S].

## 4. The input path (G-004 groundwork)

- The SYM hint addresses for `InputDevice::DeadZoneChop` 0x180530 / `DeadZoneScale` 0x180560 /
  `GetDeviceScalar` 0x180640 / `IOModule::SendGameMessages` 0x1812c0 / `PS2PadDevice::PollDevice`
  have **no decompile files at those addresses in our build** — confirmed again that the SYM is a
  different build [M-negative].
- The names remain the best search seeds: `PS2PadDevice::PollDevice`,
  `InputDevice::DeadZoneChop(float &, float, float)`, `DeadZoneScale(float &, float, float)`,
  `InputToAction::InputToAction(InputDevice *)` [D names]. Shape recipe for the next pass: tiny
  float functions doing fabs/compare/clamp on a reference, called from the pad-poll path; the
  on-foot counterparts are documented in docs/systems/input.md.
- No `scePadRead` string appears in the DRIVING decompile (the call is likely through a library
  stub or the IOP) — the pad read path is **open** [M-negative].
- The control configs exist on disc: `data\Control\Master.def` (80 B), `FrontEnd.def` (548 B),
  plain `POV 8…` text [D] — these define the button mapping data the engine reads.

## 5. Sound, music, speech

- Per mission: `<level>.MUS` (4–6 `.asf` streams, e.g. MIS01: `007logo.asf`, `all.asf`,
  `bondmove.asf`, `silence.asf`, `snipeintro.asf` [D]) and `<level>EN.SPE` (6–41 `.asf` streams —
  speech). The `.asf` container starts `SC`… [D]. The repo already decodes ASF audio for the
  on-foot side (docs/formats/sound-bank.md; `nf-audio`), reuse expected [S].
- In-game banks: `data\audio\g_common.bnk` + one per-level bank (`g1_paris.bnk` 386,111 B,
  `g11_isap.bnk` 790,416 B, `g13a_jn1.bnk` 479,280 B [D]) + `banks.ini` + `g_common.h` +
  per-level `.h` (the `.h` files look like bank-slot definitions [S]). Whether `.bnk` here is the
  same format the on-foot sound-bank reader handles is **unverified** (on-foot banks are
  `SB_*/.shf/.sfx`-shaped; the driving `.bnk` magic is RefPack or `BNKl` [D] — different).
- The volume handover (`SFXGetVolume()/2` written into the PTP on the ACTION side; string
  `Music Volume : Uber(%d) --> Driving(%f)` in DRIVING.ELF 0x26fc30 [D]) ties the two engines'
  mixers together — measured from both sides.

## 6. Port estimate (phased, [S] — sized from the census + decompile counts)

| Phase | Content | Size estimate |
|---|---|---|
| 0 | RefPack decompressor + `golden` digests for decompressed sizes of the 1,499 entries | small (~1–2 days): ~150 lines + tests |
| 1 | Formats: `.crp` (track + car model container — the core new format), `.gal`, `.ssh`, `.sfn`, `.loc`, `.mpc`, `.tun` (text), `.atr` (INI), `.def` | medium: the formats repo already covers the on-foot analogues; `.crp` is the big unknown |
| 2 | The PTP block (0xa50 struct, fields from `Boot_GetPTPData`) + the transfer window protocol + the boot-loader branch | small: mostly measured already (§2) |
| 3 | DRIVING.ELF map: function inventory (9,170 decompiled), main loop, timestep; shape-match the shared engine functions | medium research |
| 4 | Vehicle physics: `CARP` structs + integration + collision from the track `.crp`; tune against `default.tun` | large: the heart of the port |
| 5 | Actor layer in cars (`Act*`), traffic/pursuer AI (`CARP::AIHenchInfo`), camera | large |
| 6 | HUD (`hudNTSC.gal` + `.sfn` fonts — parsers reusable), audio banks (`.bnk` format work) | medium |
| 7 | Input path (G-004 close-out), integration, live checks | medium |

Reuse: BIGF (done), ASF audio (done), the animation/skeleton loaders (the driving missions ship
the same file shapes [D]), the sprite/font/HUD machinery (expected same shapes [S]).
New: RefPack, `.crp`, `.bnk`-driving, the whole vehicle/physics/AI stack, the PTP protocol.

## 7. Open questions / live checks for the main agent

1. Boot-loader branch: watch `SLUS_205.79`'s decision before the ELF load after a driving mission
   is selected (which string/flag drives `cdrom0:\DRIVING.ELF;1` vs `cdrom0:\ACTION.ELF;1`).
2. The transfer window live: dump EE 0x100000..0x101000 right after `psiLaunchDriving`'s memcpy
   and right before `psiGetDrivingData` (which fields of the 0xa50 PTP block does DRIVING
   actually read? the level key at +0x4f4 is proven; the rest is [S]).
3. DRIVING → ACTION result writer: breakpoint the stores into 0x100000..0x101000 near the end of
   a driving mission (what does DRIVING leave for `psiGetDrivingData`?).
4. Confirm the per-frame timestep in PCSX2 (frames vs vblank count over a fixed in-game interval).
5. `data\track\<level>.crp` load path: breakpoint `FUN_002d6ed8`/the BIGF read of
   `driving\mis4.viv` to confirm the track entry's consumer (function + address) for the next phase.
