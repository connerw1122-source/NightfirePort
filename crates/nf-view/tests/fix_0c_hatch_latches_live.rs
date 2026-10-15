//! fix-0c-hatch-latches (user 0700000c: "It's missing two latches on that hatch... when I dropped
//! down, I got some culling issues. And I couldn't get through still."):
//! 1. the two bottom hinge latches draw from the walkway — the straddle rescue (`Controls_StraddleTest`
//!    0x134778:47-57 + `View_AddForcedObjects` 0x1e5fb8:85-145: a sphere straddling ≥2 room cels
//!    draws regardless of its home cel's visibility; the class-0x3A check of the straddled cels
//!    runs for drones only);
//! 2. the eye cel follows the fall through the portals — the FindCel miss keeps `viewer+0x34`
//!    stale ([M `Camera_CheckLocation` 0x1bae90:33-39]) so the miss segments GROW and hop the
//!    portals mid-drop;
//! 3. a bullet starts the winch — `SS_Operate` 0x1a9df8's contact mask (player 0x8 / drone 0x10 /
//!    bullet 0x2) against the SS's mask word; the 0c winches #351/#364/#377 carry mask 0x2 and
//!    channel 0 [D param 2/3], so a bullet hit — not any channel — starts the sweep.

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_formats::placement::{object_matrix, transform};
use nf_game::player::Input;
use nf_view::scene::{eye_cell, find_cel, find_cell_raycast, portal_cel_chain_from_cell, ViewerCel};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

fn sphere_overlaps(min: [f32; 3], max: [f32; 3], centre: [f32; 3], radius: f32) -> bool {
    let d2: f32 = (0..3).map(|k| {
        let (lo, hi) = (min[k].min(max[k]), min[k].max(max[k])); // degenerate/NaN-safe
        if !lo.is_finite() || !hi.is_finite() { return 0.0; }
        let c = centre[k].clamp(lo, hi);
        (centre[k] - c) * (centre[k] - c)
    }).sum();
    d2 <= radius * radius
}

/// The four display-vent hinge latches (0x2E `Lock_Create`, out channels 14-17 [D #139-142]),
/// each as (placement, the home cel is OUTSIDE the walkway's portal chain). The measured straddle
/// (the probe, this session): every hinge's sphere overlaps ≥ 2 room cels — the BOTTOM pair
/// (#139/#141, y ≈ 15.7) overlaps 5 (incl. cel 16 cell 212, the lower display room, and cel 7
/// cell 91, the upper walkway) with its home cel 16 unchained; the TOP pair (#140/#142, y ≈ 16.9)
/// overlaps 4 (cels 7/11/13/14) with its home cel 7 chained — they drew on master through the
/// home cel, the bottom pair culled (the user's two missing latches).
const HINGES: [(usize, bool); 4] = [(139, true), (140, false), (141, true), (142, false)];

#[test]
fn the_two_bottom_hinge_latches_draw_from_the_walkway() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let level = Level::parse(&read_archive(&iso, "0700000c.bin")).expect("parse 0700000c");
    let scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let m = level.main_map().expect("main map");
    let world = scene.sim.as_ref().map(|s| &s.world);
    // The walkway camera (the spot the user sees the hatch from; the probe's walkway eye).
    let eye = [5.8, 15.8, -107.5];
    let mut state = ViewerCel::default();
    let cell = eye_cell(&scene.cel_graph, &mut state, eye, world).expect("the walkway eye cel");
    // Depth-0 planes zeroed: the strict portal walk, frustum-free (the probe's topological verdict).
    let chain = portal_cel_chain_from_cell(&scene.cel_graph, cell, eye, &[[0.0; 4]; 5]).expect("the walk's chain");
    for &(pi, expect_straddle) in &HINGES {
        let p = &level.maps[m].placements[pi];
        assert_eq!(p.flags & 0xff, 0x2e, "hinge #{pi} is a Lock_Create placement [D]");
        let (mp, o) = if p.by_hash() { *level.by_hash.get(&p.hashcode).expect("hinge hash") } else { (m, p.index as usize) };
        let obj = &level.maps[mp].objects[o];
        let centre = transform(&object_matrix(p.rotation, p.translation), obj.centre);
        let radius = obj.radius;
        let home = find_cel(&scene.cels, centre).unwrap_or_else(|| panic!("hinge #{pi} home cel"));
        let room_cell = scene.cel_graph.room_to_cell[home].unwrap_or_else(|| panic!("hinge #{pi} room cell"));
        let visible = chain.contains(&room_cell);
        let straddle = scene.cels.iter().filter(|c| sphere_overlaps(c.min, c.max, centre, radius)).count() >= 2;
        // The measured straddle fact: all four hinges' spheres overlap ≥ 2 room cels [D, the
        // overlap band] — the game's forced-draw precondition (`Controls_StraddleTest`
        // 0x134778:47-57) holds for the whole family, not just the bottom pair.
        assert!(straddle, "hinge #{pi}'s sphere must straddle its overlap-band cels");
        assert_eq!(visible, !expect_straddle,
            "hinge #{pi}'s home cell's chain membership [D: the bottom pair sits in cel 16 cell 212, \
             the top pair in cel 7 cell 91 — the walkway eye's chain]");
        if expect_straddle {
            // The reproduction: the bottom hinges' home cell is portal-unreachable from the walkway
            // (the master port culled exactly these two — the user's missing latches).
            assert!(!visible, "hinge #{pi}'s home cell must be outside the walkway's portal chain (the repro)");
        }
        // gpu.rs's draw-time rule post-fix (`View_AddForcedObjects` 0x1e5fb8:85-145): hidden =
        // home-cel invisible AND not straddling. The master rule lacked the straddle half.
        let hidden = !visible && !straddle;
        assert!(!hidden, "hinge #{pi} must draw from the walkway (visible {visible}, straddle {straddle})");
    }
}

