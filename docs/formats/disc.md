# Disc layout (SLUS-20579)

*Measured* with `nf-extract` and cross-checked with 7-Zip (WORKLOG 0001). Full per-file table with SHA-1s:
`crates/nf-formats/golden/disc_files.tsv`.

- ISO 9660, 2048-byte sectors, 1,191,072 sectors; volume id `NIGHTFIRE`; created 2002-10-20.
- Sectors not covered by any directory entry: 0–20 (system area, PVD, root directory sector, …), 276–484 (209
  sectors before `SYSTEM.CNF`, first non-zero byte at offset 1 — likely path tables/directory padding, unchecked),
  1,191,066–1,191,071 (6 sectors, non-zero data at +10240 bytes; unchecked).

| Path | Role (*measured* unless tagged) |
|---|---|
| `SYSTEM.CNF` | `BOOT2=cdrom0:\SLUS_205.79;1`, `VER=1.00`, `VMODE=NTSC` |
| `SLUS_205.79`, `BASE.ELF` | identical boot loader (0xa00000); picks ACTION or DRIVING |
| `ACTION.ELF` | Eurocom engine (unstripped) |
| `DRIVING.ELF` | EA driving engine (stripped), LBA 1153 |
| `FILES.BIN` | 764 MB, no header; indexed by `FileList` in ACTION.ELF (118 entries; docs/formats/files-bin.md, to write) |
| `MODULES/*.IRX`, `IOPRP243.IMG` | IOP modules (SFX/SNDDRV sound drivers, pad, memory card, multitap, USB) |
| `MOVIES/30_FPS/*.PSS` | 22 movies (MPEG-2 PS; pss.rs unverified here, G-008) |
| `PS2/ENGLISH/SB_n/SB_n.{SBF,SFX,SHF}` | 25 sound banks |
| `PS2/ENGLISH/STREAMS/STREAMS.{BIN,LUT}` | speech streams |
| `PS2/MUSIC/MFX_n/MFX_n.{SMF,SSD}`, `PS2/MFXINFO.MXI`, `PS2/SBINFO.SBI` | music and bank info |
| `PS2/DEBUG.TXT` | `#sfxlist` of SFX names |
| `DRIVING/*.VIV`, `*.MUS`, `*EN.SPE` | EA BIGF archives (docs/formats/bigf.md) |
| `DRIVING/DRIVING.SYM` | symbol table of another driving build (docs/formats/driving-sym.md) |
