//! MissionData record for level 0x07000011 (part 0 of mission 0x07000011)
//! [L ee_start_05.bin @ 0x2a4188 — 2 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000011,
    mission: 0x07000011,
    part: 0,
    objectives: &[
        Objective { text: 0x04000035, fail: 0xffffffff, hint: 0x0400003b, channel: 0x64, init: 0, enable: 0x00, flags: 0x00 }, // "Escape from the tower"
        Objective { text: 0x04000036, fail: 0xffffffff, hint: 0x0400003c, channel: 0x48, init: 0, enable: 0x00, flags: 0x00 }, // "Gain access to lower office floor"
    ],
};
