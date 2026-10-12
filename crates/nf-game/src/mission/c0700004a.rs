//! MissionData record for level 0x0700004a (part 2 of mission 0x07000011)
//! [L ee_start_05.bin @ 0x2a41e8 — 1 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x0700004a,
    mission: 0x07000011,
    part: 2,
    objectives: &[
        Objective { text: 0x04000039, fail: 0xffffffff, hint: 0x0400003f, channel: 0x4a, init: 0, enable: 0x4d, flags: 0x00 }, // "Gain access to lobby"
    ],
};
