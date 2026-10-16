//! Wire protocol: packet types, layouts, encode/decode.
//!
//! Every packet starts with a 4-byte header: `MAGIC` u16 (`0x4E46`), packet
//! type u8, reserved u8 (0). All integers little-endian. No packet exceeds
//! [`MAX_PACKET`] bytes; anything longer, with a bad magic or an unknown type
//! is rejected by [`decode`]. Layouts are tagged in `docs/systems/net-lockstep.md`;
//! the protocol itself is a port feature [S] (the PS2 game has no network mode).
//!
//! | type | name | payload |
//! |------|------|---------|
//! | 1 | HELLO (client→host) | proto u16, build u32, codename 32B |
//! | 2 | WELCOME (host→client) | proto u16, build u32, codename 32B, slot u8, input_delay u8, hash_every u8, rsv u8, seed u32, settings 176B |
//! | 3 | REFUSE (host→client) | reason u8, rsv 3B |
//! | 4 | INPUT | base_tick u32, agent_mask u8, count u8, ack_tick u32, ack_bits u32, count*popcount(mask)*24B inputs |
//! | 5 | HASH | tick u32, hash u64, prev_tick u32, prev_hash u64 |
//! | 6 | KEEPALIVE | seq u32 |
//! | 7 | LEAVE | slot u8, reason u8, rsv 2B |
//! | 8 | SLOTMAP | human_slots bitmask u8, rsv 3B |
//! | 9 | GO (host→client) | first_tick u32 |
//!
//! INPUT `agent_mask` bit i = slot i has an input in each tick run; inputs
//! are stored per tick in ascending slot order. The ack fields acknowledge
//! ticks received from that peer (see session module): `ack_tick` = highest
//! tick T with all ticks in `[start..=T]` received; bit i of `ack_bits` =
//! tick `T+1+i` received.

use crate::input::{Input, INPUT_WIRE_SIZE};
use crate::settings::{MatchSettings, SETTINGS_WIRE_SIZE};
use crate::{MAGIC, MAX_PACKET, PROTOCOL_VERSION};

pub const PT_HELLO: u8 = 1;
pub const PT_WELCOME: u8 = 2;
pub const PT_REFUSE: u8 = 3;
pub const PT_INPUT: u8 = 4;
pub const PT_HASH: u8 = 5;
pub const PT_KEEPALIVE: u8 = 6;
pub const PT_LEAVE: u8 = 7;
pub const PT_SLOTMAP: u8 = 8;
pub const PT_GO: u8 = 9;

/// codename field width on the wire (nul-padded).
pub const CODENAME_WIRE: usize = 32;
/// How many ticks one INPUT packet may carry client→host (24B each).
pub const MAX_INPUT_COUNT_C2H: usize = 32;
/// How many ticks one INPUT packet may carry host→client (3 agents × 24B each).
pub const MAX_INPUT_COUNT_H2C: usize = 8;

/// A decoded packet. Sizes are exact; `inputs` holds
/// `count * popcount(agent_mask)` entries in tick-major, slot-ascending order.
#[derive(Clone, Debug, PartialEq)]
pub enum Packet {
    Hello {
        proto: u16,
        build: u32,
        codename: String,
    },
    Welcome {
        proto: u16,
        build: u32,
        codename: String,
        slot: u8,
        input_delay: u8,
        hash_every: u8,
        seed: u32,
        settings: MatchSettings,
    },
    Refuse {
        reason: u8,
    },
    Input {
        base_tick: u32,
        agent_mask: u8,
        ack_tick: u32,
        ack_bits: u32,
        inputs: Vec<Input>,
    },
    Hash {
        tick: u32,
        hash: u64,
        prev_tick: u32,
        prev_hash: u64,
    },
    Keepalive {
        seq: u32,
    },
    Leave {
        slot: u8,
        reason: u8,
    },
    SlotMap {
        human_slots: u8,
    },
    Go {
        first_tick: u32,
    },
}

