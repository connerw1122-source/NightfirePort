//! playtest_spots_live — docs/playtest/spots.md checked headlessly, one test per level.
//!
//! For every playtest feature: load the level, wait out the intro the way the game's own skip does
//! (`scripts.skip_request` while disabled), place Bond exactly where the window's `--bond x,y,z,yaw,pitch`
//! puts him (`Bond::new(pos, yaw)` + raw `bond.pitch`, lib.rs:2977-2982), run 60 fixed-60 Hz ticks and
//! assert he is GROUNDED (not falling; SS_SWIM where the spot is in the pond) and the feature object is
//! WITHIN REACH — distance against the game's own constant (laser 4.0, Q-Claw bolt 50.0, E use 2.5,
//! Q-Worm shot 2.0) plus a clear eye ray where the game's own path needs one (the laser's beam, the
//! Q-Worm's bullet, the Q-Claw's bolt, a guard in view). Numbers only — no game bytes.
//!
//! Every spot's coordinates and their sources live in docs/playtest/spots.md; the object positions are
//! the disc placements ([D]) the cited tests already assert against.

use nf_game::player::{Bond, Input};
use nf_game::triggers::kind;
use nf_game::world::V3;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// XZ distance.
fn dxz(a: V3, b: V3) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// 3D distance.
fn d3(a: V3, b: V3) -> f32 { ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// Wait out the level's intro the documented way: the game's skip while control is held
/// (`fix_swim_live.rs:37-40`, `walk_07000007_live.rs:21`).
fn free(sim: &mut nf_view::scene::Sim) {
    for _ in 0..3000u32 {
        if sim.disabled { sim.drones.scripts.skip_request = true; }
        sim.step_bond(Input::default(), false, false, false);
        if !sim.disabled { return; }
    }
    panic!("control never returned within 3000 ticks");
}

/// The `--bond` contract: Bond::new at the spot, raw pitch, 60 ticks, grounded at the end.
/// Reports the drop so a spot's floor height can be read straight from the log.
fn place_and_ground(sim: &mut nf_view::scene::Sim, spot: [f32; 3], yaw: f32, pitch: f32, label: &str) {
    sim.bond = Bond::new(spot, yaw);
    sim.bond.pitch = pitch;
    let mut drop = 0.0f32;
    for _ in 0..60u32 {
        let y = sim.bond.pos[1];
        sim.step_bond(Input::default(), false, false, false);
        drop = drop.max(y - sim.bond.pos[1]);
    }
    assert!(sim.bond.grounded, "{label}: not grounded after 60 ticks (floor_ny {:.3}, dropped {drop:.2})", sim.bond.floor_ny);
    eprintln!("[{label}] grounded at {:?} (floor_ny {:.3}, dropped {drop:.2})", sim.bond.pos, sim.bond.floor_ny);
}

/// A clear eye ray to `to`, stopped 0.4 short of it so a ray ending ON the object's own mesh does not
/// self-hit (the hatch's plane, the guard's body). Returns the blocking hit for the message.
fn ray_clear(sim: &nf_view::scene::Sim, to: V3) -> Option<String> {
    let eye = sim.bond.head();
    let d = [to[0] - eye[0], to[1] - eye[1], to[2] - eye[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(0.001);
    let to2 = [to[0] - d[0] / l * 0.4, to[1] - d[1] / l * 0.4, to[2] - d[2] / l * 0.4];
    sim.world.segment_hit(eye, to2).map(|(t, p)| format!("blocked t={t:.3} at {p:?}"))
}

fn require_los(sim: &nf_view::scene::Sim, to: V3, label: &str) {
    if let Some(hit) = ray_clear(sim, to) {
        panic!("{label}: the eye ray from {:?} to {to:?} is {hit}", sim.bond.head());
    }
    eprintln!("[{label}] eye ray to {to:?} clear");
}

// ---------------------------------------------------------------------------
// 07000005 — the grapple points, the cable, the gate guards' ledge, the
// Scorpion's patrol guard, the crawl vent. docs/playtest/spots.md §05.
// ---------------------------------------------------------------------------

/// Q-Claw grapple #282 (13.752, 32.292, 57.303) from the main-agent playtest spot [D #282;
/// grapple_hook_live.rs:35-38 + fix-grapple-hook.md:151]; #281 (−3.396, 42.940, −71.769) from the
/// user's reported spot [gadget_grapple_user_spot.rs:22-25]. The bolt's range is 50.0
/// [M weapon_data[0x50]+0x1C, Bullet_update 0x12e7b0:158] and the bolt flies — distance + LOS.
#[test]
fn spots_07000005_grapple_points_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);

    // #282: the playtest spot — 4.05 out, yaw −2.01 faces the point (atan2 of the offset).
    place_and_ground(sim, [16.5, 29.6, 58.6], -2.01, 0.24, "05 grapple #282");
    let d = d3(sim.bond.pos, [13.752, 32.292, 57.303]);
    assert!(d < 50.0, "05 grapple #282: {d:.2} exceeds the Q-Claw bolt range 50.0 [M 0x12e7b0:158]");
    require_los(sim, [13.752, 32.292, 57.303], "05 grapple #282");
    eprintln!("[05 grapple #282] distance {d:.2}");

    // #281: the user's spot — 15.4 out (yaw −4.717 ≡ 1.566 wrapped).
    place_and_ground(sim, [-18.28, 38.19, -71.83], -4.717, 0.151, "05 grapple #281");
    let d = d3(sim.bond.pos, [-3.396, 42.940, -71.769]);
    assert!(d < 50.0, "05 grapple #281: {d:.2} exceeds the bolt range 50.0");
    require_los(sim, [-3.396, 42.940, -71.769], "05 grapple #281");
    eprintln!("[05 grapple #281] distance {d:.2}");
}

/// The cable #251 "Wire_Cable": the context-icon mouth spot (fix_wire2_live.rs:31-33,65 — the stand
/// the jump-grab fires from; the grab takes the nearest wire within 1000 units [M Player_HandleJump
/// 0x1916b0:118-147], the crawl-icon context scope is 12.0 [L fix-swim-vent5.md:52-57]). The wire's
/// world sphere is centre (−23.486, 34.468, 6.263) r 4.04 [L ee_start_05.bin 016dc430; wire_live.rs:20-21].
#[test]
fn spots_07000005_cable_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    // The wire object itself, from the parse [D placement #251].
    let wc = sim.wires.iter().find(|w| (w.centre[0] + 23.49).abs() < 0.5 && (w.centre[2] - 6.26).abs() < 0.5)
        .map(|w| (w.centre, w.radius))
        .expect("the 05 Wire_Cable placement parses into the sim's wires [D #251]");
    place_and_ground(sim, [-23.58, 32.24, 2.65], 0.0, 0.0, "05 cable");
    let d = d3(sim.bond.pos, wc.0);
    assert!(d < 12.0, "05 cable: Bond {d:.2} from the wire's world sphere — outside the 12.0 context scope");
    eprintln!("[05 cable] wire centre {:?} r {:.2}, distance {d:.2} (grab = JUMP within 1000 [M 0x1916b0])",
        wc.0, wc.1);
}

