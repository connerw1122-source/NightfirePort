# Eurocom engine lineage and existing Nightfire / Eurocom RE work

Research date: 2026-10-08. Target: *007: Nightfire* PS2 NTSC-U (SLUS-20579), Eurocom "Action" engine (ACTION.ELF).
Each claim is tagged *sourced:* (URL) or *unverified:*. "Measured on our disc" marks facts from our own dump.

## Summary

- **Nightfire's console action engine is not EngineX.** It predates it. The community wiki calls it an in-house engine that came before EngineX. EngineX's first shipped game was Buffy: Chaos Bleeds (2003). Nightfire's DRIVING missions run on a separate EA engine (EAGL/VIV).
- **The best prior art for our exact target is two community projects.** Both target the same disc, but neither has a licence:
  - **Raikaru/nightfire-engine**: a C++ reimplementation for SLUS-20579. Its `docs/formats.md` (~105 KB) documents FILES.BIN, the level .bin archive, map chunks, textures, PS2_GFX meshes, collision, skeletons, skins, sequences, cutscene scripts, SFX/SBF/SHF, SMF/SSD, STREAMS, SBI/MXI and the multiplayer tables, with ACTION.ELF addresses.
  - **NightfireResearch/nightfire-re** (Python): extraction scripts for PS2, GameCube and Xbox.
  - With no licence on either, we may use them for facts only. Do not copy code or text.
- **FILES.BIN has no header.** Its index is a 118-entry × 32-byte `FileList` table inside ACTION.ELF: `char name[16]; u32 offset; u32 size; u32 pad[2]`. On the US ELF the table is at file offset 0x13E5A0 (EU: 0x13F2A0). Both repos agree on this.
- **The hashcode high byte gives the asset type.** 01 asset pack, 02 mesh, 03 texture, 04 sequence, 05 skin, 06 script, 07 level/FMV, 09 driving level, 10 menu page, 40 menu. Level *nn* is `0700nnnn.bin`.
- **GameCube data is EDL-compressed and big-endian.** It uses the "EDL" magic (LZ plus Huffman-style coding). We don't know of any PS2 FILES.BIN compression.
- **Symbols:** PS2 ACTION.ELF has the best symbols (we confirmed this on our disc). Xbox shipped none, and the community transposed names from the PS2 build. GameCube has debug printfs but no symbol map that we could confirm. A Nightfire Ghidra server ("007Nightfire") is hosted on decomp.dev.
- **Sibling-game tooling exists, all for later games:** `eurotools` (eurochef, MIT, Rust; EuroSound re-implementation, GPL-3.0; hashcode lists) and vgmstream (ISC, Eurocom MUSX). It covers EngineX (Buffy 2003 onward), so its formats are relatives of ours, not identical.
- **No matching decompilation exists for any Eurocom game.** We could not confirm one.

---

## 1. Eurocom engine names and lineage

### 1.1 Names

| Name | What it is | Source |
|---|---|---|
| **EngineX** | Eurocom's in-house runtime engine, first shipped in Buffy: Chaos Bleeds (2003) and reused until the 2012 closure | *sourced:* https://sphinxandthecursedmummy.fandom.com/wiki/EngineX (search snippet only; the page returns HTTP 402 to automated fetch) |
| **EngineXT** | Later evolution of EngineX | *sourced:* https://github.com/eurotools/eurochef ("Eurocom EngineX(T) files"), https://github.com/eurotools/hashcodes |
| **EuroLand / Euroland** (and "EuroLand Redux") | Eurocom's all-in-one editor. It works on `.ELF` source-asset files (MFC-serialised, not executables) and exports `.EDB` (EngineX DataBase). It predates EngineX. | *sourced (search snippets):* https://sphinxandthecursedmummy.fandom.com/wiki/EuroLand, https://sphinxandthecursedmummy.fandom.com/wiki/ELF. eurochef ships an "EDB to Euroland 4 decompiler": https://github.com/eurotools/eurochef |
| **EuroSound** | Eurocom's internal sound-bank authoring tool. Versions 3.57 (Buffy/Sphinx, SFX v201), 3.60 (Spyro AHT, SFX v4) and 3.70 (Robots/Predator, SFX v5). | *sourced:* https://github.com/eurotools/eurosound-editor |

### 1.2 Nightfire's position

