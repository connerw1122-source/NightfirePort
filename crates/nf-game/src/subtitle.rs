//! Subtitles: the game's `TXT_MSG` type-4/5 queue (`Text_AddMsg` 0x1b0c30) for the two sources the
//! AV options' Subtitles row switches — the NIS/script cut-scene lines (op 0x1e) and the FMV
//! briefing intro lines (the 14 single-stream scripts in 07000500.bin). Data model + the per-tick
//! query the draw code calls. Docs: docs/systems/subtitles.md.
//!
//! Evidence tags: [M] decomp / disassembly read here, [D] disc, [S] suggested.

use nf_formats::nis::Nis;

/// One queued subtitle line on an absolute tick axis: displayed for every tick `start <= t < end`.
///
/// The game stores bound strings (`Text_AddMsg`'s word-wrapped copies); the port carries the raw
/// label and lets the draw code bind it through the level's/bank's Lang table (`Txt_BindLabel`
/// 0x1cad60 runs inside `Script_TextStart` on the game [M 0x1db2b8], but the label survives in the
/// op stream and the binding is a pure bank lookup — the `drones.script_msgs` pattern).
#[derive(Clone, Debug, PartialEq)]
pub struct SubtitleLine {
    pub label: u32,
    pub start: u32,
    pub end: u32,
}

/// The type-4/5 message queue as an absolute-timeline track. `enabled` is `DAT_002a37c8` — the
/// profile's Subtitles byte (`Profile::gset.subtitles`, GameState+0x60, written back by
/// `P_CNAVOPTIONS_Handler` 0x214818 via control 0x10000136): `Text_AddMsg` drops a type-4 add
/// when it is 0 [M 0x1b0c30: `bVar1 || (DAT_002a37c8 != 0) || (1 < type - 4U)` — type 4 with id 0
/// passes only through the flag; the id-0xff bypass arm never fires for scripts, whose adds are
/// always id 0 [M `Script_TextStart` 0x1db2b8: a1 = 0]].
#[derive(Clone, Debug, Default)]
pub struct SubtitleTrack {
    /// `DAT_002a37c8`: the profile's Subtitles On/Off byte. Off → `push` drops every line, so
    /// `active_subtitle` is always None.
    pub enabled: bool,
    lines: Vec<SubtitleLine>,
}

impl SubtitleTrack {
    pub fn new(enabled: bool) -> Self {
        SubtitleTrack { enabled, lines: Vec::new() }
    }

    /// `Text_AddMsg(-1, 0, 4, Txt_BindLabel(label), 0, time)` [M 0x1b0c30] at absolute tick `now`.
    ///
    /// * Off → dropped (the flag gate above).
    /// * Duration = `time` verbatim: the game scales by the word-wrap part ratio
    ///   (`time * len/origLen + 0.5`) and the disassembly's `bnel v0,zero,001b10b4` [M 0x1b1050]
    ///   skips the `max(60, …)` clamp for types 4/5 — with one wrap part the ratio is 1, so the
    ///   op's u16 time is the display time. (The port models one message per op; the game splits a
    ///   line wider than `TextMsgFormats` entry 4's 480 px into consecutive parts [M 0x1b0e80's
    ///   wrap loop] — a text-layout concern the draw code owns.)
    /// * FIFO chaining [M `Text_UpdateMsg` 0x1b11d8: only the displayed head's timer ticks; a new
    ///   add while a head shows queues behind it and starts when the head expires]: the line starts
    ///   at `max(now, previous end)`. A `time` of 0 stores an empty window — the game's head timer
    ///   loads 0 and `Text_UpdateMsg` deletes it on the next update, so it never displays.
    /// * Returns whether the line was queued.
    pub fn push(&mut self, now: u32, label: u32, time: u16) -> bool {
        if !self.enabled {
            return false;
        }
        let start = self.lines.last().map_or(now, |l: &SubtitleLine| l.end.max(now));
        let dur = time as u32;
        self.lines.push(SubtitleLine { label, start, end: start + dur });
        true
    }

    /// The displayed line's label at absolute tick `t` — `Text_Update` 0x1b1510's `SubTitleMsg`
    /// (the queue head `Text_UpdateMsg` 0x1b11d8 maintains), as a pure timeline query.
    pub fn active_subtitle(&self, t: u32) -> Option<u32> {
        self.lines.iter().find(|l| l.start <= t && t < l.end).map(|l| l.label)
    }

