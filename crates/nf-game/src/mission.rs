//! The mission/objective layer (`Mission_Init` 0x182270, `Mission_MonitorObjectives` 0x1827d8,
//! `Mission_Update` 0x182c60). The objective table itself is game data: 18 records of 0x28 bytes at
//! EE 0x2a4354, each { level code, order, objective-array pointer, count }, each objective 0x18 bytes
//! { label u32, fail-label ptr u32, +0x0C complete channel u8, +0x0F reveal channel u8, +0x10 flags
//! u8 (bit 1 = inverted), +0x14 state u32 } — read from `work/ee_start_05.bin` (file offset = EE
//! address) [L]. This file holds the whole table (the other map sessions build on it); the tick
//! monitor is called from `Drones::tick` for every level exactly as the game calls
//! `Mission_MonitorObjectives` from `Mission_Update`'s state 1 [M 0x182c60:17].
//!
//! Semantics per objective [M 0x1827d8:33-126]:
//! - state 1 (hidden) → the reveal channel fires → state 3 (visible);
//! - state 3 (visible) → the complete channel fires → "Objective Complete" (0x2000053) → state 4 (done);
//! - an INVERTED objective (flags bit 1) fails — sets **ch 99** + label 0x170 "Objective Failed" — when its
//!   channel turns ON while visible (state 2);
//! - the all-done flag ANDs every objective's channel condition **live** (a completed objective's channel
//!   must still be on) and, when all hold, sets **ch 100** + `Music_Event(8,1)` [M 0x1827d8:155-160];
//! - ch 0x61 → fail label 0x4000032 ("Alarm Raised") + ch 99; ch 0x60 → 0x4000034 ("Killed Civilians") +
//!   ch 99 [M 0x1827d8:128-140];
//! - `Mission_Update` consumes ch 99 (fail → "Mission Failed" 0x2000007, end to 0x7000048) and ch 100
//!   (complete → "Mission Complete" 0x2000006, `LevelToEndTo` per level: **0x7000008 → 0x7300008** (the
//!   driving hand-off), most levels → 0x7000048) [M 0x182c60:17-116].
//!
//! The port models the monitor + the level-end mapping; the HUD messages (Text_AddMsg) and the fail
//! screens (MenuManager_Create) are presentation and stay with the HUD session.

use crate::triggers::Channels;

#[derive(Clone, Copy, Debug)]
pub struct Objective {
    /// `Txt_BindLabel` id (the USATxt objective title).
    pub label: u32,
    /// `+0x0C`: the channel that completes the objective (or, inverted, fails it).
    pub complete_ch: u8,
    /// `+0x0F`: the channel that reveals a hidden (state 1) objective.
    pub reveal_ch: u8,
    /// `+0x10 & 2`: the objective's channel condition is inverted.
    pub inverted: bool,
    /// `+0x14`: the runtime state (1 hidden, 2/3 active, 4 done, 5 failed) — starts at the snapshot's.
    pub state: u8,
}

#[derive(Clone, Debug)]
pub struct Record {
    /// The mission's BaseMap (all of a mission's records carry the FIRST level's code: the Mayhew
    /// records all read 0x07000005 [L] — the monitor walks every record whose code matches).
    pub level: u32,
    pub objectives: Vec<Objective>,
}

