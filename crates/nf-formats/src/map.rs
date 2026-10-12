//! Map entries (level `.bin` types 0/1/2/11, hashcode prefix 01): a block file. Spec: docs/formats/map.md.
//!
//! ```text
//! 0x00  u32 version                    (1 in all 2,230 map entries on our disc)
//! 0x04  u32 offset[] (version 1)       block k starts at 4 + offset[k]; the list ends with the block whose id is 0x1D
//! block: u32 header = id << 24 | size  (size includes the 4-byte header)
//! ```
//! Mirrors `parsemap_parsenextblock` (ACTION.ELF 0x1d12e8): version 1 locates each block through the offset table, in
//! table order, which is not file order. Version ≠ 1 (a plain size-chained sequence starting at 4) is implemented as
//! the decomp describes but does not occur on our disc.

use crate::buf::{invalid, Buf, Result};

pub const END: u8 = 0x1d;

/// Block ids handled by `parsemap_handle_block_id` (0x1d1020) that occur on our disc (see docs/formats/map.md).
pub mod id {
    pub const ENTITY_PARAMS: u8 = 0x04;
    pub const AI_PATH: u8 = 0x05;
    pub const MAP_HEADER: u8 = 0x0e;
    pub const PALETTE_HEADER: u8 = 0x0f;
    pub const TEXTURE_HEADER: u8 = 0x10;
    pub const PALETTE_DATA_PSX: u8 = 0x12;
    pub const TEXTURE_DATA: u8 = 0x16;
    pub const PATH_DATA: u8 = 0x19;
    pub const MAP_DATA_STATIC: u8 = 0x1a;
    pub const MEMORY_DISCARD: u8 = 0x1c;
    pub const END: u8 = 0x1d;
    /// The second entity-params handler id: `parsemap_handle_block_id` 0x1d1020 case 4 AND 0x1f both call
    /// `parsemap_block_entity_params`. (No level bin on the disc carries a 0x1f block [D block dumps];
    /// models-world's `LOD = 0x1f` misnamed it — the game's `parsemap_block_LOD` is dispatch case **0x27**,
    /// and every 0x27 payload parses as clean s32 object indices, mostly −1 [D 0700000d].)
    pub const ENTITY_PARAMS_2: u8 = 0x1f;
    pub const PORTAL_DATA: u8 = 0x21;
    pub const AMBIENT_RADIATORS: u8 = 0x26;
    /// LOD chains (`parsemap_block_LOD` 0x1d1f10, dispatch 0x1d1020 case 0x27): 8-byte records, one per cel-list
    /// entry; the first dword is the next (lower) LOD object's index, −1 = none. `Destroy_Create` reads the same
    /// word (`celglist+0x40`) as the destroyed variant's glist (`Destroy_Smash` 0x12bb60 swaps to it).
    pub const LOD: u8 = 0x27;
    pub const MAP_SOUNDS: u8 = 0x28;
    pub const MORPH_DATA: u8 = 0x2b;
    pub const HASHLIST: u8 = 0x2c;
    pub const PS2_GFX: u8 = 0x2d;
    pub const COLL_DATA_NEW: u8 = 0x2e;
    pub const PARTICLES: u8 = 0x30;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    pub id: u8,
    /// Offset of the block header within the map entry.
    pub offset: u32,
    /// Size including the 4-byte header.
    pub size: u32,
}

impl Block {
    pub fn payload<'a>(&self, map: &'a [u8]) -> &'a [u8] { &map[self.offset as usize + 4..][..self.size as usize - 4] }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapFile {
    pub version: u32,
    /// In parse order (table order for version 1), ending with the END block.
    pub blocks: Vec<Block>,
}

impl MapFile {
    pub fn parse(data: &[u8]) -> Result<MapFile> {
        let b = Buf(data);
        let version = b.u32(0)?;
        let mut blocks = Vec::new();
        let mut k = 1usize;
        let mut chained = 4usize;
        loop {
            if blocks.len() > 1 << 16 { return invalid("map: no END block within 65,536 blocks"); }
            let offset = if version == 1 { b.u32(4 * k)? as usize + 4 } else { chained };
            let header = b.u32(offset)?;
            let (id, size) = ((header >> 24) as u8, header & 0xff_ffff);
            if size < 4 { return invalid(format!("map: block {:#x} at {offset:#x} has size {size}", id)); }
            b.check(offset, size as usize, "map block")?;
            blocks.push(Block { id, offset: offset as u32, size });
            if id == END { break; }
            k += 1;
            chained = offset + size as usize;
        }
        Ok(MapFile { version, blocks })
    }

    pub fn first(&self, id: u8) -> Option<&Block> { self.blocks.iter().find(|b| b.id == id) }
    pub fn all(&self, id: u8) -> impl Iterator<Item = &Block> { self.blocks.iter().filter(move |b| b.id == id) }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A version-1 map: blocks laid out in `file_order`, listed in the table in the given (parse) order.
    pub(crate) fn mini_map(blocks: &[(u8, &[u8])]) -> Vec<u8> {
        let table_len = 4 + 4 * blocks.len();
        let mut out = vec![0u8; table_len];
        out[..4].copy_from_slice(&1u32.to_le_bytes());
        let mut offs = Vec::new();
        // Store blocks in reverse so file order differs from table order.
        for (id, payload) in blocks.iter().rev() {
            let at = out.len();
            out.extend((((*id as u32) << 24) | (payload.len() as u32 + 4)).to_le_bytes());
            out.extend(*payload);
            offs.push(at);
        }
        offs.reverse();
        for (k, at) in offs.iter().enumerate() { out[4 + 4 * k..8 + 4 * k].copy_from_slice(&((*at - 4) as u32).to_le_bytes()); }
        out
    }

    #[test]
    fn parses_a_version_1_table_in_table_order() {
        let m = mini_map(&[(id::MAP_HEADER, &[1, 2, 3, 4]), (id::TEXTURE_HEADER, &[9; 12]), (END, &[])]);
        let f = MapFile::parse(&m).unwrap();
        assert_eq!(f.version, 1);
        assert_eq!(f.blocks.iter().map(|b| b.id).collect::<Vec<_>>(), [id::MAP_HEADER, id::TEXTURE_HEADER, END]);
        assert!(f.blocks[0].offset > f.blocks[1].offset, "table order is not file order");
        assert_eq!(f.first(id::MAP_HEADER).unwrap().payload(&m), [1, 2, 3, 4]);
    }

    #[test]
    fn rejects_missing_end_and_overruns() {
        let m = mini_map(&[(id::MAP_HEADER, &[1, 2, 3, 4])]);
        assert!(MapFile::parse(&m).is_err(), "no END block: the table runs into block data");
        let mut m = mini_map(&[(id::MAP_HEADER, &[0; 4]), (END, &[])]);
        let at = u32::from_le_bytes(m[4..8].try_into().unwrap()) as usize + 4;
        m[at..at + 4].copy_from_slice(&((id::MAP_HEADER as u32) << 24 | 0xff_ffff).to_le_bytes());
        assert!(MapFile::parse(&m).is_err());
    }
}
