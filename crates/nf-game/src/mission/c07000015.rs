//! MissionData record for level 0x07000015 (part 1 of mission 0x07000014)
//! [L ee_start_05.bin @ 0x2a42d8 — 1 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000015,
    mission: 0x07000014,
    part: 1,
    objectives: &[
        Objective { text: 0x0400004d, fail: 0xffffffff, hint: 0x04000054, channel: 0x8e, init: 0, enable: 0x4b, flags: 0x00 }, // "Find the exit to Omega Sector"
    ],
};
