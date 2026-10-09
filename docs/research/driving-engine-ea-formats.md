# Nightfire PS2 driving engine and EA-era formats (web research)

Research date: 2026-10-08. Target: 007: Nightfire PS2 NTSC-U (SLUS-20579), `DRIVING/` folder.
Every claim is tagged *sourced:* (URL) or *unverified:*. All web content was treated as data. One page (a tcrf.net URL guessed for the console version) returned text addressed to AI agents asking for actions; it was ignored. No such TCRF console page with real content was found.

## Summary

- **Who made the driving levels.** Wikipedia credits Nightfire's driving levels to Savage Entertainment plus a team at Electronic Arts. MI6-HQ says EA Vancouver and EA Redwood Shores. EA's press release says "the team behind Need For Speed". Savage's own page says Savage made two of the driving levels. For Agent Under Fire (AUF), EA Canada built the driving levels, and Wikipedia says AUF used EAGL alongside id Tech 3. Nothing I found shows the driving code descends from NFS: Hot Pursuit 2 (HP2). HP2 PS2 was made by EA Black Box, and the other versions by EA Seattle.
- **CARP/.crp links the code to Porsche Unleashed / EA Canada.** In Nightfire the `.crp` files are "CARP" object files (magic stored byte-reversed as `PRAC`). The NFS5 (Porsche Unleashed) modding scene also uses a `.crp` model format. One fan-wiki claim (EoN using a modified Porsche Unleashed engine) fits with this, but none of it is proven.
- **Most valuable find: a public reimplementation of this same game already exists.** `Raikaru/nightfire-engine` (C++20, SLUS-20579) documents in detail the BIGF, RefPack, CARP, embedded-ELF `.o`, SHPS/`.ssh`, `.atr`/`.tun`/`.def` and DRIVING.ELF physics and texture paths. **It has no licence file, so use it for facts only.**
- **DRIVING.ELF.** That project's docs treat DRIVING.ELF as a shipped disc file (vaddr = file offset + 0x107080), loaded by `BASE.ELF`. They say DRIVING.SYM names come **from an older build and are unreliable**. This contradicts our measurement that DRIVING.ELF is missing from the ISO tree, so it needs checking.
- **BIGF header.** unvivtool's parser reads the BIGF filesize as **big-endian** but notes BIG4 stores it little-endian. It also auto-accepts a byte-swapped size that matches the real file, and treats a wrong header filesize as "value unused". The Raikaru doc says Nightfire's BIGF size is **little-endian**, with count and header size big-endian. The MISC.VIV mismatch is probably harmless. Never trust the size field.
- **SSH.** EA "shape" container, magic `SHPS` on PS2, little-endian. Formats are documented on RE Wiki (CC BY-SA) and in EA Graphics Manager (GPL-3.0). The Nightfire specifics (no GS swizzle on pixels; PSMT8 CLUT bit-3/4 swap) come from the Raikaru doc.
- **Audio.** `.asf` uses EA SCHl. On PS2 (platform 5), vgmstream (ISC licence) defaults to VAG/PS-ADPCM, and EA-XA is also possible. FFmpeg's EA demuxer (LGPL-2.1+) handles SCHl, but PS2 VAG only via a fallback path.
- **MPC.** EA PS2 video: EA chunked container whose `MPCh` frames hold an MPEG-2 elementary stream. FFmpeg demuxes it.
- **Rust-relevant code.** The `refpack` crate (MPL-2.0) and `libbig` (MIT) are usable with notice. vgmstream (ISC) is usable with notice. unvivtool, EA Graphics Manager, ReverseBox and QuickBMS are GPL: use them as format references only.
- **Other RE projects.** `NightfireResearch/nightfire-re` (no licence shown; says the driving formats are "poorly understood") and an EMF 2026 talk "Reverse Engineering 007: Nightfire" (Charlie Bruce) also exist.

---

## 1. Who developed the driving missions, and from which codebase