/// The 18-record table [L ee_start_05.bin @0x2a4354, dumped 2026-10-12].
pub fn table() -> Vec<Record> {
    let o = |label: u32, complete_ch: u8, reveal_ch: u8, inverted: bool, state: u8| Objective { label, complete_ch, reveal_ch, inverted, state };
    vec![
        // The Mayhew estate (07000005-08): the truck park → the dressing scene → the party hall →
        // the gondola intro plate → the boarding → the ride.
        Record { level: 0x0700_0005, objectives: vec![o(0x4000000, 0x70, 0x45, false, 1)] },
        Record { level: 0x0700_0005, objectives: vec![o(0x4000001, 0x02, 0x70, false, 1)] },
        Record { level: 0x0700_0005, objectives: vec![o(0x4000002, 0x48, 0x47, false, 1), o(0x4000003, 0x49, 0x48, false, 1)] },
        Record { level: 0x0700_0005, objectives: vec![o(0x4000004, 0x4a, 0x49, false, 1), o(0x4000005, 0x4b, 0x4a, false, 1)] },
        // The Paris mission (07000001-04).
        Record { level: 0x0700_0001, objectives: vec![o(0x4000006, 0x47, 0x0, false, 3)] },
        Record { level: 0x0700_0001, objectives: vec![o(0x4000007, 0x48, 0x47, false, 1), o(0x4000008, 0x4b, 0x47, false, 1), o(0x4000009, 0x4d, 0x4c, false, 1)] },
        Record { level: 0x0700_0001, objectives: vec![o(0x400000a, 0x4e, 0x4d, false, 1), o(0x400000b, 0x51, 0x4d, false, 1), o(0x400000c, 0x53, 0x52, false, 1)] },
        Record { level: 0x0700_0001, objectives: vec![o(0x400000d, 0x54, 0x53, false, 1)] },
        // The Tower mission (07000009-0b).
        Record { level: 0x0700_0009, objectives: vec![o(0x4000016, 0x61, 0x0, true, 0), o(0x4000017, 0x46, 0x0, false, 0), o(0x4000018, 0x47, 0x46, false, 0)] },
        Record { level: 0x0700_0009, objectives: vec![o(0x4000019, 0x48, 0x47, false, 0), o(0x400001a, 0x49, 0x47, false, 0), o(0x400001b, 0x4b, 0x4a, false, 0)] },
        Record { level: 0x0700_0009, objectives: vec![o(0x400001c, 0x4c, 0x4b, false, 0), o(0x400001d, 0x4d, 0x4b, false, 0), o(0x400001e, 0x4e, 0x4d, false, 0), o(0x400001f, 0x59, 0x4f, false, 0), o(0x4000020, 0x5a, 0x59, false, 0)] },
        // The Power Station (0700000c/d).
        Record { level: 0x0700_000c, objectives: vec![o(0x400002c, 0x1b, 0x0, false, 0), o(0x400002d, 0x1c, 0x1b, false, 0), o(0x400002e, 0x23, 0x1c, false, 0)] },
        Record { level: 0x0700_000c, objectives: vec![o(0x400002f, 0x1d, 0x23, false, 0), o(0x4000030, 0x1e, 0x1d, false, 0), o(0x4000031, 0x1f, 0x1d, false, 0)] },
        // 0700000e.
        Record { level: 0x0700_000e, objectives: vec![o(0x4000041, 0xf, 0x0, false, 0), o(0x4000042, 0xa, 0x7, false, 0), o(0x4000043, 0xb, 0x8, false, 0), o(0x4000044, 0xc, 0x9, false, 0)] },
        Record { level: 0x0700_000e, objectives: vec![o(0x4000045, 0x0, 0x0, false, 0)] },
        // The Tower 2 mission (07000011+).
        Record { level: 0x0700_0011, objectives: vec![o(0x4000035, 0x64, 0x0, false, 0), o(0x4000036, 0x48, 0x0, false, 0)] },
        Record { level: 0x0700_0011, objectives: vec![o(0x4000037, 0x49, 0x48, false, 0), o(0x4000038, 0x4d, 0x49, false, 0)] },
        Record { level: 0x0700_0011, objectives: vec![o(0x4000039, 0x4a, 0x4d, false, 0)] },
    ]
}

/// The monitor's output for one tick — what `Drones::tick` applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    None,
    /// ch 100: every objective's channel condition holds [M 0x1827d8:155-160].
    Complete,
    /// ch 99: an inverted objective's channel turned on, or the alarm (0x61) / civilians (0x60)
    /// fail channels fired [M 0x1827d8:73-141].
    Fail,
}

/// `Mission_MonitorObjectives` (0x1827d8) for one tick. `level` is `BaseMap` (the mission's first
/// level code; the port's `Drones::level` is the loaded level, which for the Mayhew chain equals
/// BaseMap only on 07000005 — the records carry 0x07000005 for all four levels, so the caller passes
/// the level and the match uses it directly, as the game compares `record.level == BaseMap`).
pub fn monitor(records: &mut [Record], level: u32, channels: &Channels) -> Outcome {
    let mut all = true;
    let mut outcome = Outcome::None;
    for rec in records.iter_mut().filter(|r| r.level == level) {
        for ob in rec.objectives.iter_mut() {
            let cond = if ob.inverted { !channels.on(ob.complete_ch as u32) } else { channels.on(ob.complete_ch as u32) };
            match ob.state {
                // State 1: hidden — the reveal channel shows it [M 0x1827d8:52-66].
                1 if ob.reveal_ch != 0 && channels.on(ob.reveal_ch as u32) => ob.state = if ob.inverted { 2 } else { 3 },
                // State 2: inverted and visible — its channel turning ON fails the mission
                // [M 0x1827d8:73-87].
                2 if !cond => { outcome = Outcome::Fail; ob.state = 5; }
                // State 3: visible — the complete channel completes it [M 0x1827d8:88-99].
                3 if !ob.inverted && channels.on(ob.complete_ch as u32) => ob.state = 4,
                _ => {}
            }
            // The live AND: every objective's channel condition must hold right now
            // [M 0x1827d8:44-51, bVar9 &= bVar1 per objective per tick].
            if !cond { all = false; }
        }
    }
    // The alarm / civilians fail channels [M 0x1827d8:128-141].
    if channels.on(0x61) || channels.on(0x60) { outcome = Outcome::Fail; }
    if all { return Outcome::Complete; }
    outcome
}

