//! The MP HUD's data model — what the in-match HUD reads, as a per-frame snapshot with getters.
//! Port-only delivery shape (the main agent draws): the game's HUD reads these values straight
//! out of the MP globals each frame (`HUD_Update` 0x19a088 dispatches the per-pane updates over
//! the `MPPaneList` fn pointers [M 0x199b70:42, 0x199e88:36]); the port's snapshot folds the same
//! reads into one struct per frame. No rendering here — `nf_view::hud::radar_project` consumes
//! [`MpRadarObj`]s (pos/argb/kind) as-is.
//!
//! Evidence tags per item; the Ghidra `_NNN_` prints in these decomp files are DECIMAL, so
//! `MPGame._400_4_` = hex +0x190 (the clock) — the same convention the port's comments keep.

use super::{MpMatch, scenario};

/// The uplink terminal's live RGB, `MP_UplinkUpdate` 0x186d98:41-47,87-94,103-109: owner team 0
/// → (0xd2, 0x2d, 0x35), team 1 → (0x2d, 0x61, 0xd2), neutral/reset → the grey set — mirrored to
/// the level script via `SP_SetColour` and packed into the radar blip [M 0x189840:138-140]. The
/// port derives them from the owner word (the game stores the bytes at ext+0x103..5).
fn uplink_rgb(owner: i16) -> [u8; 3] {
    match owner {
        0 => [0xd2, 0x2d, 0x35],
        1 => [0x2d, 0x61, 0xd2],
        _ => [0x7f, 0x7f, 0x7f],
    }
}

/// `RadarObj` kinds — the u16 tag `MP_GetRadarObjects` 0x189840 writes at `RObj + i·0x20 + 0x14`
/// and `HUD_RadarUpdate` 0x19f6e8:137-150 feeds into the `TexUV.291` @0x2becf0 sheet (stride
/// 0x14; kind 5 is written by nothing — the sheet's record 5 is dead [M 0x189840 has no kind-5
/// arm]). Values are the game's, not renumbered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadarKind {
    /// 0: a player blip (the viewer themself is never added [M 0x189840:37-40]).
    Player = 0,
    /// 1: a CTF flag (the two `Flags` entries, scenario 0x20000004 [M 0x189840:98-121]).
    Flag = 1,
    /// 2: an uplink terminal (scenario 0x60000008, per-terminal RGB [M 0x189840:123-144]).
    Uplink = 2,
    /// 3: the demolition / protection site (white [M 0x189840:145-172]).
    Target = 3,
    /// 4: a GoldenEye item not yet picked up (skip when `ext+0xf0 & 0x10` [M 0x189840:159-194]).
    GoldenEye = 4,
    /// 6: the blueprint (scenario 0x20000100 [M 0x189840:195-232]).
    Blueprint = 6,
    /// 7: an espionage delivery base (two, red then blue [M 0x189840:207-231]).
    Base = 7,
}

/// One radar entry: world-space position + the packed RGBA the game writes verbatim into
/// `RObj + i·0x20 + 0x10` (the draw reads it straight [M 0x19f6e8:151]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MpRadarObj {
    /// `RObj + i·0x20 + 0x00` — the object's `+0xC0` position [M 0x189840:44-52,82-84].
    pub pos: [f32; 3],
    /// `RObj + i·0x20 + 0x10` — packed RGBA bytes (r<<24 | g<<16 | b<<8 | a) [M 0x189840:54-63,
    /// 138-140]. `nf_view::hud::RadarObj.argb` takes this as-is.
    pub argb: u32,
    /// `RObj + i·0x20 + 0x14` (u16) — [`RadarKind::as_u16`].
    pub kind: u16,
    /// The player slot for [`RadarKind::Player`] blips (None for objectives); the game carries
    /// only the obj pointer [M 0x189840:31,37] — the port keeps the slot too (the score row and
    /// the name tag lookups need it).
    pub slot: Option<usize>,
}

