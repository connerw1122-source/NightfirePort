//! walkreal: 07000003 "Alpine Escape" (Castle C) walked with the player's own movement — no
//! teleports after the spawn. The route (docs/levels/07000003.md + the disc dump):
//!   the 0x24 start (−8.85, 6.44, −23.19; the 0x4d objective-reveal touch fires under Bond) →
//!   the front gate (door[0] at (14.7, 6.6, −35.4) starts locked behind ch 0x31; the E-plate
//!   kind-1 mask-4 at (11.8, 6.0, −35.6) writes 0x31, which unlocks the door and plays the
//!   gate SS #502, ch 0x31, 2 keys — the "key-gated route") → the 0x21 guard-enable plates →
//!   the servants (skin 0x0500002b ×3, team 3, death channel 0x60 = protected) → the east lawn →
//!   the Load Point #451 (18.11, 6.65, 16.05; E, key gate ch 0x52).
//!
//! The objective channels: the disc's mux family writes the gate —
//!   AND(0x4f, 0x50) → 0x51 (obj1 "Rescue Mayhew's servants") and AND(0x4e, 0x51) → 0x52
//!   (obj2's reveal AND the exit's key gate) [D the kind-3 triggers, out 0x51 list 4f,50 and
//!   out 0x52 list 4e,51]. The level's scripts write only ch 0x96 [D the lib scan] and no
//!   placement, pickup, door or SS writes 0x4e/0x4f/0x50 [D the dump] — those three stay the
//!   un-ported mission/dialogue layer's writers (the walkreal-a precedent on 07000001's 0x47 and
//!   07000002's 0x4c), and the tests hand them at the moments the level offers, then let the
//!   disc's own AND triggers do the rest through the real trigger path.

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The mission-table row and objective indices for this level [L ee_start_05 @ 0x2a3f18 via
/// mission::c07000003]: obj0 0x4e "Retrieve Dragon Safe contents", obj1 0x51 "Rescue Mayhew's
/// servants", obj2 0x53 "Find door leading to Mayhew" (enable 0x52).
fn mission_row() -> (usize, usize, usize, usize) {
    let li = nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0003).expect("the 07000003 mission record");
    (li, 0, 1, 2)
}

#[test]
fn walk_the_alpine_route_to_the_exit() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000003.bin", false).expect("load 07000003"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - -8.85).abs() < 2.0 && (p[2] - -23.19).abs() < 2.0, "the 0x24 spawn [D], got {p:?}");
        assert!(!sim.disabled, "no intro blocks this level's start");
    }
    w.wait(60);
    {
        // The spawn's objective-reveal touch (out 0x4d at (−9.0,−23.2) [D dump]) — both first
        // objectives' enable channel, so the monitor announces them [M 0x1827d8:55-67].
        let sim = w.scene.sim.as_ref().unwrap();
        let (li, o0, o1, _) = mission_row();
        assert!(sim.drones.channels.on(0x4d), "the spawn reveal touch fires under Bond [D]");
        assert_eq!(sim.drones.mission.states[li][o0], 3, "obj0 announced through the real monitor");
        assert_eq!(sim.drones.mission.states[li][o1], 3, "obj1 announced through the real monitor");
    }

    // The route: north-east across the courtyard to the Load Point. The nav graph fills between
    // the anchors; the guard-enable plates (0x21/0x23/0x24/0x25) fire under Bond's feet.
    for (name, at) in [
        ("courtyard east", [0.0f32, 5.9, -20.0]),
        ("mid courtyard", [13.0, 5.9, -18.0]),
        ("the east lawn", [22.0, 5.9, 5.0]),
        ("the exit lawn", [18.5, 5.9, 13.0]),
    ] {
        w.walk_nav_to(at, name, 1.6);
    }

    // 6. The exit: E on the Load Point #451 without ch 0x52 refuses [M Trigger_Activate 0x1b2318:76-90].
    let sim = w.scene.sim.as_ref().unwrap();
    let exit = sim.drones.triggers.iter().find(|t| t.kind == walker::trigger_kind::LOAD_LEVEL).map(|t| t.centre()).expect("the Load Point");
    drop(sim);
    w.walk_to(Wp { at: exit, act: Act::Use { use_ticks: 90 }, arrive: 1.6 }, "E without the key");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x52), "neither objective fired, so the AND(0x4e,0x51) has not written 0x52 [D]");
        assert_eq!(sim.drones.level_exit, None, "the key gate refuses the exit without ch 0x52 [M 0x1b2318:76-90]");
    }
    // The mission/dialogue layer's writers for 0x4e/0x4f/0x50 are not on the disc [D the scan];
    // hand the LEAF channels the way that layer would — 0x51 and 0x52 are the mux family's OUTPUTS
    // (the AND triggers drive them every tick, so a direct hand is overwritten [M 0x1b1a78's
    // unconditional AND write; the recon]). Everything downstream is the disc's own mux family:
    // AND(0x4f,0x50) writes 0x51 [D kind-3 out 0x51 list 4f,50], then AND(0x4e,0x51) writes 0x52
    // [D out 0x52 list 4e,51], which announces obj2 (enable 0x52) and unlocks the exit.
    let sim = w.scene.sim.as_mut().unwrap();
    for &c in &[0x4eu32, 0x4f, 0x50] { sim.drones.channels.set(c, 1, sim.drones.frame); }
    drop(sim);
    w.wait(10);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let (li, _, o1, o2) = mission_row();
        assert!(sim.drones.channels.on(0x51), "the disc's AND(0x4f,0x50) writes obj1's channel [D]");
        assert!(sim.drones.channels.on(0x52), "the disc's AND(0x4e,0x51) writes the key gate [D]");
        assert_eq!(sim.drones.mission.states[li][o1], 4, "obj1 COMPLETE through the real monitor [M 0x1827d8:78-85]");
        assert_eq!(sim.drones.mission.states[li][o2], 3, "obj2 announced when its enable channel 0x52 fires [M 0x1827d8:55-67]");
    }
    w.walk_to(Wp { at: exit, act: Act::Use { use_ticks: 90 }, arrive: 1.6 }, "E with the key");
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0004), "the exit loads 07000004 through the real reload path");
    eprintln!("[03] walked to the exit in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[03] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[03] stuck: {s}"); }
}

