# Searchlights — the bullet child-list path (U-041)

The searchlight port lives in `crates/nf-game/src/searchlight.rs` (`Searchlight_Create` 0x1cb4f8 /
`Searchlight_Update` 0x1cb900; layout [L]-pinned in `work/ee_start_05.bin`; the sweep/tracking/break
state machine is covered by G-033 and its reports). This page documents what U-041 asked: how a
bullet reaches the searchlight object, what the head's child list is, and what breaks the light.

## 1. The objects

Every searchlight part is its own dynamic object, created per 0xDE placement by
`Searchlight_Create` 0x1cb4f8 (`parsemap_create_dynamic_objects` 0x1d04f0 case 0xde:341-344):

- class byte `obj+0xFF` = 0x22 [M 0x1cb4f8:66];
- `obj+0xD8` = the placement's celglist entry — `parsemap_block_map_data_dynamic` 0x1d1b68:53-64
  passes `pCelList[index]` (or the hashtable entry for by-hash placements), and create stores it at
  `obj+0xD8` [M 0x1cb4f8:73]; the entry's `+8` is the object's COLLDATA (the map block 0x2E
  collision model, `parsemap_block_Coll_Data_New` 0x1d1810 — the port's `MapObject::collision`);
- `obj+0xF0` = the celglist entry's `+0x44` flags word [M 0x1cb4f8:75];
- the record's first ushort (= the placement's param 0) is the part kind: 0 mount, 1 head
  (the controller), 2 spot, 3 beam, 4 corona; only kind-2 records get the psiLight node at
  record+0x28 [M 0x1cb4f8:125-164].

Census on this disc [D port parse]: head object `OBJECT_SearchLight1` (130 collision triangles),
mount `OBJECT_SLightMount` (138). Breakable searchlights ship on 07000005 (7 heads / 7 mounts) and
07000006 (4 / 4) among others; 07000006's courtyard pairs are the playtest set.

## 2. The collide pass: how a bullet contact reaches an object (U-041's list)

Per frame, `Collide_Update` 0x1ea398:

1. **Clears** every object's `obj+0xD0` hit list back into the LList pool 0x34b7b0 [:25-53], and
   resets each collider's `HITTEST_tag` (`obj+0xD4`: `+0x70` = the owner obj, `+0x9C` = 0).
2. Per collider: `Collide_Pick` 0x1eb320 → `Collide_PickObj` 0x1eae80 allocates a contact node per
   candidate into the collider's `obj+0xD0` (`+0x0C` = squared distance, `+0x58` = the candidate
   object, `+0x54` = 0). Candidate gate [:82-90]: `obj+0xF0 & mask == 0` (not disabled/corpse) and
   **`obj+0xD8 != 0` with `*(obj+0xD8)+8 != 0`** — an object with collision data; broadphase is the
   object's bounding sphere `obj+0x80`/`+0x8c` (or `Intersect_RayBox` against `obj+0x40`/`+0x50`
   when `obj+0x3c` carries 0x8000000).
3. `Collide_Intersect` 0x1ebaa8 per node: for a dynamic-object node it requires `obj+0xD8` [:84-90],
   copies the object's world matrix `obj+0x90` [:87], transforms the ray to the object's local
   frame, and for the bullet's type 0x201 runs `Intersect_RayGeom` 0x10a8c8 — the COLLDATA's AABB
   box tree (`ASH_Intersect_RayBox`/`ASH_RecurseBoxesRay`, 0x30-byte COLLBOX nodes) down to the
   0x40-byte triangles; writes the node's `+0x0C` distance, `+0x40` point, `+0x10` normal,
   `+0x50` material.
4. **Sorts** the surviving contacts by `+0x0C` (SrtList + QuickSort, [:118-158]; `CompHitData`
   0x1d33c0) and drops the 1e8 sentinels.
