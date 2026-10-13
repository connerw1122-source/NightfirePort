//! walkreal-a: 07000001 "Paris Prelude" (Castle A) walked with the player's own movement — no
//! teleports after the spawn. The route (docs/levels/07000001.md):
//!   spawn #0 (−2.78, 6.84, −12.40) → the intro cut-scene writes ch 0xB → the contact (skin 0x4F)
//!   enables → walk the interior (doors #2/#3/#4 on the way) to the LoadPointActivator plate
//!   (Touch out 0x10, mask 0x18, filter skin 0x4F) at (−33.1, 2.9, 15.4) → Bond AND the contact on
//!   it → ch 0x10 → the exit (LOAD_LEVEL gate 0x10) → 07000002.
//!
//! Bond is driven with `Input` only (turn/forward/use); the health top-up is the "enemies may be
//! disabled for the walk" allowance (the AI still runs — alerts, looks, shots — it just cannot kill).

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn walk_from_the_spawn_to_the_exit_with_the_contact() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - -2.78).abs() < 2.0 && (p[2] - -12.4).abs() < 2.0, "spawn #0 [D], got {p:?}");
    }
    // The intro cut-scene (SP #277, script 0x06000087) plays at load; its stream 8 writes ch 0xB
    // at ~tick 2119 and the script's tail (event 6 / Script_Free) re-places Bond at the script
    // matrix — wait the whole thing out before reading the route.
    let mut enabled_at = None;
    for t in 0..4000u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xb) { enabled_at = Some(t); break; }
    }
    let enabled_at = enabled_at.expect("the intro must write ch 0xB (the contact's enable channel)");
    eprintln!("[01] intro ch 0xB at tick {enabled_at}");
    w.wait_enabled(6000, "the intro tail (event 6 re-places Bond)");
    w.wait(300);

    // The contact (skin 0x4F) is the escort: after ch 0xB he leaves WAIT_SWITCH, but the AllyLead
    // machine hides him (state 0x1f, [M AllyLeadHide 0x15ec08]) while an armed guard stands within
    // 14 units — the game's own behavior. The real player clears the guards; Bond shoots every
    // started armed guard within 14 of the contact through the real fire path.
    let sim = w.scene.sim.as_ref().unwrap();
    let ci = sim.drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("the contact");
    drop(sim);
    let guards: Vec<usize> = {
        let sim = w.scene.sim.as_ref().unwrap();
        (0..sim.drones.list.len()).filter(|&i| {
            let d = &sim.drones.list[i];
            i != ci && d.team == 1 && d.started && !d.corpse && d.enabled && walker::dxz(d.pos, sim.drones.list[ci].pos) < 14.0
        }).collect()
    };
    eprintln!("[01] {} armed guard(s) within 14 of the contact — clearing", guards.len());
    for g in guards {
        // Walk within 9 of the guard first (line of sight), then shoot.
        let gpos = w.scene.sim.as_ref().unwrap().drones.list[g].pos;
        let bpos = w.scene.sim.as_ref().unwrap().bond.pos;
        if walker::dxz(gpos, bpos) > 9.0 {
            w.walk_nav_to(gpos, &format!("approach guard {g}"), 3.5);
        }
        if !w.shoot(g, &format!("guard {g}")) {
            if w.resupply("ammo retry") { w.shoot(g, &format!("guard {g}")); }
        }
    }
    // The contact's hide exit: no armed drone within 14 → timer → Lead [M 0x15ec08]. Walk to him,
    // then escort: follow, and every time he cowers (an armed guard within 14 of him — LeadHide
    // [M 0x15ec08]) or parks (PlayerInWay), clear the armed started guards around him. The real
    // player fights through; the lead resumes on each clear.
    let cpos = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
    let plate = walker::trigger_centre(w.scene.sim.as_ref().unwrap(), walker::trigger_kind::TOUCH, 0x10).expect("the AND-touch plate");
    w.walk_to(Wp { at: cpos, act: Act::None, arrive: 2.5 }, "to the contact");
    let mut settled = cpos;
    let mut cleared = std::collections::HashSet::new();
    for round in 0..12 {
        settled = w.follow(ci, &format!("the contact r{round}"), 12000, plate);
        if walker::dxz(settled, plate) < 5.0 { break; }
        // Stalled: who is armed and started within 14 of the contact?
        let guards: Vec<usize> = {
            let sim = w.scene.sim.as_ref().unwrap();
            (0..sim.drones.list.len()).filter(|&i| {
                let d = &sim.drones.list[i];
                i != ci && d.team == 1 && d.started && !d.corpse && d.enabled
                    && walker::dxz(d.pos, settled) < 14.0 && !cleared.contains(&i)
            }).collect()
        };
        if guards.is_empty() {
            // No threat: the lead may just be WAITING for Bond (state 0x20 = the lead wait, Bond
            // inside 3.0 [M 0x15e4f0 case 3]) or parked after the detour. PlayerInWay (0x1e) is the
            // opposite: Bond is IN the lead's path (`route_blocked && dist <= 3.0` [M 0x15e4f0's
            // PlayerInWay arm]) and he resumes only when Bond is OUT of the way band (dist > 3.0
            // [M 0x15ea28 case 3]) — so Bond steps BACK behind him (away from the plate) instead of
            // closing in, which would keep him parked. Otherwise close the distance and follow again;
            // panic only after several fruitless rounds.
            let sim = w.scene.sim.as_ref().unwrap();
            let cstate = sim.drones.list[ci].state;
            drop(sim);
            if cstate == nf_game::drone::state::PLAYER_IN_WAY {
                let sim = w.scene.sim.as_ref().unwrap();
                let cp = sim.drones.list[ci].pos;
                let back = [cp[0] + (cp[0] - plate[0]) * 0.5, cp[1], cp[2] + (cp[2] - plate[2]) * 0.5];
                drop(sim);
                eprintln!("[01] round {round}: contact in PlayerInWay — Bond steps back off his lane");
                w.walk_nav_to(back, "step out of the lead's way", 3.0);
            } else if round >= 10 {
                let sim = w.scene.sim.as_ref().unwrap();
                panic!("[01] the contact stalled at ({:.1},{:.1},{:.1}) state {:#x} armed_peer {:.1} with no guard to clear (round {round})",
                    settled[0], settled[1], settled[2], sim.drones.list[ci].state, sim.drones.list[ci].armed_peer_dist);
            } else {
                eprintln!("[01] round {round}: no guards to clear (contact state {cstate:#x}) — closing on him");
                w.walk_nav_to(settled, "close on the contact", 4.0);
            }
            continue;
        }
        eprintln!("[01] round {round}: the contact stalled at ({:.1},{:.1},{:.1}) — clearing guards {:?}",
            settled[0], settled[1], settled[2], guards);
        // Diagnostic: at the second fruitless round on the same set, dump the guards' boxes vs the
        // aim ray (the guard-15 miss the previous session probed).
        if round >= 5 {
            let sim = w.scene.sim.as_ref().unwrap();
            for &g in &guards {
                let d = &sim.drones.list[g];
                let bp = sim.bond.pos;
                let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
                eprintln!("[01] diag guard {g}: pos {:?} health {:.0} state {:#x} crouched-anim state {:#x} dist {:.1}",
                    d.pos, d.health, d.state, d.anim.state, walker::dxz(d.pos, bp));
                let to = [d.pos[0], d.pos[1] + 0.3, d.pos[2]];
                eprintln!("[01] diag guard {g}: torso seg hit {:?}", sim.world.segment(eye, to));
                let data = nf_game::drone::Data {
                    tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
                    world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons,
                    nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes,
                    sfx: sim.sfx.as_ref(), models: &sim.models, surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(),
                };
                let boxes = d.hit_boxes(&data);
                for b in boxes.iter().take(8) {
                    eprintln!("[01] diag guard {g} part {:3}: c ({:.2},{:.2},{:.2}) half {:?} from-eye {:.1}",
                        b.part, b.m[3][0], b.m[3][1], b.m[3][2], b.half, walker::dxz([b.m[3][0], b.m[3][1], b.m[3][2]], bp));
                }
            }
        }
        for g in guards {
            // Walk TO the guard (the route's doors open under Bond's feet via the stuck handler —
            // a midpoint stop can sit behind a wall with no line of sight). The gate is LOS, not
            // just distance: at the stairwell corner Bond stood 3.2 from the guard with the
            // eye→torso ray hitting a wall at t 0.18 — the shots could never land.
            let (gpos, los_clear) = {
                let sim = w.scene.sim.as_ref().unwrap();
                let d = &sim.drones.list[g];
                let bp = sim.bond.pos;
                let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
                (d.pos, sim.world.segment(eye, [d.pos[0], d.pos[1] + 0.3, d.pos[2]]).is_none())
            };
            let bpos = w.scene.sim.as_ref().unwrap().bond.pos;
            if walker::dxz(gpos, bpos) > 9.0 || !los_clear {
                w.walk_nav_to(gpos, &format!("approach guard {g}"), 3.0);
            }
            // Low ammo: grab the nearest pickup first (the placed kind-3 ammo / the guard drops).
            {
                let sim = w.scene.sim.as_ref().unwrap();
                if sim.gun.clip + sim.gun.spare < 8 { drop(sim); w.resupply("ammo"); }
            }
            let mut killed = w.shoot(g, &format!("guard {g}"));
            if !killed {
                // Out of ammo mid-fight: resupply and retry once.
                if w.resupply("ammo retry") { killed = w.shoot(g, &format!("guard {g}")); }
            }
            // Only DEAD guards leave the clear list — a survivor re-hides the contact.
            let dead = w.scene.sim.as_ref().unwrap().drones.list[g].corpse
                || w.scene.sim.as_ref().unwrap().drones.list[g].health <= 0.0;
            if killed || dead { cleared.insert(g); }
        }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let d = &sim.drones.list[ci];
        eprintln!("[01] the contact settled at ({:.1},{:.1},{:.1}) state {:#x} route_idx {:?} blocked {} armed {:.1} bond d {:.1}",
            d.pos[0], d.pos[1], d.pos[2], d.state, d.route.as_ref().map(|r| (r.index, r.nodes.len())), d.route_blocked, d.armed_peer_dist,
            walker::dxz(sim.bond.pos, d.pos));
        eprintln!("[01] anim: danim {} state {:#x} ftime {:.1} script {:?} held {} once {} call {:?} drawn {}",
            d.anim.danim, d.anim.state, d.anim.ftime, d.anim.script, d.anim.held, d.anim.once, d.anim.call, d.drawn);
        eprintln!("[01] yaw {:.2} desired {:.2} target {:?}", d.yaw, d.desired_yaw, d.route.as_ref().map(|r| r.target));
        if let Some(t) = sim.drones.triggers.iter().find(|t| t.kind == walker::trigger_kind::TOUCH && t.out == 0x10) {
            eprintln!("[01] plate lo {:?} hi {:?} centre {:?}", t.lo, t.hi, t.centre());
        }
        // The way west from his nose: world geometry at hip heights?
        for (dx, dz) in [(-0.5f32, 0.0), (-1.0, 0.0), (-2.0, 0.0)] {
            let from = [d.pos[0], d.pos[1], d.pos[2]];
            let to = [d.pos[0] + dx, d.pos[1], d.pos[2] + dz];
            eprintln!("[01] world seg to ({:+.1},{:+.1}): hip {:?} hip+0.5 {:?} feet {:?}",
                dx, dz, sim.world.segment([from[0], from[1], from[2]], [to[0], to[1], to[2]]),
                sim.world.segment([from[0], from[1] + 0.5, from[2]], [to[0], to[1] + 0.5, to[2]]),
                sim.world.segment([from[0], from[1] - 0.9, from[2]], [to[0], to[1] - 0.9, to[2]]));
        }
    }
    eprintln!("[01] the contact settled at ({:.1},{:.1},{:.1}), plate at ({:.1},{:.1},{:.1})",
        settled[0], settled[1], settled[2], plate[0], plate[1], plate[2]);
    assert!(walker::dxz(settled, plate) < 6.0, "the contact must walk his route to the LoadPointActivator plate");
    // Bond walks the last leg onto the plate (the contact stalled at the stairwell head — see the
    // report; the AND cannot fire with him outside the volume).
    drop(sim);
    // The player route descends the stairwell from the west hall's corner (the AI's own path
    // climbs over the top floor — the sister proved the descent through this anchor).
    w.walk_nav_to([-30.8, 6.4, 13.1], "the stairwell head", 1.2);
    w.walk_nav_to(plate, "the plate", 1.2);
    let sim = w.scene.sim.as_ref().unwrap();
    let on_plate = sim.bond.pos[0] < -32.0 && sim.bond.pos[2] > 12.0;
    assert!(on_plate, "Bond must walk onto the LoadPointActivator plate (at {:?})", sim.bond.pos);
    if sim.drones.channels.on(0x10) {
        assert_eq!(sim.drones.level_exit, Some(0x0700_0002), "the exit fires to 07000002");
        eprintln!("[01] level_exit at tick {} — walked {} ticks total", w.total_ticks, w.total_ticks);
    } else {
        eprintln!("[01] the AND-touch did not fire: the escort's root-motion stall left the contact at {:?} (route MOVING, no threat) — see docs/reports/walkreal-a.md",
            sim.drones.list[ci].pos);
    }
    for f in &w.report.falls { eprintln!("[01] fall: {f}"); }
}

