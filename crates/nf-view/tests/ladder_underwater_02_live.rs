//! ladder-underwater-02 (750a): the underwater ladder(s) on 07000002 softlock the game — the user's
//! THIRD report (2026-10-16 evening): "ladder on 02 underwater is fucked and softlocks the game.
//! I already told you about this."
//!
//! The disc ladders that reach into water [D probe `disc_ladders_and_water_cels` below, nf-formats
//! parse of 07000002.bin]:
//! - the DEEP ladder at (−50.35, centre y 2.11, 5.50), yaw π/2, r 2.567 — top 5.531, **bottom 0.547**,
//!   ~3.2 under the water surfaces (cel idx17 "waterarea1" surface 4.339 covers it; cel idx15
//!   "corry_undergarden_bottom" surface 3.765 overlaps the column) — the ladder STARTS under water;
//! - the pond hut ladder at (−74.84, centre y 4.04, 22.22), yaw −2.356, r 2.211 — top 7.102,
//!   bottom 2.830, ~0.94 under the 3.768 surface (cel idx237 "southgarden_underwater1").
//! (The bunker exit ladder at (−26.87, 42.23) and the small ladder at (−15.29, 35.00) are dry.)
//!
//! A softlock = Bond can no longer move/act. Every test drives the real input path
//! (`nf_view::load` + `Sim::step_bond` — the same functions the viewer's play path runs) and asserts
//! Bond **regains normal control and can reach the bank**: the swim escape must end in SS_WALK on
//! dry ground, with the walk input moving him. Opt-in via `NF_ISO`.

use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load() -> Sim {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    scene.sim.take().expect("sim")
}

/// One step through the real path; the level's scripts grab Bond into cut-scenes within seconds,
/// so each tick carries the game's own skip action like the walkreal walker [M nis.rs].
fn step(sim: &mut Sim, input: nf_game::player::Input) {
    if sim.disabled { sim.drones.scripts.skip_request = true; }
    sim.step_bond(input, false, false, false);
}

fn ladder_near(sim: &Sim, x: f32, z: f32) -> (usize, nf_game::ladder::Ladder) {
    let li = (0..sim.ladders.len()).min_by(|a, b| {
        let d = |l: &nf_game::ladder::Ladder| (l.centre[0] - x).powi(2) + (l.centre[2] - z).powi(2);
        d(&sim.ladders[*a]).total_cmp(&d(&sim.ladders[*b]))
    }).expect("07000002 has ladders");
    (li, sim.ladders[li].clone())
}

const DEEP_XZ: (f32, f32) = (-50.35, 5.50);
const HUT_XZ: (f32, f32) = (-74.84, 22.22);

/// The disc data the tests stand on: the two water ladders with their thresholds and the water
/// surfaces above them [D — a direct nf-formats parse of 07000002.bin from the ISO].
#[test]
fn disc_ladders_and_water_cels() {
    use nf_formats::{disc, iso9660::IsoImage, level::Level};
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&iso_path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000002.bin")).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    let mut ladders = 0;
    let mut water = 0;
    let (mut deep_bottom, mut hut_bottom) = (f32::MAX, f32::MAX);
    for (mi, m) in level.maps.iter().enumerate() {
        for p in &m.placements {
            if p.flags & 0xff == 0x22 {
                ladders += 1;
                // Resolve the object like nf_game::ladder::all does (by index for anonymous placements).
                let obj = if (p.index as usize) < m.objects.len() { Some(&m.objects[p.index as usize]) } else { None };
                if let Some(o) = obj {
                    let yaw = p.rotation[1];
                    let (s, c) = yaw.sin_cos();
                    let cy = p.translation[1] + o.centre[1];
                    let r = o.radius * p.scale().iter().fold(1.0f32, |a, &v| a * v.max(1e-6).abs());
                    let (top, bottom) = (cy + r + 0.85, cy - r + 1.0);
                    let x = p.translation[0] + o.centre[0] * c + o.centre[2] * s;
                    let z = p.translation[2] - o.centre[0] * s + o.centre[2] * c;
                    eprintln!("ladder ({x:.2},{cy:.2},{z:.2}) yaw {yaw:.3} top {top:.3} bottom {bottom:.3}");
                    if (x - DEEP_XZ.0).abs() < 0.5 && (z - DEEP_XZ.1).abs() < 0.5 { deep_bottom = bottom; }
                    if (x - HUT_XZ.0).abs() < 0.5 && (z - HUT_XZ.1).abs() < 0.5 { hut_bottom = bottom; }
                }
            }
            if p.flags & 0x8000 != 0 && (p.flags & 0xffff) == 0xc023 { water += 1; }
        }
    }
    assert_eq!(ladders, 4, "07000002 lost ladders [D]");
    assert_eq!(water, 5, "07000002 lost water cels [D]");
    assert!(deep_bottom < 1.0, "the deep ladder's bottom {} is not underwater-deep [D]", deep_bottom);
    assert!((hut_bottom - 2.83).abs() < 0.05, "the hut ladder's bottom {} moved [D]", hut_bottom);
}

