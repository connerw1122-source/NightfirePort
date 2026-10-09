# Map entries (level `.bin` types 0/1/2/11) and their textures

Readers: `crates/nf-formats/src/map.rs` (block table), `texture.rs` (textures). Goldens: `golden/textures.tsv`
(per archive: count, 8-bpp count, short count, SHA-1 of every texture's index and palette hashes). CLI:
`nf-extract tex-list | tex-digest | tex-export`. Confidence: high for the block table and textures (*measured*,
WORKLOG 0008); other blocks are only identified by their parser's name so far.

## Block file (*measured*: `parsemap_parsemap` 0x1d1cf0, `parsemap_parsenextblock` 0x1d12e8, all 2,230 map entries)

```
0x00 u32 version (BinFileVersion): 1 in every map entry on our disc
0x04 u32 offset[]: block k (parse order) starts at 4 + offset[k]; the list ends with the block whose id is 0x1D
block: u32 header = id << 24 | size (size includes the header), then the payload
```

- Parse order is the table order, not file order (the map header block sits near the end of the data).
- Version ≠ 1 would chain blocks by size from offset 4 (`parsenextblock`); unused on our disc.
- In single-file mode (`LoaderLoad`), `psiFileLoad` returns the `.bin` entry's buffer, so a map entry *is* the map file.
- When `MP_NeedFile(hash)` is false the parser skips every block except 0x1C (records the memory-discard point) and
  0x1D (end).

Block ids on our disc (*measured* counts / total bytes over 2,230 maps) and their handlers in
`parsemap_handle_block_id` (0x1d1020):

| Id | Count | Bytes | Handler |
|---|---|---|---|
| 0x04 | 17,612 | 1.2 MB | `parsemap_block_entity_params` (one per PS2_GFX) |
| 0x05 | 28 | 1.3 MB | `AIPath_Parse` |
| 0x0E | 2,230 | 20 each | `parsemap_block_map_header` (entity count at +4, path count at +8) |
| 0x0F | 2,230 | | `parsemap_block_palette_header` (8 bytes per texture) |
| 0x10 | 2,230 | | `parsemap_block_texture_header` (12 bytes per texture) |
| 0x12 | 25,082 | 13.3 MB | `parsemap_block_palette_data_psx` |
| 0x16 | 25,082 | 164 MB | `parsemap_block_texture_data_pc` (pixels) |
| 0x19 | 641 | 0.4 MB | `parsemap_block_path_data` (main map only) |
| 0x1A | 2,230 | 2.3 MB | `parsemap_block_map_data_static` (also saved as `DynamicBH`) |
| 0x1C | 2,230 | | `parsemap_block_memory_discard` |
| 0x1D | 2,230 | | end |
| 0x21 | 2,230 | | `parsemap_block_portal_data` |
| 0x26 | 73 | | `Light_SetAmbientRadiators` (main map only) |
| 0x27 | 2,230 | 0.15 MB | not in the switch: ignored by the parser |
| 0x28 | 47 | | `Sound_LoadMapSounds` (main map only) |
| 0x2B | 220 | 14.4 MB | `parsemap_block_morph_data` |
| 0x2C | 2,230 | | `parsemap_block_hashlist` |
| 0x2D | 17,612 | 241 MB | `parsemap_block_PS2_GFX` (geometry; next) |
| 0x2E | 7,985 | 52.8 MB | `parsemap_block_Coll_Data_New` (collision) |
| 0x30 | 34 | | `parsemap_block_particles` |

## Textures (*measured*: the four block parsers, `psiCreateMapTextures` 0x11d200, all 25,082 textures)

- Block 0x10 holds 12-byte entries, 0x0F 8-byte entries, one per texture; then one 0x16 (pixels) and one 0x12
  (palette) block per texture, in table order. Counts agree in every map.
- Texture header entry: `+0 u8 flags` (bit 0 = 256 colours / 8 bpp, else 16 / 4 bpp; values 0x00 0x01 0x10 0x11 0x18
  0x19), `+1 u8 ?`, `+2 u16 width−1`, `+4 u16 height−1`, `+6 u8 frames`, `+7 u8 rate` (frames per second: 1–59 → `60 / rate` ticks per frame, else 1; frame = (game ticks / that) % frames, `CacheTexture` 0x11cd58 [M]),
  `+8 u32` hashcode on disc (0xFFFFFFFF in 22,550, `03xxxxxx` in 2,532; replaced by the pixel pointer at load).
- Palette header entry: `+4 u32` texture hashcode (replaced by a palette pointer); the palette block registers the
  texture in the hash table under it.
- Palette: 16 × RGBA (64 bytes) for 4 bpp, 256 × RGBA (1,024 bytes) for 8 bpp; alpha on the PS2 scale, 0..=0x80 (never
  above 0x80). 256-entry palettes are in GS **CSM1** order (entries 8–15 ↔ 16–23 of every 32). 4-bpp pixels: left
  pixel in the low nibble. Both decided by measurement over all textures (mean neighbour colour difference: CSM1
  17.7 vs linear 55.3, smoother in 11,990/12,033; low-first 10.95 vs high-first 12.65).
- Pixels: `width × height × frames` indices (halved for 4 bpp), frames stacked. 25,079 textures match exactly; two
  have extra bytes and one (0700000d.bin entry 703, texture 18) is short: 512 bytes for a 2,048-byte header
  (UNKNOWNS U-017). The game deduplicates by CRC-32 of the pixel bytes.
- Export convention (port's choice): 8-bit alpha = `round(a × 255 / 128)`.
- **Cross-check** (`tools/crosscheck/nfre_textures.py`, nightfire-re's own block handlers and depalettizer):
  **27,130 / 27,130 frames pixel-exact** (24,696 single-frame + 2,434 animation frames); the short texture is
  excluded on both sides. Alpha compared on the PS2 scale (theirs floors, ours rounds).
