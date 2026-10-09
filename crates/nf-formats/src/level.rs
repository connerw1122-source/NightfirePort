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
    pub gfx: GfxObject,
}

impl MapObject {
    /// `View_AddCels` (ACTION.ELF 0x1e6ba0) never adds a cel whose glist flags have bit 0x10 to the draw list:
    /// blocking polygons, and skies (which `View_DrawSky` draws separately, GAPS G-019).
    pub fn skipped_by_cel_pass(&self) -> bool { self.flags & 0x10 != 0 }
}

pub struct LevelMap {
    pub entry: usize,
    pub kind: u8,
    pub name: String,
    pub textures: Vec<Texture>,
    pub objects: Vec<MapObject>,
    pub placements: Vec<Placement>,
}

/// One object drawn somewhere: `(map, object)` indexes into `Level::maps`, and its row-vector world matrix.
pub struct Instance {
    pub map: usize,
    pub object: usize,
    pub matrix: [[f32; 4]; 4],
    pub placement: Placement,
}

pub struct Level {
    pub maps: Vec<LevelMap>,
    /// Character skins (type-3 entries, prefix 05) by hashcode, parsed with this archive's skeletons.
    pub skins: HashMap<u32, crate::character::Skin>,
    /// Entity hashcode → (map, object); first occurrence in archive order.
    pub by_hash: HashMap<u32, (usize, usize)>,
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
            let mut placements = Vec::new();
            for blk in &mf.blocks {
                match blk.id {
                    id::PS2_GFX => pending = Some(GfxObject::parse(&data[blk.offset as usize..][..blk.size as usize])?),
                    id::ENTITY_PARAMS => {
                        let p = crate::buf::Buf(blk.payload(data));
                        let tail = p.tail(0x30, "entity name")?.bytes();
                        let name = String::from_utf8_lossy(&tail[..tail.iter().position(|&c| c == 0).unwrap_or(tail.len())]).into_owned();
                        let gfx = pending.take().ok_or_else(|| crate::buf::FormatError::Invalid("entity params without PS2_GFX".into()))?;
                        objects.push(MapObject { hashcode: p.u32(0)?, name, flags: p.u32(4)?, gfx });
                    }
                    id::MAP_DATA_STATIC => placements = placement::parse(blk.payload(data))?,
                    _ => {}
                }
            }
            maps.push(LevelMap { entry, kind: e.kind, name: e.name.clone(), textures, objects, placements });
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
        let skins = lb.entries.iter().filter(|e| e.kind == 3)
            .map(|e| crate::character::parse_skin(lb.data(archive, e), |id| skeletons.get(&id).copied()).map(|s| (s.hashcode, s)))
            .collect::<Result<_>>()?;
        Ok(Level { maps, skins, by_hash })
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

    /// The main map (loader type 1), if the archive has one.
    pub fn main_map(&self) -> Option<usize> { self.maps.iter().position(|m| m.kind == 1) }

    /// The main map's placements resolved to objects. `cels_only`: only records with the 0x8000 create-cel flag
    /// (what `parsemap_block_map_data_static` places itself); otherwise also the dynamic entities at their record
    /// transforms. Unresolvable records are skipped and counted in the second value.
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
