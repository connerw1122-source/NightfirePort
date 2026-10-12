//! MissionData record for level 0x0700000c (part 0 of mission 0x0700000c)
//! [L ee_start_05.bin @ 0x2a4080 — 3 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x0700000c,
    mission: 0x0700000c,
    part: 0,
    objectives: &[
        Objective { text: 0x0400002c, fail: 0xffffffff, hint: 0x04000061, channel: 0x1b, init: 0, enable: 0x00, flags: 0x00 }, // "Investigate building ST- 1 "
        Objective { text: 0x0400002d, fail: 0xffffffff, hint: 0x04000062, channel: 0x1c, init: 0, enable: 0x1b, flags: 0x00 }, // "Photograph the Jetpack Prototype "
        Objective { text: 0x0400002e, fail: 0xffffffff, hint: 0x04000063, channel: 0x23, init: 0, enable: 0x1c, flags: 0x00 }, // "Advance through Warehouse TR - 2"
    ],
};
