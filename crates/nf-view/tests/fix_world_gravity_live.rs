//! fix-world-gravity: the game reads the global vector `WldGravity` (0x2d8890..0x2d889c, 16 B
//! [M work/action-symtab.tsv]) for every physics consumer; Bond's jump/fall (`Player_Collision`
//! 0x194df0:107,161) and the death-anim pick (`Player_ChangeSubState` 0x190630 case 0xd/0xe:
//! `WldGravity.y == 0.0 ? anim 0x60008e0 : anim 0x6000151` [M :68-73]) are the port's scope.
//! The per-level value is set at `Player_Init` (0x1910f8): the default write x=0 / y=−9.8 /
//! z=0 (0x2d8890/94/98; disasm 0x19151c-0x191550), then `level_tag+0x38` — the player-start
//! placement's **param 3** (the 0xB0 block `Player_Start` 0x190fd0 hands over, disasm
//! `lw a2,56(s5)` 0x191580) — picks the arm: **p3 == 2 → `Vec_Zero(&WldGravity)`** (disasm
//! 0x191588→0x1915f4→0x191618, `jal Vec_Zero` with a0 = s0 = 0x2d8890), everything else keeps
//! −9.8. These tests pin the port to that rule on the real input path.
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_game::player::{Bond, Input};

fn iso_path() -> Option<String> { std::env::var("NF_ISO").ok() }

/// One type-0x24 player start's param 3 (the zero-g selector) — `level_starts`' parse plus p3.
fn starts_p3(level: &Level) -> Vec<(usize, u32)> {
    let Some(m) = level.main_map() else { return Vec::new() };
    level.maps[m].placements.iter()
        .filter(|p| p.flags & 0xff == 0x24)
        .map(|p| (p.index as usize, p.params.iter().find(|(i, _)| *i == 3).map_or(0, |&(_, v)| v)))
        .collect()
}

/// The game's rule [M Player_Init 0x1910f8:152-172]: p3 == 2 zeroes the vector, else (0, −9.8, 0).
fn gravity_for_p3(p3: u32) -> [f32; 3] {
    if p3 == 2 { [0.0; 3] } else { [0.0, -9.8, 0.0] }
}

/// Every level archive on the disc: name → the type-0x24 starts' p3 values, straight from the
/// data the game reads (the placements the loader expands into the 0xB0 blocks). Prints the
/// table and pins its rows: 0700001b is the zero-g level (p3 == 2), 07000014 carries p3 == 3
/// (the third spawn arm), every other level keeps the default.
#[test]
fn disc_scan_player_start_param3() {
    let Some(path) = iso_path() else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).expect("FILES.BIN");
    let mut table: Vec<(String, Vec<(usize, u32)>)> = Vec::new();
    for e in &entries {
        let n = e.name.as_bytes();
        if n.len() != 12 || !e.name.ends_with(".bin") || !e.name[..4].eq_ignore_ascii_case("0700")
            || !e.name[4..8].chars().all(|c| c.is_ascii_hexdigit()) { continue; }
        let Ok(data) = iso.read_bytes(bin.lba as u64 * 2048 + e.offset as u64, e.size as u64) else { continue };
        let Ok(level) = Level::parse(&data) else { continue; };
        let starts = starts_p3(&level);
        if starts.is_empty() { continue; }
        eprintln!("level {}: starts p3 {:?}", e.name, starts.iter().map(|(i, p3)| format!("#{i}=p3 {p3}")).collect::<Vec<_>>());
        table.push((e.name.to_ascii_uppercase(), starts));
    }
    eprintln!("== {} levels with type-0x24 starts ==", table.len());
    let p3_of = |name: &str| table.iter().find(|(n, _)| n == name).map(|(_, s)| s[0].1);
    // 07000005 must keep the default: [L ee_start_05.bin @ 0x2d8894 = 0xc11ccccd = −9.8 (the
    // snapshot is post-Player_Init).
    assert_eq!(p3_of("07000005.BIN"), Some(0), "07000005's start is not the default arm");
    assert_eq!(gravity_for_p3(p3_of("07000005.BIN").unwrap()), [0.0, -9.8, 0.0]);
    // The zero-g level: p3 == 2 → Vec_Zero.
    assert_eq!(p3_of("0700001B.BIN"), Some(2), "0700001b is the disc's zero-g level (p3 == 2)");
    assert_eq!(gravity_for_p3(2), [0.0; 3]);
    // The third arm's level (spawn pose 4 + BL+0x90c = 0.45; gravity stays −9.8).
    assert_eq!(p3_of("07000014.BIN"), Some(3), "07000014's start takes the p3 == 3 arm");
    assert_eq!(gravity_for_p3(3), [0.0, -9.8, 0.0]);
    // Sister: a deliberately wrong expectation fails — p3 == 2 is NOT the default gravity.
    assert_ne!(gravity_for_p3(2), [0.0, -9.8, 0.0], "the wrong rule (p3 ignored) must fail here");
}

