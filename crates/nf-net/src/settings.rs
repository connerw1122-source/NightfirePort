//! The host's match configuration sent to clients in the handshake.
//!
//! `nf-net` does not depend on `nf-game`, so this is the netcode's own
//! encoding of the `MpSettings` values that matter to the sim. The mapping
//! below is to `MpSettings` (`crates/nf-game/src/mp/mod.rs:76-124`), whose
//! authoritative PS2 hex layout is `docs/systems/mp-match-rules.md:24-29` and
//! `docs/systems/mp-menus.md:176-252` [M]. No serialization of the full
//! struct existed anywhere in the port before this crate (the only save code
//! is `LS_MakeMPSettings` 0x1ce508's 13-byte `MPSG` per-slot block, which
//! carries two unrelated profile fields) — this wire layout is new, tagged
//! [S] as a port feature (the PS2 game has no network mode).
//!
//! Wire layout (208 bytes, little-endian, protocol 2): 19 `u32` scalars in a
//! fixed order, then 8 per-player records of 4 `u32` (team, skin, armor_bonus,
//! radar_on), then a `u16` registered_pickups + 2 reserved bytes. Offsets in
//! comments are the game's `MpSettings` hex offsets for cross-reference.
//! Protocol 1 carried 176 bytes with 3-field player records and no
//! registered_pickups — the consolidation (net-consolidate 2026-10-16) added
//! the two fields the match-host layer carried and this one dropped.

/// The match configuration the host sends in `WELCOME`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchSettings {
    /// MpSettings.started (0x188)
    pub started: u32,
    /// gametype_mask, same word as scenario (0x1A4)
    pub gametype_mask: u32,
    /// player_count / human joined count, "the viewports" (0x1AC low half)
    pub player_count: u32,
    /// master (0x180)
    pub master: u32,
    /// master_copy (0x184)
    pub master_copy: u32,
    /// team_game = (scenario>>29)&1 (0x18C)
    pub team_game: u32,
    /// bit30 = (scenario>>30)&1 (0x190)
    pub bit30: u32,
    /// total_players (0x194)
    pub total_players: u32,
    /// frag_limit (0x19C)
    pub frag_limit: i32,
    /// time_limit in the game's confirmed unit — menu seconds x60 (0x1A0)
    pub time_limit: i32,
    /// scenario (0x1A4)
    pub scenario: u32,
    /// level_code (0x1A8)
    pub level_code: u32,
    /// human_count (0x1AC)
    pub human_count: u32,
    /// bot_count, clamped <= 4 in the game (0x1B0)
    pub bot_count: u32,
    /// weapon_set (0x1B4)
    pub weapon_set: u32,
    /// capture_reward (0x1BC)
    pub capture_reward: u32,
    /// spawn_policy (0x1C0)
    pub spawn_policy: u32,
    /// radar (0x1C4)
    pub radar: u32,
    /// friendly_fire (0x198 hex; Ghidra `_408_` decimal 0x198 was once
    /// misread as decimal +0x408 — the merge-review trap this field caused)
    pub friendly_fire: u32,
    /// Per-slot records: (team 0x20, skin 0x24, armor_bonus 0x2c, radar_on
    /// +0x28) per MpPlayer; name (+0x00) and saved-flag (+0x28's bit) are not
    /// carried by the port's MpPlayer. 8 records, slots 0..8. radar_on rides
    /// since protocol 2 (net-consolidate): `MP_GetRadarObjects` 0x189840 and
    /// `HUD_RadarUpdate` 0x19f6e8 read it per slot [M].
    pub players: [(u32, u32, u32, u32); 8],
    /// +0x472 (u16): the registered pickup count (protocol 2).
    pub registered_pickups: u16,
}

/// Bytes occupied by a [`MatchSettings`] on the wire.
pub const SETTINGS_WIRE_SIZE: usize = 208;

