//! civilian-mission-wait: the mission civilian's pause state (`CivilianMissionWait` 0x15dbb0,
//! `NDrone2_StateFuncs[0x17]` [L 0x29b300]).
//!
//! CivilianMission (0x16) case 0x22 parks the walker in 0x17: stand idle at alert < 0.66 (else the
//! look-around 0x21), timer C armed 2 s (+0xCC4 = GameState + FRAME_RATE_INT·2 [M 0x15dbb0:29]) —
//! `NDrone2_PreDroneControl` 0x148db0:97-111 fires msg 0xc when the clock expires and the state
//! re-arms it while Bond stands within 3.0 in the 0.525 rad view cone, else resumes 0x16 [M
//! 0x15dbb0:87-111; BondWithinAngle 0x154f98]. A second 0x22 while waiting re-arms the clock [M
//! 0x15dbb0:136-141]. The decompile holds no literal msg-0x22 sender (every send site checked,
//! 2026-10-16 — see the msg::WAIT_PAUSE doc in drone.rs); the tests drive the id through the real
//! state-machine entry (`Drone::send`), the path a data-driven sender feeds.
//!
//! State and msg ids are written raw (`0x16`, `0x17`, `0x22` …) so this file compiles — and the
//! assertions fail — against the pre-fix master. Opt-in via `NF_ISO`. Level 07000007: the party's
//! 35 dtype-10 guests carry prop 0x1D and six kind-8 mission paths sit under their placements
//! [D, docs/WORKLOG fix-drone4].

use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The walker: a dtype-10 guest with no switch channel (starts immediately) whose placement sits on
/// a kind-8 mission path node within the game's 0.3 acceptance [M AssignAIPath 0x154190: fVar11 = 0.3].
fn find_walker(sim: &Scene) -> usize {
    let data = sim.data();
    let walker = sim.drones.list.iter().position(|dr| {
        dr.present && dr.dtype == 10 && dr.switch_channel == 0
            && data.nav.paths.iter().any(|p| p.kind & 8 != 0 && p.nodes.iter().any(|n| {
                (n.pos[1] - dr.feet()[1]).abs() <= 2.0
                    && (dr.pos[0] - n.pos[0]).powi(2) + (dr.pos[2] - n.pos[2]).powi(2) < 0.09
            }))
    }).expect("07000007 places a guest on a kind-8 mission path (paths 2/4/5 node 0)");
    drop(data);
    walker
}

fn dist(a: &[f32; 3], b: &[f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// The `Data` the msg path needs, built from disjoint field borrows so the send can take
/// `&mut sim.drones.rng` alongside it (the explode_live pattern).
fn data_for(sim: &Scene) -> nf_game::drone::Data<'_> {
    nf_game::drone::Data {
        tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
        world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
        skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
        hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
        surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
    }
}

fn send_wait(scene: &mut Scene, walker: usize) {
    let sim = scene.sim.as_mut().unwrap();
    let data = data_for(sim);
    let frame = sim.drones.frame;
    sim.drones.list[walker].send(&data, &mut sim.drones.rng, frame, 0x22);
}

/// The wait request parks a walking mission guest in CivilianMissionWait 0x17: she freezes (the
/// state has no route half), holds the ~2 s timer-C wait with Bond far away, then resumes 0x16 and
/// walks again. Pre-fix the port has no msg-0x22 arm: the message drops through the GLOBAL fallback
/// and the guest walks straight through — the audit's symptom [M 0x15d860 case 0x22; 0x15dbb0].
#[test]
fn a_mission_guest_pauses_on_the_wait_message_and_resumes() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let walker = find_walker(sim);

    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    {
        let dr = &sim.drones.list[walker];
        assert_eq!(dr.state, 0x16, "the guest runs CivilianMission [M 0x15d860] — got {:#x}", dr.state);
        assert!(dr.route.is_some(), "the guest walks an assigned mission route");
    }

    // The wait request — Bond parked far away so the timer expiry takes the resume arm.
    let guest_pos = sim.drones.list[walker].pos;
    let wait_frame = sim.drones.frame;
    send_wait(scene, walker);
    let sim = scene.sim.as_mut().unwrap();
    {
        let dr = &sim.drones.list[walker];
        assert_eq!(dr.state, 0x17, "msg 0x22 parks the walker in CivilianMissionWait [M 0x15d860:130-136] — got {:#x}", dr.state);
        assert!(dr.timer_c.is_some(), "the wait clock is armed [M 0x15dbb0:29-32]");
        let idle = dr.anim.state == 0x24 || dr.anim.call.map_or(false, |c| c.0 == 0x24);
        assert!(idle, "alert {:.2} < 0.66 stands idle 0x24 [M 0x15dbb0:23-25] — anim {:#x}", dr.alert, dr.anim.state);
    }

    // She freezes: the wait state has no route half — 100 ticks of standing, not walking.
    for _ in 0..100 { sim.bond.pos = [guest_pos[0] + 40.0, guest_pos[1], guest_pos[2]]; sim.step_bond(Default::default(), false, false, false); }
    {
        let dr = &sim.drones.list[walker];
        assert_eq!(dr.state, 0x17, "still waiting inside the 2 s clock (timer fires at {}+120)", wait_frame);
        let moved = dist(&dr.pos, &guest_pos);
        assert!(moved < 0.3, "the guest pauses at the wait node (moved {moved:.2} in 100 ticks — the port walked straight through pre-fix)");
    }

    // The clock expires with Bond far: back to CivilianMission and walking again [M 0x15dbb0:87-110].
    let mut resumed = false;
    for _ in 0..60 {
        sim.bond.pos = [guest_pos[0] + 40.0, guest_pos[1], guest_pos[2]];
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.list[walker].state == 0x16 { resumed = true; break; }
    }
    assert!(resumed, "the expired wait resumes CivilianMission 0x16 [M 0x15dbb0 case 0xc → SetState(0x16)]");
    let after = sim.drones.list[walker].pos;
    for _ in 0..150 { sim.bond.pos = [guest_pos[0] + 40.0, guest_pos[1], guest_pos[2]]; sim.step_bond(Default::default(), false, false, false); }
    let dr = &sim.drones.list[walker];
    let moved = dist(&dr.pos, &after);
    assert!(moved > 0.2, "the guest walks again after the resume (moved {moved:.2} in 150 ticks)");
}