- The console action engine is an "in-house engine, predecessor to EngineX". Console driving is EAGL/VIV (possibly EA Redwood), and the PC version is GoldSrc-derived. *sourced:* https://wiki.nightfire.dev/ (engine table on the main page)
- The Valve Developer wiki says the same ("Eurocom's in-house engine (a precursor to EngineX)"). *sourced (search snippet; page returned 403):* https://developer.valvesoftware.com/wiki/James_Bond_007:_Nightfire
- Fan analysis of EuroLand's internal strings suggests EuroLand exported assets for third-party engines in Tarzan, 40 Winks, Atlantis, Nightfire and Harry Potter and the Chamber of Secrets. The same wiki says EuroSound was also used for Nightfire. *sourced (search snippets):* https://sphinxandthecursedmummy.fandom.com/wiki/EuroLand, https://sphinxandthecursedmummy.fandom.com/wiki/EuroSound
  - Treat this as likely, not confirmed. It fits our disc: SB_n.SFX/SHF/SBF are EuroSound-style banks (measured on our disc; layouts in §2.2).
- The Nightfire wiki notes that the Sphinx authoring tools may help with audio formats and scripting, but that there are "substantial differences". *sourced:* https://wiki.nightfire.dev/index.php/ACTION_Section_(Eurocom), https://wiki.nightfire.dev/index.php/Platforms
- Note: names like `C:/DOCUME~1/MAP1~1.EUR` in our ACTION.ELF (measured on our disc) are consistent with a Eurocom toolchain. We found no public statement of an internal engine name for the 2002 action engine. *unverified:* any name for it.

### 1.3 Which games share EngineX (with EDB version where known)

eurochef's support table gives these EDB versions. *sourced:* https://github.com/eurotools/eurochef

