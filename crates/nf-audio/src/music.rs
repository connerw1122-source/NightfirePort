//! The IOP music stream player (`SFX.IRX`: `SFXStartMusic`, `UpdateMusicMarkers` 0xb560,
//! `SFXJumpToMusicMarkerUpdate` 0x75fc, `SFXCheckStreamJumpInstant` 0x72d4, `ES_CheckFileBuffersForJumpMarkers` 0xc2c8,
//! `ES_MusicSetVolume` 0x8218). Spec: docs/systems/music.md.
//!
//! The PS2 streams the `.SSD` through a 0x7800-byte (interleaved) SPU ring, one file buffer per ring lap (8 buffers,
//! up to 7 read ahead). The port keeps the same buffer plan — each buffer is the rest of a ring lap, cut at loop / goto
//! / end / jump markers exactly as `UpdateMusicMarkers` cuts it — and plays the buffers back to back: buffer p is
//! audible, p + 1 is being written to the ring ("Transfer"), later ones are read ahead. Deferred jumps cut at the
//! last jump-eligible marker of the first buffer after Transfer; instant jumps start 0x4000 ring bytes (0.448 s) ahead
//! of the playhead.

use crate::Cmd;
use nf_formats::music::{Smf, SsdDecoder};
use std::collections::VecDeque;
use std::sync::Arc;

/// The ring: 0x7800 interleaved bytes = 26 880 samples per channel (0.84 s at 32 kHz).
pub const RING: i32 = 0x7800;
/// Interleaved SSD bytes → samples per channel (0x100 bytes = 224 samples).
pub fn samples(bytes: i32) -> u64 { (bytes.max(0) as u64) * 7 / 8 }
fn align(p: i32) -> i32 { p & !0xff }

#[derive(Clone, Debug)]
struct Buf { file_pos: i32, size: i32, start: u64, silent: bool, marker_passed: Option<i32>, ring_off: i32 }

/// One music stream (`Streams[0]`).
pub struct Player {
    smf: Smf,
    ssd: Arc<Vec<u8>>,
    bufs: VecDeque<Buf>,
    dec: SsdDecoder,
    /// Next read: file position, marker scan index, ring offset, absolute start sample.
    file_pos: i32,
    marker: usize,
    ring_off: i32,
    next_start: u64,
    /// Music samples handed to the output (everything before is queued there).
    sent: u64,
    /// `StreamJumpTo`, `IOPLastJump`, `IOPNowPlaying` (the section reported to the EE: the last jump target).
    jump_to: i32,
    pub last_jump: i32,
    pub now_playing: i32,
    /// The latched request (`UpdateHashCode`, `UpdateInstantPlus`), re-evaluated every frame.
    req: (i32, bool),
    /// `HasJustJumped`: non-instant requests wait until the playhead passes this sample.
    lockout: u64,
    /// A jump becomes `IOPNowPlaying` when the playhead passes its cut.
    complete: Option<(u64, i32)>,
    ending_soon: bool,
    stop_at: Option<u64>,
    pub stopped: bool,
    /// `BaseVolume` from the SMF header.
    pub base_volume: i32,
}

impl Player {
    /// `SFXStartMusic(track, section)`: the stream from `section`'s start.
    pub fn start(smf: Smf, ssd: Arc<Vec<u8>>, section: i32, out: &mut Vec<Cmd>) -> Player {
        let st = smf.starts.get(section.max(0) as usize).copied().unwrap_or(nf_formats::music::SectionStart { pos: 0, marker: 0, instant: true });
        out.push(Cmd::MusicStop);
        let base_volume = smf.base_volume;
        let mut p = Player { smf, ssd, bufs: VecDeque::new(), dec: SsdDecoder::default(), file_pos: align(st.pos), marker: st.marker.max(0) as usize,
            ring_off: 0, next_start: 0, sent: 0, jump_to: -1, last_jump: section, now_playing: section, req: (-1, false),
            lockout: samples(RING), complete: None, ending_soon: false, stop_at: None, stopped: false, base_volume };
        p.update(0, out);
        p
    }

