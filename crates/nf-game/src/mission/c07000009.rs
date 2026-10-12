//! MissionData record for level 0x07000009 (part 0 of mission 0x07000009)
//! [L ee_start_05.bin @ 0x2a3f78 — 3 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000009,
    mission: 0x07000009,
    part: 0,
    objectives: &[
        Objective { text: 0x04000016, fail: 0xffffffff, hint: 0x04000021, channel: 0x61, init: 0, enable: 0x00, flags: 0x02 }, // "Don't let the security guards raise the alarm"
        Objective { text: 0x04000017, fail: 0xffffffff, hint: 0x04000022, channel: 0x46, init: 0, enable: 0x00, flags: 0x00 }, // "Get to the lobby and activate the main elevator system"
        Objective { text: 0x04000018, fail: 0xffffffff, hint: 0x04000023, channel: 0x47, init: 0, enable: 0x46, flags: 0x00 }, // "Take main elevator to office floor"
    ],
};