/// The gate guards' ledge: the posts (28.34, 37.12, 44.86)/(32.06, 37.12, 44.86) [D; docs/levels/
/// 07000005.md §4], the stand 3.0 north of the first post facing it [S characters.md:59]. The pair
/// is spawner-held until ch 0x22 — the check is the POSTS' reach (the playtest walks the courtyard
/// first, then spawns here; see the doc's gotcha).
#[test]
fn spots_07000005_gate_guard_ledge_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [28.3, 38.2, 41.9], 0.0, 0.0, "05 gate guards' ledge");
    let d = d3(sim.bond.pos, [28.34, 37.12, 44.86]);
    assert!(d < 6.0, "05 gate ledge: the first post {d:.2} away — the stand should be ~3.0 north of it");
    require_los(sim, [32.06, 37.12, 44.86], "05 gate ledge (second post)");
    eprintln!("[05 gate ledge] post 1 {d:.2}, post 2 {:.2}",
        d3(sim.bond.pos, [32.06, 37.12, 44.86]));
}

/// The AT-600 Scorpion 0x2E's target: the courtyard patrol guard is live at load
/// [D docs/levels/07000005.md §4, posts (50.85, 28.27, 93.63), route past (61.34, 27.06, 99.49)].
/// The spot is the real game's own chase-trace stand [L work/chase_trace.tsv, docs/systems/
/// drone-cover.md:98]: Bond at (79.86, 22.92, 96.0), the gate guard live at (61.34, 99.50) — 19
/// units away, in Combat. A heat-seeker needs a live, health>0 target in view ([M Bullet_heatseekable
/// 0x12f248]; the cone is 0.66-dot) — so: a live guard within 20 with a clear eye ray (the guard
/// walks its route, so the check polls until one is in range, the way a player would wait).
#[test]
fn spots_07000005_scorpion_guard_in_view() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [79.86, 22.92, 96.0], -1.38, 0.0, "05 scorpion spot");
    // Poll for a guard in range: the patrol walks past (61.34, 27.06, 99.49), 19 out [L trace].
    let mut found: Option<(usize, f32)> = None;
    for _ in 0..1200u32 {
        sim.step_bond(Input::default(), false, false, false);
        if let Some((di, d)) = sim.drones.list.iter().enumerate()
            .filter(|(_, dr)| dr.health > 0.0)
            .map(|(i, dr)| (i, d3(sim.bond.pos, dr.pos)))
            .filter(|&(_, d)| d < 20.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
        {
            found = Some((di, d));
            break;
        }
    }
    let Some((di, d)) = found else {
        let nearest = sim.drones.list.iter().enumerate()
            .filter(|(_, dr)| dr.health > 0.0)
            .map(|(i, dr)| (i, d3(sim.bond.pos, dr.pos)))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        panic!("05 scorpion: no live guard came within 20 in 1200 ticks (nearest {nearest:?}) — no target for the heat-seeker's cone");
    };
    eprintln!("[05 scorpion] live guard #{di} at {:?}, {d:.2} away", sim.drones.list[di].pos);
    require_los(sim, sim.drones.list[di].pos, "05 scorpion guard");
}

