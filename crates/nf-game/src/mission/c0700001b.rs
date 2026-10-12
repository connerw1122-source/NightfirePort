//! MissionData record for level 0x0700001b (part 0 of mission 0x0700001b)
//! [L ee_start_05.bin @ 0x2a4320 — 2 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x0700001b,
    mission: 0x0700001b,
    part: 0,
    objectives: &[
        Objective { text: 0x04000059, fail: 0xffffffff, hint: 0x04000067, channel: 0x09, init: 0, enable: 0x00, flags: 0x00 }, // "Disable the missiles before they are launched"
        Objective { text: 0x0400005a, fail: 0xffffffff, hint: 0x04000068, channel: 0x0b, init: 0, enable: 0x09, flags: 0x00 }, // "Defeat Drake before the base goes critical"
    ],
};
