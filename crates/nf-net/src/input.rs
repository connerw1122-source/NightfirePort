//! One agent's per-tick input, in a fixed wire layout.
//!
//! `nf-net` does not depend on the sim, so this is the netcode's own input
//! contract. It carries everything the MP sim consumes per tick:
//! `nf_game::player::Input` (`crates/nf-game/src/player.rs:96-125`: forward,
//! strafe, turn, look f32; jump, crouch, aiming, zoom, scope, fire,
//! mode_switch bools; weapon_cycle, gadget_cycle i8; switch_to Option<u8>)
//! **plus** the three separate args of `Sim::step_bond`
//! (`crates/nf-view/src/scene.rs:1818`: `trigger`, `reload`, `use_key`),
//! which are not part of that struct [M].
//!
//! Layout (24 bytes, little-endian): the four f32s are sent as raw `u32` bit
//! patterns so the bytes are bit-exact on both machines — no quantisation,
//! no float parsing differences. Two flag bytes, two cycle bytes and a
//! `switch_to` byte follow; unused bits are zero.

/// A 24-byte per-agent, per-tick input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
    pub turn: f32,
    pub look: f32,
    pub jump: bool,
    pub crouch: bool,
    pub aiming: bool,
    pub zoom: bool,
    pub scope: bool,
    pub fire: bool,
    pub mode_switch: bool,
    pub trigger: bool,
    pub reload: bool,
    pub use_key: bool,
    pub weapon_cycle: i8,
    pub gadget_cycle: i8,
    /// `None` maps to `0xFF` on the wire.
    pub switch_to: Option<u8>,
}

/// The game's disconnected-pad shape: buttons 0, sticks centred. The PS2
/// engine writes exactly this for a pad that is not ready every poll
/// (`psiInput_PollDevices` 0x10ca78: buttons +0x120/+0x122 = 0, analog
/// +0x128..0x12b = 0x7f) [M]; `docs/systems/input.md` "Disconnected/not-ready
/// slots: buttons 0, analog bytes 0x7f". A timed-out remote agent is fed
/// NEUTRAL every tick — never a guess — and resumes real input on reconnect
/// (the pad state machine re-initialises on replug, `ps2_handle_pad` 0x10c738).
pub const NEUTRAL: Input = Input {
    forward: 0.0,
    strafe: 0.0,
    turn: 0.0,
    look: 0.0,
    jump: false,
    crouch: false,
    aiming: false,
    zoom: false,
    scope: false,
    fire: false,
    mode_switch: false,
    trigger: false,
    reload: false,
    use_key: false,
    weapon_cycle: 0,
    gadget_cycle: 0,
    switch_to: None,
};

/// Bytes occupied by one [`Input`] on the wire.
pub const INPUT_WIRE_SIZE: usize = 24;

const FLAG0_JUMP: u8 = 1 << 0;
const FLAG0_CROUCH: u8 = 1 << 1;
const FLAG0_AIMING: u8 = 1 << 2;
const FLAG0_ZOOM: u8 = 1 << 3;
const FLAG0_SCOPE: u8 = 1 << 4;
const FLAG0_FIRE: u8 = 1 << 5;
const FLAG0_MODE_SWITCH: u8 = 1 << 6;
const FLAG0_TRIGGER: u8 = 1 << 7;

const FLAG1_RELOAD: u8 = 1 << 0;
const FLAG1_USE_KEY: u8 = 1 << 1;

fn f32_to_bits(v: f32) -> u32 {
    v.to_bits()
}

fn bits_to_f32(b: u32) -> f32 {
    f32::from_bits(b)
}

impl Input {
    /// All-zero input except `look`/`turn` etc. remain 0 — identical to
    /// [`NEUTRAL`]; kept as an alias for readability at call sites.
    pub const fn neutral() -> Input {
        NEUTRAL
    }