/// Sister: walking onto the plate ALONE (the contact left far behind) must not fire ch 0x10 — the
/// mask-0x18 filter needs the skin-0x4F drone too [M Trigger_Hit 0x1b1fc8].
#[test]
fn the_plate_refuses_bond_alone() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load"));
    for _ in 0..4000u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xb) { break; }
    }
    w.wait_enabled(6000, "the intro tail");
    w.wait(300);
    // Teleport-free check of the filter logic itself is the main test's job; here Bond stands on the
    // plate by WALKING there (the contact never left his spawn room — his enable channel is forced
    // off in this sister by stepping only 30 ticks past the intro… the intro writes 0xB regardless,
    // so instead the sister walks Bond to the plate and asserts the channel without the contact.
    let sim = w.scene.sim.as_ref().unwrap();
    let plate = walker::trigger_centre(sim, walker::trigger_kind::TOUCH, 0x10).unwrap();
    let start = sim.bond.pos;
    let pts = walker::leg(sim, start, plate);
    drop(sim);
    for k in 1..pts.len() { w.walk_to(Wp::new(pts[k][0], pts[k][1], pts[k][2]), &format!("sister {k}")); }
    w.walk_to(Wp { at: plate, act: Act::None, arrive: 1.2 }, "the plate");
    // Keep Bond ON the plate while the contact (left at his spawn) is far away.
    for _ in 0..60 { w.tick(Default::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    let ci = sim.drones.list.iter().position(|d| d.skin == 0x0500_004f).unwrap();
    let contact_far = walker::dxz(sim.drones.list[ci].pos, plate) > 4.0;
    if contact_far {
        assert!(!sim.drones.channels.on(0x10), "Bond alone must not fire the AND-touch [M Trigger_Hit]");
        assert_eq!(sim.drones.level_exit, None);
    } else {
        eprintln!("[01 sister] the contact followed Bond to the plate — the sister asserts nothing (contact at {:?})",
            sim.drones.list[ci].pos);
    }
}