/// Sister: the wait EXTENDS while Bond stands close ahead — the expired clock's case-0xc check
/// re-arms the 2 s wait while Bond is inside the 0.525 rad cone within 3.0 (2D) [M 0x15dbb0:87-106;
/// BondWithinAngle 0x154f98], and a second wait request re-arms too [M 0x15dbb0:136-141]. With Bond
/// parked 2.0 ahead of her facing the guest is still waiting 2.5 s later; the same run with Bond far
/// resumed inside 2 s (the first test's arm).
#[test]
fn the_wait_holds_while_bond_stands_close_ahead() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let walker = find_walker(sim);

    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.drones.list[walker].state, 0x16, "the guest walks first");
    let guest_pos = sim.drones.list[walker].pos;
    let yaw = sim.drones.list[walker].yaw;
    send_wait(scene, walker);
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.drones.list[walker].state, 0x17, "the wait state entered");

    // Bond 2.0 units ahead of her facing (forward = [sin yaw, 0, cos yaw]), inside cone + radius.
    let ahead = [guest_pos[0] + yaw.sin() * 2.0, guest_pos[1], guest_pos[2] + yaw.cos() * 2.0];
    for _ in 0..150 {
        sim.bond.pos = ahead;
        sim.step_bond(Default::default(), false, false, false);
    }
    {
        let dr = &sim.drones.list[walker];
        assert_eq!(dr.state, 0x17,
            "Bond 2.0 ahead in the 0.525 cone keeps the wait alive past the first 2 s clock [M 0x15dbb0:88-106] — got {:#x}", dr.state);
        assert!(dr.timer_c.is_some(), "the clock re-armed for another 2 s");
        let moved = dist(&dr.pos, &guest_pos);
        assert!(moved < 0.3, "she still has not moved (moved {moved:.2})");
    }

    // A second wait request while waiting re-arms the clock [M 0x15dbb0:136-141].
    sim.drones.list[walker].send(&sim.data(), &mut sim.drones.rng, sim.drones.frame, 0x22);
    for _ in 0..100 {
        sim.bond.pos = [guest_pos[0] + 40.0, guest_pos[1], guest_pos[2]];
        sim.step_bond(Default::default(), false, false, false);
    }
    assert_eq!(sim.drones.list[walker].state, 0x17,
        "the re-sent wait request holds the pause past the original deadline [M 0x15dbb0:136-141]");
    // Bond leaves: the next expiry resumes the route.
    let mut resumed = false;
    for _ in 0..60 {
        sim.bond.pos = [guest_pos[0] + 40.0, guest_pos[1], guest_pos[2]];
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.list[walker].state == 0x16 { resumed = true; break; }
    }
    assert!(resumed, "Bond gone → the wait ends and the route resumes [M 0x15dbb0 case 0xc else-arm]");
}