impl RadarKind {
    pub fn as_u16(self) -> u16 {
        self as u16
    }
    pub fn from_u16(v: u16) -> Option<Self> {
        Some(match v {
            0 => Self::Player,
            1 => Self::Flag,
            2 => Self::Uplink,
            3 => Self::Target,
            4 => Self::GoldenEye,
            6 => Self::Blueprint,
            7 => Self::Base,
            _ => return None,
        })
    }
}

/// The team colors as the game packs them for the radar blips: team 0 red `0xd22d35ff`, team 1
/// blue `0x2d61d2ff` [M 0x189840:58-62]; a non-team game tints every player white `0x7f7f7fff`
/// [M 0x189840:54-56]. (The score pane tints team 0 `0xff0000ff` and the name tags use dark
/// variants — three palettes over one team datum [M 0x19f0a0:53-55, 0x19f6e8:255-264]; each
/// getter here returns its own reader's value.)
pub fn radar_team_color(team_game: u32, team: u32) -> u32 {
    if team_game == 0 {
        0x7f7f_7fff
    } else if team == 0 {
        0xd22d_35ff
    } else {
        0x2d61_d2ff
    }
}

/// One agent's score row — the values `MP_GetPlayerScore` 0x189730 formats and the pane text
/// shows [M 0x19f0a0:56-70]. The offsets are the port's own `MpSlot` (+0x18 score, +0x04 kills,
/// +0x08 deaths).
#[derive(Clone, Debug, PartialEq)]
pub struct MpScoreRow {
    /// The MPGame slot (humans 0..human_count, bots 4..4+bot_count [M 0x212198:92-111]).
    pub slot: usize,
    /// `MPSettings + slot·0x30 + 0x20` — the team (the is-target marker in scenario 0x400
    /// [M 0x189840:59, 0x18aa70:38-41]; the port keeps the marker in `MpMatch::assassin_target`).
    pub team: u32,
    /// `MPGame + slot·0x30 + 0x18` (f32) — the scenario score [M 0x189730:17].
    pub score: f32,
    /// `MPGame + slot·0x30 + 0x04` (i32) [M 0x189730:18].
    pub kills: i32,
    /// `MPGame + slot·0x30 + 0x08` (i32) [M 0x189730:19].
    pub deaths: i32,
    /// The Assassin target marker (scenario 0x400 only) [M 0x18aa70:38-41, 0x19f0a0:102-113].
    pub is_target: bool,
}

impl MpScoreRow {
    /// The display score `Menu_GetMPScore` 0x203af8 sorts and prints: the slot's score for every
    /// scoring scenario, the KILLS for 0x1 (Arena) and 0x20000002 (Team Arena) [M 0x203af8:44-54].
    pub fn display_score(&self, sc: u32) -> f32 {
        match sc {
            scenario::DEATHMATCH | scenario::TEAM_DEATHMATCH => self.kills as f32,
            _ => self.score,
        }
    }

    /// The score text's line values, `MP_GetPlayerScore` 0x189730: scenario 0x400 (Assassin) and
    /// 0x40000800 (KOTH FFA) print THREE lines `score\nkills\ndeads` [M :13-19]; everything else
    /// prints `kills\ndeads` [M :20-27]. Returns `(lines, score, kills, deaths)`.
    pub fn score_lines(&self, sc: u32) -> (usize, f32, i32, i32) {
        if sc == scenario::ASSASSIN || sc == scenario::KOTH_FFA {
            (3, self.score, self.kills, self.deaths)
        } else {
            (2, self.kills as f32, self.kills, self.deaths)
        }
    }
}

