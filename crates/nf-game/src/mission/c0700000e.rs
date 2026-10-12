//! MissionData record for level 0x0700000e (part 0 of mission 0x0700000e)
//! [L ee_start_05.bin @ 0x2a4110 — 4 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x0700000e,
    mission: 0x0700000e,
    part: 0,
    objectives: &[
        Objective { text: 0x04000041, fail: 0xffffffff, hint: 0xffffffff, channel: 0x0f, init: 0, enable: 0x00, flags: 0x00 }, // "Disable all security measures before using the lift"
        Objective { text: 0x04000042, fail: 0xffffffff, hint: 0xffffffff, channel: 0x0a, init: 0, enable: 0x07, flags: 0x00 }, // "Disable system 1 of 3"
        Objective { text: 0x04000043, fail: 0xffffffff, hint: 0xffffffff, channel: 0x0b, init: 0, enable: 0x08, flags: 0x00 }, // "Disable system 2 of 3"
        Objective { text: 0x04000044, fail: 0xffffffff, hint: 0xffffffff, channel: 0x0c, init: 0, enable: 0x09, flags: 0x00 }, // "Disable system 3 of 3"
    ],
};