impl Default for MatchSettings {
    fn default() -> Self {
        MatchSettings {
            started: 0,
            gametype_mask: 0,
            player_count: 0,
            master: 1,       // the game writes +0x180 = 1 (mp-menus.md menu confirm)
            master_copy: 0,
            team_game: 0,
            bit30: 0,
            total_players: 0,
            frag_limit: 20,
            time_limit: 10 * 60, // menu hands seconds, x60 at confirm
            scenario: 0,
            level_code: 0,
            human_count: 1,
            bot_count: 0,
            weapon_set: 0,
            capture_reward: 0,
            spawn_policy: 0,
            radar: 0,
            friendly_fire: 0,
            players: [(0, 0, 0, 1); 8],
            registered_pickups: 0,
        }
    }
}

impl MatchSettings {
    /// Encode into `out` (must be at least [`SETTINGS_WIRE_SIZE`] bytes).
    pub fn encode(&self, out: &mut [u8]) {
        for b in out[..SETTINGS_WIRE_SIZE].iter_mut() {
            *b = 0;
        }
        let mut put = |i: usize, v: u32| {
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        };
        put(0, self.started);
        put(1, self.gametype_mask);
        put(2, self.player_count);
        put(3, self.master);
        put(4, self.master_copy);
        put(5, self.team_game);
        put(6, self.bit30);
        put(7, self.total_players);
        put(8, self.frag_limit as u32);
        put(9, self.time_limit as u32);
        put(10, self.scenario);
        put(11, self.level_code);
        put(12, self.human_count);
        put(13, self.bot_count);
        put(14, self.weapon_set);
        put(15, self.capture_reward);
        put(16, self.spawn_policy);
        put(17, self.radar);
        put(18, self.friendly_fire);
        for (k, &(team, skin, armor, radar_on)) in self.players.iter().enumerate() {
            let base = 19 + k * 4;
            put(base, team);
            put(base + 1, skin);
            put(base + 2, armor);
            put(base + 3, radar_on);
        }
        // +0x472: the registered pickup count (u16) in the old reserved tail.
        out[19 * 4 + 8 * 16..19 * 4 + 8 * 16 + 2]
            .copy_from_slice(&self.registered_pickups.to_le_bytes());
    }

    /// Decode from `buf`; `None` if too short.
    pub fn decode(buf: &[u8]) -> Option<MatchSettings> {
        if buf.len() < SETTINGS_WIRE_SIZE {
            return None;
        }
        let get = |i: usize| {
            u32::from_le_bytes([buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]])
        };
        let mut players = [(0u32, 0u32, 0u32, 0u32); 8];
        for (k, p) in players.iter_mut().enumerate() {
            let base = 19 + k * 4;
            *p = (get(base), get(base + 1), get(base + 2), get(base + 3));
        }
        let registered_pickups = u16::from_le_bytes([
            buf[19 * 4 + 8 * 16],
            buf[19 * 4 + 8 * 16 + 1],
        ]);
        Some(MatchSettings {
            started: get(0),
            gametype_mask: get(1),
            player_count: get(2),
            master: get(3),
            master_copy: get(4),
            team_game: get(5),
            bit30: get(6),
            total_players: get(7),
            frag_limit: get(8) as i32,
            time_limit: get(9) as i32,
            scenario: get(10),
            level_code: get(11),
            human_count: get(12),
            bot_count: get(13),
            weapon_set: get(14),
            capture_reward: get(15),
            spawn_policy: get(16),
            radar: get(17),
            friendly_fire: get(18),
            players,
            registered_pickups,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut s = MatchSettings {
            ..Default::default()
        };
        s.scenario = 0x1234_5678;
        s.friendly_fire = 1;
        s.frag_limit = -1;
        s.players[3] = (2, 5, 300, 0);
        s.registered_pickups = 0x1234;
        let mut b = [0u8; SETTINGS_WIRE_SIZE];
        s.encode(&mut b);
        assert_eq!(MatchSettings::decode(&b).unwrap(), s);
        // Sister assertions: a flipped byte must change the decoded value.
        b[18 * 4] ^= 1; // friendly_fire low bit
        assert_ne!(MatchSettings::decode(&b).unwrap(), s);
        b[18 * 4] ^= 1;
        b[19 * 4 + 8 * 16] ^= 1; // registered_pickups low byte
        assert_ne!(MatchSettings::decode(&b).unwrap(), s);
    }

    #[test]
    fn short_buffer_is_none() {
        assert!(MatchSettings::decode(&[0u8; 10]).is_none());
    }
}