/// One uplink terminal's HUD state (`HUD_MPUpdatePane` colors the pane icons from it [M
/// 0x19f0a0:134-180]).
#[derive(Clone, Debug, PartialEq)]
pub struct MpUplinkHud {
    /// The terminal's index (the `Uplinks`[8] slot [M 0x189840:123-144]).
    pub index: usize,
    /// `MP_getUplinkStatus` 0x18ba80: the owner word — 0 team 0, 1 team 1, 2 neutral/reset
    /// [M 0x186d98's +0xf4 writes; 0x18ba80 returns it, 2 for an empty slot].
    pub owner: i16,
    /// `MPOBJECT+0x52` — the controlling player slot, −1 = none [M 0x186d98:61-65].
    pub controller: i16,
    /// The terminal's world position (the radar blip's [M 0x189840:126-135]).
    pub pos: [f32; 3],
    /// The live RGB the update mirrors to the level script and the radar [M 0x186d98:41-47,
    /// 87-94, 103-109; packed into the blip at 0x189840:138-140].
    pub rgb: [u8; 3],
}

/// The per-frame snapshot: everything the MP HUD draws, read off the match state the way the
/// game's pane updates read it. Build once per frame (`MpHudSnapshot::build`), then read.
#[derive(Clone, Debug, Default)]
pub struct MpHudSnapshot {
    /// The reading machine's slot (the game's panes are per-BLData, keyed `BLData+0x94e` [M
    /// 0x19f0a0:31]).
    pub viewer_slot: usize,
    /// Seconds remaining: `limit − clock`, clamped at 0 — the game feeds `(limit − clock)·100`
    /// to `Timer_Seconds2String` and negatives print 00:00 [M 0x184268:144-147, 0x1deaa8:21-25].
    pub time_left: f32,
    /// The score rows in the game's in-match order: the slots as they sit (the pause board walks
    /// humans then bots in slot order — no sort in the in-match paths [M 0x204408:327-399;
    /// 0x19f0a0 is one pane per viewer]).
    pub rows: Vec<MpScoreRow>,
    /// `MPGame + team·4 + 0x180` — the two team totals [M 0x18b338:20-34].
    pub team_scores: [f32; 2],
    /// Per CTF flag: the carrying slot (None = at base or dropped). The game's pane icon asks
    /// `MP_HasTeamFlag(obj, team)` 0x18b430 — the flag ext's `+0x1c` carrier pointer — NOT the
    /// slot's 0x26 bit (that one only bot AI reads) [M 0x19f0a0:89-100; 0x1861a8:173-178 writes
    /// the bit]. The port's `MpObjState::carrier` is that pointer's stand-in.
    pub flags: [Option<usize>; 2],
    /// The uplink terminals' HUD state (scenario 0x60000008; empty otherwise).
    pub uplinks: Vec<MpUplinkHud>,
    /// The Assassin marker pair (scenario 0x400): the local player's assassin and the target
    /// slots [M 0x19f0a0:102-113 — icons 0x3000180 / 0x3000189; the globals are `DAT_0030d774`
    /// (assassin) and `Assasin` 0x30d770 (target) — the names are inverted vs intuition [M
    /// 0x18b538:6, 0x18b558:6]].
    pub assassin: Option<usize>,
    pub target: Option<usize>,
    /// The radar name tags' gate: `MPSettings+0x1c4` (the port's `settings.radar`) set, or
    /// scenario 0x400 [M 0x19f6e8:177]. Colors: FFA `0x1e504bff`, teams `0x5a1414ff` /
    /// `0x14145aff` [M 0x19f6e8:255-264] — a getter, not stored per name.
    pub names_on_radar: bool,
    /// The scenario word (the getters key off it: the display score, the line count, the
    /// Assassin marker). Copied from `settings.scenario` at build [M 0x189730:13-27].
    pub scenario: u32,
    /// The radar's world-space objects, in the game's fill order (players, then the scenario's
    /// markers) — EMPTY when the viewer's radar toggle is off [M 0x189840:22; 0x19f6e8:102-105
    /// hides the ring the same way]. Feed to `nf_view::hud::radar_project`.
    pub radar: Vec<MpRadarObj>,
}

