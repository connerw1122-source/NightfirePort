//! MissionData record for level 0x07000006 (part 1 of mission 0x07000005)
//! [L ee_start_05.bin @ 0x2a3e40 — 1 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000006,
    mission: 0x07000005,
    part: 1,
    objectives: &[
        Objective { text: 0x04000001, fail: 0xffffffff, hint: 0x0400005c, channel: 0x02, init: 0, enable: 0x46, flags: 0x00 }, // "Find a way inside to the party"
    ],
};
