//! A level archive (`070000NN.bin`) assembled for viewing: every map entry's textures, objects and placements, and the
//! main map's placements resolved to objects. Spec: docs/formats/placement.md §Assembly.
//!
//! Resolution (*measured* on all maps, WORKLOG 0010): an index placement names an object (PS2_GFX + entity params, in
//! parse order) of the same map; a hashcode placement names an entity hashcode, which in the game is looked up in the
//! hash table filled by every map loaded so far — here, every map in the archive.

use crate::buf::Result;
use crate::levelbin::LevelBin;
use crate::map::{id, MapFile};
use crate::placement::{self, Placement};
use crate::ps2gfx::GfxObject;
use crate::texture::{self, Texture};
use std::collections::HashMap;

pub struct MapObject {
    pub hashcode: u32,
    pub name: String,
    /// Entity params payload +4: the glist flags (`celglist+0x44`, copied to `cel+0x94` by the static parser).
    pub flags: u32,
    /// Entity params +0x08 centre, +0x14 radius, +0x18 min, +0x24 max: the glist's bounds (`celglist+0x20` / `+0x30`,
    /// used e.g. by `Drone_AIVolume_Create` for the box size; *measured* on `AI_ShadowVolume`: centre (0, 1.5, 0),
    /// radius 2.598, min (−1.5, 0, −1.5), max (1.5, 3, 1.5)).
    pub centre: [f32; 3],
    pub radius: f32,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub gfx: GfxObject,
    /// Collision geometry in object space (block 0x2E before the object), if any.
    pub collision: Option<crate::collision::Collision>,
}

impl MapObject {
    /// `View_AddCels` (ACTION.ELF 0x1e6ba0) never adds a cel whose glist flags have bit 0x10 to the draw list:
    /// blocking polygons, and skies (which `View_DrawSky` draws separately, GAPS G-019).
    pub fn skipped_by_cel_pass(&self) -> bool { self.flags & 0x10 != 0 }
}

/// One map-block-0x21 portal record (`parsemap_block_portal_data` 0x1d15f8, docs/atlas/cel.md
/// §Portals): count = u32 at the payload start, records at +4, **stride 56** — four corner VECTORs
/// (+0x00..0x2f, the door/portal quad), +0x30 u16 `celA`, +0x32 u16 `celB` (indices into the level
/// parse's 0x50-byte pCelList record array — the main map's cel placements in placement order),
/// +0x34 u16 flags, +0x36 u16 pad. *Measured* [D]: 07000006's main-map block is 2468 bytes with count
/// 44 — (2468−4)/44 = 56.0 exactly; cel.md's earlier "stride 68" read is corrected here (its own
/// audit note flagged the number as re-measure-on-contact). The parser builds the forward plane from
/// the corners in order and the reversed plane from them backwards; the runtime node attached to
/// celB (dest celA) carries the forward plane with side flag `(flags & 2) == 0`, the node attached
/// to celA (dest celB) the reversed plane with side flag `(flags & 1) ^ 1` — both sides walkable
/// only when both flag bits are clear. *Measured* [D] on 07000006: every record's flags word is 0.
#[derive(Clone, Debug)]
pub struct PortalQuad {
    pub corners: [[f32; 3]; 4],
    pub cel_a: u16,
    pub cel_b: u16,
    pub flags: u16,
}

impl PortalQuad {
    pub fn parse_block(payload: &[u8]) -> crate::buf::Result<Vec<PortalQuad>> {
        let b = crate::buf::Buf(payload);
        let n = b.u32(0)? as usize;
        let mut out = Vec::new();
        for k in 0..n {
            let o = 4 + 56 * k;
            let mut corners = [[0.0f32; 3]; 4];
            for (c, corner) in corners.iter_mut().enumerate() {
                *corner = [b.f32(o + 12 * c)?, b.f32(o + 12 * c + 4)?, b.f32(o + 12 * c + 8)?];
            }
            out.push(PortalQuad {
                corners,
                cel_a: b.u16(o + 0x30)?,
                cel_b: b.u16(o + 0x32)?,
                flags: b.u16(o + 0x34)?,
            });
        }
        Ok(out)
    }
}