impl MpHudSnapshot {
    /// Read the frame's HUD state off the match. `viewer_slot` = the reading machine's slot
    /// (the game keys the panes off `BLData+0x94e` [M 0x19f0a0:31]); `viewer_radar_on` = the
    /// viewer's `MPSettings + slot·0x30 + 0x28` toggle [M 0x189840:22] — the port keeps it on
    /// `MpPlayer::radar_on`.
    pub fn build(m: &MpMatch, viewer_slot: usize, viewer_radar_on: bool) -> Self {
        let sc = m.settings.scenario;
        let mut snap = MpHudSnapshot {
            viewer_slot,
            scenario: sc,
            // The clock accumulates dt per fixed tick; the game accumulates the
            // `psiGetTimeIn100ths` delta (the port's *Deviation*, mp/mod.rs's module doc).
            time_left: (m.game.limit - m.game.clock).max(0.0),
            team_scores: m.game.team_scores,
            flags: [None, None],
            uplinks: Vec::new(),
            assassin: m.assassin,
            target: m.assassin_target,
            names_on_radar: m.settings.radar != 0 || sc == scenario::ASSASSIN,
            radar: Vec::new(),
            rows: Vec::new(),
        };

        // The rows: humans 0..human_count, then bots 4..4+bot_count — the game's slot walk
        // [M 0x212198:92-111; the pause board 0x204408:327-399 buckets by the same order].
        let humans = (m.settings.human_count as usize).min(4);
        let bots = (m.settings.bot_count as usize).min(4);
        for slot in (0..humans).chain(4..4 + bots) {
            let s = &m.game.slots[slot];
            snap.rows.push(MpScoreRow {
                slot,
                team: m.settings.players[slot].team,
                score: s.score,
                kills: s.kills,
                deaths: s.deaths,
                is_target: sc == scenario::ASSASSIN && m.assassin_target == Some(slot),
            });
        }

        // The radar: the viewer's toggle gates the WHOLE list [M 0x189840:22-24 — 0 returns an
        // empty list, and the ring hides with it 0x19f6e8:102-105].
        if viewer_radar_on {
            // Player blips: every live slot but the viewer [M 0x189840:29-96]. The game's
            // +0xff state gates (2/4 limbo, 3 the +0x894 arm) ride the BLData pipeline the
            // port's players don't have — the port's `MpSlot::dead` is the stand-in [S, see
            // the report's deviations].
            for slot in 0..humans {
                if slot == viewer_slot || m.game.slots[slot].dead {
                    continue;
                }
                let pos = m.game.slots[slot].pos.unwrap_or_default();
                let argb = radar_team_color(m.settings.team_game, m.settings.players[slot].team);
                snap.radar.push(MpRadarObj { pos, argb, kind: RadarKind::Player.as_u16(), slot: Some(slot) });
            }
            for slot in 4..4 + bots {
                if slot == viewer_slot || m.game.slots[slot].dead {
                    continue;
                }
                let pos = m.game.slots[slot].pos.unwrap_or_default();
                let argb = radar_team_color(m.settings.team_game, m.settings.players[slot].team);
                snap.radar.push(MpRadarObj { pos, argb, kind: RadarKind::Player.as_u16(), slot: Some(slot) });
            }
            // The scenario's markers, the game's order and colors [M 0x189840:97-232].
            match sc {
                scenario::CAPTURE_THE_FLAG => {
                    for (i, f) in m.objects_rt.flags.iter().enumerate() {
                        if let Some(f) = f {
                            // First flag blue, second red [M 0x189840:102-105 — uVar12 counts
                            // 0 then 1: 0 → 0xd22d35ff, 1 → 0x2d61d2ff].
                            let argb = if i == 0 { 0xd22d_35ff } else { 0x2d61_d2ff };
                            snap.radar.push(MpRadarObj { pos: f.pos, argb, kind: RadarKind::Flag.as_u16(), slot: None });
                        }
                    }
                }
                scenario::UPLINK => {
                    for (i, u) in m.objects_rt.uplinks.iter().enumerate() {
                        // The per-terminal RGB [M 0x189840:138-140 — the bytes at ext+0x103..5;
                        // `MP_UplinkUpdate` 0x186d98:41-47,87-94,103-109 sets them to the
                        // owner team's colors — the port derives them from the owner word].
                        let rgb = uplink_rgb(u.state);
                        let argb = (rgb[0] as u32) << 24 | (rgb[1] as u32) << 16 | (rgb[2] as u32) << 8 | 0xff;
                        snap.radar.push(MpRadarObj { pos: u.pos, argb, kind: RadarKind::Uplink.as_u16(), slot: None });
                    }
                }
                scenario::DEMOLITION | scenario::PROTECTION => {
                    if let Some(site) = &m.objects_rt.site {
                        snap.radar.push(MpRadarObj { pos: site.pos, argb: 0x7f7f_7fff, kind: RadarKind::Target.as_u16(), slot: None });
                    }
                }
                scenario::GOLDENEYE => {
                    for g in m.objects_rt.ge.iter().flatten() {
                        // Held items are skipped [M 0x189840:178 — `(ext+0xf0 & 0x10) == 0`
                        // draws; the port's state word: carried = 1/3 per the item's states].
                        if g.state == 0 || g.state == 2 {
                            snap.radar.push(MpRadarObj { pos: g.pos, argb: 0x7f7f_7fff, kind: RadarKind::GoldenEye.as_u16(), slot: None });
                        }
                    }
                }
                scenario::BLUEPRINT => {
                    if let Some(bp) = &m.objects_rt.blueprint {
                        snap.radar.push(MpRadarObj { pos: bp.pos, argb: 0x7f7f_7fff, kind: RadarKind::Blueprint.as_u16(), slot: None });
                    }
                    // The two delivery bases: red then blue [M 0x189840:207-231].
                    for (i, b) in m.objects_rt.esp_bases.iter().enumerate() {
                        if let Some(pos) = b {
                            let argb = if i == 0 { 0xd22d_35ff } else { 0x2d61_d2ff };
                            snap.radar.push(MpRadarObj { pos: *pos, argb, kind: RadarKind::Base.as_u16(), slot: None });
                        }
                    }
                }
                _ => {}
            }
        }

        // The objective status.
        for (i, f) in m.objects_rt.flags.iter().enumerate() {
            snap.flags[i] = f.as_ref().and_then(|f| f.carrier);
        }
        if sc == scenario::UPLINK {
            for u in m.objects_rt.uplinks.iter().enumerate() {
                let (i, u) = u;
                snap.uplinks.push(MpUplinkHud {
                    index: i,
                    owner: u.state,
                    controller: u.controller,
                    pos: u.pos,
                    rgb: uplink_rgb(u.state),
                });
            }
        }
        snap
    }

