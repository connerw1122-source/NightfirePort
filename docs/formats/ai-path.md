# AI paths and boundaries (map block 0x05)

Reader: `crates/nf-formats/src/aipath.rs`. Golden: `golden/ai_paths.tsv` (every record on the disc: kind, node /
link counts, extra length, out-of-range links, name, SHA-1). CLI: `nf-extract ai-path-digest <iso>`,
`nf-extract block-dump <iso> <level.bin> 05 <dir>`. Confidence: high for the layout (*measured*: `AIPath_Parse`
0x1d2598, `AIPath_BindNodes` 0x1d2af8, `AIPath_Prepare` 0x1d2cc0; all 210 records on the disc parse to their
block's exact length and every link stays inside its record's nodes, WORKLOG 0021).

## Layout (*measured*)

The parser gets the block start (id/size word); offsets below are from the payload (after that word).

```
+0 u32 version    8 — any other value and the game ignores the block (AINetwork = version)
+4 u32 count      records
+8 records
```

Record:

```
+0x00 char[0x80] name                 e.g. "NavPath - A", "PatrolPath 1 - (Dr_X3)", "Boundary Prime"
+0x80 u32 kind                        bit 0 set → boundary (AIBounds_*), else path (AIPath_*)
+0x84 u32[8] parameters               copied into the in-memory record unchanged (meaning: U-030)
+0xA4 u32 node count
+0xA8 u32 extra length                bytes before the nodes (0, 4, 8 or 12 on the disc; 49 of 210 records,
                                      almost all the first or second record of their block; meaning: U-030)
+0xAC extra[extra length]
      nodes[node count]               path: 0x40 bytes each; boundary: 0x20 bytes each
      u32 link count
      links[link count]               path: 16 bytes each; boundary: 20 bytes each
```

Node (both sizes): `+0x10 f32 x, y, z` position; the game adds **0.4 to y** at load. `+0x00` is overwritten with
(u16 index, u16 record index), `+0x04` kept (0x2468ABCE on disc → 0), `+0x0C` cleared; the rest of a path node
holds load-time pointer slots that the disc fills with markers (0x87654321, 0x9ABCDEF0, 0x89AB6745, …), and `+0x1C`
is 1.0 and `+0x20..+0x2C` 0.123 in every node seen (meaning: U-030). A path node also gets a 0x30-byte `NODE_tag`
(array at record +0x34): +4 → the node, +0x11 u8 link count, +0x12 u16 first index into the adjacency list.

Link: `+0x08 u16` node a, `+0x0A u16` node b (indices within the record). `+0x0C` becomes the link length
(`Vec_Magnitude` of the two node positions; `AIPath_Prepare`). Boundary links have 4 more bytes (0x01234567 on disc).

Adjacency (`AIPath_Prepare`): per node in order, the indices of the links with a or b equal to it, in link order.

## In memory (*measured*)

- Paths: array at 0x2c7964, 0x48 bytes each, count `0x2c7960` (`+0x2C` nodes, `+0x30` node data, `+0x34`
  `NODE_tag`s, `+0x38` links, `+0x3C` link data, `+0x40` adjacency). Largest node / link counts in `0x2c7958` /
  `0x2c795c`.
- Boundaries: array at 0x2c8778, 0x40 bytes each, count `0x2c8774`; `BNODE_tag` 8 bytes each; prepared by
  `AIBounds_Prepare`.
- `AIPath_Link2Cel` / `AIBounds_Link2Cel` tie nodes to cels (rooms) after load.

## Kinds on the disc (*measured*, golden)

| kind | records | reading (*suggested* from names) |
|---|---|---|
| 1 (bit 0) | 23 | boundary ("Boundary", "Boundry", "Boundaries") |
| 2 | 90 | navigation path ("NavPath", "Default Path", "BotPath4") |
| 4 | 41 | patrol path ("PatrolPath", "(patrol)") |
| 8 | 24 | scripted routes ("Mayhew Route", "TruckDriverPath", "Party Patrol Path") |
| 0x14 ×5, 0x201 ×2, 0x202 ×3, 0x208, 0x80000002 ×3, 0x80000004, 0x80000014 | 16 | low bits as above plus flags: U-030 |
| 0x3D01 ×7, 0x9801, 0x7A02, 0x7A04, 0xA902, 0xA904, 0xFF02 ×2, 0xFF04 ×2 | 16 | a value in bits 8–15: U-030 |

07000005: NavPath B (18 nodes), PatrolPath 1 (27), Boundary Prime (213), NavPath A (87 nodes, 122 links), PatrolPath
(Dr_17) (11), Run_To (6), Default Path (7). Drone placement parameter 16 is *suggested* to select a path (values
0/2/3 in 07000005; U-030).
