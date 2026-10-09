# Map static data (block 0x1A): placements, and level assembly

Readers: `crates/nf-formats/src/placement.rs`, `level.rs`. Golden: `golden/levels.tsv` (per level archive: maps,
main map, placements, cels, resolved, unresolved, triangles). CLI: `nf-extract level-summary | level-export`;
viewer `nf-view`. Confidence: high for the record layout and resolution (*measured*); the rotation matrix entries are
*measured* from the decomp, but the platform sine differs from the game's `PS2Sinf3` (G-017), and the world's
handedness on screen is not yet confirmed against PCSX2 (U-023).

## Record (`parsemap_block_map_data_static` 0x1d1998)

```
+0x00 u16 object index in this map (used when the hashcode is 0xFFFFFFFF)
+0x04 u32 hashcode: else an entity hashcode resolved with hashtable_getitem (another map's object)
+0x08 u32 flags: 0x8000 = create a world cel now (`build_alloc_cel`); low 16 bits stored in cel+0x3C
+0x0C f32[3] translation (cel+0x60)        +0x18 f32[3] rotation angles, radians (cel+0x70)
+0x24..+0x47 not read here
+0x48 u32 n; +0x4C n × {u32 index, u32 value} parameters (parseentity_fixup_entity)
size 0x4C + 8n; the block ends exactly at the last record in all 2,230 maps
```

Survey of all 23,329 records: 10,069 by index (all < the map's object count), 13,260 by hashcode (all resolve to an
entity hashcode in the same level archive), 5,602 with the 0x8000 flag. In level 1's main map the rooms
(`first_garden`, `corridor1`, …) are 0x8000 cels at the identity transform — room geometry is modelled in world
space; flag 0x24 = `Player1` start, 0x4E = pickups/drones by hashcode (created by the dynamic-object code).

## Matrix (`parseentity_transform_bounding_box` 0x1d0fb0 → `RotMatrix` 0x1e2368 + `Mat_SetTrans`)

Row-vector convention (`v' = x·row0 + y·row1 + z·row2 + row3`, as the VU0 code in
`parseentity_transform_bounding_box` computes it). With `s/c` = sin/cos of the angles x, y, z:

```
row0 = [ cy·cz,   sx·sy·cz + cx·sz,   −cx·sy·cz + sx·sz ]
row1 = [ −cy·sz,  cx·cz − sx·sy·sz,   sx·cz + cx·sy·sz  ]
row2 = [ sy,      −sx·cy,             cx·cy             ]
row3 = translation
```
The game computes the sines with `PS2Sinf3` (cos via sin(a + π/2)).

## Assembly (`level.rs`)

A level archive's maps are parsed in archive order; objects are (PS2_GFX, following entity params) pairs; an entity
hashcode resolves to its first occurrence in the archive. Instances = the main map's (loader type 1) placements,
optionally only the 0x8000 cels. A batch's texture index refers to the texture list of the map its object comes from.
All 36 level archives resolve every placement (golden `levels.tsv`): 2,531,353 triangles in total.