/// The [L] leg: the EE snapshot's WldGravity — file offset = EE address — must match the rule's
/// output for the level it belongs to. work/ee_start_*.bin ships in work-slim.7z.
#[test]
fn ee_snapshots_wldgravity_matches_the_rule() {
    for (name, level) in [("ee_start_05.bin", "07000005"), ("ee_start_07000001.bin", "07000001"), ("ee_start_0700000a.bin", "0700000a")] {
        let p = format!("{}/../../work/{name}", env!("CARGO_MANIFEST_DIR"));
        let Ok(d) = std::fs::read(&p) else { eprintln!("skip: {p} missing"); continue };
        let y = f32::from_le_bytes([d[0x2d8894], d[0x2d8895], d[0x2d8896], d[0x2d8897]]);
        let x = f32::from_le_bytes([d[0x2d8890], d[0x2d8891], d[0x2d8892], d[0x2d8893]]);
        let z = f32::from_le_bytes([d[0x2d8898], d[0x2d8899], d[0x2d889a], d[0x2d889b]]);
        eprintln!("{name} (level {level}): WldGravity = ({x}, {y}, {z})");
        assert_eq!([x, y, z], [0.0, -9.8, 0.0], "{name}: the snapshot disagrees with the p3 rule (level {level})");
    }
}

/// The death-anim pick follows WldGravity.y exactly [M Player_ChangeSubState 0x190630:68-73]:
/// the zero-g write is `Vec_Zero` (exact 0.0), anything else — including a hair off zero — is
/// the ground death. Sisters: the two deliberately wrong hashes fail the assertion.
#[test]
fn death_script_follows_wldgravity() {
    let mut b = Bond::new([0.0; 3], 0.0);
    assert_eq!(b.world_gravity, [0.0, -9.8, 0.0], "the constructor carries the game's default write");
    assert_eq!(b.death_script(), 0x6000_151);
    b.world_gravity = [0.0; 3];
    assert_eq!(b.death_script(), 0x6000_8e0);
    // The game's test is the exact float compare: −0.001 is NOT zero → the ground death.
    b.world_gravity = [0.0, -0.001, 0.0];
    assert_eq!(b.death_script(), 0x6000_151);
    // Sisters: the wrong picks fail.
    b.world_gravity = [0.0; 3];
    assert_ne!(b.death_script(), 0x6000_151, "the ground-death hash must NOT be picked on zero-g");
    b.world_gravity = [0.0, -9.8, 0.0];
    assert_ne!(b.death_script(), 0x6000_8e0, "the space-death hash must NOT be picked on −9.8");
}

/// Real path, the default level: the load sets Bond's WldGravity to (0, −9.8, 0) (the
/// `Player_Init` default write; p3 == 0 on 07000005's start), the death pick is the ground
/// death, and an airborne Bond falls at the game's per-tick rate `WldGravity.y · REC_FRAME_RATE`
/// [M Player_Collision 0x194df0:107/161] — the pre-fix behaviour kept bit-exact.
#[test]
fn default_level_keeps_the_fall() {
    let Some(iso) = iso_path() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 05");
    let sim = scene.sim.as_mut().unwrap();
    sim.disabled = false; // the intro script's gate — the play state (the other live tests do the same)
    assert_eq!(sim.bond.world_gravity, [0.0, -9.8, 0.0], "07000005 keeps the default WldGravity");
    assert_eq!(sim.bond.death_script(), 0x6000_151);
    assert_ne!(sim.bond.death_script(), 0x6000_8e0, "sister: the wrong pick fails");
    // Airborne in clear air (3 units up — past the floor ray's `cc + 0.6` reach): two ticks
    // accumulate `−9.8/60` per tick (bit-exact the pre-fix const; the landing snap needs the ray,
    // which misses up here).
    let y0 = sim.bond.pos[1];
    sim.bond.pos[1] += 3.0;
    sim.bond.grounded = false;
    sim.bond.floor_ny = 0.0;
    for _ in 0..2 { sim.step_bond(Input::default(), false, false, false); }
    eprintln!("05 after 2 airborne ticks: vel {:?} y {:.3} (was {y0:.3})",
        sim.bond.vel.iter().map(|v| format!("{v:.4}")).collect::<Vec<_>>(), sim.bond.pos[1]);
    assert_eq!(sim.bond.vel[1], -9.8f32 / 60.0 * 2.0, "two fall ticks at the game's rate");
}

/// Real path, the zero-g level: 0700001b's start carries p3 == 2 → the load must zero Bond's
/// WldGravity (`Vec_Zero` at the game's spawn), the death pick must be the space death, and an
/// airborne Bond must NOT fall. FAIL-FIRST: on the pre-branch tree (the hard-coded −9.8) the
/// same one-tick setup read `vel[1] = −0.16333` — this test fails there (probe recorded in the
/// report).
#[test]
fn zero_g_level_floats() {
    let Some(iso) = iso_path() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 1b");
    let sim = scene.sim.as_mut().unwrap();
    sim.disabled = false;
    assert_eq!(sim.bond.world_gravity, [0.0; 3], "0700001b zeroes WldGravity (p3 == 2)");
    assert_eq!(sim.bond.death_script(), 0x6000_8e0, "the space death anim on zero-g");
    assert_ne!(sim.bond.death_script(), 0x6000_151, "sister: the ground-death hash must NOT be picked");
    // The first airborne tick: zero WldGravity.y contributes nothing (the floor snap may land
    // him from tick 2 — the assertion reads the ONE-tick accel window).
    sim.bond.grounded = false;
    sim.bond.floor_ny = 0.0;
    sim.step_bond(Input::default(), false, false, false);
    eprintln!("1b after 1 airborne tick: vel {:?}", sim.bond.vel);
    assert_eq!(sim.bond.vel[1], 0.0, "no fall accel on zero-g (pre-fix: −9.8/60 here)");
}
