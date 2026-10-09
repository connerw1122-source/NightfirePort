# Unknowns

Open questions. Each names how it could be settled. Close an entry by moving its answer into the relevant doc and
noting the WORKLOG entry here.

| ID | Question | How to settle | Status |
|---|---|---|---|
| U-001 | Our image's SHA-1 (`03717059…`) differs from the ISO SHA-1 NightfireResearch lists for SLUS-20579 (`5dcb9e55…`, *sourced:* wiki.nightfire.dev Known_Versions), while both ELFs' SHA-256 match theirs. Different dump (padding/region of unused sectors) or different pressing? Also: PCSX2 game CRC (reported `5B86BB62`, *suggested*) and Redump entry. | Compare against Redump's SLUS-20579 entry (needs a web lookup); read the CRC from PCSX2's log when booting our image. | open |
| U-002 | ~~How does the loader reach DRIVING.ELF if unlisted?~~ It is listed (LBA 1153); WORKLOG 0001 error. | — | closed (0001) |
| U-003 | What build does `DRIVING/DRIVING.SYM` describe? 4 % of its in-text addresses fall on function boundaries of our DRIVING.ELF; its `_end` is 0x55cf04 vs 0x4279b4. NightfireResearch also say its offsets are wrong (*sourced*). Can names be transferred by matching function shapes? | Match functions by size/shape (call graph, constants) between the SYM layout and our ELF. | open |
| U-004 | MISC.VIV's BIGF size field bytes `0b 45 5c 29` match neither LE (0x295c450b) nor the file size (0xb456590); read big-endian (as unvivtool does) it is 0x0b455c29 = file size − 2,407. The other 22 archives store the exact file size LE. Meaning? (A stale size from an earlier build of the archive?) | Check the driving loader's BIGF reader in DRIVING.ELF. | open |
| U-016 | What each level `.bin` entry type is (0–16; table in docs/formats/level-bin.md, meanings only *suggested*), and where textures live (no entry has the `03` "texture" hashcode prefix NightfireResearch's wiki lists). | Read the type dispatch in `LoaderLoad` state 4 and each loader. | open |
| U-015 | `MIS3EN.SPE` holds two different entries both named `garage.asf` (#0: 3,090,020 bytes, #35: 2,923,496). Which one does the driving engine load — first match, last, or by index? | DRIVING.ELF BIGF lookup code. | open |
| U-005 | Meaning of the `L225` + BE u32 2 trailer after the entry table in `.MUS`/`.SPE` BIGFs. | EA BIGF docs; driving loader. | open |
| U-006 | ~~Are `FileList` sizes padded?~~ Yes, sector-padded, and the loader passes that size on as the length; inner lengths are per format (docs/formats/files-bin.md). | — | closed (0003) |
| U-010 | ~~Pad buffer layout and button bits?~~ Confirmed live: 0x128 RX, 0x129 RY, 0x12a LX, 0x12b LY; all 16 button bits as in input.md §2. | — | closed (0006) |
| U-011 | Names of the 40 `GameActions` (only 0–6 identified by use). | Cross-reference every `Input_Action[f]` call site; menu/options strings. | open |
| U-012 | ~~Units of the frame-time words?~~ EE timer 0 ticks; `main` sets `REG_RCNT0_MODE = 0x83` (CLKS 3 = H-blank, *suggested* register semantics), so scanlines; 262.5 = one NTSC field. Live in level 0x07000001: game ≈ 33–51, draw ≈ 34–38 scanlines (WORKLOG 0005). | — | closed (0005) |
| U-013 | In real gameplay, how often does the game run at 30 vs 60 Hz? Press/secondary sources say PS2 is 30 fps (*suggested*). **Measured in PCSX2, level 0x07000001: 98.8 % of updates at 60 Hz** (WORKLOG 0006). Still open: heavier levels, multiplayer, and whether PCSX2's EE timing matches hardware. | `tools/trace/frame_pacing.py` in more levels; real-hardware footage frame counting. | partly answered |
| U-014 | Which `GameState`+12 values are levels (0x07000048 forces 4-player input mapping). | Map against FileList names / NightfireResearch hashcode list. | open |
