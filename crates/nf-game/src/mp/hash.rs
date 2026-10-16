//! The MP lockstep state hash: `state_hash(&MpMatch) -> u64`, stable across runs and machines.
//!
//! Deterministic lockstep (the upcoming `nf-net` UDP work) detects a desync by exchanging one
//! hash per tick. That only works if the hash itself is a pure function of the sim state **bits**:
//!
//! - integers hash as their little-endian bytes;
//! - f32 values hash as their IEEE-754 **bit patterns** (`f32::to_bits`), never through
//!   formatting or `PartialEq` (−0.0 vs 0.0 and NaN payloads are distinct states);
//! - `Option<T>` hashes a 0/1 tag byte, then `T`;
//! - `Vec<T>` hashes its `u32` length, then the elements;
//! - the fold is **FNV-1a 64** (64-bit integer multiplies only), implemented here rather than
//!   `std`'s `DefaultHasher` — `SipHash`'s stability across Rust versions is explicitly not
//!   guaranteed, and lockstep hosts on different builds must agree bit-for-bit.
//!
//! Field order is fixed by the code below (settings → spawn globals → `MPGame` → the stored MP
//! objects → the scenario objects → the assassin words); adding a sim field means adding it to
//! this function. A field left outhashed is a desync the hash cannot see — when you add state to
//! `MpMatch`, add it here too (the one-bit sister test in `mp_lockstep.rs` pins a sample of
//! fields, not all of them).

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The canonical byte-fold. Only integer math: identical results on every platform, every
/// release profile, every Rust version.
pub struct Hasher {
    h: u64,
}

impl Hasher {
    pub fn new() -> Hasher {
        Hasher { h: FNV_OFFSET }
    }

    /// Folds raw bytes (FNV-1a: xor low byte first, then multiply).
    pub fn bytes(&mut self, b: &[u8]) {
        for &byte in b {
            self.h ^= byte as u64;
            self.h = self.h.wrapping_mul(FNV_PRIME);
        }
    }

    pub fn u8(&mut self, v: u8) {
        self.bytes(&[v]);
    }

    pub fn u16(&mut self, v: u16) {
        self.bytes(&v.to_le_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }

    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }

    pub fn i16(&mut self, v: i16) {
        self.u16(v as u16);
    }

    pub fn i32(&mut self, v: i32) {
        self.u32(v as u32);
    }

    pub fn usize(&mut self, v: usize) {
        self.u64(v as u64);
    }

    /// The f32's **bit pattern** — not its value: −0.0 and 0.0 hash differently, NaN payloads are
    /// preserved. Two sims whose floats differ by one bit must hash differently (the sister test).
    pub fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }

    pub fn f32_slice(&mut self, v: &[f32]) {
        self.usize(v.len());
        for &x in v {
            self.f32(x);
        }
    }

    pub fn bool(&mut self, v: bool) {
        self.u8(v as u8);
    }

    pub fn finish(self) -> u64 {
        self.h
    }
}

use super::rules::MpObjState;
use super::{MpMatch, MpObjectRecord};

fn hash_object_record(h: &mut Hasher, r: &MpObjectRecord) {
    for row in &r.matrix {
        h.f32_slice(row);
    }
    for &c in &r.config {
        h.u32(c);
    }
    h.u32(r.cel);
}

fn hash_spawn_points(h: &mut Hasher, m: &MpMatch) {
    // The table is always the game's 64 entries; hash them all in index order.
    h.u32(m.spawns.count);
    h.u32(m.spawns.team_count[0]);
    h.u32(m.spawns.team_count[1]);
    h.usize(m.spawns.points.len());
    for p in &m.spawns.points {
        h.f32_slice(&p.pos);
        h.f32_slice(&p.facing);
        h.u16(p.used);
    }
}

fn hash_slots(h: &mut Hasher, m: &MpMatch) {
    h.usize(m.game.slots.len());
    for s in &m.game.slots {
        match s.pos {
            None => h.u8(0),
            Some(p) => {
                h.u8(1);
                h.f32_slice(&p);
            }
        }
        h.u16(s.timers[0]);
        h.u16(s.timers[1]);
        h.u16(s.last_killer);
        h.u16(s.status);
        h.f32(s.score);
        h.i32(s.kills);
        h.i32(s.deaths);
        h.i32(s.streak);
        h.u16(s.status_bits);
        h.bool(s.dead);
    }
}

fn hash_settings(h: &mut Hasher, m: &MpMatch) {
    let s = &m.settings;
    h.usize(s.players.len());
    for p in &s.players {
        h.u32(p.team);
        h.u32(p.skin);
        h.u32(p.armor_bonus);
    }
    h.bool(s.started);
    h.u32(s.gametype_mask);
    h.u16(s.player_count);
    h.u32(s.master);
    h.u32(s.master_copy);
    h.u32(s.team_game);
    h.u32(s.bit30);
    h.u32(s.total_players);
    h.i32(s.frag_limit);
    h.i32(s.time_limit);
    h.u32(s.scenario);
    h.u32(s.level_code);
    h.u32(s.human_count);
    h.u32(s.bot_count);
    h.u32(s.weapon_set);
    h.u32(s.capture_reward);
    h.u32(s.spawn_policy);
    h.u32(s.radar);
    h.u16(s.registered_pickups);
    h.u32(s.friendly_fire);
}