/// The crawl vent (the real one is 05's, not 02's — docs/playtest/spots.md §02 note): creepwall
/// #35 "Wall Hug Window" (−34.103, 37.279, −31.652) [D placement; fix_swim_vent_live.rs:48-50's
/// stand spot on the wall's axis]. E with the crawl context enters SS_CREEP [M 0x196758].
#[test]
fn spots_07000005_crawl_vent_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [-34.10, 37.28, -28.65], std::f32::consts::PI, -0.1, "05 crawl vent");
    let d = d3(sim.bond.pos, [-34.103, 37.279, -31.652]);
    assert!(d < 6.0, "05 crawl vent: {d:.2} from the wall-hug window — outside the crawl context's 12.0 stance");
    eprintln!("[05 crawl vent] distance {d:.2}");
}

// ---------------------------------------------------------------------------
// 07000008 — the safe bolts and the guard-room fusebox (the Laser 0x4E's two checks).
// ---------------------------------------------------------------------------

/// The safe bolts #207 (162.2365, 43.876, −117.145) / #208 (162.2368, 42.697, −117.144) [D #207/#208;
/// fix_party_mission_live.rs:692-693]: the burn stance 1.5 in front of the safe (fix_safe08_live.rs:421).
/// The laser's range is 4.0 [L weapon_data[0x4E]+0x1C] and the burn needs beam CONTACT — distance + LOS
/// to both bolts (the bottom one from the same spot, pitched down [M fix-safe08]).
#[test]
fn spots_07000008_safe_bolts_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [162.24, 43.33, -115.64], 3.142, -0.063, "08 safe bolts");
    let top = [162.2365, 43.875843, -117.14455];
    let bottom = [162.23679, 42.697052, -117.14434];
    let dt = d3(sim.bond.pos, top);
    assert!(dt < 4.0, "08 safe bolt #207: {dt:.2} exceeds the laser's 4.0 range");
    require_los(sim, top, "08 safe bolt #207");
    let db = d3(sim.bond.pos, bottom);
    assert!(db < 4.0, "08 safe bolt #208: {db:.2} exceeds the laser's 4.0 range");
    require_los(sim, bottom, "08 safe bolt #208");
    eprintln!("[08 safe bolts] #207 {dt:.2}, #208 {db:.2}");
}

