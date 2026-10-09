# 007: Nightfire: PC version, emulator patches, mouse injectors, timing and controls

Research date: 2026-10-08. Target: PS2 NTSC-U **SLUS-20579**. Every claim is tagged *sourced:* (URL) or *unverified:*. Web content was treated as data only. Nothing was downloaded or run.

## Summary

- **The PC version is useless to the console port.** Gearbox built it on a heavily modified GoldSrc engine. It has a different storyline and missions and no driving. The console versions are Eurocom's own engine (Action) plus an EA driving engine. No console code is shared. Community PC projects (the 5.x unofficial patch, Nightfire Open on Xash3D) target the PC game only. *sourced:* https://en.wikipedia.org/wiki/James_Bond_007:_Nightfire, https://wiki.nightfire.dev/, https://github.com/noodlecollie/nightfire-open
- **The most valuable find is not a patch. It is the NightfireResearch project** (https://github.com/NightfireResearch, wiki https://wiki.nightfire.dev/). It is an active console reverse-engineering effort. It reports that the **PS2 Action ELF has function names and globals**, that the PS2 Driving ELF has a `.SYM` file with wrong offsets, and that it runs a shared Ghidra server on decomp.dev. It publishes per-version hashes, including SLUS-20579. The wiki is CC BY 4.0. The GitHub repos have **no licence**.
- **The console game is two executables:** an Action engine (Eurocom; on-foot, menus, multiplayer) and a Driving engine (EA; `DRIVING.ELF` on PS2). They hand state across through a "PTP" struct at `0x0031CF90`, which is copied to `0x100000`. *sourced:* https://wiki.nightfire.dev/index.php/Action_%E2%86%94_Driving
- **PCSX2 CRC for SLUS-20579 is `5B86BB62`.** The same CRC is reported for PAL SLES-51258/51260. *sourced:* PCSX2 wiki and forum (search snippets: https://wiki.pcsx2.net/007:_Nightfire, https://forums.pcsx2.net/Thread-James-Bond-007-Nightfire-SLUS-20579-U--28637). Both returned 403 to direct fetch. Your own PCSX2 log should confirm it.
- **There are no official PCSX2 widescreen, 60 fps or no-interlace patches for Nightfire.** I checked these: the `PCSX2/pcsx2_patches` tree (no SLUS-20579, SLES-51258 or SLES-51260 file), the KrystianLesniak patch list, and the PeterDelta patch repo. The game has a **native 16:9 option** in its AV Options. *sourced:* https://psxdatacenter.com/psx2/games2/SLUS-20579.html and the PS2 manual (see §5).
- **One real PS2 frame-rate patch exists, for DRIVING.ELF.** Writing `nop` at `0x001799B8` gives 60 fps in driving. The issue also documents a time-scaling bug (an update runs when `(now_tick - last_tick) * scalar >= 1.0`, and the remainder is thrown away). DRIVING.ELF is byte-identical between SLUS-20579 and SLES-51258. *sourced:* https://github.com/NightfireResearch/nightfire-cxbx/issues/47, https://wiki.nightfire.dev/index.php/Known_Versions
- **Mouse look design reference:** MouseInjector's `nf.c` targets the **GameCube NTSC-U (GO7E69)** build, not PS2. It writes yaw and pitch floats relative to a player pointer. Yaw is in radians, wrapped to ±π. Pitch is clamped to ±1. Both are scaled by 1/FOV. It also writes crosshair sway (±0.296 / ±0.4) and a "lookspring" flag, and has a separate sentry (heli/jet-ski turret) mode. It is GPL-2.0-or-later, so we use it for facts only. No injector supports **PS2** Nightfire. The garungorp fork covers PS2 *Agent Under Fire* (SLUS-20265), a different engine.
- **Frame rate:** secondary sources say GameCube targets 60 fps and PS2 and Xbox run at 30 fps. No measurements were found. Whether logic ticks per frame is not documented for the Action engine.
- **Controls (PS2 manual):** a controller configuration is picked per codename. Advanced Options has Auto Aim on/off (default depends on difficulty), Crosshairs, Manual Aim toggle/hold, Crouch toggle/hold and Weapon Auto Switch. The manual mentions **no look-sensitivity or invert option**. *sourced:* https://archive.org/details/ps2_007-_NightFire_USA

---

## 1. The PC version (Gearbox) and whether it helps a console port

### Facts
- Eurocom developed the home console versions (PS2, GameCube, Xbox). Gearbox Software developed Windows, Aspyr did the Mac OS X port and JV Games did the GBA version. *sourced:* https://en.wikipedia.org/wiki/James_Bond_007:_Nightfire
- The PC version runs on Valve's GoldSrc engine, "heavily modified for Nightfire". *sourced:* https://en.wikipedia.org/wiki/James_Bond_007:_Nightfire, https://en.wikipedia.org/wiki/GoldSrc
- Several sources call the modified engine "Gearcraft". *sourced (search-result summary of Valve Developer Community; direct fetch 403):* https://developer.valvesoftware.com/wiki/James_Bond_007:_Nightfire
- A 2002 GameSpot preview quoted a producer: so little Half-Life code remained that it was practically a new engine, with a new renderer. *sourced (search-result summary only; direct fetch 403):* https://gamespot.com/articles/james-bond-007-nightfire-preview/1100-2874776
- The PC version changes the storyline and missions and removes the driving sections. Some weapons use different models. Multiplayer supports online play and up to 12 bots. *sourced:* https://en.wikipedia.org/wiki/James_Bond_007:_Nightfire
- The NightfireResearch engine table lists these engines:
  - Console Action: Eurocom in-house, a predecessor of EngineX. PS2 symbols are available.
  - Console Driving: EA (Redwood, unconfirmed), EAGL/VIV, possibly NFS-like.
  - PC: Gearbox's GoldSrc-derived engine.

  *sourced:* https://wiki.nightfire.dev/
- **Shared code or assets between PC and console:** I found no source claiming any console code is shared. The engines are entirely different families. *unverified:* the PC levels may be loosely inspired by console mission locations, but the assets are rebuilt as GoldSrc BSP/MDL. No source confirmed asset sharing.

### Community PC projects (not usable for the console port)
- **Unofficial patch v5.9x** by click4dylan and team. It adds netcode fixes, crash fixes, FOV for widescreen and dynamic lights, and is PC-only. *sourced (search-result summary; direct fetch 403):* https://community.pcgamingwiki.com/files/file/559-james-bond-007-nightfire-unofficial-patch/
- **Nightfire Open** (formerly "Afterburner"). It is an open-source remake as a Half-Life mod on a hard fork of Xash3D FWGS. Its maps, models, textures and audio are "ported from the original game", meaning the PC release, and it needs files from an existing PC release. GitHub reports the licence as "Other" (NOASSERTION; there is a `copyright.txt`). *sourced:* https://github.com/noodlecollie/nightfire-open, https://api.github.com/repos/noodlecollie/nightfire-open/license, https://github.com/x6herbius/afterburner-game

### Verdict
The PC version and its community projects use a different engine (GoldSrc lineage vs Eurocom/EA), different level data and a different mission set, and they have no driving engine. **Nothing from them is reusable for a native port of the PS2 executable.** At most they are a gameplay-feel reference for mouse look in a Nightfire-themed FPS. *sourced:* the URLs above.

---

## 2. PCSX2 patches for Nightfire PS2

### Identification
| Item | Value | Source |
|---|---|---|
| Serial | SLUS-20579 (NTSC-U), released 18 Nov 2002, developer Eurocom, publisher EA | https://psxdatacenter.com/psx2/games2/SLUS-20579.html |
| Related serials | PAL SLES-51258, SLES-51260; NTSC-J SLPS-25203, SLPM-65538, SLPM-66615; Korea SLKA-25004 | https://psxdatacenter.com/psx2/games2/SLUS-20579.html |
| Video features | 60 Hz yes, progressive scan **no**, **16:9 option yes** | https://psxdatacenter.com/psx2/games2/SLUS-20579.html |
| PCSX2 game CRC | **5B86BB62** (also listed for SLES-51258/51260). A forum log line "Game CRC = 0x5B86BB62" is reported. | search snippets of https://wiki.pcsx2.net/007:_Nightfire and https://forums.pcsx2.net/Thread-James-Bond-Nightfire-not-loading (both 403 on direct fetch) |
| Disc / ELF hashes (SLUS-20579) | ISO SHA1 `5dcb9e55f08eb3376c64e1d9cf772409365976dc`; Action ELF SHA256 `a8b1ad0ce92ecac3cbb05590ff8a07d6f3ff8afd287a5011f235ada8db66e316`; Driving ELF SHA256 `94b830a78c6bbd313d98b35dd55735f8e6fda417bb7be77888821dfcbf6c0065` (identical to SLES-51258) | https://wiki.nightfire.dev/index.php/Known_Versions |

*unverified (inference):* the PCSX2 CRC hashes the boot ELF named in SYSTEM.CNF. A CRC shared by NTSC-U and PAL, while the Action ELFs differ in SHA256, suggests the boot ELF is a small common loader that chains to the Action/Driving ELFs. Check this against our disc.

### Patch availability (checked 2026-10-08)
- **PCSX2/pcsx2_patches**: I read the full recursive tree through the GitHub API. It has **no file for SLUS-20579**. The list goes from `SLUS-20578_C818BEC2.pnach` straight to `SLUS-20582_B9EA7505.pnach`. It also has none for SLES-51258/51260 (`SLES-51257` jumps to `SLES-51282`). Files are named `patches/<SERIAL>_<CRC>.pnach`. *sourced:* https://api.github.com/repos/PCSX2/pcsx2_patches/git/trees/main?recursive=1
- **KrystianLesniak/pcsx2_patches_list** (an index of the official patches): the 007 entries are Everything or Nothing, From Russia with Love and Quantum of Solace. Nightfire is absent. *sourced:* https://github.com/KrystianLesniak/pcsx2_patches_list/blob/main/markdown_patches.md
- **PeterDelta/PCSX2** (a large 60 fps/widescreen pnach collection): no file for 5B86BB62, SLUS-20579, SLES-51258/51260 or SLPS-25203. *sourced:* https://api.github.com/repos/PeterDelta/PCSX2/git/trees/main?recursive=1
- **ThirteenAG WidescreenFixesPack**: no Nightfire PS2 entry found. *sourced (search):* https://github.com/ThirteenAG/WidescreenFixesPack/releases/tag/scdaps2
- **gamehacking.org** has a Nightfire page (https://gamehacking.org/game/104017), but it returned 403, so I could not inspect its codes. The Almar's Guides CodeBreaker codes (NTSC-U) are **encrypted** CB codes for health, ammo and noclip. They contain no FOV or frame-rate codes. *sourced:* https://almarsguides.com/retro/walkthroughs/ps2/games/007Nightfire/Codebreaker/
- **Widescreen** is native. In AV Options it is "Widescreen on/off", plus "Screen Adjust". *sourced:* https://archive.org/details/ps2_007-_NightFire_USA (OCR text)
- **No-interlacing:** no patch found anywhere. *unverified:* whether the game renders field- or frame-based.
- **Mouse-related patches:** none found for PS2 Nightfire.

### The one known PS2 frame-rate / timing patch (DRIVING.ELF)
From NightfireResearch issue #47 ("Driving scheduler / time scaling", Jan 2026). *sourced:* https://github.com/NightfireResearch/nightfire-cxbx/issues/47

| Address (DRIVING.ELF, PS2) | Change | Claimed meaning |
|---|---|---|
| `0x001799B8` | `nop` | "gives 60fps during gameplay" in driving. The author says it fixes a bug affecting all versions. |
| `0x001799D8` | tail-call hook | site that sets the tick count to 0. Used to initialise a new float accumulator. |
| `0x00179E74` | replaced with a call to `0x35E040`, plus `sw s7, 0x14(s3)` | tick-decision site. The added store updates last-update-tick every time. |
| `0x0035E030`–`0x0035E078` | code cave (an unimportant string) | `0x35E03C` holds the float accumulator. Init code is at `0x35E030`, the update function at `0x35E040`. |
| `0x00307500` | `nop` | the instruction that writes 'S' into that string |
| `0x0028AF98` | many `nop`s | possible extra code cave |

Mechanism (sourced, same URL): there is a speed scalar, normally 1.0. An update runs when `(now_tick - last_update_tick) * scalar >= 1.0`, and the fractional leftover is discarded. With scalar 0.8 (Paris Prelude slow-motion), the game effectively runs at about 50% instead of 80%. Implication for us: the **Driving engine has a tick-based scheduler that can be decoupled from the frame rate**. A port should carry the remainder in an accumulator.

DRIVING.ELF SHA256 is identical for SLUS-20579 and SLES-51258, so these addresses should apply to our disc. *sourced:* https://wiki.nightfire.dev/index.php/Known_Versions

---

## 3. Mouse injection projects

### MouseInjector (Carnivorous) and forks
- Original author: "Copyright (C) 2019-2020 Carnivorous". Licence: **GNU GPL v2 or (at your option) any later version** (header of `games/nf.c`). *sourced:* https://raw.githubusercontent.com/dj0wns/MouseInjectorDolphin/master/games/nf.c
- `dj0wns/MouseInjectorDolphin` is a fork of `CrashOveride95/MouseInjectorDolphin`, and GitHub reports GPL-2.0. Its README lists 007: NightFire mouse support as **"Poor"**, with vehicle mode semi-functional and the last level broken. *sourced:* https://github.com/dj0wns/MouseInjectorDolphin, https://api.github.com/repos/dj0wns/MouseInjectorDolphin
- The `carnivoroussociety/MouseInjectorDolphin` URL cited by users returned 404 on 2026-10-08. *sourced:* https://steamcommunity.com/groups/7SMOKE/discussions/0/1738882605423155032 (cites the URL); 404 observed directly.
- `garungorp/MouseInjectorDolphinDuck` (GPL-2.0) supports Dolphin, DuckStation, PCSX2 Nightly, RetroArch, PPSSPP and Project64. Nightfire is supported **only on Dolphin (NTSC)** through the same `nf.c`, with a "Poor" rating. There is **no PS2 Nightfire driver**. The PS2 Bond title it supports is *Agent Under Fire*, SLUS-20265, as `ps2_007auf.c` ("Good"). *sourced:* https://github.com/garungorp/MouseInjectorDolphinDuck, https://api.github.com/repos/garungorp/MouseInjectorDolphinDuck/contents/games, https://api.github.com/repos/garungorp/MouseInjectorDolphinDuck/license
- **KAMI** (dio-gh) is an MIT-licensed mouse injector for PCSX2 (over IPC/PINE) and RPCS3. Nightfire is **not** in its compatibility list. *sourced:* https://github.com/dio-gh/KAMI, https://isjuhn.github.io/index.html

### `nf.c`: what it writes (GameCube NTSC-U, game ID GO7E69). Facts only; GPL code is not to be copied.
*sourced for this whole subsection:* https://raw.githubusercontent.com/dj0wns/MouseInjectorDolphin/master/games/nf.c

- Detection: GC header words `0x80000000 == 'GO7E'` and `0x80000004 == '69\0\0'`.
- Static GC addresses:
  - `0x802BE87C`: player pointer
  - `0x803780DC`: sentry (heli/jet-ski turret) pointer
  - `0x802E441C`: sentry FOV
  - `0x8024C2B4`: pause flag
- Offsets from the player struct (all float unless noted):

  | Offset | Meaning (inferred by injector author) |
  |---|---|
  | `+0x40` | camera yaw (radians, wrapped to [-π, π)) |
  | `+0x904` | camera pitch (clamped [-1, 1], unit unclear, possibly radians) |
  | `+0x92C` | FOV (degrees; must be ≥ 1) |
  | `+0x1AC` / `+0x1B0` | crosshair / gun-sway X/Y (clamped ±0.296 / ±0.4) |
  | `+0x8F0` | health (float; writes only when > 0) |
  | `+0x1BC` | "lookspring" int. The injector changes `0x03010002` to `0x01010002` to disable it. |

- Sentry offsets: `+0x1B4` is yaw (wraps in [-1, 1], so normalised) and `+0x1B0` is pitch (clamped [-0.1617, 0.1350]). Sentry scaling uses a base FOV of 41.25.
- Scaling: `look = sens/40`.
  - Yaw: `-= (dx/10) * look / (360/2π) / fov`
  - Pitch: `± (dy/10) * look / 90 / fov`
  - Crosshair: `(dx/80) * (crosshair%/100 * look) / fov`

  Mouse deltas are therefore divided by the current FOV, which keeps zoomed aiming consistent.
- Guard conditions: it writes only when the values are in range, FOV ≥ 1, health > 0 and the game is not paused. It ticks at about 1000 Hz.
- Design notes for our port:
  - Nightfire's camera state is separate yaw and pitch floats plus a separate crosshair/gun-sway offset, and the injector moves both.
  - There is a "lookspring" (auto-centre pitch) flag that mouse look must disable.
  - Vehicle and turret sections use a separate camera model.

  *unverified:* the PS2 Action engine is expected to have the same struct layout, since it is the same Eurocom codebase, but the offsets must be re-derived from the PS2 ELF symbols.

### `ps2_007auf.c` (Agent Under Fire PS2, SLUS-20265): related but a different engine
- Detection reads `0x93390` and expects the string "SLUS_202.65;".
- On-foot addresses: camera Y `0x5064D0`, camera X `0x5064D4` and FOV `0x385330`, scaled by `fov/60`. Car and tank turret cameras are stored as sin/cos pairs.
- An aim-lock flag at `[0x11D9A0]+0x7B8` is set to 0 to disable aim lock.

*sourced:* https://raw.githubusercontent.com/garungorp/MouseInjectorDolphinDuck/master/games/ps2_007auf.c. The AUF Driving engine is shared with Nightfire, so the turret and car-gun handling may be relevant. *sourced:* https://wiki.nightfire.dev/index.php/Platforms

---

## 4. Console gameplay timing and version differences
- **Frame rate:** a retrospective says GameCube targets 60 fps and PS2 and Xbox run at 30 fps. No measured data was found. *sourced:* https://scanlines.substack.com/p/the-greatest-bond-game-ever, https://www.classic-games.net/gamecube/james-bond-007-nightfire/. The classic-games review also loosely says "a perfect 60 fps" with bots, which is a conflicting claim.
- A speedrun guide says Xbox and PS2 have more lag than GameCube, and that Xbox has more effects (real-time shadows) and higher resolution. *sourced (search summary; direct fetch 403):* https://www.speedrun.com/007nf/guides/tlylh
- The PS2 version dips in complex interiors and in driving sections. *sourced:* https://www.gameswelt.de/james-bond-007-nightfire/test/james-bond-007-nightfire-3851/2 (via search summary). A contemporary PS2 review found single-player smooth, with slight multiplayer slowdown. *sourced:* https://www.impulsegamer.com/igversion01/ps2/ps2jamesbond007nightfire.htm
- Multiplayer bots: 4 on PS2, 6 on GameCube and Xbox. *sourced:* https://en.wikipedia.org/wiki/James_Bond_007:_Nightfire
- **Logic tick vs frame:**
  - Driving engine: scalar-scaled tick scheduler (see §2). *sourced:* https://github.com/NightfireResearch/nightfire-cxbx/issues/47
  - Xbox Driving uses `timeSetEvent` timing. *sourced:* https://wiki.nightfire.dev/index.php/Original_Game_Bugs
  - Action engine: *unverified*. No public source says whether it is per-frame or delta-time. Determine it from PS2 symbols.
- Platform RE value: PS2 Action has the best symbols. Xbox does not inline maths. GameCube has debug printfs, and its Driving engine resembles AUF's (AUF GC shipped with symbols). *sourced:* https://wiki.nightfire.dev/index.php/Platforms
- Engine handoff on PS2: PTP state at `0x0031CF90` is copied to `0x100000`. The Action-side functions are `Boot_LoadPTPData`, `Boot_GetPTPData` and `psiGetDrivingData`. *sourced:* https://wiki.nightfire.dev/index.php/Action_%E2%86%94_Driving
- GameCube has native 16:9. The VP6 FMVs need Manual Texture Sampling in Dolphin. GameCube IDs are GO7E69, GO7P69, GO7D69, GO7F69 and GO7S69. *sourced:* https://wiki.dolphin-emu.org/index.php?title=007:_Nightfire

---

## 5. Console control schemes (PS2 manual, NTSC-U)
Source: archived PS2 manual scan with OCR. Button glyphs are partly garbled in the OCR. *sourced:* https://archive.org/details/ps2_007-_NightFire_USA (text: https://archive.org/stream/ps2_007-_NightFire_USA/007-_NightFire_USA_djvu.txt)
- **Default on-foot:**
  - Left stick: move/turn.
  - Right stick: strafe left/right (*unverified:* the OCR is ambiguous about what the right stick's vertical axis does; possibly look up/down).
  - R1: fire / use gadget.
  - D-pad: cycle weapons (left/right) and gadgets (up/down). D-pad detail *sourced:* https://www.impulsegamer.com/igversion01/ps2/ps2jamesbond007nightfire.htm
  - START: pause.
  - Manual Aim, Crouch, Jump and Reload/Action are on face buttons or L buttons; the glyphs are unreadable.
- **Controller configuration:** chosen in the Controller Setup screen when a codename is created. It is saved per codename and can be changed from the Pause menu. The manual has a "DualShock 2 controller configurations" section. Preset names were not recoverable from the OCR.
- **Advanced Options:**
  - Vibration
  - **Auto Aim on/off** (default level varies with difficulty)
  - Crosshairs on/off
  - Crouch toggle/hold
  - **Manual Aim toggle/hold**
  - Weapon Auto Switch
  - Flashing Objects
  - HUD Always On
- **Multiplayer Options:** a separate Auto Aim setting, Radar and Health Handicap.
- **AV Options:** **Widescreen on/off**, Screen Adjust, split-screen direction, subtitles and speaker mode.
- The manual has no mention of **look sensitivity** or **invert look**. *unverified:* they may exist in-game but undocumented.
- Two aiming modes: auto aim (assists only when you are already close to the target) and manual aim. *sourced:* https://www.impulsegamer.com/igversion01/ps2/ps2jamesbond007nightfire.htm
- Driving: left stick steers, L2 looks back, L1 uses a gadget, R1 fires, Circle is the handbrake and Triangle changes the camera view. *sourced:* manual OCR (above).
- The GameCube manual is also archived: https://archive.org/details/007-nightfire-usa

---

## Licences

| Project | URL | Licence | What we may reuse |
|---|---|---|---|
| PCSX2 | https://github.com/PCSX2/pcsx2 | GPL-3.0 | Facts/format reference only |
| pcsx2_patches | https://github.com/PCSX2/pcsx2_patches | None declared (API `license: null`; README says authors are on the forums) | Facts only (and no Nightfire file exists) |
| PeterDelta/PCSX2 patches | https://github.com/PeterDelta/PCSX2 | Not checked; no Nightfire content | n/a |
| MouseInjector (Carnivorous) via dj0wns fork | https://github.com/dj0wns/MouseInjectorDolphin | GPL-2.0-or-later | Facts/format reference only (addresses, formulas as facts) |
| MouseInjectorDolphinDuck (garungorp) | https://github.com/garungorp/MouseInjectorDolphinDuck | GPL-2.0 | Facts/format reference only |
| KAMI | https://github.com/dio-gh/KAMI | MIT | Permissive: reuse with notice (no Nightfire support, though) |
| NightfireResearch nightfire-re | https://github.com/NightfireResearch/nightfire-re | None (API `license: null`) | Facts only |
| NightfireResearch nightfire-cxbx | https://github.com/NightfireResearch/nightfire-cxbx | None (API `license: null`) | Facts only (issue #47 addresses as facts) |
| wiki.nightfire.dev | https://wiki.nightfire.dev/ | CC BY 4.0 (wiki footer) | Text reusable with attribution; facts freely |
| Nightfire Open | https://github.com/noodlecollie/nightfire-open | "Other" / NOASSERTION (`copyright.txt`) | Facts only; irrelevant to the console port |
| PS2 manual scan (EA) | https://archive.org/details/ps2_007-_NightFire_USA | EA copyright | Facts only |

---

## Open questions
1. Confirm the PCSX2 CRC `5B86BB62` and the boot ELF name for our SLUS-20579 disc (SYSTEM.CNF). Is the boot ELF a shared loader, given that the CRC matches PAL?
2. Get the exact ELF file names on the PS2 disc (Action ELF, `DRIVING.ELF`, `.SYM` files). The nightfire.dev "Files" subpages (ACTION Section, DRIVING Section) did not show them via fetch.
3. Does the PS2 Action engine run game logic per frame (fixed 30 or 60) or on delta-time? Find the main loop and VSync wait via the PS2 symbols.
4. Is PS2 Action really capped at 30 fps (secondary sources only), or 60 with drops? Measure in PCSX2.
5. PS2 player-struct offsets for yaw, pitch, FOV, crosshair sway and lookspring. Map them from the GC `nf.c` layout using the PS2 symbols.
6. What do the in-game Controller Setup presets contain, and is there any hidden look-sensitivity or invert option? The manual OCR shows neither. Check in the emulator.
7. What do the gamehacking.org (403) and PCSX2 forum/wiki (403) pages contain? They might hold raw codes worth recording.
8. Check whether the PCSX2 `GameIndex.yaml` entry for SLUS-20579 carries any gsHWFixes or gameFixes (not checked; the file is too large to fetch).
9. Does NightfireResearch (Discord / decomp.dev Ghidra server) allow reuse of its symbol work? There is no licence on its repos.
