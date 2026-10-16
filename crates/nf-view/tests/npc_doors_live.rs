//! npc-doors: the drone side of door use, re-verified against the real levels after the
//! 1012-1723 / 1013-1825 playtests ("npcs cant open doors", "NPC still can't open doors" on
//! 07000001 / 06 / 07 — the escort stalls at doors, guards and guests don't path through them).
//!
//! The game's chain, all cited [M]:
//! - `NDrone_InitDoorNodes` 0x139a70: every door-flagged route node (MAPNODE flags bit 1) binds
//!   the first-then-nearest class-0x1C door object at node+0x28; a node with no door clears the
//!   door flag. Links between two door-flagged nodes get link flag 0x20
//!   (`AINetwork_ModIntersectedLinkFlags_Bounds`), and the walls the pair's segment crosses get
//!   wall flag 0x800 (`Drone_PostLoad_Init`'s door walls).
//! - `LinkCreep_Handler` 0x15a2c8: the route creep dispatches `NDrone2_ReachedDestNode`
//!   0x153358 at a reached special node; while Drone+0xBA8 stands (a door being worked) the
//!   blocked test is skipped entirely — the drone keeps creeping into the doorway.
//! - `NDrone2_ReachedDestNode` 0x153358: a door node with a door-node successor stands the
//!   node's bound door in +0xBA8, saves the state in +0x5A6 and enters `DSTATE_OpenDoor`
//!   0x1681f8 (0x81); on the pair's far side an open door clears +0xBA8.
//! - `DSTATE_OpenDoor` 0x1681f8: one `NDrone2_OpenDoor` on entry, timer C retries every 2
//!   ticks, exits to the saved state only when `NDrone2_DoorIsOpen` 0x14aaa0 sees door+0xF4 == 4.
//! - `NDrone2_OpenDoor` 0x14aa38: only a CLOSED/CLOSING door (`+0xF4 & 0x12`) of type 0x1c
//!   activates: `Door_Activate(door, drone, 0)` 0x135be0; the attempt stamps +0xBB0.
//! - `Door_Activate` 0x135be0: a lock-channelled door (`DOORINFO+0x68`) with its channel off
//!   refuses while closed (the hit sound only); a kind > 2 door refuses (marker only); the
//!   wrong-side `approach` byte refuses; everything successful sets the busy marker (+0x70 bit 4).
//! - `Door_Update` 0x1363c8: the open channel (+0x66) auto-opens a closed door while on; the
//!   auto-close waits `FRAME_RATE_INT·6` quiet ticks and needs nobody of class 0x12/0x03/0x02
//!   within `2·(r_door+r_actor)²`.
//!
//! Opt-in: `NF_ISO`.

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::Input;

use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn settle(scene: &mut nf_view::scene::Scene, ticks: u32) {
    for _ in 0..3000 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); if !scene.sim.as_ref().unwrap().disabled { break; } }
    for _ in 0..ticks { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
}

fn data_of(sim: &nf_view::scene::Sim) -> nf_game::drone::Data {
    nf_game::drone::Data {
        tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
        world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
        skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
        hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
        surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
    }
}

/// The door-flagged nav nodes and the door each binds (`NDrone_InitDoorNodes` 0x139a70's rule:
/// the first candidate unconditionally, then strictly nearer; the port recomputes the same map).
fn door_bindings(sim: &nf_view::scene::Sim) -> Vec<(usize, usize, usize)> { // (path, node, door)
    let mut out = Vec::new();
    for (pi, p) in sim.nav.paths.iter().enumerate() {
        for (ni, n) in p.nodes.iter().enumerate() {
            if n.flags & 2 == 0 || sim.drones.doors.list.is_empty() { continue; }
            let mut best: Option<(usize, f32)> = None;
            for (di, door) in sim.drones.doors.list.iter().enumerate() {
                let q = (door.pos[0] - n.pos[0]).powi(2) + (door.pos[1] - n.pos[1]).powi(2) + (door.pos[2] - n.pos[2]).powi(2);
                if best.map_or(true, |(_, bq)| q < bq) { best = Some((di, q)); }
            }
            if let Some((di, _)) = best { out.push((pi, ni, di)); }
        }
    }
    out
}