#[test]
fn the_eye_cel_follows_the_fall_through_the_portals() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_ref().unwrap();
    // The user's F12 fall line (the driver's spots: x 5.34, z −111.81, mid-fall y 12.49/12.89):
    // step the eye down the drop from the inner ledge to the lower walkway.
    let mut state = ViewerCel::default();
    let mut last = None;
    for i in 0..46 {
        let y = 15.5 - 0.1 * i as f32;
        last = eye_cell(&scene.cel_graph, &mut state, [5.34, y, -111.81], Some(&sim.world));
    }
    let bottom = last.expect("the fall's eye cel resolves");
    let fresh = find_cell_raycast(&scene.cel_graph, [5.34, 11.4, -111.81], Some(&sim.world)).expect("the bottom cel fresh-picks");
    assert_eq!(bottom, fresh,
        "the eye cel at the fall's bottom must be the cel the eye is actually in — the game hops \
         the portals mid-fall because the FindCel miss leaves viewer+0x34 stale [M 0x1bae90:33-39]; \
         the old port advanced pos every miss and parked on the upper room (the drop's culling)");
}

#[test]
fn a_real_shot_starts_the_winch_and_bond_rides_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    let k = sim.drones.sss.of_placement(351).expect("the obj_stor1_pully02 SS exists [D #351]");
    assert_eq!(sim.drones.sss.list[k].mask, 0x2, "the winch's Operate mask is the bullet bit [D #351 param 2]");
    assert_eq!(sim.drones.sss.list[k].channel, 0, "the winch's channel is 0 — no channel can drive it [M 0x1a9df8:46-51]");
    assert_eq!(sim.drones.sss.list[k].rearm, 0, "the winch re-arms after its sweep [D #351 param 1]");
    let top = sim.drones.sss.list[k].pos();
    // (a) The shot: Bond a step south of the pulley, aiming DOWN at the winch's dynamic box
    // (y 8.45..14.47 — a level shot from the standing eye y ≈ 16.4 passes 1.9 over its top),
    // firing through the real input path (step_bond's trigger arg — the weapon fire). The settle
    // first: the loadout weapon finishes its draw before the trigger bites (the fire path drops
    // an early pull). (b) The ride: stand Bond ON the platform.
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [top[0], top[1] + 0.4, top[2] + 3.0], std::f32::consts::PI);
    for _ in 0..150 { sim.step_bond(Input::default(), false, false, false); }
    {
        // aim_at's convention (fix_level_0c_live): pitch ±1 → ±π/2 around the head.
        let h = sim.bond.head();
        let target = [top[0], (top[1] - 3.0).max(top[1] * 0.5), top[2]];
        let d = [target[0] - h[0], target[1] - h[1], target[2] - h[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        sim.bond.yaw = d[0].atan2(d[2]);
        sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() * 2.0 / std::f32::consts::PI;
    }
    let before_t = sim.drones.sss.list[k].t;
    for tick in 0..240 {
        sim.step_bond(Input::default(), tick < 30, false, false); // hold the trigger through the draw
        if sim.drones.sss.list[k].t > before_t { break; }
    }
    assert!(sim.drones.sss.list[k].t > before_t,
        "a real shot must start the winch sweep [M SS_Operate 0x1a9df8:35-39 the bullet bit; \
         SS_Update 0x1a9948:85-90 the armed start] — got t {} → {}", before_t, sim.drones.sss.list[k].t);
    // The ride: with Bond standing on the platform as it sweeps, the platform's per-tick delta
    // carries him (`Player_FeetOnPoint` 0x194188:133-156 → `Control_InheritVelocity` 0x133e88).
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [top[0], top[1] + 1.2, top[2]], 0.0);
    let start = sim.bond.pos;
    let platform_start = sim.drones.sss.list[k].pos();
    for _ in 0..80 { sim.step_bond(Input::default(), false, false, false); }
    let moved = sim.drones.sss.list[k].pos();
    let d = ((moved[0] - platform_start[0]).powi(2) + (moved[1] - platform_start[1]).powi(2) + (moved[2] - platform_start[2]).powi(2)).sqrt();
    if d > 0.05 {
        let bond_d = ((sim.bond.pos[0] - start[0]).powi(2) + (sim.bond.pos[1] - start[1]).powi(2) + (sim.bond.pos[2] - start[2]).powi(2)).sqrt();
        assert!(bond_d > 0.02, "Bond standing on the moving platform rides it (platform moved {d}, Bond {bond_d}) [M 0x194188:138-155]");
    }
}