pub struct LevelMap {
    pub entry: usize,
    pub kind: u8,
    pub name: String,
    pub textures: Vec<Texture>,
    pub objects: Vec<MapObject>,
    pub placements: Vec<Placement>,
    /// AI paths and boundaries (block 0x05; main maps only on our disc).
    pub ai_network: Option<crate::aipath::AiNetwork>,
    /// Map ambient sounds (block 0x28).
    pub sounds: Vec<MapSound>,
    /// Key-list blocks (block 0x19), payloads in parse order. `parsemap_block_path_data` (0x1d24d0) stores one
    /// pointer per block into the global `m_paths` table (0x2ed828); a placement's record +0x44 indexes it (door
    /// anims, AI paths; docs/formats/map.md).
    pub path_data: Vec<Vec<u8>>,
    /// Portal records (block 0x21; main map). `parsemap_block_portal_data` 0x1d15f8.
    pub portals: Vec<PortalQuad>,
    /// LOD chains (block 0x27, `parsemap_block_LOD` 0x1d1f10): one first-dword per cel-list entry — the next
    /// (lower) LOD object's index, −1 = none. `Destroy_Create` 0x12c318 reads the same word off the placement's
    /// celglist (`celglist+0x40`) as the destroyed variant's glist, and `Destroy_Smash` 0x12bb60 swaps to it
    /// (`Control_SetGList` at 0x12bbc4, arg a1 = tag[0] proven by the disassembly 0x12bbb8 `lw a1,0(s1)`).
    /// Absent block → empty (every lookup None).
    pub lod: Vec<i32>,
    /// Ambient radiators (block 0x26; main map). `Light_SetAmbientRadiators` 0x1caeb8 creates one
    /// permanent kind-0 light per entry (fix-lighting).
    pub radiators: Vec<LightRadiator>,
}

/// One block-0x26 entry: 8 floats [x, y, z, radius², pad, r, g, b] with r/g/b 0-1
/// [M `Light_SetAmbientRadiators` 0x1caeb8: the vec4 passes x/y/z + radius² (SQRT at create),
/// the colour floats ×255 into the light's rgb bytes].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightRadiator {
    pub pos: [f32; 3],
    pub radius: f32,
    pub rgb: [f32; 3],
}

impl LightRadiator {
    pub fn parse_block(payload: &[u8]) -> crate::buf::Result<Vec<LightRadiator>> {
        let b = crate::buf::Buf(payload);
        let n = b.u32(0)? as usize;
        (0..n).map(|k| {
            let o = 4 + 32 * k;
            Ok(LightRadiator {
                pos: [b.f32(o)?, b.f32(o + 4)?, b.f32(o + 8)?],
                radius: b.f32(o + 12)?.sqrt(),
                rgb: [b.f32(o + 20)?, b.f32(o + 24)?, b.f32(o + 28)?],
            })
        }).collect()
    }
}

/// One object drawn somewhere: `(map, object)` indexes into `Level::maps`, and its row-vector world matrix.
/// A map ambient sound (block 0x28 → `Sound_LoadMapSounds` 0x1ddff8; docs/systems/sound.md "Map ambient sounds"): u32
/// count, then 32-byte records from +4: +4 sound id (flag 0x40000000), +8 position, +0x14 f32 (passed as the unused
/// pitch), +0x18 radius (start within it), +0x1C inner radius.
#[derive(Clone, Debug, PartialEq)]
pub struct MapSound { pub id: u32, pub pos: [f32; 3], pub param: f32, pub radius: f32, pub inner: f32 }

impl MapSound {
    pub fn parse_block(payload: &[u8]) -> crate::buf::Result<Vec<MapSound>> {
        let b = crate::buf::Buf(payload);
        let n = b.u32(0)? as usize;
        if n > 50 { return crate::buf::invalid(format!("{n} map sounds (the game holds 50)")); }
        (0..n).map(|k| {
            let o = 4 + 32 * k;
            Ok(MapSound { id: b.u32(o + 4)?, pos: [b.f32(o + 8)?, b.f32(o + 12)?, b.f32(o + 16)?], param: b.f32(o + 0x14)?, radius: b.f32(o + 0x18)?, inner: b.f32(o + 0x1c)? })
        }).collect()
    }
}

pub struct Instance {
    pub map: usize,
    pub object: usize,
    pub matrix: [[f32; 4]; 4],
    pub placement: Placement,
}

impl LevelMap {
    /// The object's LOD-chain target (`celglist+0x40`, block 0x27): `Some((this map, next))` when the entry is not
    /// −1 and in range. `Destroy_Smash` 0x12bb60 swaps to it; the engine's `Control_NextLOD` walks it.
    pub fn lod_target(&self, object: usize) -> Option<usize> {
        let next = *self.lod.get(object)?;
        (next >= 0 && (next as usize) < self.objects.len()).then_some(next as usize)
    }
}

