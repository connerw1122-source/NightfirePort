//! MissionData record for level 0x07000013 (part 3 of mission 0x07000011)
//! [L ee_start_05.bin @ 0x2a4200 — 1 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000013,
    mission: 0x07000011,
    part: 3,
    objectives: &[
        Objective { text: 0x0400003a, fail: 0xffffffff, hint: 0x04000040, channel: 0x50, init: 0, enable: 0x4a, flags: 0x00 }, // "Raise security gate and escape through front entrance"
    ],
};