/// The full play: every objective, the front gate, the servants, the exit — the whole route
/// through the real input (walking, turning, E). The mission layer's three writers (0x4e/0x4f/0x50)
/// are handed at the moments the level offers; the AND chain, the announce/complete states, the
/// gate and the exit are all the disc's own.
#[test]
fn walk_the_full_route_and_complete_every_objective() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000003.bin", false).expect("load 07000003"));
    let (li, o0, o1, o2) = mission_row();
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - -8.85).abs() < 2.0 && (p[2] - -23.19).abs() < 2.0, "the 0x24 spawn [D], got {p:?}");
    }
    w.wait(60);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x4d), "the spawn reveal [D]");
        assert_eq!(sim.drones.mission.states[li][o0], 3, "obj0 announced [M 0x1827d8:55-67]");
        assert_eq!(sim.drones.mission.states[li][o1], 3, "obj1 announced [M 0x1827d8:55-67]");
    }

    // 1. The front gate. The plate (kind 1, mask 4 = E-usable [M Trigger_Activate 0x1b2318:38-49])
    //    writes 0x31; the door starts locked behind it and the gate SS #502 listens on 0x31. The
    //    approach is the west lane (z≈−33..−36): the straight courtyard line crosses the garden
    //    moat ditch ((11.8,−34.5) and (16,−31) drop into the water [D the dump's 5 water cels]).
    let sim = w.scene.sim.as_ref().unwrap();
    let gate_plate = sim.drones.triggers.iter()
        .find(|t| t.kind == walker::trigger_kind::TOUCH_ONCE && t.out == 0x31).map(|t| t.centre())
        .expect("the gate plate");
    let gate_door = sim.drones.doors.list.iter()
        .position(|d| d.ch_lock == 0x31).expect("the locked gate door");
    let gate_ss = sim.drones.sss.list.iter()
        .position(|s| s.channel == 0x31).expect("the gate SS");
    drop(sim);
    for (name, at) in [
        ("west lawn", [0.0f32, 5.9, -30.0]),
        ("the west lane", [1.0, 5.9, -33.0]),
        ("along the moat", [8.0, 5.9, -36.0]),
    ] {
        w.walk_nav_to(at, name, 1.6);
    }
    w.walk_to(Wp { at: gate_plate, act: Act::None, arrive: 1.4 }, "to the gate plate");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x31), "the gate is shut before the plate [sister]");
        assert_eq!(sim.drones.doors.list[gate_door].state, 2, "the gate door sits closed at create [M Door_Create 0x136100 +0xf4 = 2]");
    }
    // Sister first: E on the locked door refuses (the ch_lock refusal [M Door_Activate 0x135be0 —
    // the ch_lock gate + the hit feedback]).
    let sim = w.scene.sim.as_ref().unwrap();
    let gate_at = sim.drones.doors.list[gate_door].pos;
    drop(sim);
    w.walk_to(Wp { at: gate_at, act: Act::Use { use_ticks: 90 }, arrive: 1.3 }, "E the locked gate door");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let d = &sim.drones.doors.list[gate_door];
        assert!(d.pos01 < 0.5 && d.state != 4, "the locked door refuses E while ch 0x31 is off [M 0x135be0] — state {:#x} pos01 {}", d.state, d.pos01);
    }
    // The plate: E writes 0x31, the door's lock channel turns on, the gate SS plays its keys —
    // a 2-key one-shot (wrap flag non-zero → the wrap disarms to state 0 [M 0x1a9948:119-128]) whose
    // pose HOLDS the end key, so pin the posed matrix leaving its rest pose.
    let ss_rest = w.scene.sim.as_ref().unwrap().drones.sss.list[gate_ss].matrix;
    w.walk_to(Wp { at: gate_plate, act: Act::Use { use_ticks: 90 }, arrive: 1.4 }, "E on the gate plate");
    w.wait(20);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x31), "the plate writes ch 0x31 [D out 0x31 mask 4]");
        let ss = &sim.drones.sss.list[gate_ss];
        assert!(ss.matrix != ss_rest || ss.t > 0.0 || ss.state == 1,
            "the gate SS plays its keys on 0x31 [D ss #502 ch 0x31], got state {:#x} t {}", ss.state, ss.t);
    }
    w.wait(90);
    // Now the door opens to E and Bond walks through to the gate's north side.
    w.walk_to(Wp { at: gate_at, act: Act::Use { use_ticks: 120 }, arrive: 1.3 }, "E the unlocked gate door");
    w.walk_nav_to([14.7, 5.9, -36.8], "through the gate", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.bond.pos[2] < -36.0, "Bond is behind the front gate [route], got {:?}", sim.bond.pos);
    }

    // 2. The servants. Three skin-0x0500002b team-3 drones, death channel 0x60 (protected
    //    [M Mission_MonitorObjectives' standing hook]) [D the dump]. The mission layer writes
    //    0x4f/0x50 when each is rescued — the level offers no writer [D the scan], so hand each
    //    as Bond stands with the servant; the disc's AND(0x4f,0x50) does the rest.
    let servants: Vec<[f32; 3]> = {
        let sim = w.scene.sim.as_ref().unwrap();
        sim.drones.list.iter().filter(|d| d.skin & 0xffff == 0x2b && d.team == 3)
            .map(|d| d.pos).collect()
    };
    assert_eq!(servants.len(), 3, "three protected servants [D the dump]");
    for (k, at) in servants.iter().enumerate() {
        w.walk_nav_to([at[0], at[1] - 0.3, at[2]], &format!("servant {k}"), 1.7);
        let sim = w.scene.sim.as_ref().unwrap();
        let si = sim.drones.list.iter().position(|d| d.skin & 0xffff == 0x2b && walker::dxz(d.pos, *at) < 1.0).expect("the servant");
        let d = &sim.drones.list[si];
        assert!(!d.corpse, "servant {k} alive when Bond arrives");
        assert_eq!(d.death_channel, 0x60, "servant {k} is protected [D death_ch]");
        drop(sim);
        let sim = w.scene.sim.as_mut().unwrap();
        // The un-ported mission layer's rescue write: 0x4f for the first, 0x50 for the second
        // (the AND's two inputs [D]). The third servant has no AND input — the level's own data
        // only ever ANDs two.
        sim.drones.channels.set(if k == 0 { 0x4f } else { 0x50 }, 1, sim.drones.frame);
    }
    w.wait(10);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x51), "the disc's AND(0x4f,0x50) writes obj1's channel [D]");
        assert_eq!(sim.drones.mission.states[li][o1], 4, "obj1 COMPLETE through the real monitor [M 0x1827d8:78-85]");
    }

    // 3. obj0 "Retrieve Dragon Safe contents": no on-level entity writes 0x4e [D the scan] — hand
    //    it the way the mission layer would.
    let sim = w.scene.sim.as_mut().unwrap();
    sim.drones.channels.set(0x4e, 1, sim.drones.frame);
    drop(sim);
    w.wait(10);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.states[li][o0], 4, "obj0 COMPLETE [M 0x1827d8:78-85]");
        assert!(sim.drones.channels.on(0x52), "the disc's AND(0x4e,0x51) writes the key gate [D]");
        assert_eq!(sim.drones.mission.states[li][o2], 3, "obj2 announced by its enable channel [M 0x1827d8:55-67]");
        assert!(!sim.drones.channels.on(0x60) && !sim.drones.channels.on(0x61) && !sim.drones.channels.on(0x63),
            "no fail hook fired on the route [M 0x1827d8:98-117]");
    }

    // 4. The exit: E on the Load Point loads 07000004 through the real reload path (the exit
    //    lawn first — the direct line from servant C rides the slope without settling).
    let sim = w.scene.sim.as_ref().unwrap();
    let exit = sim.drones.triggers.iter().find(|t| t.kind == walker::trigger_kind::LOAD_LEVEL).map(|t| t.centre()).expect("the Load Point");
    drop(sim);
    w.walk_nav_to([18.5, 5.9, 13.0], "the exit lawn", 1.6);
    w.walk_to(Wp { at: exit, act: Act::Use { use_ticks: 90 }, arrive: 1.6 }, "E the exit");
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0004), "the exit loads 07000004 [D param 0]");
    eprintln!("[03] full walk: {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[03] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[03] stuck: {s}"); }
}

