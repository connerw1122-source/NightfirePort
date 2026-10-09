# FILES.BIN and `FileList`

Reader: `crates/nf-formats/src/filelist.rs`; golden `golden/files_bin.tsv` (118 entries, content SHA-1s);
CLI `nf-extract files-manifest|files-extract`. Confidence: high (*measured*, WORKLOG 0003).

- `FILES.BIN` (root, LBA 817,977, 764,086,272 bytes) has no header. It is indexed by `FileList` in ACTION.ELF
  (symbol at 0x2446a0, 3,776 bytes = 118 × 32), first described by NightfireResearch (*sourced*, facts only).

```
0x00 char[16] name   NUL-padded ("DUTxt.dat", "07000001.bin", "TuningVars.txt", …)
0x10 u32 LE   offset (2048-aligned; the game reads from sector DVDfp + offset >> 11)
0x14 u32 LE   size   (the length the game hands to callers)
0x18 u32 × 2  always 0
```

- Loader (*measured*): `psiFileOpen` 0x10bd08 / `psiFileLoad` 0x10ba38 find a name by case-insensitive `strcmp`
  over all 0x76 entries, `Mem_Malloc` the size rounded up to 2048, `sceCdRead` whole sectors, and return `size` as
  the length. `bSingleFileMode` instead serves a buffer preloaded into `DirFileBuf`/`DirFileLen` (dev/single-file
  mode; not yet traced).
- Layout (*measured*): entries are contiguous with no gaps (next offset = offset + size in every case); 117 of 118
  sizes are multiples of 2048 (the exception is `TuningVars.txt`, 24,467 bytes, last). So content is sector-padded
  inside the stated size; per-format headers give inner lengths.
- Names: 99 `NNNNNNNN.bin` hashcode files (levels/packs; type byte in the hashcode per NightfireResearch's wiki,
  *suggested*: 01 pack, 02 mesh, 03 texture, 04 sequence, 05 skin, 06 script, 07 level, 09 driving, 10/40 menu),
  18 `*.dat` text tables (`DUTxt`, `DUTxtU`, `FRTxt`, …, one plain and one `U` per language), `TuningVars.txt`.
- Cross-check with an independent extractor: pending (G-001).