fn dump_level(sim: &nf_view::scene::Sim, tag: &str) {
    eprintln!("[{tag}] level {:#x} bond {:?}", sim.level, sim.bond.pos);
    for (i, d) in sim.drones.doors.list.iter().enumerate() {
        eprintln!("[{tag}] door #{i} ({:.2},{:.2},{:.2}) kind {} group {} approach {} locked {} ch_open {} ch_lock {} flags {:#x} state {:#x}",
            d.pos[0], d.pos[1], d.pos[2], d.kind, d.group, d.approach, d.locked_flag, d.ch_open, d.ch_lock, d.flags, d.state);
    }
    for (pi, ni, di) in door_bindings(sim) {
        let n = &sim.nav.paths[pi].nodes[ni];
        let d = &sim.drones.doors.list[di];
        eprintln!("[{tag}] binding path {pi} n{} ({:.1},{:.1},{:.1}) → door #{di} ({:.1},{:.1},{:.1}) d {:.2}",
            ni, n.pos[0], n.pos[1], n.pos[2], d.pos[0], d.pos[1], d.pos[2],
            ((d.pos[0] - n.pos[0]).powi(2) + (d.pos[1] - n.pos[1]).powi(2) + (d.pos[2] - n.pos[2]).powi(2)).sqrt());
    }
    for (pi, p) in sim.nav.paths.iter().enumerate() {
        let dn: Vec<String> = p.nodes.iter().enumerate().filter(|(_, n)| n.flags & 2 != 0)
            .map(|(i, n)| format!("n{}({:.1},{:.1},{:.1})", i, n.pos[0], n.pos[1], n.pos[2])).collect();
        if !dn.is_empty() { eprintln!("[{tag}] nav path {pi} kind {}: door nodes {}", p.kind, dn.join(" ")); }
    }
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if !dr.present || dr.corpse { continue; }
        let route = dr.route.as_ref().map(|r| format!("path {} idx {} {} nodes", r.path, r.index, r.nodes.len())).unwrap_or("-".into());
        eprintln!("[{tag}] drone #{i} skin {:#x} kind {} state {:#x} enabled {} pos ({:.1},{:.1},{:.1}) route {} goal {:?}",
            dr.skin, dr.kind, dr.state, dr.enabled, dr.pos[0], dr.pos[1], dr.pos[2], route, dr.goal.is_some());
    }
}

/// The census: what the three levels actually hold — doors, door-flagged nodes, live routes
/// crossing them. The scenarios below pick from this.
#[test]
fn census_of_doors_and_routes() {
    for (level, ticks) in [("07000005.bin", 300), ("07000006.bin", 600), ("07000007.bin", 1200)] {
        let mut scene = nf_view::load(&iso(), level, false).expect("load");
        settle(&mut scene, ticks);
        dump_level(scene.sim.as_ref().unwrap(), level);
    }
}

