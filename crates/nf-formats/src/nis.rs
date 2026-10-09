//! Level scripts (archive entry type 7, `Script_Load` 0x1d8248): the timelines ScriptPlayers run — cut-scenes, the
//! wine truck, helicopters. Spec: docs/formats/nis-script.md.
//!
//! ```text
//! u16 magic 0x1C, u16 camera flag (≠ 0 → SCRIPTINFO+0xAA8 bit 0x40), u16 stream count (≤ 0x3E), u16 loop start,
//! u16 length (ticks)
//! per stream: u16 first key, u16 last key, u32 size, size − 4 command bytes
//! u32 key count, u32 n, n bytes (skipped), key count × 0x30-byte keys
//! ```
//! A key: vec4 position (+0x00), quaternion (+0x10), f32 scale (+0x20), f32 at +0x24 (also read as bytes: colour),
//! f32 +0x28, f32 time (+0x2C) — *measured* from `Script_GetInterp` (0x1da158).

use crate::buf::{invalid, Buf, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub pos: [f32; 4],
    pub quat: [f32; 4],
    pub scale: f32,
    /// +0x24: a float (`*(f32*)(key+0x24)` → `fptoui(v·255 + 0.5)`) or, for light streams, three colour bytes.
    pub w24: [u8; 4],
    pub w28: f32,
    pub time: f32,
}

impl Key {
    pub fn f24(&self) -> f32 { f32::from_le_bytes(self.w24) }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stream {
    pub first_key: u16,
    pub last_key: u16,
    pub commands: Vec<u8>,
}

/// A decoded stream command (docs/formats/nis-script.md); offsets as `Script_Run` reads them.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    End(u16),
    Wait(u16),
    Marker(u16),
    Entity { hash: u32, parent: Option<u8>, spline: bool, obj_type: u8, params: Vec<u32> },
    EntityEnd,
    Anim { script: u32, skin: u32, start: u32 },
    AnimEnd,
    Event { kind: u8, params: Vec<u32> },
    Sound { id: u32, flat: bool, has_obj: bool, stream: u8 },
    SoundEnd,
    /// A command whose handler is not read yet (its length is unknown: decoding stops).
    Unread(u8),
    /// `Script_Run`'s default: a one-byte command.
    Other(u8),
}