/// The guard-room fusebox #219's wire (154.347, 43.406, −119.668) [D #219; fix_fusebox_cut_live.rs:30-64's
/// measured stance]. E never reaches a fusebox [M 0x1a5170 has no class-0x3C arm] — the Laser, distance + LOS.
#[test]
fn spots_07000008_fusebox_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [155.19, 43.03, -119.39], -1.889, -0.224, "08 fusebox");
    let wire = [154.3469, 43.406204, -119.66767];
    let d = d3(sim.bond.pos, wire);
    assert!(d < 4.0, "08 fusebox wire: {d:.2} exceeds the laser's 4.0 range");
    require_los(sim, wire, "08 fusebox wire");
    eprintln!("[08 fusebox] wire {d:.2}");
}

// ---------------------------------------------------------------------------
// 07000014 — the two sabotage fuseboxes #383/#385 [D; fix_fusebox_cut_live's measured stances].
// ---------------------------------------------------------------------------

#[test]
fn spots_07000014_fuseboxes_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000014.bin", false).expect("load 07000014");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);

    place_and_ground(sim, [-88.66, -1.17, 31.41], -2.339, -0.698, "14 fusebox #383");
    let wire = [-89.3055, -2.2213018, 30.78707];
    let d = d3(sim.bond.pos, wire);
    assert!(d < 4.0, "14 fusebox #383 wire: {d:.2} exceeds the laser's 4.0 range (the measured cut stood 1.97 out)");
    require_los(sim, wire, "14 fusebox #383 wire");

    place_and_ground(sim, [-89.35, -1.17, 48.59], 0.067, -0.739, "14 fusebox #385");
    let wire = [-89.29982, -2.2213018, 49.358276];
    let d = d3(sim.bond.pos, wire);
    assert!(d < 4.0, "14 fusebox #385 wire: {d:.2} exceeds the laser's 4.0 range (measured 1.92)");
    require_los(sim, wire, "14 fusebox #385 wire");
}

// ---------------------------------------------------------------------------
// 0700000c — the display vent/hatch and the jetpack photograph (camera 0x54).
// ---------------------------------------------------------------------------