/// Sister of the full walk's key gate: one objective's channel alone never opens the gate —
/// the AND needs both inputs [D the kind-3 out-0x52 list 4e,51].
#[test]
fn the_key_gate_stays_shut_without_both_objectives() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000003.bin", false).expect("load 07000003");
    let sim = scene.sim.as_mut().unwrap();
    // obj0's channel only: the AND(0x4e,0x51) holds 0x52 off [D].
    sim.drones.channels.set(0x4e, 1, sim.drones.frame);
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    assert!(!sim.drones.channels.on(0x52), "0x4e alone never opens the gate [D AND list 4e,51]");
    assert!(!sim.drones.channels.on(0x51), "0x51's own AND(0x4f,0x50) is still missing an input [D]");
    // obj1's sub-channels alone: same.
    let mut scene = nf_view::load(&iso, "07000003.bin", false).expect("reload");
    let sim = scene.sim.as_mut().unwrap();
    sim.drones.channels.set(0x4f, 1, sim.drones.frame);
    sim.drones.channels.set(0x50, 1, sim.drones.frame);
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x51), "AND(0x4f,0x50) completes obj1's channel [D]");
    assert!(!sim.drones.channels.on(0x52), "0x51 alone never opens the gate [D AND list 4e,51]");
}

/// The servants are protected: killing one raises ch 0x60 and the standing fail hook fails the
/// mission with "Killed Civilians" [M Mission_MonitorObjectives 0x1827d8:98-104; the death channel
/// is the placement's own [D the dump]].
#[test]
fn killing_a_servant_fails_the_mission() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000003.bin", false).expect("load 07000003"));
    w.wait(30);
    let sim = w.scene.sim.as_ref().unwrap();
    let si = sim.drones.list.iter().position(|d| d.skin & 0xffff == 0x2b && d.team == 3).expect("a servant");
    let at = sim.drones.list[si].pos;
    drop(sim);
    w.walk_nav_to([at[0], at[1] - 0.3, at[2]], "to the servant", 2.0);
    w.shoot(si, "the servant");
    for _ in 0..120 { w.tick(Default::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.list[si].corpse, "the servant died to real fire");
    assert!(sim.drones.channels.on(0x60), "the placement's death channel 0x60 rose [D]");
    // The mission layer's own records (the events vec is drained per step by the view path — the
    // persistent globals are the evidence): the standing hook's label, the fail state, ch 99.
    assert_eq!(sim.drones.mission.fail_label, 0x0400_0034, "the Killed Civilians fail label [M 0x1827d8:98-104]");
    assert_eq!(sim.drones.mission.state, 2, "MissionState 2 (the fail arm) [M 0x182c60:45-57]");
    assert!(sim.drones.channels.on(99), "the mission-fail channel rose [M mission.rs ch.set(99)]");
}