| Game | Year | EngineX? | EDB ver | Source |
|---|---|---|---|---|
| Buffy the Vampire Slayer: Chaos Bleeds | 2003 | Yes (first release) | 170 | eurochef; Sphinx wiki snippet |
| Sphinx and the Shadow of Set (demo disc) | 2003 | Yes | 156 | eurochef |
| Sphinx and the Cursed Mummy | 2003 | Yes | 182 | eurochef; https://en.wikipedia.org/wiki/Sphinx_and_the_Cursed_Mummy (infobox, per search) |
| Spyro: A Hero's Tail | 2004 | Yes | 240 | eurochef; https://en.wikipedia.org/wiki/Spyro:_A_Hero%27s_Tail (infobox, per search) |
| Robots | 2005 | Yes | 248 | eurochef |
| Predator: Concrete Jungle | 2005 | Yes | 250 | eurochef |
| Batman Begins | 2005 | Yes | 251 | eurochef |
| Ice Age 2: The Meltdown | 2006 | Yes | 252 | eurochef |
| Pirates of the Caribbean: At World's End | 2007 | Yes | 252 | eurochef |
| G-Force | 2009 | Yes | 259 | eurochef |
| Ice Age: Dawn of the Dinosaurs | 2009 | Yes | 260 | eurochef |
| Spider-Man 4 (cancelled) | — | Yes | 263 | eurochef |
| GoldenEye 007 (2010) | 2010 | Yes | 263 | eurochef |
| 007 Legends | 2012 | Last in-house EngineX game | — | Sphinx wiki snippet (https://sphinxandthecursedmummy.fandom.com/wiki/EngineX) |
| Athens 2004, Beijing 2008, Quantum of Solace ("robin"), The Mummy 3 ("bcourt") | — | Hashcode folders exist, so likely EngineX/XT | — | https://github.com/eurotools/hashcodes (README, folder list). *unverified* as an engine claim. |
| **007 Nightfire (console)** | 2002 | **No: predecessor engine** | — | https://wiki.nightfire.dev/ |
| **Harry Potter and the Chamber of Secrets** (Xbox, GC, GBA by Eurocom; PS2 by EA UK) | 2002 | Same era as Nightfire. Engine unknown. | — | Developer split: https://en.wikipedia.org/wiki/Harry_Potter_and_the_Chamber_of_Secrets_(video_game). *unverified:* shared code with Nightfire. A "potter" hashcode folder exists, but the game it covers isn't identified. |

- Eurocom's 2002–2006 titles (Nightfire, HP CoS, Buffy, Sphinx, Spyro AHT, Athens 2004, Robots, Predator, Batman Begins, Ice Age 2) are listed at *sourced:* https://en.wikipedia.org/wiki/Eurocom
  - That article does not name any engine.
- Implication for us: the closest public relatives of our formats are EngineX-era (2003+). Pre-EngineX 2002 titles (HP CoS GC/Xbox) are the likeliest code siblings, but none of them has public RE that we could find.

---

## 2. Existing RE, format documentation and tools

### 2.1 Nightfire console: projects

**A. Raikaru/nightfire-engine** (C++20, CMake, SDL3, OpenGL 3.3)
- Repo: https://github.com/Raikaru/nightfire-engine. Default branch `ci/windows`; created 2026-10-05; licence **null** (API: https://api.github.com/repos/Raikaru/nightfire-engine).
- It is an open reimplementation for exactly SLUS-20579. ACTION.ELF is the specification, and behaviour is checked against PCSX2. *sourced:* repo README.
- Tools:
  - `nfdump`: lists FILES.BIN; `validate` decodes all 2,230 map chunks, textures, meshes, sounds and menus.
  - `nfview`, `nfgame`, `nfdrive`, `nightfire`, `nfui`, `nfplay`.
  - `tools/oracle` (PCSX2 tracer) and `tools/xref/xref.py`, which puts PS2 pseudocode next to matched GameCube/Xbox decompilations.
  - It references a separate `../nightfire-ps2` IDA/Ghidra database project that is not public.
  - *sourced:* README.
- Format reference: https://github.com/Raikaru/nightfire-engine/blob/ci/windows/docs/formats.md. Key facts, paraphrased (facts only; the repo has no licence):
  - **FILES.BIN:** no header. Indexed by `FileList` in ACTION.ELF: 118 × 32 B `{char name[16]; u32 offset; u32 size; u32 pad[2]}`. Lookup is through `psiFileOpen` (strupr + strcmp).
    - Contents: `*Txt.dat`/`*TxtU.dat` string tables, `TuningVars.txt`, and level bins `0700xxxx`, `07F0xxxx` (animation), `0780xxxx`, `0790xxxx` and `07A0xxxx`.
    - Three bins are empty placeholders.
  - **Level .bin archive:** at 0x000, `u32 dir_size, u32 count`, padded to 0x800. At 0x800, packed entries `{u32 length; u8 type; char name[] NUL}`. Payloads follow at `0x800 + dir_size` in directory order. The entry hash is the leading hex digits of the name.
    - Types: 0/2/11 shared map chunk, 1 level map, 3/4/5 animation, 6 skeleton, 7 script, 8 menu, 12 loadable list, 15 icon file, 16 "woman" setup.
  - **Map chunk:** `u32 version`. Version 1 has a u32 offset table (block k at `4 + table[k]`). Each block header is `u32 (id<<24)|size`, and the stream ends at id 0x1D.
    - Block ids: 04/1F model, 0E map header, 0F/12 palette, 10 texture header, 15/16 pixels, 19 path, 1A static instances, 21 portals, 22/2E collision, 26 lights, 28 sound, 2B morph, 2D PS2_GFX, 30 particles.
  - **Textures:** 12-byte header `{u8 flags (bit0 = 256-colour CLUT); u8; u16 w-1; u16 h-1; u8 frames; u8 fps; u32 -1}`. RGBA8 CLUT with alpha 0x80 = opaque, in CSM1 swizzle order. Pixels are linear 4bpp (low nibble first) or 8bpp. Animation ticks at 60 Hz.
  - **PS2_GFX:** a BVH of 0x38-byte boxes. Leaf chains are VIF DMA lists. "ERSP" stubs carry no geometry.
  - **Collision:** BVH of 0x30-byte boxes plus 8-byte triangles.
  - **Static instance:** `0x4C + n*8` bytes.
  - **Skeleton (type 6):** the human rig has 73 bones.
  - **Skin (type 3):** 158 skins referencing 115 meshes.
  - **Sequence (type 4):** quintic polynomial keyframes. The channel count is `3*bones + 3*popcount(mask)`.
  - **Cutscene script (type 7):** u16 magic 28. All 468 entries parse. Opcodes include EntityStart, AnimStart, CameraStart, and others.
  - **Menu (type 8):** `08000002` is the front end (52 pages). `08000001` is the in-game menu (9 pages).
  - **Audio:**
    - VAG ADPCM uses 16-byte frames of 28 samples.
    - `.SBF` holds raw ADPCM. `.SHF` is `u32 count` + 36-byte sample headers (pitch 0x1000 = 48 kHz).
    - `.SFX` is `u32 count` + `{u32 id; u32 offset}` + 68-byte `SFXParameters` + 28-byte `SFXSamplePoolFiles` (a negative ref selects a STREAMS.BIN entry, `-ref-1`).
    - `SBINFO.SBI` and `MFXINFO.MXI` are 150 × u32 hash tables.
    - `.SSD` is stereo ADPCM at 32 kHz in 256-byte blocks. `.SMF` is a section/marker table.
    - `STREAMS.LUT` has 16-byte entries (823 usable in the USA build). Stream data is mono 22,050 Hz.
  - **ACTION.ELF data addresses:** `MissionData` 0x2a4350, `MPSettings` 0x2a47a0, `default_bot_stats` 0x26d2f0, `SleeveEnts` 0x2C6ED8, a VU1 microprogram at ELF offset 0x241C90, and others.
  - **DRIVING:** `MISxx.VIV`, `MISC.VIV` and `RACE.VIV` are BIGF archives with RefPack-compressed entries. `MISxx.MUS` and `MISxxEN.SPE` are BIGF archives of SCHl streams.
  - *sourced:* formats.md (link above).
- Status: level rendering, skinned characters, audio and player movement work; Skyrail input replay stays within about 1 cm per frame. AI, weapons gameplay and env-mapping are missing. *sourced:* README.

**B. NightfireResearch/nightfire-re** (Python)
- Repo: https://github.com/NightfireResearch/nightfire-re. Licence **null**; created 2023-08; last push 2026-01-25 (https://api.github.com/repos/NightfireResearch/nightfire-re).
- Goals: understand structure, extract and decompress, convert to modern formats, and possibly a later remake or decompilation. It focuses on the Action engine; Driving formats are "poorly understood". *sourced:* README.
- PS2 path, *sourced:* https://raw.githubusercontent.com/NightfireResearch/nightfire-re/main/platform_ps2/ps2_eurocom_handler.py
  - It reads the FILES.BIN table from ACTION.ELF at a fixed offset and unpacks `<16sIIxxxxxxxx`.
  - It names outputs by hashcode and runs `extract_bin`, `parse_map` and (EU only) `extract_audio`.
- Constants, *sourced:* https://raw.githubusercontent.com/NightfireResearch/nightfire-re/main/common/external_knowledge.py
  - `us_filetable_offset_in_elf = 0x13E5A0`, `eu = 0x13F2A0`, `filetable_length = 118`. The address-to-file mapping is `text_start 0x00107100`, ELF header 4096.
  - 38 hashcode→level-name pairs, e.g. `07000001 HT_Level_HendersonA`, `07000005 HT_Level_CastleExterior`, `07000023 HT_Level_Atlantis`, `07000090 HT_Level_MovieMap`, `07000500 LevelLoadingScreens`.
  - SFX tables: SndLbl 484×8, SFXData 1549×0x18, NumberToHashCode 1394×8, HashCodeToString 1394×16.
  - Also present: placement types (e.g. `0x0f Place_Drone`) and bone counts.
- GameCube path, *sourced:* https://raw.githubusercontent.com/NightfireResearch/nightfire-re/main/platform_gc/gc_eurocom_handler.py
  - Files beginning with "EDL" are EDL-decompressed. `common.bin` is skipped.
  - Archive layout code: `gc_archives_extract.py`.
- EDL format, *sourced:* https://raw.githubusercontent.com/NightfireResearch/nightfire-re/main/common/compression/edl.py
  - Header: magic "EDL", byte 3 = type (low nibble) plus a big-endian flag (high bit), u32 compressed size, u32 decompressed size; payload at +12.
  - Type 0 is stored. Type 1 is LZ77 with prefix-coded literal/length and distance tables (min match 3).
  - The org also forks a C# port of Zoinkity's EdlCompress: https://github.com/NightfireResearch/EDLCompress (no licence shown). *unverified:* the claim that EDL also covers Eurocom's N64-era games.
- Xbox: partial only, because `filesys.dxx` is not cracked. *sourced:* nightfire-re README.

**C. NightfireResearch/nightfire-cxbx** (C++)
- Repo: https://github.com/NightfireResearch/nightfire-cxbx. Licence null; active as of 2026-10-08 (https://api.github.com/repos/NightfireResearch/nightfire-cxbx).
- A Cxbx-Reloaded launcher plus DLL injection that patches the Xbox build. It has a Ghidra `NightfireSync` script. It notes that the Driving engine is C++ while lower-level code is C. *sourced:* README.

**D. Nightfire Wiki** (MediaWiki, CC BY 4.0)
- Site: https://wiki.nightfire.dev/ (nightfire.dev redirects there).
- Hashcode high-byte table, *sourced:* https://wiki.nightfire.dev/index.php/ACTION_Section_(Eurocom)
  - 01 asset pack (`010000a7` Ninja), 02 GraphicLists/mesh (`02000359` minigun), 03 textures/icons, 04 sequences, 05 skins (`05000078` Astronaut, 71 bones), 06 scripts (animation), 07 level/FMV, 09 driving level, 0c particles (unconfirmed), 10 menu page, 40 menu.
  - Exceptions: translations, `SKEL0000..SKEL0044`, and the SFX tables.
- Same page, loader dispatch on `DirFileType`, *sourced:*
  - 0x01 AnimPostLoadInit/parsemap; 0x00/0x02 parsemap; 0x03–05 AnimLoadFile; 0x06 AnimSkeletonProcess; 0x07 Script_Load; 0x08 MenuManager_Load.
  - 0x0b MemType=1; 0x0c LoadableIndex; 0x0f SetUpIconFile; 0x10 PS2SetUpWoman.
  - Skeletons must load before skins.
- "Nearly all game and menu logic is in the ELF"; levels carry only simple triggers. *sourced:* same page.
- Linked tools: Ghidra with ghidra-emotionengine-reloaded (https://github.com/chaoticgd/ghidra-emotionengine-reloaded) and Ghidra-GameCube-Loader, plus ImHex, QuickBMS and HxD. *sourced:* main page.
- Community: GitHub https://github.nightfire.dev and Discord https://discord.nightfire.dev. *sourced:* main page.

**E. Forum threads (older)**
- XeNTaX, "[PS2] 007 Nightfire .crp files": https://forum.xentax.com/viewtopic.php?t=22213&start=45 (now 404).
  - Search snippets said levels are compiled .bin files with a custom layout, unlike later Eurocom games. *unverified* beyond the snippet.
- ZenHAX, April 2020: http://zenhax.com/viewtopic.php@t=13420.html
  - Notes on DRIVING formats: .crp begins "PRACZ", .sfn begins "FNTS"/"EAGL240", and DRIVING has a .SYM file. FILES.BIN was unidentified at the time. *sourced.*
- NGEmu, "007 Nightfire Filesystem": https://www.ngemu.com/threads/007-nightfire-filesystem.96835/ (an old unanswered question, per search snippet).

**F. Talk:** Charlie Bruce, "Reverse engineering 007: Nightfire", EMF 2026. It covers Ghidra, transposing symbols across platforms, LTO pitfalls and emulator code injection. *sourced:* https://app.media.ccc.de/v/emf2026-277-1-reverse-engineering-007-nightfire/audio

### 2.2 Sibling Eurocom games: tools and formats (EngineX era)

All from the `eurotools` org (*sourced:* https://github.com/eurotools; licences as shown there or checked through the API):

- **eurochef** (Rust): https://github.com/eurotools/eurochef. **MIT** (LICENSE: "Copyright (c) 2023 cohaereo", https://raw.githubusercontent.com/eurotools/eurochef/main/LICENSE).
  - Crates: `eurochef-edb`, `eurochef-elx`, `eurochef-filelist`, `eurochef`.
  - It reads filelists (v4 read-only; v5–7 read/write; v9–10 read-only) and EDBs (textures, maps, entities, spreadsheets).
  - PS2 support is "secondary": broken tristrips and no transparency flags.
  - Docs: `docs/texture_formats.md`, `docs/vertex_layouts.md`.
  - It is the most reusable code for us (MIT, Rust), but it covers EngineX, not the 2002 engine.
- **eurosound-editor** (C#): https://github.com/eurotools/eurosound-editor. **GPL-3.0** (API). A re-creation of EuroSound 3.57/3.60/3.70.
- **eurosound-explorer** (C#): inspects and extracts Eurocom SFX. **GPL-3.0**.
- **binary-templates**: 010 Editor templates. **GPL-3.0**: https://github.com/eurotools/binary-templates
  - Includes `EX_Filelist_v4-7.bt`, `gc_game_dmp.bt` (GameCube `game.dmp` symbol dump), `spyro_hashcodes_enum.h` and Sphinx ELF/IMA/save templates.
- **hashcodes**: https://github.com/eurotools/hashcodes. **No licence** (API null). Name lists for EngineX/XT games collected from public pre-release, debug and demo builds. There is no Nightfire folder.
- **eurotext** (no licence), **es-eurocom-adpcm-encoder-decoder** (no licence shown), **es-ps2-vag-tool** (GPL-3.0), **es-dsp-adpcm-tool** (GPL-3.0), **es-xbox-adpcm-tool** (GPL-3.0), **es-ngca-dsp-adpcm-tool** (GPL-3.0), **es-ima-adpcm-encoder-decoder** (MIT), **blender-addon** (.ESE import/export, **Zlib**).
- **vgmstream** (ISC: https://raw.githubusercontent.com/vgmstream/vgmstream/master/COPYING) supports "Eurocom MUSX" (`.sfx .musx`; PSX/IMA/DSP/PCM) and "Eurocom ESF". *sourced:* https://raw.githubusercontent.com/vgmstream/vgmstream/master/doc/FORMATS.md
  - It has **no** entries for `.sbf/.shf/.smf/.ssd`, so Nightfire's audio formats are not in vgmstream.
- **Sphinx and the Cursed Mummy Authoring Tools** (free DLC, 19 Feb 2018). It includes EuroLand.exe/EuroLandRedux.exe, all source `.ELF` assets, beta maps and scripting docs. It contains **no engine source**.
  - *sourced:* https://www.moddb.com/news/sphinx-and-the-cursed-mummy-mod-tools-released, https://www.dsogaming.com/news/modding-tools-for-sphinx-and-the-cursed-mummy-are-now-available/, https://www.gog.com/game/sphinx_and_the_cursed_mummy_authoring_tools
  - Licence: a proprietary EULA (*unverified* wording). Facts only.
- **Sphinx PC remaster** (THQ Nordic, Nov 2017): a rewritten OpenGL 3+ renderer. Wikipedia credits Swyter with the PC version. The remaster got a large update in February 2026.
  - *sourced:* https://thqnordic.com/news/sphinx-and-the-cursed-mummy-coming-to-pc-mac-linux, https://www.gamingonlinux.com/2026/02/sphinx-and-the-cursed-mummy-remaster-gets-a-big-upgrade/
  - Its source is **not public**. *unverified:* we found no evidence of a release.
- No Noesis plugin, QuickBMS script or Blender importer for **Nightfire's** formats turned up in our searches. *unverified:* absence.

---

## 3. Decompilation and symbol work

- **Nightfire Ghidra server:** project "007Nightfire" on decomp.dev. It is best annotated on the PS2 build (Ghidra 12.1.2 with Emotion Engine Reloaded). Xbox "did not come with symbols"; names were transposed by hand from PS2. *sourced:* https://wiki.nightfire.dev/index.php/Decompilation
- **Per-platform symbols:**
  - PS2 Action ELF has function names and some globals. PS2 Driving has `SYMBOLS.SYM` with shifted offsets.
  - Xbox has the least inlining of maths code.
  - GameCube has printf/debug exceptions and less inlining. GC driving code resembles Agent Under Fire, and the AUF GameCube release shipped with symbols for both its Uber and Driving engines.
  - *sourced:* https://wiki.nightfire.dev/index.php/Platforms
- **Raikaru/nightfire-engine** cross-references "matched GameCube and Xbox decompilations" via `tools/xref`. The source of those decompilations is not named; probably private Ghidra/IDA exports. *sourced:* README. Their availability is *unverified*.
- **No matching (byte-exact) decompilation** of any Eurocom game was found. *unverified:* absence; searched GitHub and general web.
  - The closest is **HPCOS-GC-RECOMP**, a static recompilation (not a decomp) of Harry Potter and the Chamber of Secrets for GameCube (GHSE69; Eurocom developed the GC version). **GPL-3.0**: https://github.com/ant0-blase/HPCOS-GC-RECOMP
- **Compiler for ACTION.ELF:** GCC-mangled names (measured on our disc) point to EE-GCC (SN Systems ProDG or Sony GCC). *unverified:* which toolchain and version. Worth checking `.comment`/`.mdebug` sections on our ELF.

---

## 4. Community knowledge: debug features, prototypes, symbol files

- **Debug and dev leftovers (Action):**
  - Character-test and misc-test rooms (possibly PS2-only).
  - `C_CHCHFLY_Handler` / `C_CHCHDRAWALL_Handler` and noclip flags.
  - Cheats for immortality and all weapons.
  - Possibly a build number in the pause menu.
  - Driving has audio-mixer debug overlays.
  - *sourced:* https://wiki.nightfire.dev/index.php/Debug_/_Dev_Leftovers (stub page; items marked uncertain there)
- **Test levels:** hashcode names include test and menu rooms (`07000090 HT_Level_MovieMap`, others). *sourced:* external_knowledge.py (above).
- **`menu_unlock_everything`** sits at 0x30d2a7 in ACTION.ELF memory. *sourced:* Raikaru formats.md (above).
- **Retail cheats:** "PASSPORT" at Secret Unlocks gives level select. *sourced:* https://www.cheatcc.com/articles/james-bond-007-nightfire-cheats-codes-cheat-codes-walkthrough-guide-faq-unlockables-for-playstation-2-ps2-psx2/
  - NTSC-U CodeBreaker codes: https://almarsguides.com/retro/walkthroughs/ps2/games/007Nightfire/Codebreaker/
- **Prototype:** Xbox PAL build dated **18 Oct 2002**, two days before final (default.xbe V1.00 vs US V1.01 / PAL V1.02). *sourced:* https://hiddenpalace.org/007:_Nightfire_(Oct_18,_2002_prototype)
  - It does not state whether it has symbols. We found no PS2 or GameCube Nightfire prototypes on Hidden Palace (*unverified:* absence).
- **GameCube or Xbox map/pdb:** we found no evidence that Nightfire GC shipped a `.map` or that Xbox shipped a `.pdb`; the wiki says Xbox has no symbols. *sourced (Xbox):* https://wiki.nightfire.dev/index.php/Decompilation. GC is *unverified*.
  - A TASVideos thread notes that the GC disc holds "hundreds of .elf files". Those are likely Eurocom asset files, not executables. *sourced (snippet):* https://tasvideos.org/Forum/Posts/434493
- **Eurocom precedent for shipped symbols:**
  - Spyro AHT GameCube reportedly ships `game.dmp`, a EuroPack-compressed symbol map. Indirect support: `gc_game_dmp.bt` in https://github.com/eurotools/binary-templates. The direct claim is from a search snippet of TCRF and is *unverified*.
  - The Sphinx wiki says bundled debug symbol maps are used during crashes: https://sphinxandthecursedmummy.fandom.com/wiki/Disc_releases (snippet only).
- **TCRF:** no console Nightfire page was found (only https://tcrf.net/James_Bond_007:_NightFire_(Windows), the PC version, per search).
  - tcrf.net served automated fetches an "AI agent" notice page instead of article content, so we could not verify any TCRF claim. Everything attributed to TCRF here is *unverified*.
- The fbpe.org pages ("Unused Lines (On-Foot) | NightFire (Consoles)", GCN unused multiplayer) look auto-generated. *unverified:* do not rely on them.

---

## 5. Emulator notes (brief)

- **PCSX2 GameIndex, SLUS-20579:** `compat: 5`; `vuClampMode: 2` ("Fixes polygon clipping in driving missions"); `halfPixelOffset: 2` ("Fixes fog lines in the distance"). *sourced:* https://raw.githubusercontent.com/PCSX2/pcsx2/master/bin/resources/GameIndex.yaml
  - The VU clamp note hints that the driving-engine VU code relies on PS2 float clamping behaviour. That matters for our VU1 path port.
- **Dolphin** (GO7E69 etc.): rated "Playable". VP6 FMVs glitch unless Manual Texture Sampling is on; native 16:9 is in the game's options; the widescreen hack is not recommended. Nothing on symbols. *sourced:* https://wiki.dolphin-emu.org/index.php?title=007:_Nightfire
  - Note: GC uses **VP6** FMVs, while our PS2 disc uses PSS (measured on our disc).

---

## Licences

| Project | URL | Licence | What we may reuse |
|---|---|---|---|
| Raikaru/nightfire-engine | https://github.com/Raikaru/nightfire-engine | none (API: null) | Facts only. Use its format facts as cross-checks; don't copy code or prose. |
| NightfireResearch/nightfire-re | https://github.com/NightfireResearch/nightfire-re | none (API: null) | Facts only (offsets, table sizes, EDL algorithm description) |
| NightfireResearch/nightfire-cxbx | https://github.com/NightfireResearch/nightfire-cxbx | none (API: null) | Facts only |
| NightfireResearch/EDLCompress | https://github.com/NightfireResearch/EDLCompress | none shown | Facts only |
| Nightfire Wiki | https://wiki.nightfire.dev/ | CC BY 4.0 (footer) | Text reuse with attribution; facts freely |
| eurotools/eurochef | https://github.com/eurotools/eurochef | **MIT** (LICENSE file, (c) 2023 cohaereo) | Permissive: reuse with notice (EngineX formats only) |
| eurotools/eurosound-editor | https://github.com/eurotools/eurosound-editor | GPL-3.0 | Format reference only |
| eurotools/eurosound-explorer | https://github.com/eurotools/eurosound-explorer | GPL-3.0 (org page) | Format reference only |
| eurotools/binary-templates | https://github.com/eurotools/binary-templates | GPL-3.0 | Format reference only |
| eurotools/hashcodes | https://github.com/eurotools/hashcodes | none (API: null) | Facts only |
| eurotools/eurotext | https://github.com/eurotools/eurotext | none (API: null) | Facts only |
| eurotools/es-ima-adpcm-encoder-decoder | https://github.com/eurotools/es-ima-adpcm-encoder-decoder | MIT (org page) | Permissive: reuse with notice |
| eurotools/es-ps2-vag-tool | https://github.com/eurotools/es-ps2-vag-tool | GPL-3.0 (org page) | Format reference only (VAG is a public, well-known format anyway) |
| eurotools/blender-addon | https://github.com/eurotools/blender-addon | Zlib (org page) | Permissive: reuse with notice |
| vgmstream | https://github.com/vgmstream/vgmstream | ISC | Permissive: reuse with notice (PS-ADPCM decoder etc.) |
| HPCOS-GC-RECOMP | https://github.com/ant0-blase/HPCOS-GC-RECOMP | GPL-3.0 | Reference only |
| ghidra-emotionengine-reloaded | https://github.com/chaoticgd/ghidra-emotionengine-reloaded | not checked | Tool use only (not linked into our code) |
| Sphinx Authoring Tools (THQ Nordic) | https://www.gog.com/game/sphinx_and_the_cursed_mummy_authoring_tools | proprietary EULA (*unverified* terms) | Facts only |
| PCSX2 GameIndex.yaml | https://github.com/PCSX2/pcsx2 | GPL-3.0 (project; *unverified* for this file) | Facts only (settings hints) |

---

## Open questions

1. Does Raikaru's `../nightfire-ps2` IDA/Ghidra database, or the "matched GameCube and Xbox decompilations", exist publicly? Contact the author or the Nightfire Discord.
2. Can we get read access to the decomp.dev "007Nightfire" Ghidra server, and would its symbol and type annotations come with usable terms?
3. Does Nightfire's SFX/SHF layout match an early EuroSound version (pre-3.57)? Compare with the SFX v201 structures in eurosound-editor, as a reference only since it's GPL.
4. Does any Nightfire build ship a GameCube `.map`/`game.dmp`-style symbol file or an Xbox `.pdb`? List the GC disc files to confirm.
5. Is EDL compression used anywhere in the PS2 data? So far it's only seen on GameCube.
6. What is `DEBUG.TXT`'s relation to `HashCodeToString` / `SFXOutputData` (1394 vs 1548/1549 entries)?
7. Do Harry Potter and the Chamber of Secrets GC/Xbox (Eurocom, 2002) share the Action engine and file formats? If so, are there HP CoS symbols or prototypes that would help?
8. Which compiler and version built ACTION.ELF? Check `.comment` and `.mdebug`.
9. Exact FILES.BIN table offsets: the US table at file offset 0x13E5A0 is a third-party claim. Verify it on our ELF; with `text_start` = 0x107100, the memory address is 0x2446A0 (0x13E5A0 − 0x1000 + 0x107100).
10. Is the TCRF content on Nightfire console debug features real? It needs a manual browser check, since automated fetch was blocked.
