//! fix-npc-doors-escort: the NPCs open the doors on their own paths (07000001 "Paris Prelude").
//!
//! The user's playtest (2026-10-12): "npcs cant open doors" on every level; "mahew doesnt really
//! follow properly" — the screenshot (`07000001 --bond 6.28,6.47,6.20,6.338,-0.083`) shows Mayhew
//! stuck facing a shoji door. The game's path: `NDrone2_ReachedDestNode` (0x153358) stands the
//! route's door node binding (`NDrone_InitDoorNodes` 0x139a70) in `Drone+0xBA8` and enters
//! `DSTATE_OpenDoor` 0x1681f8 (state 0x81 [L 0x29b300 + 0x81·4]), which calls `NDrone2_OpenDoor`
//! 0x14aa38 → `Door_Activate(door, drone, 0)` and re-tries on timer C every 2 ticks until
//! `NDrone2_DoorIsOpen` 0x14aaa0 sees state 4.
//!
//! The test drives the REAL input path only (the walker: Bond's `Input`), follows the contact
//! (skin 0x4F, the escort) from his enable, and asserts he walks THROUGH the interior doors —
//! no settle next to a closed shoji — and that the doors he passes end OPEN (the sister).

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// A short dump of the escort + nearby doors (a failure reads like the user's bug report).
fn doors_dump(w: &Walker, ci: usize) -> String {
    let sim = w.scene.sim.as_ref().unwrap();
    let c = sim.drones.list[ci].pos;
    let mut s = format!("contact at ({:.2},{:.2},{:.2})\n", c[0], c[1], c[2]);
    for (i, d) in sim.drones.doors.list.iter().enumerate() {
        if walker::dxz(d.pos, c) < 6.0 {
            s.push_str(&format!("  door #{i} ({:.1},{:.1},{:.1}) state {:#x} pos01 {:.2}\n", d.pos[0], d.pos[1], d.pos[2], d.state, d.pos01));
        }
    }
    s
}