| Claim | Status |
|---|---|
| Eurocom developed the console versions of Nightfire. | *sourced:* https://en.wikipedia.org/wiki/James_Bond_007:_Nightfire |
| "Driving levels were developed by Savage Entertainment and a team at Electronic Arts"; they were left out of the PC version. | *sourced:* https://en.wikipedia.org/wiki/James_Bond_007:_Nightfire |
| Savage Entertainment "created two driving levels" for Nightfire; Savage also did the AUF Xbox port; it was based in Los Angeles. | *sourced:* https://en.wikipedia.org/wiki/Savage_Entertainment |
| EA Vancouver and EA Redwood Shores studios developed the vehicle-based missions. | *sourced:* https://www.mi6-hq.com/sections/articles/gaming_nightfire_making1?id=01423 |
| EA's 2002 press release credits the console driving levels to "the award winning team behind the Need For Speed racing games". | *sourced (via search snippet of EA/Scoop press release):* https://m.scoop.co.nz/stories/CU0211/S00214.htm |
| AUF (PS2, Nov 2001) was developed by EA Redwood Shores, with "additional work by EA Canada". EA Canada built the driving levels, having worked on NFS. AUF runs on id Tech 3 together with EA's "Electronic Arts Graphics Library" (EAGL). | *sourced:* https://en.wikipedia.org/wiki/James_Bond_007:_Agent_Under_Fire |
| EAGL was created by EA Canada. NFS HP2 and NFS Underground use "EAGL 1". A Russian source claims instead that the "fourth version" was built for HP2/PS2. The version numbering is disputed. | *sourced (search-result summaries only; pages refused direct fetch):* https://ru.ruwiki.ru/wiki/EA_Graphics_Library , https://en.gamegpu.com/test-gpu/retro-test-gpu/need-for-speed-most-wanted-2005-retro-test-gpu |
| NFS HP2 PS2 was developed by EA Black Box (Vancouver). The GC/Xbox/PC versions were by EA Seattle. | *sourced:* https://en.wikipedia.org/wiki/Need_for_Speed:_Hot_Pursuit_2 |
| The Nightfire driving code descends directly from NFS HP2. | *unverified:* no source found. The `.crp` CARP format and `EAGL_*` symbols point to EA Canada's EAGL-era tech generally, not specifically to HP2. |
| `.crp` is a car-model format in NFS: Porsche Unleashed (NFS5, EA Canada); modders used a Z-Modeler ".crp filter" by "Arushan". | *sourced:* https://www.nfsaddons.com/downloads/nfspu/cars/koenig/1330/koenig-c62-race-version.html |
| Everything or Nothing (EA Canada driving) used "a modified version of the engine used for Porsche Unleashed". | *sourced (fan wiki, via search snippet; direct fetch returned HTTP 402):* https://nfs.fandom.com/wiki/EA_Canada |
| The Raikaru reimplementation describes DRIVING.ELF as an "EA Redwood Shores engine" with the EAGL renderer, a "Simulation" physics module and CARP object files. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving.md |
| Our SYM path `d:\ToBurn\BondPS2\PS2_EE_Release\...` suggests a shared "BondPS2" EA codebase carried over from AUF. | *unverified:* inference from our own disc measurement; no web source mentions "BondPS2". |

**Working hypothesis (unverified).** The driving runtime is EA's internal Bond driving engine. EA Canada (NFS team) built it for AUF on EAGL plus the Porsche-Unleashed-lineage CARP object format. EA Redwood Shores, an EA Canada team, and Savage reused it for Nightfire. It is a cousin of HP2 (shared EAGL and BIGF/SSH/ASF tooling), not a fork of it.

## 2. Formats and tools

### 2.1 BIGF / VIV archives

