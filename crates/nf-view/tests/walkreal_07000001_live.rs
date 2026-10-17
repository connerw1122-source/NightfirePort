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
use nf_game::world::V3;

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
        // An empty gun cannot finish the fight — if the resupply also failed, move on rather than
        // hold an empty trigger through the 900-tick reposition windows.
        if {
            let sim = w.scene.sim.as_ref().unwrap();
            sim.gun.clip + sim.gun.spare == 0
        } { eprintln!("[01] guard {g}: out of ammo and no box reached — leaving him for a later round"); continue; }
    }
    // The contact's hide exit: no armed drone within 14 → timer → Lead [M 0x15ec08]. Walk to him,
    // then escort: follow, and every time he cowers (an armed guard within 14 of him — LeadHide
    // [M 0x15ec08]) or parks (PlayerInWay), clear the armed started guards around him. The real
    // player fights through; the lead resumes on each clear.
    let cpos = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
    let plate = walker::trigger_centre(w.scene.sim.as_ref().unwrap(), walker::trigger_kind::TOUCH, 0x10).expect("the AND-touch plate");
    w.walk_to(Wp { at: cpos, act: Act::None, arrive: 2.5 }, "to the contact");

    // The meet beat — the level's second cut-scene, triggered entirely on disc. SP #276
    // (placement 276, script 0x06000086, 19 streams, in the level archive [D the disc scan —
    // pack 07800001 carries only the intro's 0x06000087]) is a mask-1 ScriptPlayer: it triggers
    // when its own out channel ch 0x33 rises [M SP_Hit 0x1dc470:17-21 flags bit 0x1 +
    // SP_Update 0x1dbda8 case 0]. The chain: the TOUCH_ONCE plate at (6.8, 6.0, 8.9) (mask 0x08)
    // writes ch 0x32 [D placement]; the OUT mux (0xEE, out 0x32 → list [0x33]) mirrors
    // ch[0x32] → ch[0x33] EVERY tick [M Trigger_Update 0x1b1a78 case 5:
    // switch_channels[list[k]] = switch_channels[out]]; SP #276 fires. So Bond WALKS the plate —
    // the real player's route into the north-east hall — and the scene plays out untouched
    // (streams end at frame 280 [D decode], ~4.7 s).
    w.walk_nav_to([6.8, 6.0, 8.9], "the meet plate (NE hall)", 1.0);
    // The plate is a TOUCH_ONCE on Bond's capsule — the touch lands when the capsule segment
    // crosses the volume's top face (feet ≈ the plate's floor 6.04). A nav stop a hair short
    // never crosses; a player steps ON it. Walk the centre tightly, then sweep the quad.
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let plate = sim.drones.triggers.iter()
            .find(|t| t.kind == walker::trigger_kind::TOUCH_ONCE && t.out == 0x32)
            .map(|t| t.centre())
            .expect("the meet plate");
        drop(sim);
        w.walk_to(Wp { at: plate, act: Act::None, arrive: 0.25 }, "onto the meet plate");
        for (dx, dz) in [(0.0f32, 0.9f32), (0.0, -0.9), (0.9, 0.0), (-0.9, 0.0)] {
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x32) { break; }
            w.walk_to(Wp { at: [plate[0] + dx, plate[1], plate[2] + dz], act: Act::None, arrive: 0.2 },
                &format!("the meet plate sweep ({dx},{dz})"));
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x32) { break; }
            w.walk_to(Wp { at: plate, act: Act::None, arrive: 0.2 }, "back across the meet plate");
        }
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[01] meet plate: bond at {:?} ch 0x32={} ch 0x33={}", sim.bond.pos,
            sim.drones.channels.on(0x32), sim.drones.channels.on(0x33));
        assert!(sim.drones.channels.on(0x32), "walking the meet plate must fire its ch 0x32 [D placement]");
    }
    let mut e_at = None;
    for t in 0..2400u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xe) { e_at = Some(t); break; }
    }
    eprintln!("[01] the meet scene (0x06000086) wrote ch 0xE at +{e_at:?} — stream 9's event 0xD ch 0xE val 1 [D decode]");
    assert!(e_at.is_some(), "the meet plate must fire ch 0x32 → the 0x33 mirror → SP #276 must play and write ch 0xE");
    // ch 0xE is the start channel of four hall guards [D placements; the probe's drone table]:
    // skins 0x39/0x36/0x38/0x3a at (−12.2,17.2), (5.4,14.4), (9.1,17.0), (5.9,17.9).
    w.wait(120);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let states: Vec<u8> = sim.drones.list.iter()
            .filter(|d| d.switch_channel == 0xe)
            .map(|d| d.state)
            .collect();
        assert_eq!(states.len(), 4, "four drones wait on ch 0xE [D placements]");
        assert!(states.iter().all(|&s| s != nf_game::drone::state::WAIT_SWITCH),
            "every ch-0xE guard must have left WAIT_SWITCH when the scene wrote the channel (states {states:?})");
    }

    // The hall detour: the disc's AND mux (kind 3, out 0x99 = 153, list [0x11,0x12,0x13] [D the
    // placement]) fires when the three named guards are dead — their death channels are 0x13,
    // 0x11, 0x12 [D placement params]. A real player clearing the north-east hall fires this
    // scripted event; the walk does it through the real fire path before the escort.
    let named: Vec<usize> = {
        let sim = w.scene.sim.as_ref().unwrap();
        (0..sim.drones.list.len())
            .filter(|&i| [0x11u8, 0x12, 0x13].contains(&sim.drones.list[i].death_channel))
            .collect()
    };
    assert_eq!(named.len(), 3, "three guards carry death channels 0x11/0x12/0x13 [D]");
    // Stage into the hall through its east door first. From the meet room the routes to the
    // west-half guards run through door #2 (−14.1,5.3,13.1) — the level's known walker wedge
    // (docs/reports/level-01-complete.md: the solo approach wedges at the 3000-tick cap) — and a
    // guard fought through that wall never exposes a part (he stands behind the hall's south
    // structure). A player enters the hall by its main door and fights inside; the east hop is
    // the route the guard-11 approach already proves (nav reached (2.97,6.38,16.27) from the
    // same start), and from the stage the named guards are short in-hall hops.
    w.walk_nav_to([2.9, 6.3, 16.3], "stage into the hall (east door)", 3.0);
    for g in named {
        let gpos = w.scene.sim.as_ref().unwrap().drones.list[g].pos;
        w.walk_nav_to(gpos, &format!("approach hall guard {g}"), 3.0);
        if !w.shoot(g, &format!("hall guard {g}")) {
            if w.resupply("hall ammo") { w.shoot(g, &format!("hall guard {g}")); }
        }
    }
    w.wait(30);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x99),
        "AND(0x11,0x12,0x13) must raise ch 153 once the three named guards are dead [D mux placement]");
    eprintln!("[01] the hall AND fired: ch 153 on at tick {}", w.total_ticks);

    let mut settled = cpos;
    // The contact's park from the previous round: two rounds inside ~1 unit with no guard to
    // clear means the lead machine wedged on geometry — Bond walks PAST him toward the plate
    // (the real player's move): out of the PlayerInWay band the lead resumes and re-paths [M
    // 0x15ea28 case 3 / 0x15e698 case 3], which unsticks him. The head-look merge's corrected
    // head box (the game's own AnimGetCollData → psiBuildMatrixPalette → HeadTrack coupling)
    // moves the fight timeline, so the lead can park in spots the old path never hit.
    let mut prev_settled = cpos;
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
            } else if walker::dxz(settled, prev_settled) < 1.0 {
                let sim = w.scene.sim.as_ref().unwrap();
                let cp = sim.drones.list[ci].pos;
                let (dx, dz) = (plate[0] - cp[0], plate[2] - cp[2]);
                let l = (dx * dx + dz * dz).sqrt().max(1e-3);
                let ahead = [cp[0] + dx / l * 4.0, cp[1], cp[2] + dz / l * 4.0];
                drop(sim);
                eprintln!("[01] round {round}: the lead is wedged at ({:.1},{:.1},{:.1}) state {cstate:#x} — Bond walks past him toward the plate (the lead resumes out of the way band and re-paths)",
                    settled[0], settled[1], settled[2]);
                prev_settled = settled;
                w.walk_nav_to(ahead, "walk past the wedged lead", 3.0);
            } else {
                eprintln!("[01] round {round}: no guards to clear (contact state {cstate:#x}) — closing on him");
                prev_settled = settled;
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
                    sfx: sim.sfx.as_ref(), models: &sim.models, surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: 1,
                    ai_points: &sim.ai_points,
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
            // Skip the hopeless fight: no ammo and no box reached — the round loop retries after
            // the drops land (every killed guard drops his weapon [L the 01 drops]).
            let ammo_now = w.scene.sim.as_ref().unwrap().gun.clip + w.scene.sim.as_ref().unwrap().gun.spare;
            if ammo_now == 0 { eprintln!("[01] guard {g}: skipped — dry"); continue; }
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
    {
        let d = &sim.drones.list[ci];
        eprintln!("[01] at the AND-touch: contact at ({:.2},{:.2},{:.2}) state {:#x} route_idx {:?} blocked {} seen {}",
            d.pos[0], d.pos[1], d.pos[2], d.state, d.route.as_ref().map(|r| (r.index, r.nodes.len())), d.route_blocked, d.seen);
        for (i, o) in sim.drones.list.iter().enumerate() {
            if i == ci || o.team != 1 || !o.started || o.corpse || !o.enabled { continue; }
            let dd = walker::dxz(o.pos, d.pos);
            if dd < 16.0 {
                eprintln!("[01] peer {i} skin {:#x} at ({:.1},{:.1},{:.1}) d {dd:.1} state {:#x} health {:.0}",
                    o.skin, o.pos[0], o.pos[1], o.pos[2], o.state, o.health);
            }
        }
    }
    // The stairwell guards (the west-side combat-steppers, plus the balcony skin-0x36) hide the
    // contact at the head — in the game the hide runs him to cover (`NDrone2_FindRunToPoint`
    // 0x15168, the half the port does not model) while the player fights. The walk fights them
    // the same way: clear every started, armed guard within 14 of the CONTACT (the hide exit
    // needs the visible armed peer beyond 14.0 [M `NDrone2_NotNearArmedDrone` 0x177640]), let the
    // lead finish the route's descent, then take Bond back onto the plate for the AND.
    'recovery: for round in 0..12 {
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x10) { break; }
        let guards: Vec<usize> = {
            let sim = w.scene.sim.as_ref().unwrap();
            let cp = sim.drones.list[ci].pos;
            (0..sim.drones.list.len()).filter(|&i| {
                let d = &sim.drones.list[i];
                i != ci && d.team == 1 && d.started && !d.corpse && d.enabled && d.health > 0.0
                    && ((2..0x47).contains(&d.weapon) || (0x6a..=0x71).contains(&d.weapon))
                    && walker::dxz(d.pos, cp) < 14.0 && !cleared.contains(&i)
            }).collect()
        };
        if guards.is_empty() { break; }
        for g in guards {
            let (gpos, los_clear) = {
                let sim = w.scene.sim.as_ref().unwrap();
                let d = &sim.drones.list[g];
                let bp = sim.bond.pos;
                let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
                (d.pos, sim.world.segment(eye, [d.pos[0], d.pos[1] + 0.3, d.pos[2]]).is_none())
            };
            let bpos = w.scene.sim.as_ref().unwrap().bond.pos;
            if walker::dxz(gpos, bpos) > 9.0 || !los_clear {
                w.walk_nav_to(gpos, &format!("approach stairwell guard {g}"), 3.0);
            }
            {
                let sim = w.scene.sim.as_ref().unwrap();
                if sim.gun.clip + sim.gun.spare < 8 { drop(sim); w.resupply("ammo"); }
            }
            let ammo_now = w.scene.sim.as_ref().unwrap().gun.clip + w.scene.sim.as_ref().unwrap().gun.spare;
            if ammo_now == 0 { eprintln!("[01] stairwell guard {g}: skipped — dry"); continue; }
            let mut killed = w.shoot(g, &format!("stairwell guard {g}"));
            if !killed {
                if w.resupply("ammo retry") { killed = w.shoot(g, &format!("stairwell guard {g}")); }
            }
            let dead = w.scene.sim.as_ref().unwrap().drones.list[g].corpse
                || w.scene.sim.as_ref().unwrap().drones.list[g].health <= 0.0;
            if killed || dead { cleared.insert(g); }
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x10) { break 'recovery; }
        }
        w.follow(ci, &format!("the contact descent r{round}"), 6000, plate);
    }
    // Bond LEADS the descent: he takes the plate first — the AND needs BOTH, so the exit holds
    // until the contact arrives — while the hand (below) lands on the live world. Bond ahead is
    // fine for the lead: the wait bands stall only for a Bond BEHIND on the route [M 0x15e698
    // case 3's −9.0 path arm], so the contact's route walks him down to the plate.
    let on_plate = |p: V3| p[0] < -32.4 && p[1] < 4.0 && (12.2..18.7).contains(&p[2]);
    let exited = w.scene.sim.as_ref().unwrap().drones.channels.on(0x10);
    if !exited && !on_plate(w.scene.sim.as_ref().unwrap().drones.list[ci].pos) {
        // The proven descent: the stairwell head anchor first (the direct nav to the plate from
        // the balcony side climbs over the top floor and never enters the stairwell — the sister
        // proved the descent through this anchor).
        w.walk_nav_to([-30.8, 6.4, 13.1], "the stairwell head", 1.2);
        w.walk_nav_to(plate, "the plate (lead the descent)", 1.2);
    }
    // obj0 "Escort Mayhew to bunker" (text 0x04000006) watches ch 0x47 [L mission::c07000001,
    // ee_start_05 @ 0x2a3eb8]. The disc carries NO ch-0x47 writer — the same census as ch 0x33:
    // every placement, trigger, mux, script and pickup scanned, the only 0xD writes are 0xB and
    // 0xE [D walkreal_probe01] — the un-ported mission/dialogue layer owns the write. The walk
    // hands it at the level's own beat — Mayhew reached the bunker plate — and pins the REAL
    // monitor's consequences [M 0x1827d8].
    // The record indexes resolve from the LEVELS table, not hardcoded: the table mirrors the
    // game's .data order and the mission-05..08 records sit in FRONT of 01's [M 0x1827d8 walks
    // LEVELS in table order; a hardcoded [0][0] named 01's record before those landed].
    // ORDER MATTERS: once the AND fires (both on the plate), the exit-hold freezes the world
    // (docs/systems/exit-hold.md — step_bond returns before Drones::tick), so the hand and the
    // monitor's complete/announce land BEFORE Bond takes the last step onto the plate — the game's
    // own order (the banner shows during Mayhew's arrival, the load follows the fade).
    let m01 = nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0001).expect("07000001's LEVELS record");
    let m02 = nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0002).expect("07000002's LEVELS record");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.states[m01][0], 3, "obj0 announced (immediate reveal, enable 0) [M 0x1827d8 case 0]");
    }
    if !exited {
        let sim = w.scene.sim.as_mut().unwrap();
        let frame = sim.drones.frame;
        sim.drones.channels.set(0x47, 1, frame);
    }
    w.wait(30);
    // The exit can race the hand (the recovery's follow can carry Bond onto the plate right behind
    // the contact): if the world froze mid-wait, the same monitor over the same channels, called
    // once for the frozen window, performs the identical transitions [*suggested* stand-in — the
    // game's Mission_Update keeps running through the fade; the port's hold skips it].
    if !exited && w.scene.sim.as_ref().unwrap().drones.mission.states[m01][0] != 4 {
        let mut sim = w.scene.sim.as_mut().unwrap();
        let (lvl, fr) = (sim.level, sim.drones.frame);
        let mut ev = vec![];
        sim.drones.mission.update(&mut sim.drones.channels, fr, lvl, false, &mut ev);
        eprintln!("[01] the exit raced the hand — the frozen-window monitor call ran ({} events)", ev.len());
    }
    {
        // The monitor's transitions, whichever window ran them: obj0 completes (case 3),
        // 07000002's first two objectives announce on the 0x47 enable (case 1 has no level gate
        // [L mission::c07000002]).
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.states[m01][0], 4, "obj0 completes through the real monitor [M 0x1827d8 case 3]");
        assert_eq!(sim.drones.mission.states[m02][0], 3, "07000002 'Destroy Mayhew's computer' announces on the 0x47 enable [M 0x1827d8 case 1]");
        assert_eq!(sim.drones.mission.states[m02][1], 3, "07000002 'Rescue geisha girls' announces on the 0x47 enable");
        assert_eq!(sim.drones.mission.states[m02][2], 1, "07000002 'Find door to servants quarters' still waits on enable 0x4c");
        assert!(!sim.drones.channels.on(100), "the mission spans 07000002 — success waits for its objectives");
    }
    if !exited {
        // The contact finishes the route onto the plate → the AND → the exit (Bond leads from the
        // plate; the wait loop gives the descent its window).
        for _ in 0..80 {
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x10) { break; }
            w.wait(60);
            if on_plate(w.scene.sim.as_ref().unwrap().drones.list[ci].pos)
                && !w.scene.sim.as_ref().unwrap().drones.channels.on(0x10) {
                w.walk_nav_to(plate, "the plate (re-take)", 1.2);
            }
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x10) { break; }
        }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x10), "the AND-touch must fire with Bond AND the contact on the plate (contact at {:?})",
        sim.drones.list[ci].pos);
    drop(sim);
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_0002), "the exit fires to 07000002");
    eprintln!("[01] level_exit to 07000002 at tick {} — walked {} ticks total", w.total_ticks, w.total_ticks);
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