5. **Mirrors** each surviving contact into the HIT object's own `obj+0xD0` list [:160-234]: a fresh
   0x60-byte pool node with **`+0x08` = the collider's `HITTEST+0x90` damage**,
   `+0x58` = the collider (the bullet obj), `+0x51 |= 1`, and `+0x10/+0x30/+0x40/+0x50/+0x52`
   copied. `HITTEST+0x90` is `weapon_definition+0xC` — the weapon's damage — written once when
   `Bullet_update` 0x12e7b0 fills the bullet's embedded HITTEST (:391, :419-465).
   The walk **stops after the first non-pass-through contact** [:226-229] — the stopping contact
   itself still mirrors (the common case: the bullet dies on the lamp glass); nothing beyond it
   registers. `HITDATA+0x08`'s full reader/writer census lives in
   docs/fields/hitdata-spawner-ally.md §1.

So U-041's questions: **`+0x08` is the bullet's damage** (the weapon's `weapon_definition+0xC`),
and **the writer is `Collide_Update`'s mirror phase** 0x1ea398:160-234.

## 3. The searchlight side: what reads the list

`control_funcs` 0x26f890 [L, ee_snapshot.bin] — class 0x22's entry is
`update = Searchlight_Update` 0x1cb900, **collision handler = 0** (class 5 = the bullet:
update `Bullet_update` 0x12e7b0, handler `Bullet_CollisionHandler` 0x12ce80). So no per-contact
handler ever runs for a searchlight part; the ONLY consumer of a part's `obj+0xD0` list is
`Searchlight_Update` — and it returns immediately unless the record's kind is 1
[the kind-1 gate, 0x1cb900:88-90]. A contact mirrored into a mount's, spot's, beam's or corona's
list is written but never read: **only a hit on the HEAD's collision cels breaks the light.**

The break [M 0x1cb900, the walk LAB_001cc1d4-0x1cc2f8]: for each list entry whose `+0x58` is a
class-5 object (`*(entry+0x58)+0xFF == 5`) and whose `+0x08` (damage) is > 0.0, the head breaks:

- head state `obj+0xF4` = 2 (the droop);
- the psiLight nodes at record+0x28/+0x2c/+0x30 are deleted (`LList_Remove` pool 0x2c74cc,
  `psiLight_Delete`);
- `Control_NextLOD` (the broken visual);
- `Sound_Play3D` id 0xE6 at the head — the glass break.

(The walk runs in states 0/1/3 — a broken head's droop tail returns before it.)

## 4. The port

`searchlight.rs::Searchlights::bullet_check` implements the same contract per head per tick:

- the contact = the tick's bullet segments (the port's `bullet_paths`, `Collide_PickObj`'s
  counterpart) against the head part's own parsed collision triangles (`coll_tris`, map block 0x2E
  = the game's `obj+0xD8` cels) in the part's current matrix frame (the aim matrix = the game's
  `obj+0x90`);
- the mirror gates: the segment arrives truncated at the tick's stop (`bullet_path_stops`, the
  port's form of the mirror walk's stop at 0x1ea398:226-229) and the break needs the weapon's
  `weapon_definition+0xC` damage > 0 (read off `bullet_path_weapons`, = the node's `+0x08`);
- the break applies the game's effects: state 2, the linked parts hidden, the spot's light deleted,
  the 0xE6 sound.

Only the head is tested (the kind-1 gate); the pre-fix stand-in — a segment vs the head's AND
mount's placement gfx box — was replaced (it was also frame-mismatched for the sweeping parts: it
tested a current-frame segment against placement-frame box coords, so on 07000006's courtyard it
missed even registered housing shots while its mount chain could break lights the game leaves
alone).

## 5. Live checks for the main agent

- PCSX2, 07000006 courtyard: shoot a lamp housing square-on — the light breaks (state droop, glass
  0xE6, the spot goes out). Shoot the mount pole just below the housing — the light must NOT break
  (U-041's kind-1 gate).
- `--bond 4.8,42.1,-16.0,-2.2,0` / `7.1,48.6,-23.6,-1.4,0.2` (07000006 courtyard): after the break,
  the head parts hide and the spot's light disappears; nf-view draws the same.
