//! MissionData record for level 0x07000002 (part 1 of mission 0x07000001)
//! [L ee_start_05.bin @ 0x2a3ed0 — 3 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000002,
    mission: 0x07000001,
    part: 1,
    objectives: &[
        Objective { text: 0x04000007, fail: 0xffffffff, hint: 0x0400000f, channel: 0x48, init: 0, enable: 0x47, flags: 0x00 }, // "Destroy Mayhew's computer"
        Objective { text: 0x04000008, fail: 0xffffffff, hint: 0x04000010, channel: 0x4b, init: 0, enable: 0x47, flags: 0x00 }, // "Rescue geisha girls"
        Objective { text: 0x04000009, fail: 0xffffffff, hint: 0x04000011, channel: 0x4d, init: 0, enable: 0x4c, flags: 0x00 }, // "Find door to servants quarters"
    ],
};