/// Diagnostic: which AI wall grazed the escort's pre-fix stall segment (2.4,6.3,4.3)→(4.8,5.8,5.8)?
#[test]
fn probe_01_wall_at_the_stall() {
    let mut scene = nf_view::load(&iso(), "07000001.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    let a = [2.4f32, 5.7, 4.3]; // the FEET (pos.y − 0.6)
    let b = [4.8f32, 5.8, 5.8];
    for r in [0.05f32, 0.1, 0.2, 0.3, 0.4] {
        eprintln!("[probe] bounds_node_test({r}) = {}", sim.nav.bounds_node_test(r, a, b));
    }
    for (wi, w) in sim.nav.walls.iter().enumerate() {
        // endpoint near the segment?
        for n in [w.a, w.b] {
            let t = (((n[0]-a[0])*(b[0]-a[0]) + (n[1]-a[1])*(b[1]-a[1]) + (n[2]-a[2])*(b[2]-a[2]))
                / ((b[0]-a[0]).powi(2) + (b[1]-a[1]).powi(2) + (b[2]-a[2]).powi(2))).clamp(0.0, 1.0);
            let p = [a[0]+(b[0]-a[0])*t, a[1]+(b[1]-a[1])*t, a[2]+(b[2]-a[2])*t];
            let d2 = (p[0]-n[0]).powi(2) + (p[1]-n[1]).powi(2) + (p[2]-n[2]).powi(2);
            if d2 < 0.04 && (p[1]-n[1]).abs() < 2.0 {
                eprintln!("[probe] wall #{wi} flags {:#x} a ({:.2},{:.2},{:.2}) b ({:.2},{:.2},{:.2}) — endpoint within {} of the segment",
                    w.flags, w.a[0], w.a[1], w.a[2], w.b[0], w.b[1], w.b[2], d2.sqrt());
            }
        }
    }
}

/// Zero-interference watch on 07000007: the party guests and staff walk their CivilianMission
/// routes through the doorway node pairs (paths 3/6/7 carry door-flagged pairs → doors #3/#1).
/// The playtests say "guards and guests don't path through them" — this watches whether the
/// guests' own routes work the doors, stall in DSTATE_OpenDoor, or never reach the pair.
#[test]
fn watch_07_guests_path_through_doors() {
    let mut scene = nf_view::load(&iso(), "07000007.bin", false).expect("load 07000007");
    settle(&mut scene, 600);
    let pairs = [(3usize, 0usize, 6usize, 3usize), (6, 9, 10, 1), (7, 1, 2, 1)]; // (path, a, b, door)
    let mut opened = std::collections::HashSet::new();
    let mut stalls: std::collections::HashMap<usize, (u32, u8, u16)> = std::collections::HashMap::new(); // drone → (ticks near closed door, last state, last door)
    for t in 0..24000u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        for (i, d) in sim.drones.doors.list.iter().enumerate() {
            if d.state == nf_game::door::OPEN && !opened.contains(&i) {
                opened.insert(i);
                let near: Vec<(usize, f32, u8)> = sim.drones.list.iter().enumerate()
                    .map(|(di, dr)| (di, ((dr.pos[0]-d.pos[0]).powi(2)+(dr.pos[2]-d.pos[2]).powi(2)).sqrt(), dr.state))
                    .filter(|(_, dist, _)| *dist < 5.0).collect();
                eprintln!("[07] +{t}: door #{i} OPENED; drones within 5: {near:?}");
            }
        }
        for (pi, a, b, di) in pairs {
            let (na, nb, door) = (&sim.nav.paths[pi].nodes[a].pos, &sim.nav.paths[pi].nodes[b].pos, &sim.drones.doors.list[di]);
            for (dri, dr) in sim.drones.list.iter().enumerate() {
                if !dr.present || dr.corpse { continue; }
                let da = ((dr.pos[0]-na[0]).powi(2) + (dr.pos[2]-na[2]).powi(2)).sqrt();
                let db = ((dr.pos[0]-nb[0]).powi(2) + (dr.pos[2]-nb[2]).powi(2)).sqrt();
                if da.min(db) < 2.5 && door.state & 0x12 != 0 {
                    let linger = { let e = stalls.entry(dri).or_insert((0u32, dr.state, di as u16)); e.0 += 1; e.1 = dr.state; e.2 = di as u16; e.0 };
                    if linger % 300 == 0 {
                        let rinfo = dr.route.as_ref().map(|r| (r.path, r.index));
                        eprintln!("[07] +{t}: drone #{dri} state {:#x} lingering {linger} ticks near CLOSED door #{di} (pair {pi} n{a}-n{b}) at ({:.1},{:.1},{:.1}) route {rinfo:?}",
                            dr.state, dr.pos[0], dr.pos[1], dr.pos[2]);
                    }
                }
            }
            let _ = nb;
        }
        if t % 3000 == 0 {
            let moving: Vec<String> = sim.drones.list.iter().enumerate()
                .filter(|(_, dr)| dr.route.is_some() && dr.state != nf_game::drone::state::CIVILIAN_GUARD)
                .map(|(di, dr)| format!("#{di} {:#x} path {:?}", dr.state, dr.route.as_ref().map(|r| r.path))).collect();
            eprintln!("[07] +{t}: routed drones [{}]", moving.join("; "));
        }
    }
    eprintln!("[07] doors opened by themselves: {opened:?}; lingers: {:?}",
        stalls.iter().map(|(k, v)| (k, *v)).collect::<Vec<_>>());
}

