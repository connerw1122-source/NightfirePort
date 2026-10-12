//! MissionData record for level 0x07000012 (part 1 of mission 0x07000011)
//! [L ee_start_05.bin @ 0x2a41b8 — 2 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000012,
    mission: 0x07000011,
    part: 1,
    objectives: &[
        Objective { text: 0x04000037, fail: 0xffffffff, hint: 0x0400003d, channel: 0x49, init: 0, enable: 0x48, flags: 0x00 }, // "Activate elevator override control in security center"
        Objective { text: 0x04000038, fail: 0xffffffff, hint: 0x0400003e, channel: 0x4d, init: 0, enable: 0x49, flags: 0x00 }, // "Escape through the elevator shaft"
    ],
};
