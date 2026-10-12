//! MissionData record for level 0x07000007 (part 2 of mission 0x07000005)
//! [L ee_start_05.bin @ 0x2a3e58 — 2 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000007,
    mission: 0x07000005,
    part: 2,
    objectives: &[
        Objective { text: 0x04000002, fail: 0xffffffff, hint: 0x0400005d, channel: 0x48, init: 0, enable: 0x47, flags: 0x00 }, // "Rendezvous with undercover agents and maintain cover as party guest"
        Objective { text: 0x04000003, fail: 0xffffffff, hint: 0x0400005e, channel: 0x49, init: 0, enable: 0x48, flags: 0x00 }, // "Spy on secret meeting"
    ],
};