/// The 07000001 escort, with NO use key anywhere: Mayhew must work the courtyard shoji himself
/// (DSTATE_OpenDoor 0x81 → NDrone2_OpenDoor 0x14aa38 → Door_Activate 0x135be0) and walk through.
/// Bond trails 3.9 behind (outside PlayerInWay's 3.0 band [M 0x15ea28]) and never presses E; the
/// escort's own `AllyLeadHide` pauses (an armed started guard within 14 [M 0x15ec08]) are the
/// game's own escort behaviour, so Bond clears the intro's guards through the real fire path
/// (the walkreal-a/fix_npc_doors pattern) — the DOOR work itself is all Mayhew's.
#[test]
fn the_escort_opens_his_door_without_bonds_help() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    // The intro cut-scene writes ch 0xB (the contact's enable channel) ~tick 2119; the tail
    // re-places Bond.
    let mut enabled_at = None;
    for t in 0..4000u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xb) { enabled_at = Some(t); break; }
    }
    let enabled_at = enabled_at.expect("the intro must write ch 0xB");
    w.wait_enabled(6000, "the intro tail (event 6 re-places Bond)");
    w.wait(300);
    let ci = w.scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("Mayhew exists");
    // The courtyard shoji (fix_npc_doors_live: door #1 at (7.92, 5.34, 11.14)); the route runs +z.
    let (door_idx, door_pos) = {
        let sim = w.scene.sim.as_ref().unwrap();
        let (i, d) = sim.drones.doors.list.iter().enumerate()
            .min_by(|(_, a), (_, b)| ((a.pos[0] - 7.0).abs() + (a.pos[2] - 11.0).abs())
                .total_cmp(&((b.pos[0] - 7.0).abs() + (b.pos[2] - 11.0).abs()))).unwrap();
        (i, d.pos)
    };
    eprintln!("[01] ch 0xB at {enabled_at}; escort at {:?} state {:#x}; shoji #{door_idx} at {door_pos:?}",
        w.scene.sim.as_ref().unwrap().drones.list[ci].pos, w.scene.sim.as_ref().unwrap().drones.list[ci].state);
    let start_ticks = w.total_ticks;
    let (mut saw_0x81, mut door_opened, mut crossed) = (None, false, false);
    let mut cleared: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut stall = 0u32;
    let mut last = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
    'follow: for _round in 0..8 {
        loop {
            {
                let sim = w.scene.sim.as_ref().unwrap();
                let c = &sim.drones.list[ci];
                if saw_0x81.is_none() && c.state == 0x81 {
                    saw_0x81 = Some(w.total_ticks - start_ticks);
                    eprintln!("[01] +{}: escort entered 0x81 at ({:.2},{:.2},{:.2}) hit_door {:?}",
                        w.total_ticks - start_ticks, c.pos[0], c.pos[1], c.pos[2], c.hit_door);
                }
                if !door_opened && sim.drones.doors.list[door_idx].state == nf_game::door::OPEN {
                    door_opened = true;
                    eprintln!("[01] +{}: shoji #{door_idx} OPENED (bond {:.1} away, no use key)",
                        w.total_ticks - start_ticks, walker::dxz(sim.bond.pos, door_pos));
                }
                if (c.pos[0] - door_pos[0]).abs() < 3.0 && c.pos[2] > door_pos[2] + 1.2 {
                    crossed = true;
                    eprintln!("[01] +{}: escort crossed the shoji at ({:.2},{:.2},{:.2})",
                        w.total_ticks - start_ticks, c.pos[0], c.pos[1], c.pos[2]);
                }
                // The stall watch: the escort parked > 2700 ticks → clear the armed guards near him
                // (LeadHide's band [M 0x15ec08]); 4.0+ movement resets the window (the nav-gap rule).
                if walker::dxz(c.pos, last) > 0.05 { stall = 0; last = c.pos; } else { stall += 1; }
            }
            if crossed && door_opened { break 'follow; }
            if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { break 'follow; }
            if stall > 2700 {
                stall = 0;
                let guards: Vec<usize> = {
                    let sim = w.scene.sim.as_ref().unwrap();
                    (0..sim.drones.list.len()).filter(|&i| {
                        let d = &sim.drones.list[i];
                        i != ci && d.team == 1 && d.started && !d.corpse && d.enabled
                            && walker::dxz(d.pos, sim.drones.list[ci].pos) < 16.0 && !cleared.contains(&i)
                    }).collect()
                };
                if !guards.is_empty() {
                    let glist = guards.clone();
                    eprintln!("[01] +{}: escort parked — clearing guards {glist:?}",
                        w.total_ticks - start_ticks);
                    for g in guards {
                        cleared.insert(g);
                        let gpos = w.scene.sim.as_ref().unwrap().drones.list[g].pos;
                        if walker::dxz(gpos, w.scene.sim.as_ref().unwrap().bond.pos) > 9.0 {
                            w.walk_nav_to(gpos, &format!("approach guard {g}"), 3.5);
                        }
                        if !w.shoot(g, &format!("guard {g}")) { cleared.remove(&g); }
                    }
                    let cpos = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
                    w.walk_nav_to(cpos, "back to the escort", 3.9);
                    continue 'follow;
                }
            }
            // Trail: 3.9 behind along the escort's own heading (outside PlayerInWay's 3.0).
            let (mp, myaw, bp) = { let sim = w.scene.sim.as_ref().unwrap();
                (sim.drones.list[ci].pos, sim.drones.list[ci].yaw, sim.bond.pos) };
            let trail = [mp[0] + myaw.sin() * -3.9, mp[1], mp[2] + myaw.cos() * -3.9];
            let d = walker::dxz(bp, trail);
            let desired = (trail[0] - bp[0]).atan2(trail[2] - bp[2]);
            let input = Input {
                turn: walker::wrap_pi(desired - w.scene.sim.as_ref().unwrap().bond.yaw).clamp(-0.07, 0.07),
                forward: if d > 5.0 { 1.0 } else if d > 4.2 { 0.45 } else { 0.0 },
                ..Default::default()
            };
            w.tick(input, false);
            if (w.total_ticks - start_ticks) % 3000 == 0 {
                let sim = w.scene.sim.as_ref().unwrap();
                let c = &sim.drones.list[ci];
                eprintln!("[01] +{} t: escort ({:.1},{:.1},{:.1}) state {:#x} hit_door {:?} peer {:.1}; shoji {} {:#x} pos01 {:.2}",
                    w.total_ticks - start_ticks, c.pos[0], c.pos[1], c.pos[2], c.state, c.hit_door, c.armed_peer_dist,
                    door_idx, sim.drones.doors.list[door_idx].state, sim.drones.doors.list[door_idx].pos01);
            }
            if w.total_ticks - start_ticks > 60_000 { break 'follow; }
        }
    }
    eprintln!("[01] result: 0x81 at {saw_0x81:?}, opened {door_opened}, crossed {crossed}");
    assert!(saw_0x81.is_some(), "the escort never entered DSTATE_OpenDoor at the courtyard shoji");
    assert!(door_opened, "the escort worked the door but it never reached state 4 (OPEN)");
    assert!(crossed, "the door opened but the escort never walked through");
}

/// Isolation drive: Mayhew (07000001) with a GoToGoalPosition (0x63) goal at the courtyard shoji,
/// minus the guards (whose armed cower `AllyLeadHide` 0x15ec08 is the game's own escort pause).
/// The 0x63 goal route runs on the guard's own region path (kind-8 paths are excluded from
/// `NavPathForPosition` [M 0x158488's kind & 0xD filter]), which carries NO door nodes — so this
/// drive proves the follow side only: pre-fix the drive WEDGED 8 units short of the shoji (the
/// wall-graze block, never recovered); with the step-back it walks up to the closed shoji and
/// stops against it — and the closed, never-activated door BLOCKS the drone (the sister).
#[test]
fn the_01_escort_driven_through_the_shoji_opens_it_and_crosses() {
    let mut scene = nf_view::load(&iso(), "07000001.bin", false).expect("load 07000001");
    let mut ch_b = false;
    for t in 0..4000u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.channels.on(0xb) { ch_b = true; }
        if ch_b && t > 100 && !scene.sim.as_ref().unwrap().disabled { break; }
    }
    let mi = scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("Mayhew exists");
    let (door_idx, door_pos) = {
        let sim = scene.sim.as_ref().unwrap();
        let (i, d) = sim.drones.doors.list.iter().enumerate()
            .min_by(|(_, a), (_, b)| ((a.pos[0] - 7.0).abs() + (a.pos[2] - 11.0).abs())
                .total_cmp(&((b.pos[0] - 7.0).abs() + (b.pos[2] - 11.0).abs()))).unwrap();
        (i, d.pos)
    };
    // The pair's nodes (the kind-8 Mayhew route, path 0): find the door-flagged nodes binding door_idx.
    let pair: Vec<(usize, usize)> = {
        let sim = scene.sim.as_ref().unwrap();
        let mut v = Vec::new();
        for (pi, p) in sim.nav.paths.iter().enumerate() {
            for (ni, n) in p.nodes.iter().enumerate() {
                if n.flags & 2 == 0 { continue; }
                let q = (door_pos[0] - n.pos[0]).powi(2) + (door_pos[1] - n.pos[1]).powi(2) + (door_pos[2] - n.pos[2]).powi(2);
                if q < 16.0 { v.push((pi, ni)); }
            }
        }
        v
    };
    eprintln!("[01d] shoji #{door_idx} at {door_pos:?}; door-flagged nodes near it: {pair:?}");
    // The goal: 2.5 beyond the pair's far node along the pair axis.
    let beyond = {
        let sim = scene.sim.as_ref().unwrap();
        let (pi, a) = pair[0];
        let nodes = &sim.nav.paths[pi].nodes;
        // The two nodes nearest the door on this path.
        let mut ns: Vec<usize> = (0..nodes.len()).filter(|&n| nodes[n].flags & 2 != 0
            && (nodes[n].pos[0] - door_pos[0]).powi(2) + (nodes[n].pos[2] - door_pos[2]).powi(2) < 16.0).collect();
        ns.sort_by_key(|&n| n);
        assert!(ns.len() >= 2, "the shoji pair needs two door nodes: {ns:?}");
        let (na, nb) = (nodes[ns[0]].pos, nodes[ns[1]].pos);
        let dir = [nb[0] - na[0], nb[1] - na[1], nb[2] - na[2]];
        let l = (dir[0] * dir[0] + dir[2] * dir[2]).sqrt().max(1e-4);
        [nb[0] + dir[0] / l * 2.5, nb[1], nb[2] + dir[2] / l * 2.5]
    };
    eprintln!("[01d] driving Mayhew to beyond {beyond:?}");
    {
        let sim = scene.sim.as_mut().unwrap();
        let difficulty = sim.drones.difficulty;
        let d = nf_game::drone::Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
            skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
            hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
            surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty,
        };
        let drones = &mut sim.drones;
        let dr = &mut drones.list[mi];
        dr.goal = Some(beyond);
        dr.next_state = Some(nf_game::drone::state::GO_TO_GOAL);
        dr.send(&d, &mut drones.rng, drones.frame, 0);
        assert_eq!(drones.list[mi].state, nf_game::drone::state::GO_TO_GOAL, "the 0x63 entry did not happen");
    }
    let (mut closest, mut crossed) = (1e9f32, false);
    for t in 0..7200u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let st = sim.drones.list[mi].state;
        let mp = sim.drones.list[mi].pos;
        closest = closest.min(((mp[0] - door_pos[0]).powi(2) + (mp[2] - door_pos[2]).powi(2)).sqrt());
        if (mp[0] - beyond[0]).powi(2) + (mp[2] - beyond[2]).powi(2) < 2.0_f32.powi(2) { crossed = true; }
        if t % 600 == 0 {
            eprintln!("[01d] +{t}: Mayhew state {st:#x} at ({:.1},{:.1},{:.1}) door {} state {:#x} pos01 {:.2}",
                mp[0], mp[1], mp[2], door_idx, sim.drones.doors.list[door_idx].state, sim.drones.doors.list[door_idx].pos01);
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[01d] result: closest approach to the shoji {closest:.2}, crossed {crossed}, door state {:#x}",
        sim.drones.doors.list[door_idx].state);
    assert!(closest < 2.0, "the driven Mayhew never reached the shoji (closest {closest:.2}) — the follow still wedges short of it");
    assert!(!crossed, "the driven Mayhew crossed the CLOSED, never-activated shoji — a closed door did not block");
    assert_eq!(sim.drones.doors.list[door_idx].state, nf_game::door::CLOSED, "the shoji moved with nobody opening it");
}

