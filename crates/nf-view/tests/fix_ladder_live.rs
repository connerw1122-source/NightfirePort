//! fix-ladder: ladders and swimming through the REAL viewer path (`nf_view::load` + `Sim::step_bond`).
//!
//! The user's playtest (docs/playtests/2026-10-11.md, the 07000002 report): "Bond starts in an
//! underground bunker and can't get out — the exit is a ladder, and ladders aren't implemented", and
//! "07000002 has water that Bond should be able to swim in; right now he walks on top of it."
//!
//! The tests drive the exact functions the viewer's play path uses: the input reaches
//! `Player_Update` 0x193600 through `Sim::step_bond`, which now hands the tick the ladder context
//! (`BLData+0x884`) and the water surface (`cel+0x90`). Opt-in via `NF_ISO`.

use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Sim {
    let mut scene = nf_view::load(iso, "07000002.bin", false).expect("load 07000002");
    // fix-cutscenes: 07000002's mode-1 ScriptPlayer #400 (0x06000084, pack 07800002.bin) plays at load
    // like the game's (before the fix the SP deleted itself — the pack script was missing). Skip it the
    // way the game lets the player — hold the skip action until the script ends and control returns.
    {
        let sim = scene.sim.as_mut().unwrap();
        let pending = |sim: &nf_view::scene::Sim| sim.drones.scripts.players.iter().any(|p| p.scripts[0] == 0x0600_0084 && (matches!(p.state, 1 | 2 | 7) || p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)));
        for t in 0..600u32 {
            let sim = scene.sim.as_mut().unwrap();
            if t >= 2 && !pending(sim) && !sim.disabled { break; }
            sim.drones.scripts.skip_request = true;
            sim.step_bond(Default::default(), false, false, false);
        }
    }
    assert!(!scene.sim.as_ref().unwrap().disabled, "07000002's level-start cut-scene never handed control back");
    scene.sim.take().expect("sim")
}

/// The bunker ladder and the garden floor, from the disc. Run with NF_LADDER_PROBE=1 to dump.
#[test]
#[ignore]
fn probe() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set"); return };
    let sim = load(&iso);
    println!("== ladders ({}):", sim.ladders.len());
    for l in &sim.ladders {
        println!("  centre {:?} radius {} yaw {} channel {:#x}", l.centre, l.radius, l.yaw, l.channel);
        println!("    top {} bottom {}", l.top(), l.bottom());
    }
    println!("== cels with water:");
    for (i, c) in sim.cels.iter().enumerate() {
        if c.water > -400.0 { println!("  cel {i}: min {:?} max {:?} water {}", c.min, c.max, c.water); }
    }
    println!("== all cels:");
    for (i, c) in sim.cels.iter().enumerate() { println!("  cel {i}: min {:?} max {:?} water {}", c.min, c.max, c.water); }
    println!("== bond at {:?}", sim.bond.pos);
    for (ci, c) in sim.cels.iter().enumerate() {
        if c.water <= -400.0 { continue; }
        let p = [0.5 * (c.min[0] + c.max[0]), 0.0, 0.5 * (c.min[2] + c.max[2])];
        let floor = sim.world.floor_below([p[0], c.water, p[2]], 14.0);
        println!("water cel {ci}: centre {:?} water {} floor {:?}", [p[0], p[2]], c.water, floor);
        if let Some(f) = floor { println!("   stand y {} inside {:?} cel_at {:?}", f + 1.1, [c.min[1], c.max[1]], sim.cel_at([p[0], f + 1.1, p[2]])); }
    }
}

#[test]
fn ladder_walk_into_it_climbs_to_the_garden_and_walks_away() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    // The bunker's exit ladder (the probe dump above pins it): the ladder nearest the level start.
    let start = sim.bond.pos;
    let Some(li) = (0..sim.ladders.len()).min_by(|a, b| {
        let d = |l: &nf_game::ladder::Ladder| (l.centre[0] - start[0]).powi(2) + (l.centre[2] - start[2]).powi(2);
        d(&sim.ladders[*a]).total_cmp(&d(&sim.ladders[*b]))
    }) else { panic!("07000002 has no ladders on the disc — the bunker exit can't exist") };
    let lad = sim.ladders[li].clone();
    // Stand in front of the ladder's climbable face (the ladder's yaw points out of the face —
    // `Player_CollLadder` needs |AngleDiff(ladder.yaw, yaw)| > π/2), on the floor below its bottom.
    let (s, c) = (lad.yaw.sin(), lad.yaw.cos());
    let stand = [lad.centre[0] + s * 1.2, 0.0, lad.centre[2] + c * 1.2];
    let floor = sim.world.floor_below([stand[0], lad.centre[1], stand[2]], 8.0).expect("floor in front of the ladder");
    let stand = [stand[0], floor + 1.1, stand[2]];
    let yaw = lad.yaw + std::f32::consts::PI; // facing the ladder
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, stand, yaw);
    sim.bond.yaw = yaw;
    let bunker_y = sim.bond.pos[1];
    // Walk forward: the grab fires (`Player_CollLadder`), then the climb carries Bond up.
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut grabbed = None;
    for t in 0..900 {
        sim.step_bond(fwd, false, false, false);
        if grabbed.is_none() && sim.bond.substate == nf_game::player::SS_CLIMB { grabbed = Some(t); }
        if sim.bond.pos[1] >= 5.8 - 0.5 && sim.bond.grounded && sim.bond.substate == nf_game::player::SS_WALK { break; }
    }
    let grabbed = grabbed.expect("walking into the bunker ladder never attached (substate stayed 0) — the user's symptom");
    // The climb ends on the garden floor (y ≈ 5.8 near (−38, 39.7) per the playtest).
    assert!(sim.bond.pos[1] > 5.8 - 0.6, "Bond left the ladder at y {} — not the garden floor 5.8 (grab at tick {grabbed})", sim.bond.pos[1]);
    assert!(sim.bond.substate == nf_game::player::SS_WALK, "still in substate {} after the climb", sim.bond.substate);
    // …and walks away (the climb released him onto walkable ground, not stuck on the ladder).
    sim.bond.yaw = 0.0;
    for _ in 0..30 { sim.step_bond(fwd, false, false, false); }
    assert!(sim.bond.substate == nf_game::player::SS_WALK && !sim.bond.ladder.is_some(), "re-grabbed right after the climb");
    // Sister: the climb actually raised Bond (the bunker floor is well below the garden).
    assert!(sim.bond.pos[1] - bunker_y > 2.0, "climb only gained {} — not the bunker-to-garden height", sim.bond.pos[1] - bunker_y);
}