    /// `SFXJumpToMusicMarker(section, instant)`: latched; acted on by the next updates.
    pub fn request(&mut self, section: i32, instant: bool) { self.req = (section, instant); }

    /// `UpdateMusicMarkers`: plans the next buffer from the current file position.
    fn plan(&mut self) {
        let size = (RING - self.ring_off).min(RING);
        if self.ending_soon {
            // After an end marker the "reads" are 0x1800 bytes of silence.
            let size = size.min(0x1800);
            self.push(Buf { file_pos: self.file_pos, size, start: self.next_start, silent: true, marker_passed: None, ring_off: self.ring_off });
            return;
        }
        let m = &self.smf.markers;
        while self.marker < m.len() && m[self.marker].pos < self.file_pos { self.marker += 1; }
        let (mut bsize, mut next, mut passed) = (size, self.file_pos + size, None);
        let mut i = self.marker;
        while i < m.len() && m[i].pos < self.file_pos + size {
            let mk = m[i];
            let cut = (align(mk.pos) - self.file_pos).max(0);
            if mk.mtype == 9 || mk.mtype == 5 {
                bsize = cut;
                self.ending_soon = true;
                self.stop_at = Some(self.next_start + samples(cut));
                next = self.file_pos + cut;
                break;
            }
            if self.jump_to >= 0 && mk.mtype != 7 {
                // A pending deferred jump executes at this marker.
                let t = self.jump_to;
                self.last_jump = t;
                bsize = cut;
                let st = self.smf.starts.get(t as usize).copied();
                next = st.map_or(self.file_pos + cut, |s| align(s.pos));
                i = st.map_or(i, |s| s.marker.max(0) as usize);
                self.jump_to = -1;
                self.complete = Some((self.next_start + samples(cut), t));
                self.marker = i;
                return self.finish(bsize, next, passed);
            }
            if mk.mtype != 7 { passed = Some(cut); }
            if mk.mtype == 6 || mk.mtype == 7 {
                bsize = cut;
                next = align(mk.loop_start);
                self.marker = mk.loop_marker.max(0) as usize;
                return self.finish(bsize, next, passed);
            }
            i += 1;
        }
        self.marker = i;
        self.finish(bsize, next, passed)
    }

    fn finish(&mut self, bsize: i32, next: i32, passed: Option<i32>) {
        self.push(Buf { file_pos: self.file_pos, size: bsize, start: self.next_start, silent: false, marker_passed: passed, ring_off: self.ring_off });
        self.file_pos = next;
    }

    fn push(&mut self, b: Buf) {
        self.next_start = b.start + samples(b.size);
        self.ring_off = (b.ring_off + b.size) % RING;
        self.bufs.push_back(b);
    }

    /// One IOP frame (`SFXUpdateStreams`): `played` = music samples the output has played.
    pub fn update(&mut self, played: u64, out: &mut Vec<Cmd>) {
        if self.stopped { return; }
        if let Some((at, s)) = self.complete { if played >= at { self.now_playing = s; self.lockout = at + samples(RING); self.complete = None; } }
        if self.stop_at.is_some_and(|s| played >= s) { self.stopped = true; out.push(Cmd::MusicStop); return; }
        // The playing buffer p (front after dropping what has been played); Transfer = p + 1.
        while self.bufs.len() > 1 && self.bufs[1].start <= played { self.bufs.pop_front(); }
        // `SFXJumpToMusicMarkerUpdate`.
        let (req, inst) = self.req;
        if req >= 0 && req != self.last_jump && req != self.now_playing && !(played < self.lockout && !inst) {
            self.jump_to = req;
            let target = self.smf.starts.get(req as usize).copied();
            if target.is_some_and(|t| t.instant) || inst {
                if !self.ending_soon {
                    if let Some(t) = target { self.instant(played, req, t, out); }
                }
                self.jump_to = -1;
            }
        }
        // `ES_CheckFileBuffersForJumpMarkers`: a deferred jump cuts the first read-ahead buffer after Transfer that has
        // passed an eligible marker, at its last one; the buffers after it are re-read from the target.
        if self.jump_to >= 0 {
            if let Some(k) = (2..self.bufs.len()).find(|&k| self.bufs[k].marker_passed.is_some() && !self.bufs[k].silent) {
                let t = self.jump_to;
                let cut = self.bufs[k].marker_passed.unwrap();
                self.bufs.truncate(k + 1);
                let b = &mut self.bufs[k];
                b.size = cut;
                let (start, ring_off) = (b.start, b.ring_off);
                self.next_start = start + samples(cut);
                self.ring_off = (ring_off + cut) % RING;
                let st = self.smf.starts[t as usize];
                self.file_pos = align(st.pos);
                self.marker = st.marker.max(0) as usize;
                self.last_jump = t;
                self.jump_to = -1;
                self.ending_soon = false;
                self.stop_at = None;
                self.complete = Some((start + samples(cut), t));
            }
        }
        // Read ahead: up to 7 buffers after the playing one (an end stops the reads after its silence).
        let mut guard = 0;
        while self.bufs.len() < 8 && guard < 64 && !(self.ending_soon && self.bufs.back().is_some_and(|b| b.silent)) { self.plan(); guard += 1; }
        // Hand the playing and the Transfer buffer's audio to the output.
        let upto = self.bufs.get(1).map_or(self.next_start, |b| b.start + samples(b.size));
        self.send_until(upto, out);
    }