    /// `Text_FlushAllSubtitles()` (0x1b15c8, `Script_FFwd`'s skip arm 0x1dac78:28-29) at tick
    /// `now`: the displayed head stops at `now` and every queued line dies (the game's loop deletes
    /// every type-4/5 node in the list [M 0x1b15c8]). Later pushes start fresh from the clamped end.
    pub fn flush(&mut self, now: u32) {
        for l in self.lines.iter_mut() {
            if l.start < now && now < l.end {
                l.end = now;
            }
        }
        self.lines.retain(|l| l.start < now && l.start < l.end);
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// The number of queued lines (tests / diagnostics).
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// The queued lines (tests / diagnostics).
    pub fn lines(&self) -> &[SubtitleLine] {
        &self.lines
    }

    /// The FMV briefing kind: build the track from a decoded intro-subtitle script.
    ///
    /// `ResetMap_Load` (0x1d6360) phase 0 — with the subtitles flag on — creates one ScriptPlayer
    /// per movie code from the pack 07000500.bin (14 tiny kind-7 scripts, every one a single stream
    /// of op-0x1e `Script_TextStart` commands [D docs/systems/levels.md §5.4]) and plays it at once
    /// (`SP_Create(0x2d6200, 0x2d6200, script, 0, 1, 1, 0, 0)` [M 0x1d6360:96-99] — script A's
    /// mode 1 = play at once, docs/systems/script-player.md) while `psiStartBackgroundMovie` plays
    /// the briefing, so the script's tick axis is the movie's.
    ///
    /// Timing per `Script_Run` (0x1d8548): each update runs the stream commands with the
    /// pre-increment clock, then `Script_Update` (0x1da998) adds `FRAME_RATE_MUL` (1.0 [L]) — so a
    /// command after `Wait(t)` fires on the update whose pre-clock is `t`, i.e. tick `t + 1`
    /// (ticks count from 1). A `Text` with no preceding `Wait` fires on the first update. Streams
    /// other than 0 are walked the same way (the 14 scripts are single-stream [D]; the general
    /// script's parallel streams each keep their own clock).
    pub fn from_script(nis: &Nis, enabled: bool) -> SubtitleTrack {
        let mut track = SubtitleTrack::new(enabled);
        if !enabled {
            return track;
        }
        for stream in nis.streams.iter() {
            let cmds = &stream.commands;
            // The stream's wait-until (`SSTREAM.wait`), and whether the first release tick passed.
            let mut wait: u32 = 0;
            let mut p = 0usize;
            while p < cmds.len() {
                let op = cmds[p];
                // Every command is op + args + 1 trailing byte (`Script_Run` 0x1d8548:120).
                let len = command_len(op, cmds, p);
                match op {
                    // `Script_Wait` (op 5): wait-until = t [M nis.rs's run_stream arm 5].
                    5 => {
                        if let Some(t) = read_u16(cmds, p + 1) {
                            wait = t as u32;
                        }
                    }
                    // `Script_TextStart` (op 0x1e) [M 0x1db2b8]: u32 label, u16 time.
                    0x1e => {
                        if let (Some(label), Some(time)) = (read_u32(cmds, p + 1), read_u16(cmds, p + 5)) {
                            track.push(wait + 1, label, time);
                        }
                    }
                    // `Script_StreamEnd` (op 4) ends the walk (the game rewinds onto it; a
                    // non-looping one-stream script never re-runs).
                    4 => break,
                    _ => {}
                }
                p += len;
            }
        }
        track
    }
}

/// The movie-code → intro-subtitle script table [M `ResetMap_Load` 0x1d6360:40-99 — the compiled
/// if/else chain, no jump table]. Movie codes 0x710000xx map 1:1 from the level codes 0x070000xx
/// per `ResetMap_LevelToLoad` 0x1d3800, except 0x0700001b → 0x7100018 and 0x0700004a → 0x710004a,
/// which is not in the table → no intro [M]. The 0x73 driving codes are the mission outros
/// (`Mission_Update` 0x182c60 case 6's switch) — 0x730000f is not here (not on the disc's whitelist
/// either [M docs/systems/movies.md]).
pub fn intro_script_for(movie_code: u32) -> Option<u32> {
    match movie_code {
        0x0710_0001 => Some(0x0600_095a),
        0x0710_0005 => Some(0x0600_0961),
        0x0710_0009 => Some(0x0600_0963),
        0x0710_000c => Some(0x0600_0965),
        0x0710_0011 => Some(0x0600_0967),
        0x0710_0014 => Some(0x0600_0969),
        0x0710_0018 => Some(0x0600_096b),
        0x0730_0004 => Some(0x0600_095d),
        0x0730_0008 => Some(0x0600_0962),
        0x0730_000b => Some(0x0600_0964),
        0x0730_000d => Some(0x0600_0966),
        0x0730_0013 => Some(0x0600_0968),
        0x0730_0017 => Some(0x0600_097a),
        0x0730_001b => Some(0x0600_096c),
        _ => None,
    }
}

fn read_u16(cmds: &[u8], p: usize) -> Option<u16> {
    cmds.get(p..p + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
}

fn read_u32(cmds: &[u8], p: usize) -> Option<u32> {
    cmds.get(p..p + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// The decode lens of the ops the intro scripts use (nf_formats::nis's lens, replicated for the
/// single-stream walk: op + args + 1 trailing byte).
fn command_len(op: u8, cmds: &[u8], p: usize) -> usize {
    match op {
        4 | 5 | 6 => 4,               // u16 arg
        0x1e => 8,                    // u32 label + u16 time
        0x12 => 4 + 4 * cmds.get(p + 1).copied().unwrap_or(0) as usize, // Event
        0x13 => 6,                    // Fade
        0x14 => 16,                   // Sprite
        0x15 | 0x1a | 0x1d => 2,      // Skip
        0x18 => 5,                    // Light
        0x1b => 8,                    // SubScript
        0x0d => 7,                    // Camera
        9 | 0x0c | 0x17 | 0x0f => 2,  // *End
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Text_AddMsg`'s FIFO chaining [M 0x1b11d8]: the first line displays from its push tick, a
    /// line pushed while a head displays waits for it, a line pushed after it starts at once.
    #[test]
    fn fifo_chaining_matches_the_queue_semantics() {
        let mut t = SubtitleTrack::new(true);
        assert!(t.push(184, 0x0600_0068, 1132));
        assert_eq!(t.active_subtitle(183), None, "sister: nothing displays before the push");
        assert_eq!(t.active_subtitle(184), Some(0x0600_0068));
        assert_eq!(t.active_subtitle(1315), Some(0x0600_0068), "184 + 1132 − 1 = the last displaying tick");
        assert_eq!(t.active_subtitle(1316), None, "the head's timer hit 0 — Text_UpdateMsg deletes it");
        // Pushed while the head displays → waits for it [M the queue advance].
        assert!(t.push(200, 0x0600_0069, 100));
        assert_eq!(t.active_subtitle(1200), Some(0x0600_0068), "the head still displays; the queued line waits");
        assert_eq!(t.active_subtitle(1316), Some(0x0600_0069), "the head expired; the queue advanced");
        assert_eq!(t.active_subtitle(1415), Some(0x0600_0069));
        assert_eq!(t.active_subtitle(1416), None);
        // Pushed after everything expired → displays at once.
        assert!(t.push(2000, 0x0600_0070, 50));
        assert_eq!(t.active_subtitle(2000), Some(0x0600_0070));
        assert_eq!(t.active_subtitle(2049), Some(0x0600_0070));
        assert_eq!(t.active_subtitle(2050), None);
    }

    /// The duration is the op's u16 time verbatim [M 0x1b1050's `bnel` skips the 60-clamp for
    /// types 4/5; one wrap part = the un-scaled time], and a 0 time never displays [M
    /// `Text_UpdateMsg` deletes a ≤ 0 head on the next update].
    #[test]
    fn duration_is_the_op_time_and_zero_never_displays() {
        let mut t = SubtitleTrack::new(true);
        assert!(t.push(10, 0x0600_0001, 45));
        assert_eq!(t.active_subtitle(54), Some(0x0600_0001), "45 ticks, not the 60 floor the other types get");
        assert_eq!(t.active_subtitle(55), None);
        assert!(t.push(20, 0x0600_0002, 0));
        assert_eq!(t.active_subtitle(21), Some(0x0600_0001), "the first head is still up (ends at 55)");
        assert_eq!(t.active_subtitle(55), None, "sister: the zero-time head's empty window never displays");
        assert_eq!(t.active_subtitle(1000), None);
        assert_eq!(t.len(), 2);
    }

    /// Off (the profile's fresh default [M profile.rs `subtitles: false`]) → `push` drops every
    /// line and `active_subtitle` is None everywhere [M the 0x1b0c30 gate].
    #[test]
    fn subtitles_off_queues_nothing() {
        let mut t = SubtitleTrack::new(false);
        assert!(!t.push(184, 0x0600_0068, 1132), "the flag gate drops the add");
        assert!(t.is_empty());
        assert_eq!(t.active_subtitle(184), None);
        assert_eq!(t.active_subtitle(100000), None);
    }

    /// `Text_FlushAllSubtitles` [M 0x1b15c8] at the skip tick: the head stops there, the queued
    /// lines die; later pushes start fresh from the clamped end.
    #[test]
    fn flush_stops_the_head_and_drops_the_queue() {
        let mut t = SubtitleTrack::new(true);
        assert!(t.push(184, 0x0600_0068, 1132));
        assert!(t.push(200, 0x0600_0069, 100));
        t.flush(400);
        assert_eq!(t.active_subtitle(399), Some(0x0600_0068), "the head was displaying at 399");
        assert_eq!(t.active_subtitle(400), None, "the flush kills it at 400");
        assert_eq!(t.active_subtitle(100000), None, "the queued line died with it");
        // A later push chains from the clamped end.
        assert!(t.push(500, 0x0600_0070, 10));
        assert_eq!(t.active_subtitle(500), Some(0x0600_0070));
    }

    /// `from_script` walks a single-stream script's Wait + Text commands with `Script_Run`'s clock
    /// (a command after `Wait(t)` fires on tick `t + 1` [M the pre-increment clock]) and the FIFO
    /// chain orders same-tick texts.
    #[test]
    fn from_script_walks_waits_and_texts() {
        // op 5 Wait(183), op 0x1e Text(label 0x06000068, time 1132), op 4 End — the disc's
        // 0x06000961 opening (fix-script-commands_live's `1e 68 00 00 06 6c 04 1e` [D]).
        let cmds: Vec<u8> = vec![0x05, 0x00, 0xb7, 0x00, 0x1e, 0x06, 0x00, 0x00, 0x68, 0x04, 0x6c, 0x01, 0x04, 0x00, 0x2c, 0x01];
        let nis = Nis { hashcode: 0x0600_0961, camera: false, loop_start: 0, length: 300, streams: vec![nf_formats::nis::Stream { first_key: 0, last_key: 0, commands: cmds }], keys: Vec::new() };
        let t = SubtitleTrack::from_script(&nis, true);
        assert_eq!(t.active_subtitle(183), None, "sister: the Wait(183) releases on the next tick");
        assert_eq!(t.active_subtitle(184), Some(0x0600_0068));
        assert_eq!(t.active_subtitle(184 + 1132 - 1), Some(0x0600_0068));
        assert_eq!(t.active_subtitle(184 + 1132), None);
        // Off → empty.
        assert!(SubtitleTrack::from_script(&nis, false).is_empty());
    }

    /// The movie-code → script table [M 0x1d6360:40-99]: all 14 entries and the two absences the
    /// game's chain implies (0x710004a has no intro; unknown codes none).
    #[test]
    fn intro_script_table_is_the_games_switch() {
        for (code, script) in [
            (0x0710_0001, 0x0600_095a), (0x0710_0005, 0x0600_0961), (0x0710_0009, 0x0600_0963),
            (0x0710_000c, 0x0600_0965), (0x0710_0011, 0x0600_0967), (0x0710_0014, 0x0600_0969),
            (0x0710_0018, 0x0600_096b), (0x0730_0004, 0x0600_095d), (0x0730_0008, 0x0600_0962),
            (0x0730_000b, 0x0600_0964), (0x0730_000d, 0x0600_0966), (0x0730_0013, 0x0600_0968),
            (0x0730_0017, 0x0600_097a), (0x0730_001b, 0x0600_096c),
        ] {
            assert_eq!(intro_script_for(code), Some(script), "code {code:08x}");
        }
        assert_eq!(intro_script_for(0x0710_004a), None, "0x0700004a wraps to it but the table has no row");
        assert_eq!(intro_script_for(0x0730_000f), None, "not in the chain (and not on the disc)");
        assert_eq!(intro_script_for(0x0700_0005), None, "a plain level code is not a movie code");
    }
}