    /// The timer text, `Timer_Seconds2String` 0x1deaa8 mode 2: whole seconds (rounded — the game
    /// passes centiseconds), `ss = secs % 60`, `mm = (secs / 60) % 60`, `"%02d:%02d"` [M
    /// 0x1deaa8:17-33]; negatives clamp to 00:00 [M :21-25]. Hours never show in mode 2.
    pub fn timer_string(&self) -> String {
        let secs = (self.time_left + 0.5).floor() as i64;
        let secs = secs.max(0);
        let ss = secs % 60;
        let mm = (secs / 60) % 60;
        format!("{mm:02}:{ss:02}")
    }

    /// The rows sorted the way the game sorts its board: `P_MPDEBRIEFING_Handler` 0x212198's
    /// bubble pass, descending by `Menu_GetMPScore` 0x203af8's display score [M 0x212198:112-130;
    /// 0x203af8:44-54]. The bubble swaps only on a strictly greater score, so equal scores keep
    /// slot order — this getter is that stable order (a merge-safe re-derivation, not a port of
    /// the sort's swap loop).
    pub fn sorted_rows(&self) -> Vec<&MpScoreRow> {
        let sc = self.scenario;
        let mut rows: Vec<&MpScoreRow> = self.rows.iter().collect();
        rows.sort_by(|a, b| {
            let (sa, sb) = (a.display_score(sc), b.display_score(sc));
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        rows
    }
}