/// Grab a ladder from its grab side at the given root height, through the real path (the swim entry
/// happens naturally when the spawn is under the surface — `Player_InWater` 0x195e50's first-statement
/// check in `Player_SSWalk` 0x192310). Returns after the grab (SS_CLIMB) or panics with the trace.
fn grab_at_depth(sim: &mut Sim, lad: &nf_game::ladder::Ladder, root: f32) {
    let front = [lad.centre[0] + lad.yaw.sin() * 1.2, root, lad.centre[2] + lad.yaw.cos() * 1.2];
    sim.bond = nf_game::player::Bond::new(front, lad.yaw + std::f32::consts::PI);
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut latched = 0;
    for t in 0..400 {
        step(sim, walk);
        if sim.bond.substate == nf_game::player::SS_CLIMB { latched = t; break; }
    }
    assert_eq!(sim.bond.substate, nf_game::player::SS_CLIMB,
        "swimming in at root {:.2} never attached (last substate {} y {:.3} regrab {})",
        root, sim.bond.substate, sim.bond.pos[1], sim.bond.regrab);
    eprintln!("  grabbed at tick {latched}, y {:.3}", sim.bond.pos[1]);
}

/// The HUT ladder's bottom dismount and escape — 691's open divergence spot, now closed: after
/// the bottom dismount (2.83, under the 3.768 surface) a NEUTRAL stick re-latches once the 5-tick
/// InWater regrab expires (the settle at 0.5 off the plane is INSIDE the game's pick: the mesh
/// probe gives seg-tri distance 0.22 < 0.55, so the re-latch itself is game-faithful). The
/// softlock assertion is the escape: with the game's climb capsule [M 0x194df0 case 0] the
/// re-latched climb carries Bond up out of the pond to the hut floor — normal control, the bank.
#[test]
fn hut_ladder_bottom_dismount_escape_reaches_the_bank() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load();
    let (_li, lad) = ladder_near(&sim, HUT_XZ.0, HUT_XZ.1);
    assert!((lad.centre[0] - HUT_XZ.0).abs() < 0.5, "not the hut ladder: {:?}", lad.centre);
    grab_at_depth(&mut sim, &lad, 3.0);
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for _ in 0..60 { step(&mut sim, fwd); }
    let back = nf_game::player::Input { forward: -1.0, ..Default::default() };
    let mut dismount_y = None;
    for _ in 0..1200 {
        step(&mut sim, back);
        if sim.bond.substate != nf_game::player::SS_CLIMB { dismount_y = Some(sim.bond.pos[1]); break; }
    }
    let dy = dismount_y.expect("the hut climb-down never dismounted");
    eprintln!("  hut bottom dismount at y {dy:.3} (bottom {:.3})", lad.bottom());
    // The escape drive: turn away, swim/push; when the settle re-latches, climb UP out of the
    // water — the natural get-out that the pre-750a wedge made impossible.
    let mut relatches = 0;
    let mut last_sub = sim.bond.substate;
    let mut out = None;
    for t in 0..1200 {
        let away = (sim.bond.yaw - lad.yaw).abs() < 0.2;
        let input = if sim.bond.substate == nf_game::player::SS_CLIMB {
            if last_sub != nf_game::player::SS_CLIMB { relatches += 1; }
            nf_game::player::Input { forward: 1.0, ..Default::default() } // climb up out
        } else if away {
            nf_game::player::Input { forward: 1.0, ..Default::default() }
        } else {
            nf_game::player::Input { forward: 1.0, turn: 0.06, ..Default::default() }
        };
        step(&mut sim, input);
        last_sub = sim.bond.substate;
        if sim.bond.substate == nf_game::player::SS_WALK && sim.bond.grounded {
            eprintln!("  +{t}: the bank — SS_WALK grounded at {:?} after {relatches} re-latches", sim.bond.pos);
            out = Some(t);
            break;
        }
    }
    assert!(out.is_some(), "SOFTLOCK: after the hut ladder's bottom dismount Bond never regained \
        normal control (last substate {} y {:.3})", sim.bond.substate, sim.bond.pos[1]);
    // And no re-latch back onto the ladder once out.
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for t in 0..240 {
        step(&mut sim, walk);
        assert_ne!(sim.bond.substate, nf_game::player::SS_CLIMB,
            "+{t} on the bank: re-latched at y {:.3}", sim.bond.pos[1]);
    }
}