/// The hatch #138 "ventmesh-front" (5.780, 16.404, −112.318) [D #138] from the burn stance [D
/// fix_level_0c_live.rs:89 — the lower walkway at (6.0, −110.5), past the upper walkway's end
/// wall]. The hinges #139–142 take the Laser's beam (E does nothing [M Lock_Activate]); the reach
/// proof is the laser's 4.0 range plus a clear ray to a hinge — the burn's own instrument.
#[test]
fn spots_0700000c_hatch_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    // The burn stance, floor-snapped like the green test (fix_level_0c_live.rs:89 — the lower
    // walkway PAST the upper walkway's end wall at z −110.02; from the higher walkway spot that
    // wall face blocks the ray, the same finding as fix-0c-hatch's parked ascent leg). The floor
    // height goes to the log for the doc's --bond line.
    let Some(floor) = sim.world.floor_below([6.0, 15.0, -110.5], 6.0) else {
        panic!("0c hatch: no floor under the burn stance (6.0, −110.5)");
    };
    eprintln!("[0c hatch] floor at {floor:.3} → --bond 6.0,{:.2},-110.5,0,0", floor + 1.1);
    place_and_ground(sim, [6.0, floor + 1.1, -110.5], 0.0, 0.0, "0c hatch");
    let hatch = [5.780, 16.404, -112.318];
    let d = d3(sim.bond.pos, hatch);
    assert!(d < 4.0, "0c hatch: {d:.2} from ventmesh-front — the burn stance should stand ~2.4 out (laser 4.0)");
    // The beam's own instrument is the burn: hold fire aimed at hinge #14 — the same path the
    // suite-green burn drives [M Player_LaserPointer 0x1a65d8 → Lock_Update 0x1ae3d8 case 0]. (An
    // eye-ray to the hinge is NOT the beam's check — the green burn works from here while a plain
    // segment ray hits a −x face at t 0.71, so the beam's reach is wider than a body ray.)
    let hinge = sim.drones.locks.iter().find(|l| l.out == 14).map(|l| l.pos)
        .expect("hinge lock out-14 parses [D #139]");
    sim.gun.weapon = 0x4e; // the Laser [L USATxt 0x05000043; the old 0x50 id is the Grapple]
    sim.gun.switch_request = 0x4e; // the green burn's exact setup (fix_level_0c_live.rs:86-88)
    let mut burned = false;
    for _ in 0..90u32 {
        {
            let h = sim.bond.head();
            let dd = [hinge[0] - h[0], hinge[1] - h[1], hinge[2] - h[2]];
            let l = (dd[0] * dd[0] + dd[1] * dd[1] + dd[2] * dd[2]).sqrt();
            sim.bond.yaw = dd[0].atan2(dd[2]);
            sim.bond.pitch = (dd[1] / l).clamp(-1.0, 1.0).asin() * 2.0 / std::f32::consts::PI;
        }
        sim.step_bond(Input::default(), true, false, false); // fire HELD — the burn runs while held
        if sim.drones.locks.iter().any(|l| l.out == 14 && l.burned) { burned = true; break; }
    }
    assert!(burned, "0c hatch: the beam from the burn stance never burnt hinge #14 in 60 ticks — out of the laser's reach");
    eprintln!("[0c hatch] distance {d:.2}, hinge #14 at {hinge:?} burnt from the stance");
}

/// The jetpack photograph: camera subjects #414 (5.502, 7.555, −110.366) / #454 (5.316, 8.771,
/// −109.136) [D; docs/levels/0700000c.md:36-39] from the display-ground spot [D
/// walkreal_0700000c_live.rs:106]. The camera's shot flies [M CamSubject_Update 0x1bc668 — a
/// camera-bullet CONTACT writes ch 0x1c] — distance + LOS.
#[test]
fn spots_0700000c_jetpack_photo_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [5.4, 7.35, -113.0], 0.0, 0.0, "0c jetpack photo");
    let subject = [5.502, 7.555, -110.366];
    let d = d3(sim.bond.pos, subject);
    assert!(d < 5.0, "0c photo subject #414: {d:.2} away — the display spot should stand ~2.9 out");
    require_los(sim, subject, "0c photo subject #414");
    eprintln!("[0c jetpack photo] subject {d:.2}, big subject {:.2}",
        d3(sim.bond.pos, [5.316, 8.771, -109.136]));
}

// ---------------------------------------------------------------------------
// 07000002 — the water bank, the pond swim + the ladder out, the bathhouse hatches.
// ---------------------------------------------------------------------------

/// The bank spot [D fix_swim_live.rs:43 — the user's own reported spot]: floor 4.155, the pond's west
/// edge 2.0 ahead (+x, yaw 1.57). Grounded on the bank, and the pond's west edge (water cel 17
/// x −80.3..−54.1, surface 3.765 [D fix_swim_entry_live.rs:17-24]) within a stride.
#[test]
fn spots_07000002_water_bank_grounded() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [-82.3, 5.3, 32.05], 1.57, 0.0, "02 water bank");
    // The pond edge: two strides ahead of the bank the floor gives way to the water cel.
    let edge = [-80.3, 3.765, 32.05];
    let d = d3(sim.bond.pos, edge);
    assert!(d < 4.0, "02 water bank: the pond's west edge {d:.2} away — the bank spot should face it 2.0 out");
    eprintln!("[02 water bank] pond edge {d:.2}");
}