| Claim | Status |
|---|---|
| NFS3-era spec: 16-byte header `BIGF`, file size ("size of entire file"), directory count, then a 4th field that "appears to always point to the first object". Entries: 4-byte offset, 4-byte length, NUL-terminated name. The spec does not state endianness. | *sourced:* https://raw.githubusercontent.com/bfut/unvivtool/main/references/unofficial_nfs3_file_specs_10.txt |
| unvivtool's `libnfsviv.h`: header = format, `h_filesize`, `num_direntries`, `header_size` (header length including the directory). For BIGF/BIGH, the filesize is byte-swapped (big-endian). For BIG4 it is little-endian ("BIG4 encodes filesize in little endian"). Count and header_size are always big-endian. | *sourced:* https://raw.githubusercontent.com/bfut/unvivtool/main/libnfsviv.h |
| unvivtool treats an incorrect header filesize as "value unused" and bounds-checks against the real file size. If the byte-swapped filesize equals the real size, it accepts it and flips its endian flag. A header_size outside the file gives a warning, not an error. | *sourced:* https://raw.githubusercontent.com/bfut/unvivtool/main/libnfsviv.h |
| OpenSage (C&C BIG/BIG4): size is little-endian; NumEntries and OffsetFirst are big-endian; entries are BE offset, BE size, C-string. | *sourced:* https://opensage.readthedocs.io/file-formats/big/ |
| Nightfire `DRIVING/*.VIV`: `BIGF`, u32 file size at +4 **little-endian**, u32 count and u32 header size **big-endian**. Entries are BE offset, BE size, NUL-terminated backslash path (`data\track\...`). Members are usually RefPack-compressed. MISC.VIV holds `eaglrm.o`, `bondrm.o` (relocatable ELF with VU1 microcode), HUD, localisation and civilian vehicles. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving.md |
| A late Xbox Nightfire prototype ships both `misc.viv` and `misc ntsc.viv` in the driving folder. They are swapped to make driving missions launch on NTSC. | *sourced:* https://hiddenpalace.org/007:_Nightfire_(Oct_18,_2002_prototype) |
| Our MISC.VIV size mismatch reflects a byte-swapped size or a region-variant rebuild. | *unverified.* Recommendation: ignore the field; derive bounds from the real file length and the directory, as unvivtool does. |

Tools: unvivtool (GPL-3.0+), libbig (MIT, C&C BIGF/BIG4), QuickBMS (GPL-2.0). See the Licences table.

### 2.2 RefPack (QFS) compression (used inside BIGF members and possibly SSH)

| Claim | Status |
|---|---|
| Header byte 0 = `0x10 | flags`, byte 1 = `0xFB`. Unpacked size is 3 bytes big-endian, or 4 bytes if flag 0x80 is set. A compressed size precedes it if flag 0x01 is set. Command stream: 2/3/4-byte copies, literal runs 0xE0–0xFB, stop codes 0xFC–0xFF. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving.md |
| RefPack is also called QFS. It was used by many EA games from the 1990s to the late 2000s, often with custom headers. | *sourced:* https://segaretro.org/RefPack_compression |
| Rust crate `refpack` 5.0.4 (MPL-2.0) supports "Reference", "Maxis" and "SimEA" header formats. Repo: https://github.com/actioninja/refpack-rs | *sourced:* https://docs.rs/crate/refpack/latest |

### 2.3 SSH (EA "shape", PS2 = `SHPS`)

