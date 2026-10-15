//! map-tower2 walkthrough: 07000014 "HT Tower2 Elevator" — Drake's underground facility, the real path
//! from the level's own data.
//! Run with: `NF_ISO=… cargo test -p nf-view --test walk_07000014_live -- --nocapture --test-threads=1`
//!
//! The chain [D placements; M decompile addresses cited inline]:
//! 1. Cold start #666/#667 (gate 0) at (40.81, 2.40, 65.08) — the vent store room [D]. Objective 1
//!    "Infiltrate Drake's facility undetected" (0x04000049, ch 0x47) announces [L c07000014.rs].
//! 2. TouchOnce #301 (out 0xD9; OR #302 [0xDA, 0xD9] → 0x7E) [D] fires ScriptPlayer #276 (trigger
//!    ch 0x7E): its script 0x06000833 (pack 07800014) writes event 0xD [0x47, 1] — the objective —
//!    and 0xD [0x0E, 1] [D nis decode].
//! 3. TouchOnce #381 (out 0x08) [D] fires SP #382 (trigger ch 8): script 0x0600062F (pack 07900014)
//!    writes 0xD [0x48, 1] — "Follow Kiko to the Security Control Center" — and [0x10, 1], [0xB8, 0].
//! 4. The sabotage: FuseBox #383 (p1 0x87, p2 0x71) and #385 (p1 0x88, p2 0xD8) [D]; cutting each
//!    writes its channels [M FuseBox_Update 0x1adfe8] → AND #391 (type 3, [0x87, 0x88] → 0x8C) [D].
//! 5. Touch #456 (type 2, out 0x0C) with 0x8C on → SEQ_AND #445 ([0x8C, 0x0C] → 0x09) [D] fires
//!    SP #407 (trigger ch 9): script 0x06000848 (pack 07A00014) writes 0xD [0x4A, 1] — "Sabotage base
//!    security and escape from room" — and [0x11, 1].
//! 6. Touch #476 (out 0x0F) with 0x8C → SEQ_AND #475 ([0x8C, 0x0F] → 0x0D) [D] wakes Rook (skin
//!    0x05000066 at (27.81, 0.20, 45.78), switch channel 0x0D, death channel 0x54 [D drone params]).
//!    His death raises ch 0x54 — "Prevent Rook from cutting the power to Delta Sector" [M
//!    set_death_channel 0x13e660 on the DEATH_ANIM exit].
//! 7. The exit: "Load Level Point" #944 (next 0x07000015, key gate 0x54, fallback 0xCE) [D] — E with
//!    ch 0x54 on writes ch 0xCE ("Take the exit to Delta Sector") and loads 07000015 [M 0x1b2318
//!    case 10]. Sister: with 0x54 off the key gate locks it (no load, no channel).

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn free(sim: &mut nf_view::scene::Sim, ticks: u32) -> bool {
    for _ in 0..ticks { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { return true; } }
    !sim.disabled
}

fn mission_state(sim: &nf_view::scene::Sim, obj: usize) -> u32 {
    let (i, _) = nf_game::mission::LEVELS.iter().enumerate().find(|(_, d)| d.level == 0x0700_0014).unwrap();
    sim.drones.mission.states[i][obj]
}