    /// Encode into `out` at `off`; `out` must have `off + INPUT_WIRE_SIZE` bytes.
    pub fn encode(&self, out: &mut [u8], off: usize) {
        let p = off;
        out[p..p + 4].copy_from_slice(&f32_to_bits(self.forward).to_le_bytes());
        out[p + 4..p + 8].copy_from_slice(&f32_to_bits(self.strafe).to_le_bytes());
        out[p + 8..p + 12].copy_from_slice(&f32_to_bits(self.turn).to_le_bytes());
        out[p + 12..p + 16].copy_from_slice(&f32_to_bits(self.look).to_le_bytes());
        let mut f0 = 0u8;
        if self.jump {
            f0 |= FLAG0_JUMP;
        }
        if self.crouch {
            f0 |= FLAG0_CROUCH;
        }
        if self.aiming {
            f0 |= FLAG0_AIMING;
        }
        if self.zoom {
            f0 |= FLAG0_ZOOM;
        }
        if self.scope {
            f0 |= FLAG0_SCOPE;
        }
        if self.fire {
            f0 |= FLAG0_FIRE;
        }
        if self.mode_switch {
            f0 |= FLAG0_MODE_SWITCH;
        }
        if self.trigger {
            f0 |= FLAG0_TRIGGER;
        }
        let mut f1 = 0u8;
        if self.reload {
            f1 |= FLAG1_RELOAD;
        }
        if self.use_key {
            f1 |= FLAG1_USE_KEY;
        }
        out[p + 16] = f0;
        out[p + 17] = f1;
        out[p + 18] = self.weapon_cycle as u8;
        out[p + 19] = self.gadget_cycle as u8;
        out[p + 20] = self.switch_to.unwrap_or(0xFF);
        out[p + 21] = 0;
        out[p + 22] = 0;
        out[p + 23] = 0;
    }

    /// Decode from `buf` at `off`. Returns `None` if the slice is too short.
    pub fn decode(buf: &[u8], off: usize) -> Option<Input> {
        if buf.len() < off + INPUT_WIRE_SIZE {
            return None;
        }
        let bits = |p: usize| u32::from_le_bytes([buf[p], buf[p + 1], buf[p + 2], buf[p + 3]]);
        let f0 = buf[off + 16];
        let f1 = buf[off + 17];
        let sw = buf[off + 20];
        Some(Input {
            forward: bits_to_f32(bits(off)),
            strafe: bits_to_f32(bits(off + 4)),
            turn: bits_to_f32(bits(off + 8)),
            look: bits_to_f32(bits(off + 12)),
            jump: f0 & FLAG0_JUMP != 0,
            crouch: f0 & FLAG0_CROUCH != 0,
            aiming: f0 & FLAG0_AIMING != 0,
            zoom: f0 & FLAG0_ZOOM != 0,
            scope: f0 & FLAG0_SCOPE != 0,
            fire: f0 & FLAG0_FIRE != 0,
            mode_switch: f0 & FLAG0_MODE_SWITCH != 0,
            trigger: f0 & FLAG0_TRIGGER != 0,
            reload: f1 & FLAG1_RELOAD != 0,
            use_key: f1 & FLAG1_USE_KEY != 0,
            weapon_cycle: buf[off + 18] as i8,
            gadget_cycle: buf[off + 19] as i8,
            switch_to: if sw == 0xFF { None } else { Some(sw) },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_roundtrip() {
        let mut b = [0u8; INPUT_WIRE_SIZE];
        NEUTRAL.encode(&mut b, 0);
        let back = Input::decode(&b, 0).unwrap();
        assert_eq!(back, NEUTRAL);
    }

    #[test]
    fn full_roundtrip_bit_exact() {
        let inp = Input {
            forward: -0.5,
            strafe: 1.0,
            turn: 3.14159,
            look: -777.125,
            jump: true,
            crouch: false,
            aiming: true,
            zoom: false,
            scope: true,
            fire: true,
            mode_switch: false,
            trigger: true,
            reload: true,
            use_key: false,
            weapon_cycle: -3,
            gadget_cycle: 2,
            switch_to: Some(7),
        };
        let mut b = [0u8; INPUT_WIRE_SIZE];
        inp.encode(&mut b, 0);
        assert_eq!(Input::decode(&b, 0).unwrap(), inp);
        // Sister assertion: a flipped wire byte must not decode to the same input.
        b[0] ^= 0x01;
        assert_ne!(Input::decode(&b, 0).unwrap(), inp);
    }

    #[test]
    fn short_buffer_is_none() {
        let b = [0u8; 10];
        assert!(Input::decode(&b, 0).is_none());
    }
}