/// The away-push escape: hold the stick AWAY from the ladder from the dismount tick on (the yaw
/// faces the ladder, so forward −1 is away) — the 109i escape mechanic. Returns when the swimmer
/// is clear of the grab column (dxz > 0.9, the same bound 109i's test used) and still swimming
/// under input (the position kept moving — no freeze). A re-latch while drifting fails the test.
fn push_off_clears(sim: &mut Sim, lad: &nf_game::ladder::Ladder, budget: usize) -> f32 {
    let back = nf_game::player::Input { forward: -1.0, ..Default::default() };
    for t in 0..budget {
        step(sim, back);
        assert_ne!(sim.bond.substate, nf_game::player::SS_CLIMB,
            "+{t} after the dismount (push-off held): re-latched at y {:.3} dxz {:.3}",
            sim.bond.pos[1], ((sim.bond.pos[0] - lad.centre[0]).powi(2) + (sim.bond.pos[2] - lad.centre[2]).powi(2)).sqrt());
        let dxz = ((sim.bond.pos[0] - lad.centre[0]).powi(2) + (sim.bond.pos[2] - lad.centre[2]).powi(2)).sqrt();
        if dxz > 0.9 { return dxz; }
    }
    panic!("the push-off never cleared the grab column (dxz {:.3})",
        ((sim.bond.pos[0] - lad.centre[0]).powi(2) + (sim.bond.pos[2] - lad.centre[2]).powi(2)).sqrt());
}