pub struct Level {
    pub maps: Vec<LevelMap>,
    /// Character skins (type-3 entries, prefix 05) by hashcode, parsed with this archive's skeletons.
    pub skins: HashMap<u32, crate::character::Skin>,
    /// Entity hashcode → (map, object); first occurrence in archive order.
    pub by_hash: HashMap<u32, (usize, usize)>,
    /// Level scripts (type-7 entries, docs/formats/nis-script.md) by hashcode; ones `Script_Load` refuses are left out.
    pub nis: HashMap<u32, crate::nis::Nis>,
}

/// `AnimPostLoadInit` (ACTION.ELF 0x1be1a8): when skin 0x05000007 is loaded, every skin on skeleton 0 or 1 takes its
/// datums instead of its own, except 0x0500002F, which gets 0x05000007's followed by its own. Measured live in
/// 07000005: 0x0500002E and 0x05000030 point at 0x05000007's 60 datums; 0x0500002F has 64 (60 + its 4).
pub fn share_datums(skins: &mut HashMap<u32, crate::character::Skin>) {
    let Some(common) = skins.get(&0x0500_0007).map(|s| s.datums.clone()) else { return };
    for (&hash, skin) in skins.iter_mut() {
        if skin.skeleton >= 2 { continue; }
        skin.datums = if hash == 0x0500_002f {
            common.iter().cloned().chain(skin.datums.iter().cloned()).collect()
        } else {
            common.clone()
        };
    }
}

impl Level {
    pub fn parse(archive: &[u8]) -> Result<Level> {
        let lb = LevelBin::parse(archive)?;
        let mut maps = Vec::new();
        for (entry, e) in lb.entries.iter().enumerate().filter(|(_, e)| matches!(e.kind, 0 | 1 | 2 | 11)) {
            let data = lb.data(archive, e);
            let mf = MapFile::parse(data)?;
            let (textures, _short) = texture::textures(data, &mf)?;
            let mut objects = Vec::new();
            let mut pending = None;
            // A collision block (0x2E) comes before the PS2_GFX / entity params of the object it belongs to
            // (bounds match the following object in 6,959 of 7,985 blocks; docs/formats/collision.md).
            let mut pending_coll = None;
            let mut placements = Vec::new();
            let mut ai_network = None;
            let mut sounds = Vec::new();
            let mut path_data = Vec::new();
            let mut portals = Vec::new();
            let mut lod = Vec::new();
            let mut radiators = Vec::new();
            for blk in &mf.blocks {
                match blk.id {
                    0x2e => pending_coll = Some(crate::collision::Collision::parse(blk.payload(data))?),
                    id::PS2_GFX => pending = Some(GfxObject::parse(&data[blk.offset as usize..][..blk.size as usize])?),
                    id::ENTITY_PARAMS => {
                        let p = crate::buf::Buf(blk.payload(data));
                        let tail = p.tail(0x30, "entity name")?.bytes();
                        let name = String::from_utf8_lossy(&tail[..tail.iter().position(|&c| c == 0).unwrap_or(tail.len())]).into_owned();
                        let gfx = pending.take().ok_or_else(|| crate::buf::FormatError::Invalid("entity params without PS2_GFX".into()))?;
                        let v = |o: usize| -> crate::buf::Result<[f32; 3]> { Ok([p.f32(o)?, p.f32(o + 4)?, p.f32(o + 8)?]) };
                        objects.push(MapObject { hashcode: p.u32(0)?, name, flags: p.u32(4)?, centre: v(8)?, radius: p.f32(0x14)?, min: v(0x18)?, max: v(0x24)?, gfx, collision: pending_coll.take() });
                    }
                    id::MAP_DATA_STATIC => placements = placement::parse(blk.payload(data))?,
                    0x05 => ai_network = Some(crate::aipath::AiNetwork::parse(blk.payload(data))?),
                    0x28 => sounds = MapSound::parse_block(blk.payload(data))?,
                    0x19 => path_data.push(blk.payload(data).to_vec()),
                    id::PORTAL_DATA => portals.extend(PortalQuad::parse_block(blk.payload(data))?),
                    // `parsemap_block_LOD` 0x1d1f10: per cel entry an 8-byte record; the first dword is the next
                    // LOD object's index (−1 = none). The decomp reads the first dword of each pair
                    // (`iVar3 = *piVar5`, stride `piVar5 += 2`) and writes `cel+0x40 = index·0x50 + pCelList`.
                    id::LOD => lod = blk.payload(data).chunks_exact(8)
                        .map(|c| i32::from_le_bytes(c[..4].try_into().unwrap())).collect(),
                    0x26 => radiators = LightRadiator::parse_block(blk.payload(data))?,
                    _ => {}
                }
            }
            maps.push(LevelMap { entry, kind: e.kind, name: e.name.clone(), textures, objects, placements, ai_network, sounds, path_data, portals, lod, radiators });
        }
        let mut by_hash = HashMap::new();
        for (m, map) in maps.iter().enumerate() {
            for (o, obj) in map.objects.iter().enumerate() {
                if obj.hashcode != u32::MAX { by_hash.entry(obj.hashcode).or_insert((m, o)); }
            }
        }
        let skeletons: HashMap<u8, u8> = lb.entries.iter().filter(|e| e.kind == 6)
            .map(|e| crate::character::parse_skeleton(lb.data(archive, e)).map(|s| (s.id as u8, s.bones)))
            .collect::<Result<_>>()?;
        let mut skins = lb.entries.iter().filter(|e| e.kind == 3)
            .map(|e| crate::character::parse_skin(lb.data(archive, e), |id| skeletons.get(&id).copied()).map(|s| (s.hashcode, s)))
            .collect::<Result<_>>()?;
        share_datums(&mut skins);
        let nis = lb.entries.iter().filter(|e| e.kind == 7)
            .filter_map(|e| u32::from_str_radix(&e.name, 16).ok().and_then(|h| crate::nis::Nis::parse(h, lb.data(archive, e)).ok()).map(|n| (n.hashcode, n)))
            .collect();
        Ok(Level { maps, skins, by_hash, nis })
    }