impl Stream {
    /// The commands in order, up to the first unread one.
    pub fn decode(&self) -> Vec<Cmd> {
        let c = &self.commands;
        let rd = |o: usize, n: usize| -> Option<u32> { c.get(o..o + n).map(|b| b.iter().rev().fold(0u32, |a, &x| a << 8 | x as u32)) };
        let mut out = Vec::new();
        let mut p = 0;
        while let Some(&op) = c.get(p) {
            let (cmd, len) = match op {
                4 | 5 | 6 => {
                    let Some(t) = rd(p + 1, 2) else { break };
                    (match op { 4 => Cmd::End(t as u16), 5 => Cmd::Wait(t as u16), _ => Cmd::Marker(t as u16) }, 4)
                }
                7 => {
                    let (Some(hash), Some(h)) = (rd(p + 1, 4), c.get(p + 5..p + 10)) else { break };
                    let n = h[4] as usize;
                    let params: Vec<u32> = (0..n).filter_map(|k| rd(p + 10 + 4 * k, 4)).collect();
                    if params.len() < n { break; }
                    (Cmd::Entity { hash, parent: (h[0] != 0).then_some(h[1]), spline: h[2] != 0, obj_type: h[3], params }, 11 + 4 * n)
                }
                9 => (Cmd::EntityEnd, 2),
                0x0a => {
                    let (Some(script), Some(skin), Some(start)) = (rd(p + 1, 4), rd(p + 5, 4), rd(p + 9, 4)) else { break };
                    (Cmd::Anim { script, skin, start }, 14)
                }
                0x0c => (Cmd::AnimEnd, 2),
                0x12 => {
                    let (Some(&n), Some(&kind)) = (c.get(p + 1), c.get(p + 2)) else { break };
                    let params: Vec<u32> = (0..n as usize).filter_map(|k| rd(p + 3 + 4 * k, 4)).collect();
                    if params.len() < n as usize { break; }
                    (Cmd::Event { kind, params }, 4 + 4 * n as usize)
                }
                0x16 => {
                    let (Some(id), Some(b)) = (rd(p + 1, 4), c.get(p + 5..p + 8)) else { break };
                    (Cmd::Sound { id, flat: b[0] == 1, has_obj: b[1] != 0, stream: b[2] }, 9)
                }
                0x17 => (Cmd::SoundEnd, 2),
                0x0d | 0x0f | 0x13 | 0x14 | 0x15 | 0x18 | 0x1a | 0x1b | 0x1d | 0x1e => { out.push(Cmd::Unread(op)); break; }
                _ => (Cmd::Other(op), 1),
            };
            out.push(cmd);
            p += len;
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Nis {
    pub hashcode: u32,
    pub camera: bool,
    pub loop_start: u16,
    pub length: u16,
    pub streams: Vec<Stream>,
    pub keys: Vec<Key>,
}

impl Nis {
    pub fn parse(hashcode: u32, data: &[u8]) -> Result<Nis> {
        let b = Buf(data);
        if b.u16(0)? != 0x1c { return invalid("script magic is not 0x1C"); }
        let camera = b.u16(2)? != 0;
        let n = b.u16(4)? as usize;
        if n > 0x3e { return invalid("more than 62 script streams"); }
        let (loop_start, length) = (b.u16(6)?, b.u16(8)?);
        let mut p = 10;
        let mut streams = Vec::with_capacity(n);
        for _ in 0..n {
            let (first_key, last_key, size) = (b.u16(p)?, b.u16(p + 2)?, b.u32(p + 4)? as usize);
            p += 8;
            let Some(len) = size.checked_sub(4) else { return invalid("stream size below 4") };
            streams.push(Stream { first_key, last_key, commands: b.sub(p, len, "script stream")?.bytes().to_vec() });
            p += len;
        }
        let count = b.u32(p)? as usize;
        let skip = b.u32(p + 4)? as usize;
        p += 8 + skip;
        let mut keys = Vec::with_capacity(count);
        for k in 0..count {
            let q = p + k * 0x30;
            let f = |o: usize| b.f32(q + o);
            keys.push(Key {
                pos: [f(0)?, f(4)?, f(8)?, f(12)?], quat: [f(0x10)?, f(0x14)?, f(0x18)?, f(0x1c)?], scale: f(0x20)?,
                w24: b.sub(q + 0x24, 4, "script key")?.bytes().try_into().unwrap(), w28: f(0x28)?, time: f(0x2c)?,
            });
        }
        Ok(Nis { hashcode, camera, loop_start, length, streams, keys })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_streams_and_keys() {
        let mut v: Vec<u8> = Vec::new();
        for w in [0x1cu16, 1, 1, 5, 100] { v.extend(w.to_le_bytes()); }
        v.extend(0u16.to_le_bytes()); v.extend(1u16.to_le_bytes()); v.extend(8u32.to_le_bytes());
        v.extend([5, 10, 0, 6]);
        v.extend(2u32.to_le_bytes()); v.extend(3u32.to_le_bytes()); v.extend([9, 9, 9]);
        for k in 0..2 { for i in 0..12 { v.extend(((k * 100 + i) as f32).to_le_bytes()); } }
        let s = Nis::parse(0x0600_0001, &v).unwrap();
        assert_eq!((s.camera, s.loop_start, s.length, s.streams.len()), (true, 5, 100, 1));
        assert_eq!(s.streams[0], Stream { first_key: 0, last_key: 1, commands: vec![5, 10, 0, 6] });
        assert_eq!((s.keys.len(), s.keys[1].pos[0], s.keys[1].time, s.keys[0].f24()), (2, 100.0, 111.0, 9.0));
        assert_eq!(s.streams[0].decode(), vec![Cmd::Wait(10)]);
        // Sister: a wrong magic is refused.
        v[0] = 0x1d;
        assert!(Nis::parse(0, &v).is_err());
    }
}
