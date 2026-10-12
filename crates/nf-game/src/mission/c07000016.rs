//! MissionData record for level 0x07000016 (part 2 of mission 0x07000014)
//! [L ee_start_05.bin @ 0x2a42f0 — 2 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000016,
    mission: 0x07000014,
    part: 2,
    objectives: &[
        Objective { text: 0x0400004e, fail: 0xffffffff, hint: 0x04000055, channel: 0x8f, init: 0, enable: 0x8e, flags: 0x00 }, // "Reach Drake's launch silo"
        Objective { text: 0x0400004f, fail: 0xffffffff, hint: 0x04000056, channel: 0x90, init: 0, enable: 0x8f, flags: 0x00 }, // "Prevent Kiko from launching into space"
    ],
};