#[test]
fn walk_into_water_enters_the_swim_substate() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    // Find a water cel with a standable floor inside its box (the game's room link needs Bond inside the
    // cel bbox — `build_FindCel` 0x1c8cb0 walks the boxes; `Player_InWater` then compares his y against
    // that cel's `cel+0x90`). 07000002's moat cel (the garden, water 3.76, box down to −0.73) qualifies;
    // the bunker channels' boxes start above their floors.
    let mut placed = None;
    'outer: for (ci, c) in sim.cels.iter().enumerate() {
        if c.water <= -400.0 { continue; }
        // Scan a grid over the cel's x/z for deep water: a floor at least 1.2 below the surface means
        // Bond standing there is IN the water (y − 0.3 < surface fires `Player_InWater`).
        for k in 1..8 { for j in 1..8 {
            let x = c.min[0] + (c.max[0] - c.min[0]) * k as f32 / 8.0;
            let z = c.min[2] + (c.max[2] - c.min[2]) * j as f32 / 8.0;
            let Some(floor) = sim.world.floor_below([x, c.water - 0.4, z], 14.0) else { continue };
            if floor > c.water - 1.2 { continue; } // shallow/bridge — Bond would stand on dry ground
            let y = floor + 1.1;
            if y < c.min[1] || y > c.max[1] { continue; }
            if sim.cel_at([x, y, z]) != Some(ci) { continue; }
            placed = Some(([x, y, z], c.water));
            break 'outer;
        } }
    }
    let (p, w) = placed.expect("no 07000002 water cel with a standable floor inside its box");
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, p, 0.0);
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut swam = false;
    for _ in 0..300 {
        sim.step_bond(fwd, false, false, false);
        if sim.bond.substate == nf_game::player::SS_SWIM { swam = true; break; }
    }
    assert!(swam, "walking into the 07000002 water (cel surface {w}) never entered substate 3 — Bond walked on top of it (the user's symptom)");
    assert!(sim.bond.pos[1] - 0.3 < w, "swimming but Bond's y {} is above the surface {w}", sim.bond.pos[1]);
    // Sister: dry land at the bunker start never enters swim.
    let mut sim = load(&iso);
    for _ in 0..60 { sim.step_bond(fwd, false, false, false); }
    assert!(sim.bond.substate != nf_game::player::SS_SWIM, "swimming without water at the level start");
}

/// The levels-with-ladders sweep for docs/systems/levels.md §2: every story level's ladder (type 0x22)
/// placement count and water-cel (0xC023) count, read through the same `Level::parse` the viewer uses.
#[test]
fn ladder_and_water_counts_per_level() {
    use nf_formats::{disc, level::Level, iso9660::IsoImage};
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&iso).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    for code in [0x07000001u32, 0x07000002, 0x07000003, 0x07000004, 0x07000005, 0x07000006, 0x07000007, 0x07000008, 0x07000009, 0x0700000a, 0x0700000b, 0x0700000c, 0x0700000d, 0x07000011, 0x07000012, 0x07000013, 0x07000014, 0x07000015, 0x07000016, 0x0700001b] {
        let name = format!("{code:08x}.bin");
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)) else { continue };
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let ladders: Vec<String> = level.maps.iter().flat_map(|m| &m.placements).filter(|p| p.flags & 0xff == 0x22)
            .map(|p| format!("({:.1},{:.1},{:.1}) yaw {:.2}", p.translation[0], p.translation[1], p.translation[2], p.rotation[1])).collect();
        let water: Vec<(usize, f32)> = level.maps.iter().enumerate().flat_map(|(mi, m)| m.objects.iter().enumerate().map(move |(oi, o)| (mi * 1000 + oi, o.max[1])))
            .collect(); // placeholder; the real count comes from the cel placements below
        let water_cels = level.maps.iter().flat_map(|m| &m.placements).filter(|p| p.creates_cel() && (p.flags & 0xffff) == 0xc023).count();
        println!("{code:08x}: ladders {} {:?} water_cels {}", ladders.len(), ladders, water_cels);
        let _ = water;
        // Sister: 07000002 must have the bunker-exit ladder (the playtest's item).
        if code == 0x07000002 { assert!(ladders.len() >= 4, "07000002 lost its ladders"); }
    }
}
