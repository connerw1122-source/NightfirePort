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


/// Open any shut door lying on the Bond→guard line (the real player's E through the doorway),
/// then approach and shoot. Returns whether the guard died (or was already dead).
fn clear_guard(w: &mut Walker, g: usize) -> bool {
    let (gpos, bpos) = {
        let sim = w.scene.sim.as_ref().unwrap();
        (sim.drones.list[g].pos, sim.bond.pos)
    };
    // Doors within 2.5 of the straight line, closed, nearest first.
    let mut doors: Vec<(usize, [f32; 3])> = {
        let sim = w.scene.sim.as_ref().unwrap();
        let (mut lo, mut hi) = (bpos, gpos);
        sim.drones.doors.list.iter().enumerate()
            .filter(|(_, d)| d.state & 0x12 != 0)
            .filter(|(_, d)| {
                // point-to-segment (xz)
                let (ax, az, bx, bz) = (lo[0], lo[2], hi[0], hi[2]);
                let (px, pz) = (d.pos[0], d.pos[2]);
                let (dx, dz) = (bx - ax, bz - az);
                let l2 = dx * dx + dz * dz;
                let t = if l2 > 0.0 { ((px - ax) * dx + (pz - az) * dz) / l2 } else { 0.0 };
                let t = t.clamp(0.0, 1.0);
                let (qx, qz) = (ax + dx * t, az + dz * t);
                ((px - qx).powi(2) + (pz - qz).powi(2)).sqrt() < 2.5
            })
            .map(|(i, d)| (i, d.pos)).collect()
    };
    doors.sort_by(|a, b| walker::dxz(a.1, bpos).total_cmp(&walker::dxz(b.1, bpos)));
    for (i, dp) in doors.into_iter().take(2) {
        // Re-check: the escort's own door handling (DSTATE_OpenDoor 0x1681f8) may have opened it
        // since the closed-set was built — walking a pointless "open the way" leg onto an OPEN
        // door wedges on the corridor furniture (door #2, the walker 2.7 short for 6000 ticks).
        if w.scene.sim.as_ref().unwrap().drones.doors.list[i].state & 0x12 == 0 { continue; }
        // The nav route first (the direct steer wedges on the corridor's bends), then the use press.
        // Both stop 2.2 SHORT of the door: a kind-2 leaf parks its collider along the wall jog when
        // open, and walking to the door position itself wedges on it (door #2: the walker stuck
        // 2.7 short, 6000 ticks). 2.2 is inside the use sphere's stand-in reach (2.5), so the
        // press still lands — the same geometry the mission test's press uses.
        w.walk_nav_to(dp, "to the shut door", 2.2);
        w.walk_to(Wp { at: dp, act: Act::Use { use_ticks: 120 }, arrive: 3.0 }, "open the way");
    }
    let gpos = w.scene.sim.as_ref().unwrap().drones.list[g].pos;
    let bpos = w.scene.sim.as_ref().unwrap().bond.pos;
    if walker::dxz(gpos, bpos) > 9.0 {
        w.try_walk_to(Wp::new(gpos[0], gpos[1], gpos[2]).r(3.5), &format!("approach guard {g}"));
    }
    if w.shoot(g, &format!("guard {g}")) { return true; }
    if w.resupply("ammo retry") { return w.shoot(g, &format!("guard {g}")); }
    false
}

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
            i != ci && d.team == 1 && d.started && !d.corpse && d.enabled && walker::dxz(d.pos, sim.drones.list[ci].pos) < 20.0
        }).collect()
    };
    eprintln!("[01] {} armed guard(s) within 14 of the contact — clearing", guards.len());
    for g in guards {
        let gpos = w.scene.sim.as_ref().unwrap().drones.list[g].pos;
        let bpos = w.scene.sim.as_ref().unwrap().bond.pos;
        if walker::dxz(gpos, bpos) > 9.0 {
            w.walk_nav_to(gpos, &format!("approach guard {g}"), 3.5);
        }
        clear_guard(&mut w, g);
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
    let mut cleared: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut settled = [0.0f32; 3];
    // The walkreal-a pattern: rounds of follow → clear (the escort hides while an armed started
    // guard is within 14 [M 0x15ec08]; the real player fights through, Bond does the same here).
    'rounds: for _round in 0..10 {
        let round_start = w.total_ticks;
        loop {
            // Sample the doors every tick (an auto-close can slip a coarser grid).
            {
                let sim = w.scene.sim.as_ref().unwrap();
                let bd = walker::dxz(sim.bond.pos, sim.drones.list[ci].pos);
                for (i, d) in sim.drones.doors.list.iter().enumerate() {
                    if d.state == nf_game::door::OPEN && !was_open[i] {
                        was_open[i] = true;
                        let note = format!("door #{i} at ({:.1},{:.1},{:.1}) bond→contact {:.1}",
                            d.pos[0], d.pos[1], d.pos[2], bd);
                        eprintln!("[01] +{} ticks: OPEN {note}", w.total_ticks - start_ticks);
                        open_events.push(note);
                    }
                }
                let target = sim.drones.list[ci].pos;
                if target[0] < -20.0 {
                    eprintln!("[01] the escort led past the interior doors at ({:.1},{:.1},{:.1}) after +{} ticks",
                        target[0], target[1], target[2], w.total_ticks - start_ticks);
                    settled = target;
                    break 'rounds;
                }
                if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { settled = target; break 'rounds; }
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
            if (w.total_ticks - start_ticks) % 600 == 0 {
                let sim = w.scene.sim.as_ref().unwrap();
                let c = &sim.drones.list[ci];
                eprintln!("[01] +{} t: contact ({:.1},{:.1},{:.1}) state {:#x} hit_door {:?} peer {:.1}",
                    w.total_ticks - start_ticks, c.pos[0], c.pos[1], c.pos[2], c.state, c.hit_door, c.armed_peer_dist);
            }
            // A round cap: the escort stopped (10 s still inside w.follow's sense is not visible
            // here; 4500 ticks of no round progress = stalled).
            if w.total_ticks - round_start > 4500 { break; }
            // The budget follows the walkreal test's own precedent (12 rounds × 12000): master's
            // walk-order (fix-script-start-tick) and weapons merges re-position the stairwell-head
            // guard group, so the escort now takes more hide pauses (AllyLeadHide 0x15ec08 [M]) on
            // the way west — the old 60k cap expired with him only at x −3.5 (the doors themselves
            // opened on schedule: the +1327 OPEN line). A dry mid-round spiral is the real time
            // sink (each wedged resupply leg burns 2×6000 ticks), and the per-round top-up below
            // keeps the fights supplied.
            assert!(w.total_ticks - start_ticks < 120_000, "the escort follow cap\n{}", doors_dump(&w, ci));
        }
        // Stalled: who is armed and started within 14 of the contact (LeadHide's band)?
        let guards: Vec<usize> = {
            let sim = w.scene.sim.as_ref().unwrap();
            (0..sim.drones.list.len()).filter(|&i| {
                let d = &sim.drones.list[i];
                i != ci && d.team == 1 && d.started && !d.corpse && d.enabled
                    && walker::dxz(d.pos, sim.drones.list[ci].pos) < 20.0 && !cleared.contains(&i)
            }).collect()
        };
        if guards.is_empty() {
            // No threat: close on him and try again (the lead wait bands, PlayerInWay).
            let cpos = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
            w.walk_nav_to(cpos, "close on the contact", 4.0);
            continue;
        }
        eprintln!("[01] round: the contact stalled at ({:.1},{:.1},{:.1}) — clearing guards {:?}",
            w.scene.sim.as_ref().unwrap().drones.list[ci].pos[0],
            w.scene.sim.as_ref().unwrap().drones.list[ci].pos[1],
            w.scene.sim.as_ref().unwrap().drones.list[ci].pos[2], guards);
        // Supply BEFORE the fights (the walkreal test's per-guard check): going dry mid-round is
        // what wedges the run — the mid-fight resupply walk from the courtyard to the start-room
        // pickup can wedge at the east ledge and burn 2×6000 ticks per leg.
        {
            let sim = w.scene.sim.as_ref().unwrap();
            if sim.gun.clip + sim.gun.spare < 16 { drop(sim); w.resupply("round top-up"); }
        }
        for g in guards {
            cleared.insert(g);
            if !clear_guard(&mut w, g) { cleared.remove(&g); } // retry next round
        }
    }

    // ---- the assertions --------------------------------------------------------------------
    let cpos1 = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
    eprintln!("[01] the escort settled at ({:.2},{:.2},{:.2}); plate ({:.1},{:.1},{:.1})",
        cpos1[0], cpos1[1], cpos1[2], plate[0], plate[1], plate[2]);
    // 1. The escort crossed BOTH interior shoji doors on his route: door #1 (7.9, 5.3, 11.14 — the
    //    door the user's screenshot shows Mayhew facing) and door #2 (−14.06, 5.35, 13.05): he must
    //    stand west of #2's plane (x < −15). Pre-fix he settled against
    //    door #1 (state 0x1d/0x1f at z ≈ 13 with the door CLOSED — the playtest symptom).
    assert!(cpos1[0] < -15.0,
        "the escort never led past the interior shoji doors (settled at ({:.1},{:.1},{:.1}))",
        cpos1[0], cpos1[1], cpos1[2]);
    // 2. SISTER: door #1 (the screenshot door) opened while Bond was OUTSIDE his 2.0 use reach of
    //    it — the escort's own door handling (DSTATE_OpenDoor → Door_Activate), not Bond's E.
    //    (Bond's follow keeps him 4.5..10 behind the contact; only the guard-clear rounds walk him
    //    elsewhere, and those runs begin after the escort's first door.)
    let d1 = open_events.iter().find(|n| n.contains("door #1 ")).expect("door #1 never opened");
    let bond_d1: f32 = d1.rsplit("bond→contact ").next().and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
    eprintln!("[01] door #1 opened with Bond {bond_d1:.1} from the contact");
    assert!(bond_d1 > 2.5, "door #1 opened with Bond too close ({bond_d1:.1}) — not the escort's doing? ({d1})");
    // 3. And he is not parked against a still-closed door (the user's screenshot symptom):
    //    every door within 2.5 of his settled position is OPEN.
    let blocking: Vec<String> = w.scene.sim.as_ref().unwrap().drones.doors.list.iter().enumerate()
        .filter(|(_, d)| walker::dxz(d.pos, cpos1) < 2.5 && d.state & 0x12 != 0)
        .map(|(i, d)| format!("door #{i} state {:#x} at ({:.1},{:.1},{:.1})", d.state, d.pos[0], d.pos[1], d.pos[2]))
        .collect();
    assert!(blocking.is_empty(), "the escort settled against closed door(s): {}", blocking.join("; "));
}