fn put_codename(out: &mut Vec<u8>, name: &str) {
    let mut buf = [0u8; CODENAME_WIRE];
    let bytes = name.as_bytes();
    let n = bytes.len().min(CODENAME_WIRE - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    out.extend_from_slice(&buf);
}

fn get_codename(buf: &[u8], off: usize) -> Option<String> {
    let raw = buf.get(off..off + CODENAME_WIRE)?;
    let end = raw.iter().position(|&b| b == 0).unwrap_or(CODENAME_WIRE);
    Some(String::from_utf8_lossy(&raw[..end]).into_owned())
}

/// Encode a packet to bytes (little-endian, header first).
pub fn encode(p: &Packet, out: &mut Vec<u8>) {
    out.clear();
    out.reserve(64);
    out.extend_from_slice(&MAGIC.to_le_bytes());
    match p {
        Packet::Hello { proto, build, codename } => {
            out.push(PT_HELLO);
            out.push(0);
            out.extend_from_slice(&proto.to_le_bytes());
            out.extend_from_slice(&build.to_le_bytes());
            put_codename(out, codename);
        }
        Packet::Welcome { proto, build, codename, slot, input_delay, hash_every, seed, settings } => {
            out.push(PT_WELCOME);
            out.push(0);
            out.extend_from_slice(&proto.to_le_bytes());
            out.extend_from_slice(&build.to_le_bytes());
            put_codename(out, codename);
            out.push(*slot);
            out.push(*input_delay);
            out.push(*hash_every);
            out.push(0);
            out.extend_from_slice(&seed.to_le_bytes());
            let mut sb = [0u8; SETTINGS_WIRE_SIZE];
            settings.encode(&mut sb);
            out.extend_from_slice(&sb);
        }
        Packet::Refuse { reason } => {
            out.push(PT_REFUSE);
            out.push(0);
            out.push(*reason);
            out.extend_from_slice(&[0, 0, 0]);
        }
        Packet::Input { base_tick, agent_mask, ack_tick, ack_bits, inputs } => {
            out.push(PT_INPUT);
            out.push(0);
            out.extend_from_slice(&base_tick.to_le_bytes());
            out.push(*agent_mask);
            let per_tick = agent_mask.count_ones().max(1) as usize;
            out.push((inputs.len() / per_tick) as u8);
            out.extend_from_slice(&ack_tick.to_le_bytes());
            out.extend_from_slice(&ack_bits.to_le_bytes());
            for inp in inputs {
                let mut b = [0u8; INPUT_WIRE_SIZE];
                inp.encode(&mut b, 0);
                out.extend_from_slice(&b);
            }
        }
        Packet::Hash { tick, hash, prev_tick, prev_hash } => {
            out.push(PT_HASH);
            out.push(0);
            out.extend_from_slice(&tick.to_le_bytes());
            out.extend_from_slice(&hash.to_le_bytes());
            out.extend_from_slice(&prev_tick.to_le_bytes());
            out.extend_from_slice(&prev_hash.to_le_bytes());
        }
        Packet::Keepalive { seq } => {
            out.push(PT_KEEPALIVE);
            out.push(0);
            out.extend_from_slice(&seq.to_le_bytes());
        }
        Packet::Leave { slot, reason } => {
            out.push(PT_LEAVE);
            out.push(0);
            out.push(*slot);
            out.push(*reason);
            out.extend_from_slice(&[0, 0]);
        }
        Packet::SlotMap { human_slots } => {
            out.push(PT_SLOTMAP);
            out.push(0);
            out.push(*human_slots);
            out.extend_from_slice(&[0, 0, 0]);
        }
        Packet::Go { first_tick } => {
            out.push(PT_GO);
            out.push(0);
            out.extend_from_slice(&first_tick.to_le_bytes());
        }
    }
}

/// Decode bytes into a [`Packet`]; `None` if malformed (bad magic, unknown
/// type, truncated payload, over-long, inconsistent INPUT count/mask).
pub fn decode(buf: &[u8]) -> Option<Packet> {
    if buf.len() < 4 || buf.len() > MAX_PACKET {
        return None;
    }
    if u16::from_le_bytes([buf[0], buf[1]]) != MAGIC {
        return None;
    }
    let ptype = buf[2];
    let body = &buf[4..];
    match ptype {
        PT_HELLO => {
            if body.len() < 6 + CODENAME_WIRE {
                return None;
            }
            Some(Packet::Hello {
                proto: u16::from_le_bytes([body[0], body[1]]),
                build: u32::from_le_bytes([body[2], body[3], body[4], body[5]]),
                codename: get_codename(body, 6)?,
            })
        }
        PT_WELCOME => {
            if body.len() < 6 + CODENAME_WIRE + 4 + 4 + SETTINGS_WIRE_SIZE {
                return None;
            }
            Some(Packet::Welcome {
                proto: u16::from_le_bytes([body[0], body[1]]),
                build: u32::from_le_bytes([body[2], body[3], body[4], body[5]]),
                codename: get_codename(body, 6)?,
                slot: body[6 + CODENAME_WIRE],
                input_delay: body[6 + CODENAME_WIRE + 1],
                hash_every: body[6 + CODENAME_WIRE + 2],
                seed: u32::from_le_bytes([
                    body[6 + CODENAME_WIRE + 4],
                    body[6 + CODENAME_WIRE + 5],
                    body[6 + CODENAME_WIRE + 6],
                    body[6 + CODENAME_WIRE + 7],
                ]),
                settings: MatchSettings::decode(&body[6 + CODENAME_WIRE + 8..])?,
            })
        }
        PT_REFUSE => {
            if body.is_empty() {
                return None;
            }
            Some(Packet::Refuse { reason: body[0] })
        }
        PT_INPUT => {
            if body.len() < 11 {
                return None;
            }
            let base_tick = u32::from_le_bytes([body[0], body[1], body[2], body[3]]);
            let agent_mask = body[4];
            let count = body[5] as usize;
            let ack_tick = u32::from_le_bytes([body[6], body[7], body[8], body[9]]);
            let ack_bits = u32::from_le_bytes([body[10], body[11], body[12], body[13]]);
            if agent_mask == 0 {
                return None;
            }
            let per_tick = agent_mask.count_ones() as usize;
            if count == 0 || count > MAX_INPUT_COUNT_C2H {
                return None;
            }
            let need = 14 + count * per_tick * INPUT_WIRE_SIZE;
            if body.len() < need {
                return None;
            }
            let mut inputs = Vec::with_capacity(count * per_tick);
            for i in 0..count * per_tick {
                inputs.push(Input::decode(body, 14 + i * INPUT_WIRE_SIZE)?);
            }
            Some(Packet::Input { base_tick, agent_mask, ack_tick, ack_bits, inputs })
        }
        PT_HASH => {
            if body.len() < 24 {
                return None;
            }
            Some(Packet::Hash {
                tick: u32::from_le_bytes([body[0], body[1], body[2], body[3]]),
                hash: u64::from_le_bytes([
                    body[4], body[5], body[6], body[7], body[8], body[9], body[10], body[11],
                ]),
                prev_tick: u32::from_le_bytes([body[12], body[13], body[14], body[15]]),
                prev_hash: u64::from_le_bytes([
                    body[16], body[17], body[18], body[19], body[20], body[21], body[22], body[23],
                ]),
            })
        }
        PT_KEEPALIVE => {
            if body.len() < 4 {
                return None;
            }
            Some(Packet::Keepalive {
                seq: u32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            })
        }
        PT_LEAVE => {
            if body.len() < 2 {
                return None;
            }
            Some(Packet::Leave { slot: body[0], reason: body[1] })
        }
        PT_SLOTMAP => {
            if body.is_empty() {
                return None;
            }
            Some(Packet::SlotMap { human_slots: body[0] })
        }
        PT_GO => {
            if body.len() < 4 {
                return None;
            }
            Some(Packet::Go {
                first_tick: u32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            })
        }
        _ => None,
    }
}

/// Convenience: default protocol version check used by both roles.
pub fn version_matches(proto: u16) -> bool {
    proto == PROTOCOL_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::NEUTRAL;

    #[test]
    fn hello_roundtrip() {
        let p = Packet::Hello {
            proto: PROTOCOL_VERSION,
            build: 42,
            codename: "nightfire-port".into(),
        };
        let mut b = Vec::new();
        encode(&p, &mut b);
        assert_eq!(decode(&b).unwrap(), p);
    }

    #[test]
    fn input_roundtrip() {
        let mut inputs = Vec::new();
        for i in 0..6u32 {
            inputs.push(Input {
                forward: i as f32,
                ..NEUTRAL
            });
        }
        let p = Packet::Input {
            base_tick: 100,
            agent_mask: 0b0111,
            ack_tick: 97,
            ack_bits: 0b101,
            inputs,
        };
        let mut b = Vec::new();
        encode(&p, &mut b);
        assert_eq!(decode(&b).unwrap(), p);
    }

    #[test]
    fn welcome_roundtrip() {
        let p = Packet::Welcome {
            proto: PROTOCOL_VERSION,
            build: 7,
            codename: "nightfire-port".into(),
            slot: 2,
            input_delay: 3,
            hash_every: 30,
            seed: 0xdead_beef,
            settings: MatchSettings::default(),
        };
        let mut b = Vec::new();
        encode(&p, &mut b);
        assert_eq!(decode(&b).unwrap(), p);
    }

    #[test]
    fn garbage_is_none() {
        assert!(decode(&[]).is_none());
        assert!(decode(&[0x00, 0x00, 0, 0, 1, 2, 3]).is_none());
        let mut b = Vec::new();
        encode(&Packet::Keepalive { seq: 1 }, &mut b);
        b[2] = 200; // unknown type
        assert!(decode(&b).is_none());
        // Sister assertion: truncated payload must not decode.
        assert!(decode(&b[..3]).is_none());
    }
}
