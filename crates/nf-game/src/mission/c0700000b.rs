//! MissionData record for level 0x0700000b (part 2 of mission 0x07000009)
//! [L ee_start_05.bin @ 0x2a4008 — 5 objective record(s); docs/formats/mission-data.md;
//! the quoted texts resolved from USATxt.dat by the port's Lang::bind algorithm].
use super::{LevelDef, Objective};

pub static DEF: LevelDef = LevelDef {
    level: 0x0700000b,
    mission: 0x07000009,
    part: 2,
    objectives: &[
        Objective { text: 0x0400001c, fail: 0xffffffff, hint: 0x04000027, channel: 0x4c, init: 0, enable: 0x4b, flags: 0x00 }, // "Install Q-Worm on Mayhew's computer system"
        Objective { text: 0x0400001d, fail: 0xffffffff, hint: 0x04000028, channel: 0x4d, init: 0, enable: 0x4b, flags: 0x00 }, // "Locate the Secure Terminal Room"
        Objective { text: 0x0400001e, fail: 0xffffffff, hint: 0x04000029, channel: 0x4e, init: 0, enable: 0x4d, flags: 0x00 }, // "Retrieve the NightFire files from the Secure Terminal"
        Objective { text: 0x0400001f, fail: 0xffffffff, hint: 0x0400002a, channel: 0x59, init: 0, enable: 0x4f, flags: 0x00 }, // "Escape to the roof"
        Objective { text: 0x04000020, fail: 0xffffffff, hint: 0x0400002b, channel: 0x5a, init: 0, enable: 0x59, flags: 0x00 }, // "Acquire parachute from helicopter then jump off tower roof"
    ],
};