/// The locked doors refuse the drone's activation, and a closed door that no drone opens still
/// blocks it. 07000005 holds four lock-channelled sliders (ch_lock 200, kind 2) — [M]
/// `Door_Activate` 0x135be0's early gate: `ch_lock != 0 && switch_channels[200] == 0 && state &
/// 0x12` refuses with the hit sound only — and one unlocked swing door (#1, the nav pair's
/// binding) as the control. Sister: the drone-side `Door_Activate` (what `Drones::tick`'s
/// door_attempt service calls, [M] `NDrone2_OpenDoor` 0x14aa38) cannot move the locked doors,
/// and a 0x63-driven guard walking into the locked pair's doorway never crosses.
#[test]
fn a_locked_door_stays_shut_for_drones() {
    let mut scene = nf_view::load(&iso(), "07000005.bin", false).expect("load 07000005");
    settle(&mut scene, 300);
    let locked: Vec<usize> = {
        let sim = scene.sim.as_ref().unwrap();
        sim.drones.doors.list.iter().enumerate().filter(|(_, d)| d.ch_lock == 200).map(|(i, _)| i).collect()
    };
    assert_eq!(locked, vec![0, 2, 3, 4], "the four lock-channelled doors");
    // Drone-side activation (from_player = false — the NDrone2_OpenDoor arm) on every locked door:
    // refused, the door never leaves CLOSED, and the refusal is not the player-flag path.
    for &t in &locked {
        let sim = scene.sim.as_mut().unwrap();
        let mut evs = Vec::new();
        let ok = sim.drones.doors.activate(t, &[(sim.drones.list[6].pos, 0.4, false)], false,
            &mut sim.drones.channels, sim.drones.frame, &mut evs);
        eprintln!("[05] drone-activate door #{t} → {ok}, state {:#x}", sim.drones.doors.list[t].state);
        assert!(!ok, "door #{t} (ch_lock 200) let a drone activate it");
        assert_eq!(sim.drones.doors.list[t].state, nf_game::door::CLOSED, "door #{t} moved behind the drone's back");
    }
    // The control: the same drone-side call on the unlocked swing door #1 OPENS it.
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut evs = Vec::new();
        let ok = sim.drones.doors.activate(1, &[(sim.drones.list[2].pos, 0.4, false)], false,
            &mut sim.drones.channels, sim.drones.frame, &mut evs);
        assert!(ok, "the unlocked control door refused the drone-side activation");
        assert_eq!(sim.drones.doors.list[1].state, nf_game::door::OPENING, "the control door did not start opening");
    }
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().drones.doors.list[1].state, nf_game::door::OPEN, "the control door did not reach OPEN");
    // Sister: a 0x63-driven guard (drone #6, IDLE at (99.7,16.8,101.6)) with a goal beyond the
    // locked pair #2/#3 walks into the closed, unopenable doorway and never crosses it — a closed
    // door still blocks a drone that does not open it.
    let (di, dpos) = (2usize, {
        let sim = scene.sim.as_ref().unwrap();
        sim.drones.doors.list[2].pos
    });
    let beyond = [dpos[0], dpos[1], dpos[2] - 2.5];
    {
        let sim = scene.sim.as_mut().unwrap();
        let difficulty = sim.drones.difficulty;
        let d = nf_game::drone::Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
            skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
            hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
            surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty,
        };
        let drones = &mut sim.drones;
        let dr = &mut drones.list[6];
        dr.goal = Some(beyond);
        dr.next_state = Some(nf_game::drone::state::GO_TO_GOAL);
        dr.send(&d, &mut drones.rng, drones.frame, 0);
        assert_eq!(drones.list[6].state, nf_game::drone::state::GO_TO_GOAL, "the 0x63 entry did not happen");
    }
    let start6 = scene.sim.as_ref().unwrap().drones.list[6].pos;
    let mut min_side = 0.0f32; // the drone's z minus the door plane's z (the near side is +z)
    let mut reached = 0.0f32;
    for t in 0..3600u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let p = sim.drones.list[6].pos;
        min_side = min_side.max(p[2] - dpos[2]);
        reached = reached.max((p[0] - dpos[0]).abs());
        if t % 900 == 0 {
            eprintln!("[05] +{t}: guard #6 at ({:.1},{:.1},{:.1}) state {:#x}; door #2 state {:#x}",
                p[0], p[1], p[2], sim.drones.list[6].state, sim.drones.doors.list[2].state);
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[05] sister: closest approach to the door plane Δz {:.2}, Δx {:.2}; door #2 state {:#x}",
        min_side, reached, sim.drones.doors.list[2].state);
    assert_eq!(sim.drones.doors.list[2].state, nf_game::door::CLOSED, "the locked door opened anyway");
    // The drive was not vacuous: the guard left his spawn (the level's floor between his post and
    // the doorway is open — he closed to Δx {reached}) and the doorway's plane was never crossed.
    assert!(walker::dxz(start6, sim.drones.list[6].pos) > 2.0, "the 0x63 drive never moved the guard");
    assert!(min_side < 1.0, "the guard crossed the locked door's plane (Δz {min_side}) — a closed unopenable door did not block");
}