/// The swim + the ladder out: the in-pond spot [D fix_ladder_water_live.rs:63-64] — SS_SWIM (the
/// swim, not grounded — root − 0.3 < surface [M Player_InWater 0x195e50]), hut ladder #381
/// "Ladder_medium" (−74.838, 3.396, 22.221) [D placement] ~1.2 away; hold forward grabs from the
/// swim [M Player_CollLadder 0x196118].
#[test]
fn spots_07000002_pond_ladder_in_reach() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    sim.bond = Bond::new([-75.7, 3.0, 21.4], 0.785);
    sim.bond.pitch = 0.0;
    let mut swam = None;
    for t in 0..60u32 {
        sim.step_bond(Input::default(), false, false, false);
        if swam.is_none() && sim.bond.substate == nf_game::player::SS_SWIM { swam = Some(t); }
    }
    let Some(t) = swam else { panic!("02 pond: Bond never entered the swim (substate {} at {:?})", sim.bond.substate, sim.bond.pos) };
    assert!(sim.bond.substate == nf_game::player::SS_SWIM, "02 pond: the swim dropped out at tick {t}");
    let ladder = [-74.838, 3.396, 22.221];
    let d = d3(sim.bond.pos, ladder);
    assert!(d < 3.0, "02 pond ladder #381: {d:.2} away — the swim spot should float ~1.2 from its base");
    eprintln!("[02 pond] swimming from tick {t}, ladder base {d:.2}");
}

/// The 02 "vent": the bathhouse hatch SS props #308/#309 [D; docs/levels/07000002.md — props on a
/// script, NOT crawlable; the crawl vents are 05/0d/41 per fix-swim-vent.md:58-60]. The stand is
/// floor-snapped (the bathhouse floor's height is not in any test — this measures it, numbers only).
#[test]
fn spots_07000002_bathhouse_hatch_measured() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    let Some(floor) = sim.world.floor_below([-49.75, 8.0, 6.13], 8.0) else {
        panic!("02 bathhouse: no floor under the hatch props' xz (−49.75, 6.13)");
    };
    eprintln!("[02 bathhouse] floor at {floor:.3} under hatch #308 (−49.754, 4.756, 6.130) — --bond -49.75,{:.2},6.13 works", floor + 1.05);
    place_and_ground(sim, [-49.75, floor + 1.05, 6.13], 3.142, 0.0, "02 bathhouse hatch");
    let d = dxz(sim.bond.pos, [-49.754, 4.756, 6.130]);
    assert!(d < 3.0, "02 bathhouse: hatch #308 {d:.2} out on xz — the stand should be at its face");
    eprintln!("[02 bathhouse] hatch #308 {d:.2} out on xz");
}

// ---------------------------------------------------------------------------
// 07000006 — the searchlight beams (the alarm chain is the reach proof).
// ---------------------------------------------------------------------------

/// The beam spot [D fix_06_alarm_live.rs:29 — the real game's alarm spot; trace frame 140866+]: the
/// head #2 (−1.125, 47.568, −17.329) [D placement 2] sweeps the courtyard entrance; the arm is the
/// head's own ray landing its spot within 2.0 of Bond [M Searchlight_Update 0x1cb900] — so the game's
/// own alarm channel 55 rising IS the LOS proof. The heads light in sequence from script 0x06000066
/// (ch 64 @ tick 1 …) — the wait mirrors fix_06_alarm's 720-tick budget.
#[test]
fn spots_07000006_searchlight_beam_arms() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [4.8, 42.09, -16.0], -2.21, 0.0, "06 beam spot");
    let head = [-1.125, 47.568, -17.329];
    let d = d3(sim.bond.pos, head);
    assert!(d < 15.0, "06 searchlight head #2: {d:.2} away — the beam spot should stand ~8 under its sweep");
    eprintln!("[06 beam] head #2 {d:.2} away");
    let mut alarm = None;
    for t in 0..720u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(55) { alarm = Some(t); break; }
    }
    let Some(t) = alarm else { panic!("06 beam: ch 55 never rose in 720 ticks — standing at the beam spot did not arm head #2 [M 0x1cb900:142-144]") };
    eprintln!("[06 beam] alarm ch 55 at tick {t}");
}

