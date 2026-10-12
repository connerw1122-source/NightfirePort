//! MissionData record for level 0x07000010 (part 0 of mission 0x07000010)
//! [L ee_start_05.bin @ 0x2a4218 — 3 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000010,
    mission: 0x07000010,
    part: 0,
    objectives: &[
        Objective { text: 0x04000046, fail: 0xffffffff, hint: 0xffffffff, channel: 0x00, init: 0, enable: 0x00, flags: 0x00 }, // "Mission Objective 1"
        Objective { text: 0x04000047, fail: 0xffffffff, hint: 0xffffffff, channel: 0x00, init: 0, enable: 0x00, flags: 0x00 }, // "Mission Objective 2"
        Objective { text: 0x04000048, fail: 0xffffffff, hint: 0xffffffff, channel: 0x00, init: 0, enable: 0x00, flags: 0x00 }, // "Mission Objective 3"
    ],
};
