//! MissionData record for level 0x0700000d (part 1 of mission 0x0700000c)
//! [L ee_start_05.bin @ 0x2a40c8 — 3 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x0700000d,
    mission: 0x0700000c,
    part: 1,
    objectives: &[
        Objective { text: 0x0400002f, fail: 0xffffffff, hint: 0x04000064, channel: 0x1d, init: 0, enable: 0x23, flags: 0x00 }, // "Investigate building ST- 3 "
        Objective { text: 0x04000030, fail: 0xffffffff, hint: 0x04000065, channel: 0x1e, init: 0, enable: 0x1d, flags: 0x00 }, // "Photograph the Laser Prototype"
        Objective { text: 0x04000031, fail: 0xffffffff, hint: 0x04000066, channel: 0x1f, init: 0, enable: 0x1d, flags: 0x00 }, // "Destroy door C5 with mounted laser on catwalk"
    ],
};