/// The mission's BaseMap for a level: every record of a mission carries the FIRST level's code
/// ([L] the Mayhew records all read 0x07000005), and `Mission_MonitorObjectives` matches
/// `record.level == BaseMap` [M 0x1827d8:34] — so the monitor runs against the base, not the
/// current level. The chains follow the story order (ResetMap_LevelCode2Img 0x1d5258) and the
/// record grouping above; the driving hand-offs in `level_end_to` confirm the membership of
/// 04/08/0b/0d/0f/13/16/1b [S: the chain ranges, pinned by the table's base codes].
pub fn base_map(level: u32) -> u32 {
    match level {
        0x0700_0001..=0x0700_0004 => 0x0700_0001,
        0x0700_0005..=0x0700_0008 => 0x0700_0005,
        0x0700_0009..=0x0700_000b => 0x0700_0009,
        0x0700_000c..=0x0700_000d => 0x0700_000c,
        0x0700_000e..=0x0700_000f => 0x0700_000e,
        0x0700_0011..=0x0700_001b => 0x0700_0011,
        l => l,
    }
}

/// `Mission_Update` state 6's `LevelToEndTo` switch [M 0x182c60:85-116]: the level a mission
/// complete hands off to. 0x7300008 and friends are the DRIVING levels (out of scope for the
/// on-foot port — the exit code is still the game's).
pub fn level_end_to(level: u32) -> u32 {
    match level {
        0x0700_0004 => 0x0730_0004,
        0x0700_0008 => 0x0730_0008,
        0x0700_000b => 0x0730_000b,
        0x0700_000d => 0x0730_000d,
        0x0700_000f => 0x0730_000f,
        0x0700_0013 => 0x0730_0013,
        0x0700_0016 => 0x0730_0017,
        0x0700_001b => 0x0730_001b,
        _ => 0x0700_0048,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chans(on: &[u8]) -> Channels {
        let mut c = Channels::default();
        let frame = 0;
        for &k in on { c.set(k as u32, 1, frame); }
        c
    }

    /// The Mayhew chain: each stage's channel reveals the next objective; the sixth completes the
    /// mission (ch 100) — the walkthrough's spine, from the [L] table.
    #[test]
    fn mayhew_chain_completes_in_order() {
        let mut recs = table();
        // Nothing on: everything hidden, no outcome.
        assert_eq!(monitor(&mut recs, 0x0700_0005, &chans(&[])), Outcome::None);
        assert_eq!(recs[0].objectives[0].state, 1, "objective 1 stays hidden before its reveal channel");
        // ch 0x45 reveals objective 1 (its writer is not on disc — likely the mission layer's own init;
        // the walkthrough sets it). The complete channel 0x70 alone leaves it hidden, per the state
        // machine [M 0x1827d8:52-66 — state 1 only leaves through the reveal channel].
        assert_eq!(monitor(&mut recs, 0x0700_0005, &chans(&[0x70])), Outcome::None);
        assert_eq!(recs[0].objectives[0].state, 1, "0x70 alone does not reveal or complete the hidden objective");
        assert_eq!(monitor(&mut recs, 0x0700_0005, &chans(&[0x45])), Outcome::None);
        assert_eq!(recs[0].objectives[0].state, 3, "0x45 reveals objective 1");
        // ch 0x70 (the truck park) completes the revealed objective and reveals the second.
        assert_eq!(monitor(&mut recs, 0x0700_0005, &chans(&[0x45, 0x70])), Outcome::None);
        assert_eq!(recs[0].objectives[0].state, 4, "0x70 completes the first objective");
        assert_eq!(recs[1].objectives[0].state, 3, "0x70 reveals the second (reveal ch 0x70)");
        // The rest of the chain in order: each stage's channel reveals/completes its objective; the
        // mission stays incomplete until the last one.
        let mut on = vec![0x45u8, 0x70];
        for ch in [0x02u8, 0x48, 0x49, 0x4a] {
            on.push(ch);
            assert_eq!(monitor(&mut recs, 0x0700_0005, &chans(&on)), Outcome::None, "ch {ch:#x}");
        }
        // All six on: complete.
        on.push(0x4b);
        assert_eq!(monitor(&mut recs, 0x0700_0005, &chans(&on)), Outcome::Complete);
        assert_eq!(level_end_to(0x0700_0008), 0x0730_0008, "the gondola hands off to the driving level [M 0x182c60:100]");
    }

    /// Sister: skipping a stage leaves the mission incomplete (ch 100 never fires).
    #[test]
    fn mayhew_chain_skipping_a_stage_stays_incomplete() {
        let mut recs = table();
        // 0x48 without 0x49: the hall walk-in done, the gondola plate skipped.
        assert_eq!(monitor(&mut recs, 0x0700_0005, &chans(&[0x70, 0x02, 0x48])), Outcome::None);
        // The alarm channel fails the mission regardless of progress [M 0x1827d8:128-131].
        let mut recs = table();
        assert_eq!(monitor(&mut recs, 0x0700_0005, &chans(&[0x61])), Outcome::Fail);
    }
}
