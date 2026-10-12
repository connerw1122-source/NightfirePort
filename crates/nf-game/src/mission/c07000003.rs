//! MissionData record for level 0x07000003 (part 2 of mission 0x07000001)
//! [L ee_start_05.bin @ 0x2a3f18 — 3 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000003,
    mission: 0x07000001,
    part: 2,
    objectives: &[
        Objective { text: 0x0400000a, fail: 0xffffffff, hint: 0x04000012, channel: 0x4e, init: 0, enable: 0x4d, flags: 0x00 }, // "Retrieve Dragon Safe contents"
        Objective { text: 0x0400000b, fail: 0xffffffff, hint: 0x04000013, channel: 0x51, init: 0, enable: 0x4d, flags: 0x00 }, // "Rescue Mayhew's servants"
        Objective { text: 0x0400000c, fail: 0xffffffff, hint: 0x04000014, channel: 0x53, init: 0, enable: 0x52, flags: 0x00 }, // "Find door leading to Mayhew"
    ],
};
