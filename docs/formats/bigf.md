# EA BIGF archives (`DRIVING/*.VIV`, `*.MUS`, `*EN.SPE`)

Reader: `crates/nf-formats/src/bigf.rs`. Golden: `golden/bigf_entries.tsv` (23 archives, 1,499 entries, per-entry
SHA-1). Confidence: high for the layout (*measured* on all 23 archives); the size field is not understood (U-004).

```
0x00  "BIGF"
0x04  u32 LE  archive size   (= file size in 22/23; MISC.VIV: 0x295c450b vs 0xb456590)
0x08  u32 BE  entry count
0x0c  u32 BE  header size    (.VIV: = end of entry table; .MUS/.SPE: table end + 8)
0x10  entries: u32 BE offset, u32 BE size, NUL-terminated name (backslash paths, e.g. data\render\eaglrm.o)
      .MUS/.SPE: after the table, "L225" + u32 BE 2 (U-005)
```

- Offsets are archive-relative, sorted ascending, first data at a 64/2048-byte-aligned offset after the header.
- Contents (*measured* by extension): mission `.VIV`: `.dir .atr .tun .crp .def .ssh .dat .ini .h .bnk .o .bin .txt
  .rel .gal .sfn`; `MISC.VIV`: render modules `eaglrm.o`/`bondrm.o`, HUD/galleries `.gal`, locale `.loc`, loading
  screens `.ssh`, `.mpc` cutscenes, control configs `data\control\*.def`, car models; `.MUS`/`.SPE`: `.asf` audio.
- Community notes (*suggested*, docs/research/driving-engine-ea-formats.md): some EA tools read the size field
  big-endian and ignore it when it disagrees with the file; Raikaru/nightfire-engine reads it LE with count/header BE.
- Cross-check against an independent BIGF tool: not done yet (G-002, needs a download).