    /// `SFXCheckStreamJumpInstant`: the target's data replaces everything from 0x4000 ring bytes ahead of the
    /// playhead (0x2000 per channel = 14 336 samples, 0.448 s).
    fn instant(&mut self, played: u64, section: i32, t: nf_formats::music::SectionStart, out: &mut Vec<Cmd>) {
        let at = played + 14_336;
        self.last_jump = section;
        // Keep the buffers before `at`, the one containing it cut there.
        while self.bufs.back().is_some_and(|b| b.start >= at && self.bufs.len() > 1) { self.bufs.pop_back(); }
        if let Some(b) = self.bufs.back_mut() { if b.start + samples(b.size) > at { b.size = (((at - b.start) * 8 / 7) as i32) & !0xff; } }
        let end = self.bufs.back().map_or(at, |b| b.start + samples(b.size));
        if self.sent > end { out.push(Cmd::MusicTruncate(end)); self.sent = end; }
        self.next_start = end;
        self.file_pos = align(t.pos);
        self.marker = t.marker.max(0) as usize;
        self.ring_off = 0;
        self.ending_soon = false;
        self.stop_at = None;
        // `ES_JumpRequestComplete` when the new data is in the ring: reported at once; non-instant requests wait a lap.
        self.now_playing = section;
        self.complete = None;
        self.lockout = end + samples(RING);
    }

    fn send_until(&mut self, upto: u64, out: &mut Vec<Cmd>) {
        let mut pcm = Vec::new();
        for b in &self.bufs {
            let (s, e) = (b.start, b.start + samples(b.size));
            if e <= self.sent || s >= upto { continue; }
            if b.size <= 0 { continue; }
            let from = self.sent.max(s);
            if from != s { continue; } // buffers are sent whole
            if b.silent { pcm.extend(std::iter::repeat_n([0i16; 2], samples(b.size) as usize)); } else {
                let lo = b.file_pos.max(0) as usize;
                let hi = (lo + b.size as usize).min(self.ssd.len());
                if lo < hi { pcm.extend(self.dec.decode(&self.ssd[lo..hi])); }
            }
            self.sent = e;
        }
        if !pcm.is_empty() { out.push(Cmd::MusicAppend(pcm)); }
    }

    /// `ES_MusicSetVolume`: `v = master · ducker · base / 10000 · fade / 10000`, level = v² / 100, each channel at
    /// GSV(level) / 0x3FFF.
    pub fn gain(&self, master: i32, ducker: i32, fade: i32) -> f32 {
        let v = (master * ducker * self.base_volume) / 10000;
        let v = (v * fade) / 10000;
        let level = (v * v) / 100;
        (((level * 0x3fff) / 100).clamp(0, 0x3fff)) as f32 / 0x3fff as f32
    }
}