// ---------------------------------------------------------------------------
// 07000007 — the Kimiko scene trigger.
// ---------------------------------------------------------------------------

/// TouchOnce #412 (box 93.3..95.2 × 43.2..44.1 × −46.4..−45.4, out ch 0x48) [D; docs/systems/
/// levels.md:149] at the canonical spot [D fix_kimiko_handback_live.rs:11]: Bond lands in the box,
/// the scene fires within ~90 ticks [D fix_cutscenes2_live.rs:82-96].
#[test]
fn spots_07000007_kimiko_trigger_fires() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    // The trigger's own box centre, from the parse [D #412].
    let trig = sim.drones.triggers.iter().find(|t| t.kind == kind::TOUCH_ONCE && u32::from(t.out) == 0x48)
        .expect("the ch-0x48 TouchOnce #412 parses [D]");
    let c = trig.centre();
    eprintln!("[07 kimiko] trigger #412 centre {c:?}");
    // NOTE: no grounded assert here — the scene takes Bond over (Player_Disable) the tick the box
    // fires, freezing him where --bond put him, 1.0 above the hall floor (airborne by the sim's
    // definition). The feature check is the scene itself.
    sim.bond = Bond::new([94.2, 44.6, -45.9], 0.0);
    let d = d3(sim.bond.pos, c);
    assert!(d < 1.5, "07 kimiko: Bond {d:.2} from the trigger's centre — the spot should land inside its 1-unit box");
    let mut fired = false;
    for _ in 0..120u32 {
        if sim.disabled { sim.drones.scripts.skip_request = false; } // the scene holds control — let it play
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(0x48) { fired = true; break; }
    }
    assert!(fired, "07 kimiko: ch 0x48 never rose — standing in the box did not fire TouchOnce #412");
    eprintln!("[07 kimiko] ch 0x48 on at Bond {:?} — the scene is running (camera {})", sim.bond.pos, sim.script_camera().is_some());
}

// ---------------------------------------------------------------------------
// 07000016 — the corridor door.
// ---------------------------------------------------------------------------

/// door[7] = placement 183 "Door_5", leaf plane z ≈ −74.2 [D; walkreal_07000016_live.rs:51-52]: the
/// stand 1.2 north of it (yaw π), inside the E reach 2.5 [M 0x1a5170]. The E press opening the door
/// is the reach proof (the same path walkreal drives); the leaf's west half is sealed — x ≥ 4.4.
#[test]
fn spots_07000016_corridor_door_opens() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000016.bin", false).expect("load 07000016");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [4.9, 0.9, -73.0], 3.142, 0.0, "16 corridor door");
    let di = sim.drones.doors.list.iter().position(|d| (d.pos[2] + 74.2).abs() < 0.5)
        .expect("door[7] parses [D]");
    let d = d3(sim.bond.pos, sim.drones.doors.list[di].pos);
    assert!(d < 2.5, "16 corridor door: {d:.2} from door[7] — outside the E reach 2.5 [M 0x1a5170]");
    let mut opened = false;
    for _ in 0..90u32 {
        sim.step_bond(Input::default(), false, false, true); // the use key
        let st = sim.drones.doors.list[di].state;
        if st == nf_game::door::OPEN || st == nf_game::door::OPENING { opened = true; break; }
    }
    assert!(opened, "16 corridor door: door[7] never left state {:#x} on Bond's E — the spot is out of its use reach",
        sim.drones.doors.list[di].state);
    eprintln!("[16 corridor door] door[7] {d:.2} away, opens on E");
}

// ---------------------------------------------------------------------------
// 0700000a — the Q-Worm PC.
// ---------------------------------------------------------------------------

