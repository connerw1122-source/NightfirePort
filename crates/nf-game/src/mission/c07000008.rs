//! MissionData record for level 0x07000008 (part 3 of mission 0x07000005)
//! [L ee_start_05.bin @ 0x2a3e88 — 2 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000008,
    mission: 0x07000005,
    part: 3,
    objectives: &[
        Objective { text: 0x04000004, fail: 0xffffffff, hint: 0x0400005f, channel: 0x4a, init: 0, enable: 0x49, flags: 0x00 }, // "Retrieve guidance chip from "Safe Room""
        Objective { text: 0x04000005, fail: 0xffffffff, hint: 0x04000060, channel: 0x4b, init: 0, enable: 0x4a, flags: 0x00 }, // "Escape with Zoe in gondola"
    ],
};