#[test]
fn the_escort_opens_the_interior_doors_on_his_route() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    // The intro cut-scene writes ch 0xB (the contact's enable channel) ~tick 2119; the script tail
    // re-places Bond (the walkreal-a test's setup).
    let mut enabled_at = None;
    for t in 0..4000u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xb) { enabled_at = Some(t); break; }
    }
    let enabled_at = enabled_at.expect("the intro must write ch 0xB (the contact's enable channel)");
    eprintln!("[01] intro ch 0xB at tick {enabled_at}");
    w.wait_enabled(6000, "the intro tail (event 6 re-places Bond)");
    w.wait(300);

    let sim = w.scene.sim.as_ref().unwrap();
    let ci = sim.drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("the contact");
    let cpos0 = sim.drones.list[ci].pos;
    // The interior doors the route crosses (the user's screenshot has Mayhew facing one of them).
    let doors0: Vec<(usize, [f32; 3])> = sim.drones.doors.list.iter().enumerate()
        .map(|(i, d)| (i, d.pos)).collect();
    drop(sim);
    eprintln!("[01] the contact starts at ({:.2},{:.2},{:.2})", cpos0[0], cpos0[1], cpos0[2]);
    for (i, p) in &doors0 { eprintln!("[01] door #{i} at ({:.2},{:.2},{:.2})", p[0], p[1], p[2]); }

    // The contact hides while an armed started guard stands within 14 [M AllyLeadHide 0x15ec08] —
    // clear the intro's guards through the real fire path (the walkreal-a test's clearing).
    let guards: Vec<usize> = {
        let sim = w.scene.sim.as_ref().unwrap();
        (0..sim.drones.list.len()).filter(|&i| {
            let d = &sim.drones.list[i];
            i != ci && d.team == 1 && d.started && !d.corpse && d.enabled && walker::dxz(d.pos, sim.drones.list[ci].pos) < 14.0
        }).collect()
    };
    eprintln!("[01] {} armed guard(s) within 14 of the contact — clearing", guards.len());
    for g in guards {
        let gpos = w.scene.sim.as_ref().unwrap().drones.list[g].pos;
        let bpos = w.scene.sim.as_ref().unwrap().bond.pos;
        if walker::dxz(gpos, bpos) > 9.0 {
            w.walk_nav_to(gpos, &format!("approach guard {g}"), 3.5);
        }
        if !w.shoot(g, &format!("guard {g}")) {
            if w.resupply("ammo retry") { w.shoot(g, &format!("guard {g}")); }
        }
    }
    let cpos = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
    w.walk_to(Wp { at: cpos, act: Act::None, arrive: 2.5 }, "to the contact");

    // Follow the escort, sampling his progress and the doors near him every 60 ticks.
    // The route's end for this test: the LoadPointActivator plate (Touch out 0x10) at
    // (−33.1, 2.9, 15.4) [D; the walkreal-a route] — the escort leads to it.
    let plate = {
        let sim = w.scene.sim.as_ref().unwrap();
        walker::trigger_centre(sim, walker::trigger_kind::TOUCH, 0x10).expect("the AND-touch plate")
    };
    let start_ticks = w.total_ticks;
    let mut open_events: Vec<String> = Vec::new();
    let mut was_open = vec![false; w.scene.sim.as_ref().unwrap().drones.doors.list.len()];
    let _settled = loop {
        // The walker's follow loop, inlined for the door sampling: steer at the escort from the
        // 4.5..10 band (PlayerInWay parks him when Bond is within 3 [M 0x15ea28 case 3]).
        {
            let sim = w.scene.sim.as_ref().unwrap();
            let target = sim.drones.list[ci].pos;
            for (i, d) in sim.drones.doors.list.iter().enumerate() {
                if d.state == nf_game::door::OPEN && !was_open[i] {
                    was_open[i] = true;
                    let note = format!("door #{i} at ({:.1},{:.1},{:.1})", d.pos[0], d.pos[1], d.pos[2]);
                    eprintln!("[01] +{} ticks: OPEN {note}", w.total_ticks - start_ticks);
                    open_events.push(note);
                }
            }
            if walker::dxz(target, plate) < 5.0 {
                eprintln!("[01] the escort reached the plate at ({:.1},{:.1},{:.1}) after +{} ticks",
                    target[0], target[1], target[2], w.total_ticks - start_ticks);
                break target;
            }
            let desired = (target[0] - sim.bond.pos[0]).atan2(target[2] - sim.bond.pos[2]);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let d = walker::dxz(sim.bond.pos, target);
            let input = nf_game::player::Input {
                turn: err.clamp(-0.07, 0.07),
                forward: if d > 5.0 { 1.0 } else if d > 4.0 { 0.45 } else { 0.0 },
                ..Default::default()
            };
            drop(sim);
            w.tick(input, false);
        }
        assert!(w.total_ticks - start_ticks < 14_000, "the escort follow cap\n{}", doors_dump(&w, ci));
        if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { break cpos; }
    };

    // ---- the assertions --------------------------------------------------------------------
    // 1. The escort left his post-intro position by a real margin (the route's first leg) and the
    //    follow ended near the plate (the escort leads the whole interior in the game).
    let cpos1 = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
    eprintln!("[01] the escort settled at ({:.2},{:.2},{:.2}); plate ({:.1},{:.1},{:.1})",
        cpos1[0], cpos1[1], cpos1[2], plate[0], plate[1], plate[2]);
    assert!(walker::dxz(cpos1, plate) < 8.0,
        "the escort never led to the plate (settled {:.1} away at ({:.1},{:.1},{:.1}))",
        walker::dxz(cpos1, plate), cpos1[0], cpos1[1], cpos1[2]);
    // 2. SISTER: at least one interior door opened under him — the ones on his route start CLOSED
    //    [L ee_start_07000001 state 2] and only Bond's use key or the escort's own door handling
    //    opens them; Bond never pressed E on this walk.
    assert!(!open_events.is_empty(),
        "no door opened during the escort's lead — the NPCs' door handling did not run");
    // 3. And he is not parked against a still-closed door (the user's screenshot symptom):
    //    every door within 2.5 of his settled position is OPEN (or a lift/kind-inert door).
    let blocking: Vec<String> = w.scene.sim.as_ref().unwrap().drones.doors.list.iter().enumerate()
        .filter(|(_, d)| walker::dxz(d.pos, cpos1) < 2.5 && d.state & 0x12 != 0)
        .map(|(i, d)| format!("door #{i} state {:#x} at ({:.1},{:.1},{:.1})", d.state, d.pos[0], d.pos[1], d.pos[2]))
        .collect();
    assert!(blocking.is_empty(), "the escort settled against closed door(s): {}", blocking.join("; "));
}