    /// Characters: main-map placements whose parameter 0 is a skin hashcode (prefix 05) found in this archive (`oDrone`,
    /// animated `Tree`s), drawn as the skin's main meshes at the placement transform in bind pose. Sleeve entries
    /// (substituted at run time) and skins from other archives are skipped; the second value counts placements
    /// skipped for a missing skin.
    pub fn character_instances(&self) -> (Vec<Instance>, usize) {
        let Some(m) = self.main_map() else { return (Vec::new(), 0) };
        let (mut out, mut missing) = (Vec::new(), 0);
        for p in &self.maps[m].placements {
            let Some(&(_, skin)) = p.params.iter().find(|(i, _)| *i == 0) else { continue };
            if skin >> 24 != 5 { continue; }
            let Some(s) = self.skins.get(&skin) else { missing += 1; continue };
            for h in s.main.iter().flatten() {
                if let Some(&(map, object)) = self.by_hash.get(h) {
                    out.push(Instance { map, object, matrix: placement::object_matrix(p.rotation, p.translation), placement: p.clone() });
                }
            }
        }
        (out, missing)
    }

    /// Sky objects, as `parsemap_create_dynamic_objects` (0x1d04f0) case 0x2A creates them with `View_AddSkyObj`
    /// (0x1e79a0): main-map placements of object type 0x2A (placement flags & 0xFF); slot = parameter 0, sorted by slot
    /// (the order `View_DrawSky` 0x1e5ad0 draws them). Lightning variants (parameter 1 ≠ 0) are skipped. The bool is
    /// glist flag bit 2: drawn centred on the viewer instead of at the placement translation.
    /// Also the clouds (case 0xD4 → `Cloud_Create` 0x18d620, `View_SetDrawInNoViews` + `View_AddSkyObj` variant
    /// flag 0): the dispatcher passes the expanded param block as Cloud_Create's param 3 [M 001d1b68:93-97 call;
    /// 001d04f0 case 0xd4 passes param_2], so its slot word `*(u16*)(block+0x3C)` is **parameter 4** and the drift
    /// rates are params 0–3 [M 0018d620:20-28].
    pub fn sky_instances(&self) -> Vec<(Instance, bool)> {
        let Some(m) = self.main_map() else { return Vec::new() };
        let mut out: Vec<(u32, Instance, bool)> = Vec::new();
        for p in self.maps[m].placements.iter().filter(|p| matches!(p.flags & 0xff, 0x2a | 0xd4)) {
            let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
            // 0x2A's param 1 is the lightning-variant flag (View_AddSkyObj's param_6 byte); the cloud's
            // params 1/2 are its drift rates [M 0018d620:22-24], never a variant flag.
            if p.flags & 0xff == 0x2a && param(1) != 0 { continue; }
            // 0x2A: slot = param 0 [M 001d04f0 case 0x2a reads the param block at +0x2C]. 0xD4: slot = param 4
            // [M 0018d620:27 reads *(u16*)(block+0x3C) = param block offset 0x3C = param 4].
            let slot = if p.flags & 0xff == 0x2a { param(0) } else { param(4) };
            let target = if p.by_hash() { self.by_hash.get(&p.hashcode).copied() } else { Some((m, p.index as usize)) };
            let Some((map, object)) = target else { continue };
            let follows_camera = self.maps[map].objects[object].flags & 2 != 0;
            out.push((slot, Instance { map, object, matrix: placement::object_matrix(p.rotation, p.translation), placement: p.clone() }, follows_camera));
        }
        out.sort_by_key(|(slot, _, _)| *slot);
        out.into_iter().map(|(_, i, c)| (i, c)).collect()
    }