fn hash_game(h: &mut Hasher, m: &MpMatch) {
    let g = &m.game;
    hash_slots(h, m);
    h.f32(g.team_scores[0]);
    h.f32(g.team_scores[1]);
    h.u32(g.state);
    h.f32(g.clock);
    h.f32(g.limit);
    h.f32(g.restart_timer);
    h.f32(g.elapsed);
    h.f32(g.debrief_timer);
    h.f32(g.pause_stamp);
    h.bool(g.time_up);
    h.bool(g.frag_latched);
    h.i32(g.high);
}

fn hash_stored_objects(h: &mut Hasher, m: &MpMatch) {
    let o = &m.objects;
    for group in [&o.created, &o.demolition, &o.blueprint_items, &o.protection] {
        h.usize(group.len());
        for r in group {
            hash_object_record(h, r);
        }
    }
    for pool in &o.goldeneye {
        h.usize(pool.len());
        for r in pool {
            hash_object_record(h, r);
        }
    }
}

fn hash_obj_state(h: &mut Hasher, s: &MpObjState) {
    h.i16(s.state);
    h.i16(s.sub);
    h.u16(s.counter);
    match s.carrier {
        None => h.u8(0),
        Some(c) => {
            h.u8(1);
            h.usize(c);
        }
    }
    h.i16(s.last_hitter);
    h.i16(s.controller);
    h.u16(s.team);
    h.f32_slice(&s.pos);
    h.f32_slice(&s.home);
}

fn hash_opt_obj(h: &mut Hasher, s: &Option<MpObjState>) {
    match s {
        None => h.u8(0),
        Some(v) => {
            h.u8(1);
            hash_obj_state(h, v);
        }
    }
}

fn hash_opt_pos(h: &mut Hasher, p: &Option<[f32; 3]>) {
    match p {
        None => h.u8(0),
        Some(v) => {
            h.u8(1);
            h.f32_slice(v);
        }
    }
}

fn hash_objects_rt(h: &mut Hasher, m: &MpMatch) {
    let o = &m.objects_rt;
    for f in &o.flags {
        hash_opt_obj(h, f);
    }
    for b in &o.bases {
        hash_opt_pos(h, b);
    }
    h.usize(o.uplinks.len());
    for u in &o.uplinks {
        hash_obj_state(h, u);
    }
    hash_opt_obj(h, &o.hill);
    hash_opt_obj(h, &o.site);
    for g in &o.ge {
        hash_opt_obj(h, g);
    }
    match o.ge_target {
        None => h.u8(0),
        Some(v) => {
            h.u8(1);
            h.usize(v);
        }
    }
    hash_opt_obj(h, &o.blueprint);
    for b in &o.esp_bases {
        hash_opt_pos(h, b);
    }
}

/// The per-tick lockstep hash of the whole MP match state. Pure function of the state bits:
/// the same state on any machine, any run, hashes to the same `u64`.
pub fn state_hash(m: &MpMatch) -> u64 {
    let mut h = Hasher::new();
    hash_settings(&mut h, m);
    hash_spawn_points(&mut h, m);
    hash_game(&mut h, m);
    hash_stored_objects(&mut h, m);
    hash_objects_rt(&mut h, m);
    match m.assassin {
        None => h.u8(0),
        Some(v) => {
            h.u8(1);
            h.usize(v);
        }
    }
    match m.assassin_target {
        None => h.u8(0),
        Some(v) => {
            h.u8(1);
            h.usize(v);
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp::{MpMatch, MpSettings};

    /// FNV-1a 64 reference vectors (the standard test vectors for the empty input and
    /// "a"/"foobar") — pins the fold itself against a silent change.
    #[test]
    fn fnv1a_reference_vectors() {
        let mut h = Hasher::new();
        assert_eq!(h.finish(), 0xcbf2_9ce4_8422_2325);
        let mut h = Hasher::new();
        h.bytes(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
        let mut h = Hasher::new();
        h.bytes(b"foobar");
        assert_eq!(h.finish(), 0x8594_4171_f739_67e8);
    }

    /// Two equal states hash equal; the −0.0/0.0 distinction holds (bit-pattern hashing).
    #[test]
    fn f32_bits_not_values() {
        let mut a = Hasher::new();
        a.f32(0.0);
        let mut b = Hasher::new();
        b.f32(-0.0);
        assert_ne!(a.finish(), b.finish(), "−0.0 is a different state bit pattern");
        let mut c = Hasher::new();
        c.f32(f32::NAN);
        let mut d = Hasher::new();
        d.f32(f32::from_bits(0x7f80_0001));
        assert_ne!(c.finish(), d.finish(), "NaN payloads are distinct states");
    }

    /// The hash is a pure function of state: two freshly-built identical matches hash equal, and
    /// a settings difference (the map id alone) is visible.
    #[test]
    fn equal_states_hash_equal() {
        let m1 = MpMatch::init(MpSettings::deathmatch(0x07000011));
        let m2 = MpMatch::init(MpSettings::deathmatch(0x07000011));
        assert_eq!(state_hash(&m1), state_hash(&m2));
        let m3 = MpMatch::init(MpSettings::deathmatch(0x07000023));
        assert_ne!(state_hash(&m1), state_hash(&m3), "the map id is hashed");
    }
}
