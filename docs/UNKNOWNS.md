# Unknowns

Open questions. Each names how it could be settled. Close an entry by moving its answer into the relevant doc and
noting the WORKLOG entry here.

| ID | Question | How to settle | Status |
|---|---|---|---|
| U-001 | Our image's SHA-1 (`03717059…`) differs from the ISO SHA-1 NightfireResearch lists for SLUS-20579 (`5dcb9e55…`, *sourced:* wiki.nightfire.dev Known_Versions), while both ELFs' SHA-256 match theirs. Different dump (padding/region of unused sectors) or different pressing? Also: PCSX2 game CRC (reported `5B86BB62`, *suggested*) and Redump entry. | Compare against Redump's SLUS-20579 entry (needs a web lookup); read the CRC from PCSX2's log when booting our image. | open |
| U-002 | ~~How does the loader reach DRIVING.ELF if unlisted?~~ It is listed (LBA 1153); WORKLOG 0001 error. | — | closed (0001) |
| U-003 | What build does `DRIVING/DRIVING.SYM` describe? 4 % of its in-text addresses fall on function boundaries of our DRIVING.ELF; its `_end` is 0x55cf04 vs 0x4279b4. NightfireResearch also say its offsets are wrong (*sourced*). Can names be transferred by matching function shapes? | Match functions by size/shape (call graph, constants) between the SYM layout and our ELF. | open |
| U-004 | MISC.VIV's BIGF size field is 0x295c450b, not the file size 0xb456590 (all 22 other archives match). Meaning? | Check the driving loader's BIGF reader in DRIVING.ELF; EA tool docs. | open |
| U-005 | Meaning of the `L225` + BE u32 2 trailer after the entry table in `.MUS`/`.SPE` BIGFs. | EA BIGF docs; driving loader. | open |
| U-006 | ~~Are `FileList` sizes padded?~~ Yes, sector-padded, and the loader passes that size on as the length; inner lengths are per format (docs/formats/files-bin.md). | — | closed (0003) |
| U-010 | Pad buffer layout: data[4..7] = RX, RY, LX, LY and the button bits (assumed from libpad). | PCSX2 PINE read of `tSlot`+0x124 while pressing known buttons/sticks. | open |
| U-011 | Names of the 40 `GameActions` (only 0–6 identified by use). | Cross-reference every `Input_Action[f]` call site; menu/options strings. | open |
| U-012 | Units of `PS2GameCodeTimeW`/`PS2DrawCodeTimeW`/`PS2DrawFrameTimeW` (threshold 262.5 — scanlines?). | Find where they are written (RCNT reads). | open |
| U-013 | In real gameplay, how often does the game run at 30 vs 60 Hz (`PS2FramesToSkip` 2 vs 1)? NightfireResearch/press say PS2 is 30 fps (*suggested*, unmeasured). | PINE-sample `PS2FramesToSkip` (0x30c70c) during gameplay in several levels. | open |
| U-014 | Which `GameState`+12 values are levels (0x07000048 forces 4-player input mapping). | Map against FileList names / NightfireResearch hashcode list. | open |
