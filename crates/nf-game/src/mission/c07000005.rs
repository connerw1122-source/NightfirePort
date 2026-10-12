//! MissionData record for level 0x07000005 (part 0 of mission 0x07000005)
//! [L ee_start_05.bin @ 0x2a3e28 — 1 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000005,
    mission: 0x07000005,
    part: 0,
    objectives: &[
        Objective { text: 0x04000000, fail: 0xffffffff, hint: 0x0400005b, channel: 0x46, init: 0, enable: 0x45, flags: 0x00 }, // "Breach the castle walls"
    ],
};