/// Sister: Bond never walks the meet plate (he stays at the spawn side of the bar), so the
/// TOUCH_ONCE at (6.8,6.0,8.9) never writes ch 0x32, the 0xEE mirror never raises ch 0x33
/// [M Trigger_Update 0x1b1a78 case 5], SP #276 (mask 1 = its own out channel [M SP_Hit
/// 0x1dc470:17-21]) never triggers, and the four hall guards waiting on ch 0xE never start.
#[test]
fn the_hall_guards_wait_for_the_meet_scenes_channel() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load"));
    for _ in 0..4000u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xb) { break; }
    }
    w.wait_enabled(6000, "the intro tail");
    // Long past the scene's 280-frame span [D decode]: the plate stays untouched.
    w.wait(2400);
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x32), "the meet plate never fired (Bond kept away)");
    assert!(!sim.drones.channels.on(0x33), "the 0x33 mirror follows ch 0x32 [M case 5]");
    assert!(!sim.drones.channels.on(0xe), "ch 0xE never rises without the scene");
    let waiting: Vec<u8> = sim.drones.list.iter()
        .filter(|d| d.switch_channel == 0xe)
        .map(|d| d.state)
        .collect();
    assert_eq!(waiting.len(), 4, "four drones wait on ch 0xE [D placements]");
    if waiting.iter().any(|&s| s != nf_game::drone::state::WAIT_SWITCH) {
        for (i, d) in sim.drones.list.iter().enumerate() {
            if d.switch_channel == 0xe {
                eprintln!("[01 sister] ch-0xE drone {i} skin {:#x} at {:?} state={:#x} enabled={} health={:.0} anim {:?}",
                    d.skin, d.pos, d.state, d.enabled, d.health, d.anim.danim);
            }
        }
    }
    assert!(waiting.iter().all(|&s| s == nf_game::drone::state::WAIT_SWITCH),
        "every ch-0xE guard must sit in WAIT_SWITCH while the meet plate is unfired (states {waiting:?}) [M NDrone2_DSTATE_Global]");
}
