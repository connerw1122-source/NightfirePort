//! AI paths and boundaries (map block 0x05, `AIPath_Parse` 0x1d2598, `AIPath_Prepare` 0x1d2cc0). Spec:
//! docs/formats/ai-path.md.
//!
//! Payload (after the block's id/size word): u32 version (8; any other value and the game ignores the block), u32
//! record count, then the records. Record: name `[u8; 0x80]`, `+0x80 u32` kind (bit 0 = boundary), `+0x84..+0xA0`
//! seven u32 parameters plus one at +0xA0, `+0xA4 u32` node count, `+0xA8 u32` length of an extra byte area, the
//! area, then the nodes (0x40 bytes each for a path, 0x20 for a boundary), a u32 link count and the links (16 bytes
//! for a path, 20 for a boundary). A node's position is at +0x10; the game adds 0.4 to its y at load.

use crate::buf::{invalid, Buf, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct PathNode {
    /// Position as on disc (the game raises y by 0.4 at load: [`PathNode::game_pos`]).
    pub pos: [f32; 3],
    /// Disc words +0x04 and +0x1C..+0x3C (the in-memory node keeps +4; the rest are load-time slots).
    pub raw: Vec<u32>,
}

impl PathNode {
    /// `AIPath_Parse`: `node+0x14 += 0.4` (PS2 single-precision add; exact for these magnitudes up to rounding mode).
    pub fn game_pos(&self) -> [f32; 3] { [self.pos[0], crate::ps2float::add(self.pos[1], 0.4), self.pos[2]] }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathLink {
    pub a: u16,
    pub b: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiPath {
    pub name: String,
    /// `+0x80`: bit 0 = boundary (`AIBounds_*`); paths seen: 2 (nav), 4 (patrol).
    pub kind: u32,
    /// `+0x84..=+0xA0` (eight u32, copied into the path record unchanged).
    pub params: [u32; 8],
    /// The `+0xA8`-byte area before the nodes.
    pub extra: Vec<u8>,
    pub nodes: Vec<PathNode>,
    pub links: Vec<PathLink>,
}

impl AiPath {
    pub fn is_boundary(&self) -> bool { self.kind & 1 != 0 }

    /// `AIPath_Prepare`: the links touching each node, in link order (the game's per-node adjacency list).
    pub fn adjacency(&self) -> Vec<Vec<u16>> {
        let mut adj = vec![Vec::new(); self.nodes.len()];
        for (k, l) in self.links.iter().enumerate() {
            if let Some(v) = adj.get_mut(l.a as usize) { v.push(k as u16); }
            if l.b != l.a {
                if let Some(v) = adj.get_mut(l.b as usize) { v.push(k as u16); }
            }
        }
        adj
    }
}

#[derive(Clone, Debug, Default)]
pub struct AiNetwork {
    pub paths: Vec<AiPath>,
}

impl AiNetwork {
    pub fn parse(payload: &[u8]) -> Result<AiNetwork> {
        let b = Buf(payload);
        if b.u32(0)? != 8 { return invalid(format!("AI path version {}", b.u32(0)?)); }
        let count = b.u32(4)? as usize;
        let mut o = 8;
        let mut paths = Vec::with_capacity(count);
        for _ in 0..count {
            let name_bytes = b.sub(o, 0x80, "AI path name")?.bytes();
            let name = String::from_utf8_lossy(&name_bytes[..name_bytes.iter().position(|&c| c == 0).unwrap_or(0x80)]).into_owned();
            let kind = b.u32(o + 0x80)?;
            let mut params = [0; 8];
            for (k, p) in params.iter_mut().enumerate() { *p = b.u32(o + 0x84 + 4 * k)?; }
            let nodes_n = b.u32(o + 0xa4)? as usize;
            let extra_len = b.u32(o + 0xa8)? as usize;
            let extra = b.sub(o + 0xac, extra_len, "AI path extra")?.bytes().to_vec();
            let boundary = kind & 1 != 0;
            let (node_size, link_size) = if boundary { (0x20, 20) } else { (0x40, 16) };
            let mut q = o + 0xac + extra_len;
            b.check(q, nodes_n * node_size, "AI path nodes")?;
            let mut nodes = Vec::with_capacity(nodes_n);
            for _ in 0..nodes_n {
                let raw = std::iter::once(4).chain((0x1c..node_size).step_by(4)).map(|i| b.u32(q + i)).collect::<Result<_>>()?;
                nodes.push(PathNode { pos: [b.f32(q + 0x10)?, b.f32(q + 0x14)?, b.f32(q + 0x18)?], raw });
                q += node_size;
            }
            let links_n = b.u32(q)? as usize;
            q += 4;
            b.check(q, links_n * link_size, "AI path links")?;
            let links = (0..links_n).map(|k| Ok(PathLink { a: b.u16(q + k * link_size + 8)?, b: b.u16(q + k * link_size + 10)? })).collect::<Result<_>>()?;
            q += links_n * link_size;
            paths.push(AiPath { name, kind, params, extra, nodes, links });
            o = q;
        }
        if o != payload.len() { return invalid(format!("AI path block: {} of {} bytes used", o, payload.len())); }
        Ok(AiNetwork { paths })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-path block built by hand: two nodes, one link; must use every byte.
    #[test]
    fn parses_a_minimal_path() {
        let mut d = vec![];
        d.extend(8u32.to_le_bytes());
        d.extend(1u32.to_le_bytes());
        let mut name = [0u8; 0x80];
        name[..4].copy_from_slice(b"Test");
        d.extend(name);
        d.extend(2u32.to_le_bytes());
        d.extend([0u8; 0x20]);
        d.extend(2u32.to_le_bytes()); // nodes
        d.extend(2u32.to_le_bytes()); // extra
        d.extend([7, 9]);
        for x in [1.0f32, 5.0] {
            let mut n = [0u8; 0x40];
            n[0x10..0x14].copy_from_slice(&x.to_le_bytes());
            d.extend(n);
        }
        d.extend(1u32.to_le_bytes());
        d.extend([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
        let net = AiNetwork::parse(&d).unwrap();
        let p = &net.paths[0];
        assert_eq!((p.name.as_str(), p.nodes.len(), p.extra.as_slice()), ("Test", 2, &[7u8, 9][..]));
        assert_eq!(p.links, vec![PathLink { a: 0, b: 1 }]);
        assert_eq!(p.adjacency(), vec![vec![0], vec![0]]);
        // A truncated block fails.
        assert!(AiNetwork::parse(&d[..d.len() - 1]).is_err());
    }
}
