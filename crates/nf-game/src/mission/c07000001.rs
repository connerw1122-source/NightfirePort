//! MissionData record for level 0x07000001 (part 0 of mission 0x07000001)
//! [L ee_start_05.bin @ 0x2a3eb8 — 1 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000001,
    mission: 0x07000001,
    part: 0,
    objectives: &[
        Objective { text: 0x04000006, fail: 0xffffffff, hint: 0x0400000e, channel: 0x47, init: 0, enable: 0x00, flags: 0x00 }, // "Escort Mayhew to bunker"
    ],
};
