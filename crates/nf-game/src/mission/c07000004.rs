//! MissionData record for level 0x07000004 (part 3 of mission 0x07000001)
//! [L ee_start_05.bin @ 0x2a3f60 — 1 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000004,
    mission: 0x07000001,
    part: 3,
    objectives: &[
        Objective { text: 0x0400000d, fail: 0xffffffff, hint: 0x04000015, channel: 0x54, init: 0, enable: 0x53, flags: 0x00 }, // "Defeat the assassin"
    ],
};