/// The 07000006 chase: Bond beyond the closed door #1 pair (path 7 n0–n1), firing at a guard on
/// the near side — the guard's investigate/chase route crosses the doorway pair, and the guard
/// must open the door himself (the combat-route dispatch reaches `route_reached_special`,
/// fix-guard-misc2; the creep step-back keeps the chase from wedging on the doorway's walls).
#[test]
fn the_06_guard_chasing_bond_opens_the_closed_door() {
    let mut scene = nf_view::load(&iso(), "07000006.bin", false).expect("load 07000006");
    settle(&mut scene, 300);
    // The pair and door #1 (the fix_guard_misc census: path 7 n0 (35.9,40.7,-15.7) — n1 —, door #1 at (36.5,40.0,-16.3)).
    let (di, dpos) = (1usize, {
        let sim = scene.sim.as_ref().unwrap();
        sim.drones.doors.list[1].pos
    });
    // The enabled IDLE guard nearest the pair on the +z side (the census: #2 at (27.5,39.0,-10.3)).
    let gi = {
        let sim = scene.sim.as_ref().unwrap();
        sim.drones.list.iter().enumerate().filter(|(_, dr)| dr.present && !dr.corpse && dr.enabled && dr.health > 0.0
            && dr.pos[2] > dpos[2] && dr.pos[2] < dpos[2] + 10.0)
            .min_by(|a, b| ((a.1.pos[0]-dpos[0]).powi(2) + (a.1.pos[2]-dpos[2]).powi(2))
                .total_cmp(&((b.1.pos[0]-dpos[0]).powi(2) + (b.1.pos[2]-dpos[2]).powi(2))))
            .map(|(i, _)| i).expect("an enabled guard on the near side of door #1")
    };
    eprintln!("[06c] guard #{gi} at {:?} state {:#x}; door #{di} at {dpos:?}",
        scene.sim.as_ref().unwrap().drones.list[gi].pos, scene.sim.as_ref().unwrap().drones.list[gi].state);
    // Bond beyond the door (the far side), facing the guard: the shot's noise pulls the chase.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [dpos[0], dpos[1] + 1.1, dpos[2] - 4.5];
        sim.bond.yaw = (sim.drones.list[gi].pos[0] - sim.bond.pos[0]).atan2(sim.drones.list[gi].pos[2] - sim.bond.pos[2]);
    }
    let (mut saw_0x81, mut opened, mut crossed) = (None, false, false);
    let mut shots = 0;
    for t in 0..9000u32 {
        let (aim_err, dist) = {
            let sim = scene.sim.as_ref().unwrap();
            let (bp, gp) = (sim.bond.pos, sim.drones.list[gi].pos);
            let want = (gp[0] - bp[0]).atan2(gp[2] - bp[2]);
            (walker::wrap_pi(want - sim.bond.yaw), walker::dxz(bp, gp))
        };
        let input = Input {
            turn: aim_err.clamp(-0.08, 0.08),
            forward: 0.0,
            ..Default::default()
        };
        let trigger = shots < 8 && t % 40 == 0 && aim_err.abs() < 0.05 && t > 30;
        if trigger { shots += 1; }
        scene.sim.as_mut().unwrap().step_bond(input, trigger, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let (st, gp) = (sim.drones.list[gi].state, sim.drones.list[gi].pos);
        if saw_0x81.is_none() && st == 0x81 {
            saw_0x81 = Some(t);
            eprintln!("[06c] +{t}: guard entered 0x81, hit_door {:?}", sim.drones.list[gi].hit_door);
        }
        if !opened && sim.drones.doors.list[di].state == nf_game::door::OPEN {
            opened = true;
            eprintln!("[06c] +{t}: door #{di} OPENED by the chasing guard");
        }
        if gp[2] < dpos[2] - 1.0 { crossed = true; }
        if crossed && opened { break; }
        if t % 900 == 0 {
            eprintln!("[06c] +{t}: guard #{gi} state {st:#x} at ({:.1},{:.1},{:.1}); door {} {:#x} pos01 {:.2}; bond {:?} shots {shots}",
                gp[0], gp[1], gp[2], di, sim.drones.doors.list[di].state, sim.drones.doors.list[di].pos01, sim.bond.pos);
        }
    }
    eprintln!("[06c] result: 0x81 at {saw_0x81:?}, opened {opened}, crossed {crossed}");
    assert!(opened, "the chasing guard never opened door #{di}");
    assert!(crossed, "the door opened but the chasing guard never crossed to Bond's side");
    assert!(saw_0x81.is_some(), "the chase opened the door without the DSTATE_OpenDoor dispatch");
}