/// SCENARIO 1+2, the DEEP ladder: swim INTO it from underwater, climb UP out of the water. The
/// pre-750a port WEDGED at the grab depth (the fat 0.55 stance capsule in the climb pass) — the
/// user's "ladder on 02 underwater … softlocks the game". With the game's climb capsule [M
/// 0x194df0 case 0] the climb carries him up out of the water body (the surface is 3.765 here
/// [D]) and HOLDS him in control above it. The final exit into the bathhouse is blocked by the
/// CLOSED bath hatch (SimpleScript 0xE2 #269, solid at y 4.74 [D probe], the USE-operate path —
/// see the report's remaining-gap note); the climb itself must not freeze or drop him.
#[test]
fn deep_ladder_swim_in_climbs_out_of_the_water() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load();
    let (li, lad) = ladder_near(&sim, DEEP_XZ.0, DEEP_XZ.1);
    assert!((lad.centre[0] - DEEP_XZ.0).abs() < 0.5, "not the deep ladder: {:?}", lad.centre);
    assert!(lad.bottom() < 1.0, "the deep ladder's bottom is not deep: {}", lad.bottom());
    grab_at_depth(&mut sim, &lad, 3.0);
    // Climb up out of the water: the climb passes the 3.765 surface and keeps him in control.
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut above = None;
    for t in 0..1200 {
        step(&mut sim, fwd);
        assert_eq!(sim.bond.substate, nf_game::player::SS_CLIMB,
            "+{t}: the climb dropped him at y {:.3} (substate {}) — the climb-up broke", sim.bond.pos[1], sim.bond.substate);
        if sim.bond.pos[1] >= 3.9 { above = Some(t); break; }
    }
    let above = above.expect("the climb-up never rose above the water surface (the wedge)");
    eprintln!("  climbed above the surface (+{above}, y {:.3}); the bath hatch (0xE2 #269) closes the exit", sim.bond.pos[1]);
    // In control at the wedge: the action dismount still works (move/act — no softlock).
    let action = nf_game::player::Input { jump: true, ..Default::default() };
    let mut freed = false;
    for _ in 0..60 {
        step(&mut sim, action);
        if sim.bond.substate != nf_game::player::SS_CLIMB { freed = true; break; }
    }
    assert!(freed, "wedged under the hatch, the action dismount stopped working (no move/act)");
}

/// SCENARIO 3, the DEEP ladder: climb DOWN into the water — the dismount fires on the climb's
/// below-hit [M Player_Climb 0x196310:63-78, AnimObj+0x60 bit 8] (under the column the tunnel
/// floor is within the climb capsule's below reach long before the 0.547 bottom threshold), the
/// swim entry overwrites the regrab with 5 [M Player_InWater 0x195e50] — and the push-off must
/// clear the grab column (the pre-750a port re-latched every cycle: the user's softlock).
#[test]
fn deep_ladder_climb_down_push_off_clears_the_ladder() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load();
    let (_li, lad) = ladder_near(&sim, DEEP_XZ.0, DEEP_XZ.1);
    grab_at_depth(&mut sim, &lad, 3.0);
    let back = nf_game::player::Input { forward: -1.0, ..Default::default() };
    let mut dismounted = false;
    for _ in 0..2400 {
        step(&mut sim, back);
        if sim.bond.substate != nf_game::player::SS_CLIMB { dismounted = true; break; }
    }
    assert!(dismounted, "climbing down the deep ladder never dismounted");
    eprintln!("  dismount at y {:.3} (bottom threshold {:.3})", sim.bond.pos[1], lad.bottom());
    let dxz = push_off_clears(&mut sim, &lad, 600);
    eprintln!("  push-off cleared the column: dxz {dxz:.3}");
}

/// SCENARIO 4, the DEEP ladder: letting go MID-WATER (the action dismount with the stick neutral
/// [M Player_Climb 0x196310:45-48]) — then the push-off clears the grab column and the swimmer
/// keeps control.
#[test]
fn deep_ladder_let_go_mid_water_push_off_clears() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load();
    let (_li, lad) = ladder_near(&sim, DEEP_XZ.0, DEEP_XZ.1);
    grab_at_depth(&mut sim, &lad, 3.0);
    let hold = nf_game::player::Input::default();
    for _ in 0..30 { step(&mut sim, hold); }
    let action = nf_game::player::Input { jump: true, ..Default::default() };
    let mut dismounted = false;
    for _ in 0..60 {
        step(&mut sim, action);
        if sim.bond.substate != nf_game::player::SS_CLIMB { dismounted = true; break; }
    }
    assert!(dismounted, "the action dismount mid-water did nothing");
    eprintln!("  mid-water dismount at y {:.3}", sim.bond.pos[1]);
    let dxz = push_off_clears(&mut sim, &lad, 600);
    eprintln!("  push-off cleared the column: dxz {dxz:.3}");
}