| Claim | Status |
|---|---|
| Magic depends on platform: SHPI (PC), SHPP (PS1), SHPS (PS2), SHPX (Xbox), SHPM (PSP), SHPG (GC/Wii), SHPA (GBA). Header: magic, u32 size, u32 count, 4-char version (G354, GIMX, …). Directory: 4-char tag + u32 offset. 16-byte record header: type byte (7 bits) plus compression bit, 24-bit next-offset, w, h, centre x/y, shape-x/y with swizzle/mip bits. Types: 1 PAL4, 2 PAL8, 3 RGBA5551, 4 RGB888, 5 RGBA8888, 8–15 PS2 GST. Palettes 0x21/0x22/0x23/0x24/0x2A. Attachments 0x69 metal bin, 0x6F comment, 0x70 name, 0x7C hotspots. Licence of page: CC BY-SA 4.0. | *sourced:* https://rewiki.miraheze.org/wiki/EA_SSH_FSH_Image_(Type_1) |
| PC FSH (`SHPI`) basics: 16-byte header, 8-byte directory entries, `!pal` global palette. | *sourced:* https://modthesims.info/wiki.php?title=FSH |
| Nightfire `.ssh`: little-endian, magic `SHPS` (the loader also accepts `SHPE`). The size matches the real size. A "Buy ERTS" text pads up to the first entry. Record codes on disc: 1, 2, 4, 5, 0x20 (16-bit 5551 palette), 0x21 (32-bit palette), 0x69 (240-byte "EAGL240" metal bin), 0x70 (name). Pixels are **not** GS-swizzled. PSMT4 low nibble comes first. The PSMT8 CLUT index swaps bits 3 and 4. Alpha 0x80 → 255. Mips: `word14>>12` = levels−1. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving-textures.md |
| EA Graphics Manager (Python GUI) reads FSH/SSH/XSH/PSH/GSH/MSH and lists PS2 type IDs 1, 2, 3, 4, 5, 11, 13, 14, 130, 131. Licence GPL-3.0. Its PS2 game list does not mention Nightfire or AUF. | *sourced:* https://github.com/bartlomiejduda/EA-Graphics-Manager , https://raw.githubusercontent.com/bartlomiejduda/EA-Graphics-Manager/main/README.md |
| ReverseBox (its backend) does RefPack (wrapper), PS2 swizzle and palette swizzle. GPL-3.0. | *sourced:* https://github.com/bartlomiejduda/ReverseBox |
| Q3R_tools (MIT) has a `.ssh`→TGA converter for Quake III Revolution (PS2), with 8bpp palette-swizzle handling. | *sourced:* https://github.com/Yagotzirck/Q3R_tools |
| FSHTool 1.22 lists NFSHP2 support. No licence is given ("copyright (c) Denis Auroux"). | *sourced:* https://www.cmls.polytechnique.fr/cmat/auroux/nfs/index.html |

### 2.4 CRP (CARP object files) and `.o` relocatable modules

| Claim | Status |
|---|---|
| Nightfire `.crp` = CARP. The tag is stored byte-reversed (`PRAC`), as are all tags. Header: u32 head_flags, u32 plain_count, u32 1. 16-byte records {id, flags, count, offset}. `head_flags>>5` = number of heads. Flag bit0 = id is {u16 index, 2-char tag}; bit1 = offset relative; bits 8+ = payload size. Heads: `Arti` (Base bbox, Name, as, in, sr, ca), `CDat` (collision), `Map`, `RNgp` (road network/AI). Plain records: DBN, ELFd, ELFr, Sect, MapN, sn (names of `.ssh` libraries). | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving.md |
| `ELFd`/`ELFr` split a relocatable MIPS ELF32 object (.data / then strtab, symtab, rel.data, section headers). The EAGL DynamicLoader applies `R_MIPS_32`. Symbols include `__Model:::...`. Track geometry is VIF chains (V4-32, V4-8, V3-16 positions dequantised into the bbox, V2-16 UVs `(v-0x8000)/256`, optional V3-8 colours), as triangle strips. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving.md |
| `.o` in MISC.VIV = relocatable ELF renderer modules (`eaglrm.o`, `bondrm.o`) containing VU1 microcode. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving.md |
| NFS5 `.crp` had a community Z-Modeler filter. It is not known whether NFS5 CARP layout equals Nightfire's. | *sourced (filter exists):* https://www.nfsaddons.com/downloads/nfspu/cars/koenig/1330/koenig-c62-race-version.html ; layout equality *unverified* |
| No public NFS HP2 PS2 `.crp`/`.o` documentation or tool was found. | *unverified* (negative search result) |

### 2.5 ASF / MUS / SPE audio (EA SCHl)

