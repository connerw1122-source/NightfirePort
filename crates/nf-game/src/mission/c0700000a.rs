//! MissionData record for level 0x0700000a (part 1 of mission 0x07000009)
//! [L ee_start_05.bin @ 0x2a3fc0 — 3 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x0700000a,
    mission: 0x07000009,
    part: 1,
    objectives: &[
        Objective { text: 0x04000019, fail: 0xffffffff, hint: 0x04000024, channel: 0x48, init: 0, enable: 0x47, flags: 0x00 }, // "Install Q-Worm on office computer systems"
        Objective { text: 0x0400001a, fail: 0xffffffff, hint: 0x04000025, channel: 0x49, init: 0, enable: 0x47, flags: 0x00 }, // "Find the security center and unlock exterior door"
        Objective { text: 0x0400001b, fail: 0xffffffff, hint: 0x04000026, channel: 0x4b, init: 0, enable: 0x4a, flags: 0x00 }, // "Find door to outer balcony"
    ],
};
