//! MissionData record for level 0x0700000f (part 1 of mission 0x0700000e)
//! [L ee_start_05.bin @ 0x2a4170 — 1 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x0700000f,
    mission: 0x0700000e,
    part: 1,
    objectives: &[
        Objective { text: 0x04000045, fail: 0xffffffff, hint: 0xffffffff, channel: 0x00, init: 0, enable: 0x00, flags: 0x00 }, // "Mission Objective 5"
    ],
};