| Claim | Status |
|---|---|
| Nightfire `MISxx.MUS` / `MISxxEN.SPE` are BIGF archives of `*.asf` SCHl streams. Driving sample banks hold "SPU ADPCM and EA-XA (`decxa16c`)" sounds. Stream headers include a 24-byte "CNYS" sync word. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/formats.md |
| EA chunk = FourCC + 4-byte size (including the 8-byte preamble). Byte order is little-endian on PC/PS/Xbox. Chunks: SCHl header, SCCl count, SCDl data, SCLl loop, SCEl end. The PT patch header has platform 5 = PS2. Tags: 0x80 version, 0x82 channels, 0x83 compression, 0x84 rate, 0x85 samples, 0xA0 codec2 (0x05 VAG, 0x0A EA-XA …). Defaults: 16-bit, mono, 22050 Hz, XA. | *sourced:* https://wiki.multimedia.cx/index.php/Electronic_Arts_SCxl , https://wiki.multimedia.cx/index.php/Electronic_Arts_Formats |
| vgmstream `ea_schl.c`: platform PS2 = 0x05. If codec2 is absent, PS2 defaults to `EA_CODEC2_VAG` (PS-ADPCM), version V1, 22050 Hz. Extensions include `.asf .str .sng …`. MPF/MUS (`.mpf` + `.mus`) is a different container from Nightfire's BIGF `.MUS`. | *sourced:* https://raw.githubusercontent.com/vgmstream/vgmstream/master/src/meta/ea_schl.c , https://raw.githubusercontent.com/vgmstream/vgmstream/master/doc/FORMATS.md |
| vgmstream licence: ISC. | *sourced:* https://raw.githubusercontent.com/vgmstream/vgmstream/master/COPYING |
| FFmpeg `electronicarts.c` (LGPL-2.1+) handles SCHl. Compression 7 → ADPCM_EA, revisions → EA R1/R2/R3. It falls back to ADPCM_PSX only for platform 0x01; there is no PS2 platform-5 default. | *sourced:* https://raw.githubusercontent.com/FFmpeg/FFmpeg/master/libavformat/electronicarts.c |
| NFS Underground PS2 MPC files use SCHl/PT/adpcm_ea_r2 audio. | *sourced:* https://wiki.multimedia.cx/index.php/Electronic_Arts_Formats |

### 2.6 MPC (EA PS2 video)

| Claim | Status |
|---|---|
| "MPC is another EA video format mainly used on the Sony PlayStation 2". `MPCh` is the only frame chunk and contains an elementary MPEG-2 video stream. | *sourced:* https://wiki.multimedia.cx/index.php/Electronic_Arts_MPC |
| FFmpeg's EA demuxer maps `MPCh` → `AV_CODEC_ID_MPEG2VIDEO` (header parsing on, keyframe flag). The 2008 patch added PS2 MPEG-2 support. | *sourced:* https://raw.githubusercontent.com/FFmpeg/FFmpeg/master/libavformat/electronicarts.c , https://ffmpeg.org/pipermail/ffmpeg-devel/2008-August/041788.html |
| Our `.mpc` in mission VIVs is this format. | *unverified:* check for `SCHl`/`MPCh` FourCCs. |

### 2.7 Text, config and misc (`.atr .tun .def .ini .dat .rel .gal .sfn .loc .sub .dir .h .bnk .bin .txt`)

| Claim | Status |
|---|---|
| `.atr` = attribute files (`sim/attrib/pvehicle/default.atr` overlaid by `<car>.atr`). `.tun` = tuning (`tuning/physics/...`, `tuning/Render/` fog and lighting). `drivecfg.def` and `camera.ini` use `[section]`, `KEY=value` / `KEY.f=value` (types f, i, u, b, s, v), with `//` comments. All eight level archives ship byte-identical `.tun`/`.def`. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving.md , https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving-physics.md |
| `loading.sfn` is an `FNTS` font. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving.md |
| `.sfn` is associated generically with "EA SFN Font". | *sourced (low quality):* https://filext.com/de/dateiendung/SFN |
| `.gal`, `.rel`, `.dir`, `.loc`, `.sub`, `.h`, `.dat` semantics. | *unverified:* no EA-specific documentation found. |
| EA "changes extensions from game to game", so the same structure can appear under different names. | *sourced:* https://wiki.multimedia.cx/index.php/Electronic_Arts_Formats |