/// PC #688 "Q-WORM_PC_1" (−16.88, 1.20, −32.15), contact sphere r 0.7, out ch 0x48 [D;
/// docs/levels/0700000a.md:21,79]: the suite-green install spot [D walk_0700000a_live.rs:74-85].
/// The plant is FIRE, not E [M PCQWorm_Update 0x1aee20; param 2 = 0], the shot's range 2.0
/// [D weapon_data[0x58]+0x1C] — distance + LOS + the install through the real trigger path.
#[test]
fn spots_0700000a_qworm_pc_installs() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000a.bin", false).expect("load 0700000a");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    // The spot is the suite-green install stand [D walk_0700000a_live.rs:74-85]: placed and FIRED
    // immediately (the game's gadget use is aim-held fire taps, not a settled stand — the desk edge
    // slopes and a settled Bond slides ~1.1 off the spot).
    sim.bond = Bond::new([-16.38, 0.50, -31.65], -2.356);
    let pc = sim.drones.useobjs.pcs.iter().find(|p| p.ch == 0x48).map(|p| p.pos)
        .expect("the ch-0x48 Q-Worm PC parses [D #688]");
    let d = d3(sim.bond.pos, pc);
    assert!(d < 2.0, "0a qworm: the PC {d:.2} away — outside the Q-Worm shot's 2.0 range [D +0x1C]");
    // No eye-ray here: the port's cubicle blocker is a BOND-body stand-in (the eye ray is blocked at
    // t≈0.46 by a wall Bond can't pass but bullets can — walkreal_0700000a's 112 aim shots missed
    // only because Bond himself couldn't reach the desk). The shot path is the real check: the
    // install below IS the line-of-sight proof [M PCQWorm_Update 0x1aee20 waits for the bullet's
    // r-0.7 sphere contact].
    // The install through the real path: the gadget fires like a gun [M 0x1aee20].
    sim.drones.inventory.owned[0x58] = true;
    sim.gun.weapon = 0x58;
    sim.gun.switch_request = 0x58;
    let mut installed = false;
    for _ in 0..120u32 {
        sim.step_bond(Input::default(), true, false, false); // trigger taps
        if sim.drones.channels.on(0x48) { installed = true; break; }
    }
    assert!(installed, "0a qworm: firing from the spot never wrote ch 0x48 — the PC is out of the shot's path");
    eprintln!("[0a qworm] PC {d:.2} away at placement, ch 0x48 on at {:?} — the worm installed (grounded {})",
        sim.bond.pos, sim.bond.grounded);
}

// ---------------------------------------------------------------------------
// 07000012 — the lift call buttons.
// ---------------------------------------------------------------------------

/// The call button #702 (TouchOnce mask 4, volume −1.80..−1.30 × 0.86..0.91 × 8.07..8.34, out ch
/// 0x59; the same volume carries the override console #731 → ch 0x49) [D; docs/levels/07000012.md]:
/// the stance [D walk_07000012_live.rs:127-128], the E presses writing both channels.
#[test]
fn spots_07000012_lift_buttons_answer_e() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load 07000012");
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    place_and_ground(sim, [-1.55, 0.5, 7.2], 0.0, 0.0, "12 lift buttons");
    // The button's own volume, from the parse [D #702].
    let trig = sim.drones.triggers.iter()
        .find(|t| t.kind == kind::TOUCH_ONCE && t.mask & 4 != 0 && t.lo[0] <= -1.55 && t.hi[0] >= -1.55 && t.lo[2] <= 8.2 && t.hi[2] >= 8.2)
        .expect("the lift call button's E-touch volume parses [D #702]");
    let c = trig.centre();
    let d = d3(sim.bond.pos, c);
    assert!(d < 2.5, "12 lift: the call button's centre {c:?} is {d:.2} away — outside the E reach 2.5");
    let mut ch59 = false;
    for _ in 0..8u32 { sim.step_bond(Input::default(), false, false, true); if sim.drones.channels.on(0x59) { ch59 = true; break; } }
    assert!(ch59, "12 lift: E never wrote ch 0x59 — the buttons are out of the stance's use reach");
    eprintln!("[12 lift buttons] volume centre {c:?} {d:.2} away, ch 0x59 on");
}