/// The five-stage chain: the vent drop (ch 0x47), Kiko's walk (ch 0x48), the fusebox sabotage
/// (0x87+0x88 → 0x8C), the server-room escape (ch 0x4A), Rook (ch 0x54) and the exit (ch 0xCE +
/// 07000015). Sisters: the room escape refuses without 0x8C, and the exit refuses without 0x54.
#[test]
fn the_facility_chain_runs_to_the_delta_sector_exit() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000014.bin", false).expect("load 07000014");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0014);
    let p = sim.bond.pos;
    assert!(p[0] > 39.0 && p[0] < 42.0 && p[1] > 1.5 && p[1] < 6.0 && p[2] > 63.0 && p[2] < 67.0,
        "the cold start spawns in the vent store room (#666 [D]), got {p:?}");
    assert!(free(sim, 400));
    assert_eq!(mission_state(sim, 0), 3, "'Infiltrate Drake's facility' announces at the start");

    // Stage 1: the vent drop — TouchOnce #301 (39.68…41.75, 2.07…4.14, 37.62…40.40) [D]. The
    // cut-scene 0x06000833 runs 1400 ticks and writes ch 0x47 at its last tick (0x577) [D].
    sim.bond.pos = [40.7, 2.6, 39.0];
    let mut ch47 = false;
    for _ in 0..1700u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(0x47) { ch47 = true; break; }
    }
    assert!(ch47, "the vent cut-scene 0x06000833 must write ch 0x47 [D]");
    // fix-objective-banner: the monitor completes the objective the tick after the channel.
    for _ in 0..2 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(mission_state(sim, 0), 4, "the infiltrate objective completes");
    assert!(free(sim, 900), "the cut-scene releases Bond");

    // Stage 2: Kiko's walk — TouchOnce #381 (−51.97…−46.39, 0…2.07, 37.28…39.46) [D].
    sim.bond.pos = [-49.2, 0.9, 38.4];
    let mut ch48 = false;
    for _ in 0..1500u32 { // 0x0600062F runs 1200 ticks [D].
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(0x48) { ch48 = true; break; }
    }
    assert!(ch48, "the Kiko cut-scene 0x0600062F must write ch 0x48 [D]");
    // fix-objective-banner: the monitor completes the objective the tick after the channel.
    for _ in 0..2 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(mission_state(sim, 1), 4, "the follow-Kiko objective completes");
    assert!(free(sim, 900));

    // Sister for the sabotage: the server-room escape touch #456 (type 2, out 0x0C) with 0x8C off
    // must not fire the SEQ_AND (#445 [0x8C, 0x0C] → 0x09).
    sim.bond.pos = [-33.1, 0.9, 26.8];
    for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x0c), "the room touch #456 writes 0x0C on contact [D]");
    assert!(!sim.drones.channels.on(0x09), "without 0x8C the SEQ_AND #445 must not fire [D]");

    // Stage 3: the sabotage through the REAL cut — the two west generator-room fuseboxes [D #383
    // (p1 0x87), #385 (p1 0x88); the level's other three carry 0x22/0xE1/0xD1/0x6F]. Each box's
    // script registers its wires (the event-0xF hand-off, `FuseBox_Register` 0x1b0970 [L
    // NF_SCRIPTLOG: streams 2/6 → groups 0/1]); the Laser's beam bullets (category word 0x0002,
    // which survives `Collide_FilterBullets`'s 0xfffd [L weapon_data; M 0x1ece30]) cut them and
    // `FuseBox_Update` writes the channels [M 0x1adfe8]. `Player_Activate` 0x1a5170 has no
    // class-0x3C arm — E does nothing [M 0x1a5170:95-160].
    for want in [0x87u32, 0x88] {
        let (fb, wires) = {
            let sim = scene.sim.as_ref().unwrap();
            let f = sim.drones.fuseboxes.iter().position(|f| f.ch[0] as u32 == want)
                .unwrap_or_else(|| panic!("the fusebox with group-0 channel {want:#x} exists [D]"));
            let list: Vec<[f32; 3]> = sim.drones.fuseboxes[f].alive_wires()
                .filter_map(|(_, _, o)| nf_game::drone::Drones::script_obj_pos(&sim.drones.scripts, &sim.data(), o))
                .collect();
            assert!(!list.is_empty(), "box {f}: the script registered wires (groups 0 and 1 [L])");
            (f, list)
        };
        for wp in &wires {
            {   // settle near the box (floor pre-screened — never over a void), close inside the
                // Laser's 4.0 range [L weapon_data 0x4E +0x1C], draw, aim through the wire, burn.
                let sim = scene.sim.as_mut().unwrap();
                let bp = sim.drones.fuseboxes[fb].pos;
                let cands = [[0.6f32, -1.2, 0.0], [-0.6, -1.2, 0.0], [0.0, -1.2, 0.6], [0.0, -1.2, -0.6]];
                let mut stood = false;
                for c in cands {
                    let spot = [bp[0] + c[0], bp[1] + c[1], bp[2] + c[2]];
                    let floor = sim.world.segment(spot, [spot[0], spot[1] - 30.0, spot[2]]).map(|t| spot[1] - 30.0 * t);
                    if !matches!(floor, Some(f) if (f - (bp[1] - 2.4)).abs() < 3.0) { continue; }
                    sim.bond.pos = spot;
                    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
                    let head = sim.bond.head();
                    let d = [wp[0] - head[0], wp[1] - head[1], wp[2] - head[2]];
                    if (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() <= 3.4 { stood = true; break; }
                }
                assert!(stood, "a stance inside the Laser's range exists for box {fb}'s wire at {wp:?}");
            }
            for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x4e), ..Default::default() }, false, false, false); }
            let mut dead = false;
            for _ in 0..200u32 {
                {
                    let sim = scene.sim.as_mut().unwrap();
                    let head = sim.bond.head();
                    let dir = [wp[0] - head[0], wp[1] - head[1], wp[2] - head[2]];
                    let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
                    sim.bond.yaw = dir[0].atan2(dir[2]);
                    sim.bond.pitch = (dir[1] / l).asin() / (std::f32::consts::FRAC_PI_2);
                }
                scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
                if scene.sim.as_ref().unwrap().drones.channels.on(want) { dead = true; break; }
            }
            assert!(dead, "the laser cuts box {fb}'s wire → ch {want:#x} [M 0x1adfe8]");
        }
    }
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(0x8c), "AND #391 [0x87, 0x88] raises 0x8C [D]");

    // The live-wire hurt: the game hurts on a group-1 wire death ONLY while +0x56 (param 2) == 0
    // [M 0x1adfe8:99-101 `if (*(short *)(iVar1 + 0x56) == 0) Player_Hurt(0x40a00000,…)`]. Every
    // fusebox on 07000014 carries param 2 non-zero (placements 291/383/385/833/919 → 225/113/216/
    // 224/112 [D, fields_live.rs census]), so cutting the live wire here NEVER hurts — the old
    // "the live wire costs 5.0" reading was the inverted polarity docs/fields/fusebox.md fixed.
    assert_eq!(scene.sim.as_ref().unwrap().drones.health.health, 100.0,
        "no live-wire hurt with +0x56 configured [M 0x1adfe8:99-101]");

    // Stage 4: the server-room escape — the touch again, now with 0x8C on → 0x09 → SP #407 (0x06000848)
    // → ch 0x4A.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [-33.1, 0.9, 26.8];
    }
    let mut ch4a = false;
    for _ in 0..1300u32 { // 0x06000848 runs 950 ticks [D].
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.channels.on(0x4a) { ch4a = true; break; }
    }
    assert!(ch4a, "the escape cut-scene 0x06000848 must write ch 0x4A [D]");
    // fix-objective-banner: the monitor completes the objective the tick after the channel.
    for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(mission_state(scene.sim.as_ref().unwrap(), 2), 4, "the sabotage objective completes");
    assert!(free(scene.sim.as_mut().unwrap(), 900));

    // Stage 5: Rook — the touch #476 (19.22…21.30, −26.26…−20.69) with 0x8C → 0x0D wakes him [D].
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [20.3, 0.9, -23.5];
    }
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let rook_i = scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.skin == 0x0500_0066).expect("Rook (skin 0x05000066) exists");
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(0x0d), "SEQ_AND #475 [0x8C, 0x0F] raises 0x0D [D]");
    assert!(scene.sim.as_ref().unwrap().drones.list[rook_i].started, "Rook leaves his switch-channel wait [M 0x14b300]");
    // Kill him through the game's own explosive path (the castle walkthrough's grenade route).
    // fix-weapons-all [M Bullet_CollisionHandler 0x12ce80]: a landed grenade never explodes on
    // contact — it bounces/sits and pops at its tick fuse. This drop is set one tick from its fuse
    // (travelled 59 of range 60) so the blast lands before Rook's AI walks him out of the 3-unit
    // radius — the game's own timing for a grenade that lands and immediately goes off.
    let rp = scene.sim.as_ref().unwrap().drones.list[rook_i].pos;
    scene.sim.as_mut().unwrap().drones.grenades.push(nf_game::grenade::Grenade {
        pos: [rp[0], rp[1] + 0.5, rp[2]], velocity: [0.0, 0.0, 0.0], speed: 0.0,
        travelled: 59.0, range: 60.0, damage: 100.0, radius: 3.0, weapon_type: 0x34, mesh: 0x0200_0726, owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, resting: false,
        stuck: false, seek_target: None,
        phase: None,
    });
    let mut dead = false;
    for _ in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.list[rook_i].corpse { dead = true; break; }
    }
    assert!(dead, "the explosion must kill Rook");
    let mut ch54 = false;
    for _ in 0..600u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.channels.on(0x54) { ch54 = true; break; }
    }
    assert!(ch54, "Rook's death channel 0x54 fires [D drone param 12; M 0x13e660]");
    // fix-objective-banner: the monitor completes the objective the tick after the channel.
    for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(mission_state(scene.sim.as_ref().unwrap(), 3), 4, "the Rook objective completes");

    // Sister for the exit: the key gate 0x54 was off before Rook died — a fresh load proves the lock.
    // (Here: with 0x54 ON, E at the exit #944 writes ch 0xCE and requests 07000015.)
    {
        let sim = scene.sim.as_mut().unwrap();
        let exit = sim.drones.triggers.iter()
            .find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.next_level == 0x0700_0015)
            .expect("the Load Level Point #944 exists").clone();
        assert_eq!(exit.key_gate, 0x54, "the exit's key gate is ch 0x54 [D]");
        sim.bond.pos = [(exit.lo[0] + exit.hi[0]) * 0.5, exit.lo[1] + 0.1, (exit.lo[2] + exit.hi[2]) * 0.5];
        sim.bond.yaw = 0.0;
        for _ in 0..6 { sim.step_bond(Input::default(), false, false, true); }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0xce), "the exit writes ch 0xCE ('Take the exit…') [M 0x1b2318 fallback arm]");
        assert_eq!(sim.drones.level_exit, Some(0x0700_0015), "the exit loads 07000015 (EvilBase) [D]");
    }
    let next = nf_view::level_archive_name(0x0700_0015).to_string();
    let scene2 = nf_view::load(&iso, &next, false).expect("07000015 must load after the exit");
    assert_eq!(scene2.sim.as_ref().expect("sim").level, 0x0700_0015);
}

/// The exit's key gate: a fresh load, E at #944 with ch 0x54 off — locked, no channel, no load
/// [M Trigger_Activate 0x1b2318:76-90].
#[test]
fn the_exit_stays_locked_until_rook_is_dealt_with() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000014.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim, 400));
    let exit = sim.drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.next_level == 0x0700_0015)
        .expect("the exit exists").clone();
    sim.bond.pos = [(exit.lo[0] + exit.hi[0]) * 0.5, exit.lo[1] + 0.1, (exit.lo[2] + exit.hi[2]) * 0.5];
    sim.bond.yaw = 0.0;
    for _ in 0..6 { sim.step_bond(Input::default(), false, false, true); }
    assert_eq!(sim.drones.level_exit, None, "the key-gated exit refuses E while 0x54 is off [M 0x1b2318:76-90]");
    assert!(!sim.drones.channels.on(0xce), "no fallback write while locked");
}
