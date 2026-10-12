//! MissionData record for level 0x07000014 (part 0 of mission 0x07000014)
//! [L ee_start_05.bin @ 0x2a4260 — 5 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x07000014,
    mission: 0x07000014,
    part: 0,
    objectives: &[
        Objective { text: 0x04000049, fail: 0xffffffff, hint: 0x04000050, channel: 0x47, init: 0, enable: 0x00, flags: 0x00 }, // "Infiltrate Drake's facility undetected"
        Objective { text: 0x0400004a, fail: 0xffffffff, hint: 0x04000051, channel: 0x48, init: 0, enable: 0x47, flags: 0x00 }, // "Follow Kiko to the Security Control Center"
        Objective { text: 0x0400004b, fail: 0xffffffff, hint: 0x04000052, channel: 0x4a, init: 0, enable: 0x48, flags: 0x00 }, // "Sabotage base security and escape from room"
        Objective { text: 0x0400004c, fail: 0xffffffff, hint: 0x04000053, channel: 0x54, init: 0, enable: 0x4a, flags: 0x00 }, // "Prevent Rook from cutting the power to Delta Sector"
        Objective { text: 0x04000069, fail: 0xffffffff, hint: 0x0400006a, channel: 0xce, init: 0, enable: 0x54, flags: 0x00 }, // "Take the exit to Delta Sector"
    ],
};