## 3. DRIVING.ELF / DRIVING.SYM status

| Claim | Status |
|---|---|
| `BASE.ELF` swaps between `cdrom0:\ACTION.ELF;1` and `cdrom0:\DRIVING.ELF;1`. DRIVING.ELF is "Stripped; names in `DRIVING/DRIVING.SYM`". Driving missions use sp_level `0x0900000x`. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/formats.md |
| That project analyses a shipped DRIVING.ELF (vaddr = file offset + 0x107080). It uses IDA `sub_` addresses "in the shipped ELF, not the SYM". It says DRIVING.SYM names functions at addresses **from an older build**, so the names are unreliable, and it uses names only where "the SYM ordering and the code agree". Example: `sub_169288` = `PS2PadDevice::PollDevice`. | *sourced:* https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving.md , https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving-physics.md , https://github.com/Raikaru/nightfire-engine/blob/master/docs/driving-textures.md |
| A TCRF page for the PS2/GC/Xbox console version documenting DRIVING.SYM. | *unverified:* none found. Only the Windows page exists: https://new.tcrf.net/James_Bond_007:_NightFire_(Windows) |
| The Hidden Palace "Project Deluge" PS2 list has only a retail-matching Nightfire (Europe) ISO, not a prototype. | *sourced:* https://hiddenpalace.org/Special:Search?search=Nightfire&fulltext=1 |
| DRIVING.ELF is absent from our ISO's directory tree. | *unverified / conflicts with the source above.* Possibilities: our ISO9660 walker missed an entry (e.g. a multi-extent or Joliet/UDF-only record, or a different directory level); our image differs; or the file is present under a different case or path. Re-check before assuming it is packed. |

## 4. Prior reverse-engineering of Nightfire / AUF / BondPS2

| Claim | Status |
|---|---|
| `Raikaru/nightfire-engine`: open C++20 reimplementation for SLUS-20579. It uses ACTION.ELF as a spec and checks against PCSX2. On the driving side, BIGF/RefPack/CARP, embedded-ELF track objects, track collision, car physics and chase camera work. Every `DRIVING/*.VIV` level loads and is drivable except the submarine mission. Not done: traffic, AI, weapons, damage, non-car vehicles. **No LICENSE file is visible**; FFmpeg is bundled under LGPL. | *sourced:* https://github.com/Raikaru/nightfire-engine , https://github.com/Raikaru/nightfire-engine/tree/master |
| `NightfireResearch/nightfire-re`: Python notes and tools for the console versions (PS2/GC/Xbox), focused on the Action engine. The driving engine formats are "poorly understood right now". No licence is shown. Links: http://nightfire.dev , https://discord.nightfire.dev | *sourced:* https://github.com/NightfireResearch/nightfire-re |
| EMF 2026 talk "Reverse Engineering 007: Nightfire" (Charlie Bruce): team decompiling/rewriting the PS2/Xbox/GC game; Ghidra; link-time optimisation and missing-symbol pitfalls; emulator code injection. | *sourced:* https://media.ccc.de/v/emf2026-277-1-reverse-engineering-007-nightfire , https://app.media.ccc.de/v/emf2026-277-1-reverse-engineering-007-nightfire/audio |
| `noodlecollie/nightfire-open`: a GoldSrc (PC) remake, not relevant to the console driving code. | *sourced:* https://github.com/noodlecollie/nightfire-open |
| An anecdote (HN comment) about attempting to reverse-engineer AUF PS2, where the maps and models are compressed. | *sourced:* https://www.libhunt.com/posts/340154-a-digital-museum-of-video-game-levels |
| Any public RE of AUF driving code, or anything named "BondPS2". | *unverified:* none found. |

