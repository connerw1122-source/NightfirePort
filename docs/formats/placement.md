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

## Sky objects (WORKLOG 0014)

`parsemap_create_dynamic_objects` (0x1d04f0) switches on the placement's **object type = flags & 0xFF** (0x0F →
`Drone_Create`, 0x28 sensor, 0x29 switch, 0x2A sky, …); its parameter block puts parameter k at +0x2C + 4k.
Type 0x2A calls `View_AddSkyObj` (0x1e79a0): slot = parameter 0 (21 slots in `GfxList`, 0x40 bytes each), parameter 1
≠ 0 = the lightning-flash variant of that slot (shown at random intervals from parameters 2/3), position/rotation from
the record. `View_DrawSky` (0x1e5ad0) draws the slots in order before the world; glist flag bit 2 = drawn around the
viewer (skyboxes, moons), otherwise at the stored position (distant silhouettes such as `Sky_Tower*`,
`Skyline_Buildings`); level 0x0700001B rotates its sky. 74 sky placements on the disc.
nf-view: sky pass first (slot order, no depth test or write; viewer-centred objects use the view without
translation), then the world. Lightning variants and the rotating sky are not shown.