    /// The main map (loader type 1), if the archive has one.
    pub fn main_map(&self) -> Option<usize> { self.maps.iter().position(|m| m.kind == 1) }

    /// The main map's 0x19 key-list payloads, in block (parse) order — the game's `m_paths` table.
    /// `parsemap_block_path_data` is a main-map-only handler (docs/formats/map.md: 0x19 is "main map only"), so a
    /// door placement's record +0x44 indexes the MAIN map's 0x19 blocks. Verified against `work/ee_start_05.bin`'s
    /// DOORINFO+0xC on 07000005 (7 blocks, index 3 = `Door_UNLocked`'s 3 keys) and against the 5 doors on 07000001
    /// (their record +0x44 indices match the main map's blocks; sub-maps' 0x19 blocks would shift the indices and the
    /// doors would render at the wrong positions — the bug behind the user's "doors don't work" report on 07000001).
    pub fn path_blocks(&self) -> Vec<&[u8]> {
        self.main_map()
            .map(|m| self.maps[m].path_data.iter().map(Vec::as_slice).collect())
            .unwrap_or_default()
    }

    /// The main map's placements resolved to objects. `cels_only`: only records with the 0x8000 create-cel flag
    /// (what `parsemap_block_map_data_static` places itself); otherwise also the dynamic entities at their record
    /// transforms. Unresolvable records are skipped and counted in the second value.
    /// The static scene: `instances(cels_only)` plus, when `cels_only`, the breakables (placement type 0x20,
    /// `parsemap_create_dynamic_objects` 0x1d04f0 case 0x20 → `Break_Create`): vases, plates, glass panes. They carry
    /// no create-cel flag, so the cel pass skipped them and every breakable prop was missing. Drawn in their
    /// unbroken state (the port does not make them solid yet); breaking them is not ported yet.
    pub fn scene_instances(&self, cels_only: bool) -> (Vec<Instance>, usize) {
        let (mut out, missing) = self.instances(cels_only);
        if cels_only {
            // 0xE2 = `SS_Create` 0x1a9c58 ("SimpleScript"): a placed object (class 0x25) with its own glist, drawn
            // (`obj+0xFA |= 0x44`) and room-linked at load; it sits at its placement until a channel activates it
            // (`SS_Activate` 0x1a9ea8) and `SS_Update` 0x1a9948 plays its keys [M]. 07000006's barrel trap
            // (`Script_Barrelroll` / `Script_Barrelleg`) was missing. The game creates one only when its animation's
            // key count (`level_tag+2`) is non-zero; the port doesn't read that header, so every 0xE2 with a model draws [S].
            // The animated ones (key count non-zero — `SS_Create` 0x1a9c58 creates them and
            // `SS_Update` 0x1a9948 poses them every frame) are drawn by the ss pass instead
            // (map-tower2: the 07000012 lift screens/shutters/doors swap on their channel).
            out.extend(self.instances(false).0.into_iter().filter(|i| matches!(i.placement.flags & 0xff, 0x20 | 0xe2)
                && !i.placement.creates_cel()
                && !(i.placement.flags & 0xff == 0xe2 && i.placement.extra[7] & 0xffff != 0)));
        }
        (out, missing)
    }

    pub fn instances(&self, cels_only: bool) -> (Vec<Instance>, usize) {
        let Some(m) = self.main_map() else { return (Vec::new(), 0) };
        let mut out = Vec::new();
        let mut missing = 0;
        for p in &self.maps[m].placements {
            if cels_only && !p.creates_cel() { continue; }
            let target = if p.by_hash() { self.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < self.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
            match target {
                Some((map, object)) => out.push(Instance { map, object, matrix: placement::object_matrix(p.rotation, p.translation), placement: p.clone() }),
                None => missing += 1,
            }
        }
        (out, missing)
    }
}