## Licences

| Project | URL | Licence (as found) | Use for us |
|---|---|---|---|
| vgmstream | https://github.com/vgmstream/vgmstream | ISC (COPYING) | Reusable with notice (SCHl/EA-XA/VAG logic) |
| refpack-rs (`refpack` crate) | https://github.com/actioninja/refpack-rs | MPL-2.0 | Reusable as a dependency (file-level copyleft on modified files) |
| libbig | https://github.com/feliwir/libbig | MIT (LICENSE.md) | Reusable with notice |
| Q3R_tools | https://github.com/Yagotzirck/Q3R_tools | MIT | Reusable with notice (ssh → TGA, Q3R variant) |
| FFmpeg electronicarts.c | https://github.com/FFmpeg/FFmpeg | LGPL-2.1+ | Format reference / dynamic link only |
| unvivtool / libnfsviv.h | https://github.com/bfut/unvivtool | GPL-3.0+ | Format reference only |
| unvivtool `references/*.md` specs | https://github.com/bfut/unvivtool/tree/main/references | CC BY 4.0 (stated in `nfs5_can_specs.md`, `viv_C0FB_specs.md`) | Facts usable with attribution |
| EA Graphics Manager | https://github.com/bartlomiejduda/EA-Graphics-Manager | GPL-3.0 | Format reference only |
| ReverseBox | https://github.com/bartlomiejduda/ReverseBox | GPL-3.0 | Format reference only |
| QuickBMS | http://aluigi.altervista.org/quickbms.htm | GPL-2.0 (per AUR / README mirror) | Format reference only |
| RE Wiki EA SSH/FSH pages | https://rewiki.miraheze.org/wiki/EA_SSH_FSH_Image_(Type_1) | CC BY-SA 4.0 | Facts; attribute if text is quoted |
| Auroux NFS tools (FSHTool, T3ED) | https://www.cmls.polytechnique.fr/cmat/auroux/nfs/index.html | None stated ("copyright (c) Denis Auroux") | Facts only |
| Raikaru/nightfire-engine | https://github.com/Raikaru/nightfire-engine | **None found** | Facts only; do not copy code |
| NightfireResearch/nightfire-re | https://github.com/NightfireResearch/nightfire-re | None stated in README view | Facts only |
| MultimediaWiki | https://wiki.multimedia.cx/ | Not stated on fetched pages | Facts only |

## Open questions

1. Is DRIVING.ELF really absent from our ISO? Raikaru's docs treat it as a normal disc file (vaddr = offset + 0x107080). Re-walk the ISO9660 tree (all directory levels, multi-extent records), compare the image hash, and grep raw sectors for an ELF header with `.text` near 0x107080+.
2. Is our DRIVING.SYM from an older build than the shipped DRIVING.ELF, as Raikaru claims? Test this by matching SYM addresses against function prologues.
3. What is the endianness of the BIGF size field per archive? Raikaru says LE for Nightfire; unvivtool says BE for BIGF. Does MISC.VIV's mismatch disappear under the other byte order, or does it reflect a PAL/NTSC rebuild (cf. Xbox `misc ntsc.viv`)?
4. Does Nightfire's CARP layout match NFS5 (Porsche Unleashed) `.crp`, or AUF's driving data? Compare against an NFS5 PC `.crp` and the AUF PS2 disc.
5. Which audio codec is in the driving `.asf` (VAG vs EA-XA R1/R2)? Read PT tag 0x80/0x83/0xA0 per file.
6. Are the `.mpc` files present and of the `MPCh` MPEG-2 variant? Can FFmpeg play them as-is?
7. What are `.gal`, `.rel`, `.dir`, `.loc`, `.sub` and `.h`? No public docs were found.
8. Who exactly wrote the driving code (EA Canada vs EA Redwood Shores vs Savage)? Sources conflict. The in-game credits on our disc would settle it (MobyGames credits returned HTTP 403).
9. EAGL version used (EAGL "1" vs "4") — sources conflict.
