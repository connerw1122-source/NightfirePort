# Level / pack `.bin` archives (the `NNNNNNNN.bin` files in FILES.BIN)

Reader: `crates/nf-formats/src/levelbin.rs`. Golden: `golden/level_bins.tsv` (per archive: entry count, type
histogram, total bytes, SHA-1 of the full entry table with per-entry content SHA-1s). CLI: `nf-extract bin-list`.
**Cross-check:** `tools/crosscheck/nfre_level_bins.py` runs nightfire-re's `extract_bin` — 96 archives, 41,951 named
entries (name, type, SHA-1) + 26 unnamed (type, SHA-1): **0 differences** (WORKLOG 0007).
Confidence: high (*measured*: decomp of `LoaderLoad` 0x1d4a00 and `psiFileRead` 0x10bdf0, plus all 99 files,
WORKLOG 0007). First described by NightfireResearch (`extract_bin.py`, *sourced*, facts only).

## Loader (ACTION.ELF, *measured*)

- `LoaderLoad(type, hashcode, buf, variant)` builds the name `"%s%8.8x.bin"` (prefix string at 0x30d018) from the
  hashcode with its 0x00F00000 nibble rewritten: `variant ≠ 0` → `(variant + 7) << 20`; loader type 0 or 3 → `|0xF`
  (e.g. 07F00048); type 4 → `CommonAnimFile`; type 5 → `|0xD`. It then reads in states:
  1. header: one 0x800-byte read; word 0 = directory size, word 1 = entry count; if the count is non-zero, the
     directory (word 0 bytes);
  2. directory walk, up to `MaxFilesToLoad` entries per batch (initialised to 0xFFFFFFFF: one batch);
  3. one read of the batch's summed entry lengths into `DirFileBuf`;
  4. per entry, dispatch on the type byte to the type's loader (table below).
- `psiFileRead` reads `ceil(n / 2048)` whole sectors and advances the stream by that much (it prints a warning when
  `n` is not a multiple of 2048). So the directory occupies `round_up(dir_size, 2048)` and each batch is sector-aligned.
- `Europacked` (0x30d003) selects a decompress-from-`Euro_Decomp_Buf` path; it is initialised to 0 and never
  written in ACTION.ELF, so PS2 archives are stored uncompressed.
- Names: the loader upper-cases each name and parses `DirFileHash` from up to 8 hex digits of its last path
  component (after `\` or `/`).

## Layout

```
0x000  u32 dir_size      (multiple of 2048 in all 96 non-empty archives)
0x004  u32 entry_count   (0 in 07F00048, 07F00500, 07F00999: 2048 zero bytes)
0x008  zero padding to 0x800
0x800  directory: entry_count × { u32 length, u8 type, char name[] NUL-terminated }, zero-padded to dir_size
0x800 + dir_size: entry data, back to back in directory order (no alignment between entries)
file size = 0x800 + dir_size + Σ length    (exact in all 96 archives)
```

## Entry types (*measured* counts over all 96 archives, 41,977 entries; meanings *suggested* from NightfireResearch until each
loader is read)

| Type | Count | Name form | Hashcode prefix | Suggested meaning |
|---|---|---|---|---|
| 0 | 978 | 8 hex | 01 | map data (no AnimPostLoadInit) |
| 1 | 35 | 8 hex | 01 | map parser (level map) |
| 2 | 551 | 8 hex | 01 | map data |
| 3 | 1,217 | 8 hex | 05 | animation: skin? |
| 4 | 18,252 | 8 hex | 04 (8 × 00) | animation sequence data |
| 5 | 18,231 | 8 hex | 06 | animation skin/script data |
| 6 | 1,485 | `SKELnnnn` | — | skeleton |
| 7 | 468 | 8 hex | 06 | script (keyframe animation) |
| 8 | 34 | 8 hex | 08 | menu manager |
| 11 | 666 | 8 hex | 01 | map data, alternate memory type |
| 12 | 26 | empty | — | always 32 bytes; modifies `LoadableIndex` (deferred load?) |
| 15 | 1 | `007` | — | icon (memory card), only in 07000048.bin |
| 16 | 33 | `woman` | — | "woman" data (NightfireResearch: compressed) |

Open: the per-type loaders (U-016), the meaning of type 12 entries, and whether type 4 entries with prefix `00` are
special.
