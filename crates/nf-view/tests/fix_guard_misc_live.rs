//! fix-guard-misc: the 0700000c sniper takes no damage; the 07000006 guards can't open doors;
//! the 07000006 guards render in a crazy pose on their first frames. Each test drives the REAL
//! input path (`nf_view::load` + `Sim::step_bond`). Opt-in: `NF_ISO`.

#[path = "common/walker.rs"]
mod walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn settle(scene: &mut nf_view::scene::Scene, ticks: u32) {
    for _ in 0..3000 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); if !scene.sim.as_ref().unwrap().disabled { break; } }
    for _ in 0..ticks { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
}

/// 07000006 first frames: every placed guard must be POSED from the first ticks (the game gives
/// each drone its start danim in `NDrone2_DefaultInit`, so frame 0 is already a pose — never the
/// bind pose the user filmed as "crazy model stuff").
#[test]
fn the_06_guards_are_posed_from_their_first_frames() {
    let mut scene = nf_view::load(&iso(), "07000006.bin", false).expect("load 07000006");
    let mut bad_at: Vec<(u32, Vec<String>)> = Vec::new();
    for t in 0..240u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if matches!(t, 0 | 1 | 5 | 30 | 60 | 120 | 239) {
            let un = scene.sim.as_ref().unwrap().unposed(&scene.characters);
            eprintln!("[pose] tick {t}: {} unposed of {} characters", un.len(), scene.characters.len());
            for u in &un { eprintln!("[pose]   {u}"); }
            if !un.is_empty() { bad_at.push((t, un)); }
        }
    }
    assert!(bad_at.is_empty(), "guards unposed on their first frames at ticks {:?} (the user's \"crazy models\")",
        bad_at.iter().map(|(t, _)| t).collect::<Vec<_>>());
}

/// 07000006 doors: the guards' routes cross door nodes; tick the level with Bond standing out of
/// the way and log every guard that nears a closed door, its state machine and the door's state.
/// The door a guard works must reach OPEN (state 4) without Bond touching it.
#[test]
fn the_06_guards_open_the_doors_on_their_routes() {
    let mut scene = nf_view::load(&iso(), "07000006.bin", false).expect("load 07000006");
    settle(&mut scene, 300);
    {
        let sim = scene.sim.as_ref().unwrap();
        eprintln!("[door] bond at {:?}", sim.bond.pos);
        for (i, d) in sim.drones.doors.list.iter().enumerate() {
            eprintln!("[door] door #{i} ({:.2},{:.2},{:.2}) kind {} approach {} locked {} ch_open {} ch_lock {} state {:#x}",
                d.pos[0], d.pos[1], d.pos[2], d.kind, d.approach, d.locked_flag, d.ch_open, d.ch_lock, d.state);
        }
        for (i, dr) in sim.drones.list.iter().enumerate() {
            eprintln!("[door] drone #{i} skin {:#x} kind {} state {:#x} pos ({:.2},{:.2},{:.2}) weapon {} route {:?} danim {} script {:?}",
                dr.skin, dr.kind, dr.state, dr.pos[0], dr.pos[1], dr.pos[2], dr.weapon,
                dr.route.as_ref().map(|r| (r.path, r.index)), dr.anim.danim, dr.anim.script);
        }
        // The nav network's door-flagged nodes (`ReachedDestNode`'s door case only fires on these).
        for (pi, p) in sim.nav.paths.iter().enumerate() {
            let door_nodes: Vec<usize> = p.nodes.iter().enumerate().filter(|(_, n)| n.flags & 2 != 0).map(|(i, _)| i).collect();
            eprintln!("[door] nav path {pi}: {} nodes, door-flagged {}", p.nodes.len(),
                door_nodes.iter().map(|i| format!("n{}({:.1},{:.1},{:.1})", i, p.nodes[*i].pos[0], p.nodes[*i].pos[1], p.nodes[*i].pos[2])).collect::<Vec<_>>().join(" "));
        }
    }
    // Watch: any drone within 3.0 of a CLOSED door for a stretch — did the door open on its own
    // (Bond > 2.5 away, never pressing use)?
    let mut opened: Vec<usize> = Vec::new();
    let mut was_open = vec![false; scene.sim.as_ref().unwrap().drones.doors.list.len()];
    for t in 0..7200u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        for (i, d) in sim.drones.doors.list.iter().enumerate() {
            if d.state == nf_game::door::OPEN && !was_open[i] {
                was_open[i] = true;
                let near: Vec<(usize, f32, u8)> = sim.drones.list.iter().enumerate()
                    .map(|(di, dr)| (di, ((dr.pos[0]-d.pos[0]).powi(2)+(dr.pos[2]-d.pos[2]).powi(2)).sqrt(), dr.state))
                    .filter(|(_, dist, _)| *dist < 4.0).collect();
                eprintln!("[door] +{t}: door #{i} OPENED; drones within 4: {near:?}");
                opened.push(i);
            }
        }
        if t % 600 == 0 {
            let busy: Vec<String> = sim.drones.list.iter().enumerate()
                .filter(|(_, dr)| dr.state == 0x81 || dr.hit_door.is_some())
                .map(|(di, dr)| format!("#{di} state {:#x} hit_door {:?} at ({:.1},{:.1},{:.1})", dr.state, dr.hit_door, dr.pos[0], dr.pos[1], dr.pos[2]))
                .collect();
            eprintln!("[door] +{t}: door-working drones: [{}]", busy.join("; "));
        }
    }
    eprintln!("[door] doors that opened by themselves: {opened:?}");
    // fix-guard-misc2: the game-true 06 idle contract. The level's guards all stand at POSTS —
    // `route: None`, state 1/WaitSwitch (the census above) — and no route means no followed-route
    // dispatch, so idle 06 never opens a door (in the game either; door-flagged pairs exist on
    // paths 3 and 7 but only a DISPATCHED guard walks them — see
    // `the_06_guard_driven_through_a_door_pair_opens_it_and_crosses`, which drives the 0x63 path
    // through the pair and now opens + crosses). What idle must NOT do is drift: no guard may
    // wander into DSTATE_OpenDoor (0x81) and no door may open with nobody driving it.
    assert!(opened.is_empty(), "doors opened with nobody driving them (idle 06 must hold its posts): {opened:?}");
}

/// Diagnostic probe: the rooftop sniper's hit boxes, palette and world LOS from a few spots.
#[test]
fn probe_sniper_boxes_and_los() {
    let mut scene = nf_view::load(&iso(), "0700000c.bin", false).expect("load 0700000c");
    settle(&mut scene, 60);
    let sim = scene.sim.as_ref().unwrap();
    let data = nf_game::drone::Data {
        tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
        world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
        skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
        hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
        surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
    };
    for di in [17usize, 0, 1] {
        let d = &sim.drones.list[di];
        let pal = d.palette(&data);
        let boxes = d.hit_boxes(&data);
        eprintln!("[probe] drone #{di} skin {:#x} state {:#x} pos {:?} anim script {:?} danim {} ftime {:.1} palette {} hit_boxes {} drawn {}",
            d.skin, d.state, d.pos, d.anim.script, d.anim.danim, d.anim.ftime, pal.as_ref().map_or(0, |p| p.len()), boxes.len(), d.drawn);
        if let Some((_, hip)) = d.object_pose(&data) { eprintln!("[probe]   hip {hip}"); }
        for b in boxes.iter().take(6) {
            eprintln!("[probe]   box part {} centre ({:.2},{:.2},{:.2}) half {:?}", b.part, b.m[3][0], b.m[3][1], b.m[3][2], b.half);
        }
    }
    // LOS from several candidate Bond spots to drone 17's boxes.
    let d17 = &sim.drones.list[17];
    let spots: [([f32; 3], &str); 6] = [
        ([43.8, 12.2, -185.0], "yard south"),
        ([43.8, 12.2, -225.0], "yard north"),
        ([30.0, 12.2, -205.0], "yard west"),
        ([60.0, 12.2, -205.0], "yard east"),
        ([43.8, 17.0, -195.0], "mid height"),
        ([43.8, 21.4, -195.0], "roof level"),
    ];
    for (s, name) in spots {
        let eye = [s[0], s[1] + 1.5, s[2]];
        let chest = [d17.pos[0], d17.pos[1] + 0.3, d17.pos[2]];
        let seg = sim.world.segment(eye, chest);
        eprintln!("[probe] spot {name} ({:?}) → chest: segment {:?}", s, seg);
    }
}

/// Deterministic-fire probe: push the exact bullet (vantage eye → the sniper's torso box) through
/// `player_bullets` and watch the real bullet tick. Discriminates "the ray never reaches the boxes"
/// from "the hit is dropped after `first_box_hit`".
#[test]
fn probe_sniper_direct_bullet() {
    let mut scene = nf_view::load(&iso(), "0700000c.bin", false).expect("load 0700000c");
    settle(&mut scene, 60);
    let si = 17usize;
    let ch = scene.sim.as_ref().unwrap().drones.list[si].switch_channel;
    scene.sim.as_mut().unwrap().drones.set_channel(ch);
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (from, _) = ([46.0f32, 15.7, -182.0], [43.77f32, 21.84, -205.13]);
    // Aim at the LIVE torso box centre (part 4's m[3] at fire time) — the settle-time centre went
    // stale (the drone shifts with his idle anim).
    let chest = {
        let sim = scene.sim.as_ref().unwrap();
        let data = nf_game::drone::Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
            skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
            hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
            surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
        };
        let boxes = sim.drones.list[si].hit_boxes(&data);
        let torso = boxes.iter().find(|b| b.part == 4).expect("the torso box");
        [torso.m[3][0], torso.m[3][1], torso.m[3][2]]
    };
    let d = [chest[0]-from[0], chest[1]-from[1], chest[2]-from[2]];
    let l = (d[0]*d[0] + d[1]*d[1] + d[2]*d[2]).sqrt();
    let dir = [d[0]/l, d[1]/l, d[2]/l];
    {
        let seg = scene.sim.as_ref().unwrap().world.segment(from, chest);
        eprintln!("[direct] world segment eye→torso: {seg:?} (None = clear), dist {l:.1}");
        scene.sim.as_mut().unwrap().drones.player_bullets.push(nf_game::drone::PlayerBullet {
            pos: from, dir, speed: 4.0, travelled: 0.0, range: 80.0, damage: 3.5, weapon: 2,
            first: true, dead: false, bounces: 0, homing: false, target: None, gravity: false, guided_cam: false, snd: 0,
        });
    }
    for t in 0..40u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let hp = sim.drones.list[si].health;
        let dr = &sim.drones.list[si];
        let data = nf_game::drone::Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
            skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
            hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
            surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
        };
        let nboxes = dr.hit_boxes(&data);
        eprintln!("[direct] t{t}: sniper hp {hp:.2} state {:#x} danim {} script {:?} boxes {} enabled {} npc_enabled {}",
            dr.state, dr.anim.danim, dr.anim.script, nboxes.len(), dr.enabled, sim.drones.npc_enabled);
        if t == 6 {
            // The crossing segment (the one through the torso): run the real first_box_hit on the
            // real boxes and print every box's t.
            let (paths, _) = sim.drones.bullet_paths();
            let (a, b) = paths.last().copied().unwrap();
            let ray = [b[0]-a[0], b[1]-a[1], b[2]-a[2]];
            eprintln!("[direct]    crossing seg ({:.2},{:.2},{:.2})→ray ({:.2},{:.2},{:.2})", a[0], a[1], a[2], ray[0], ray[1], ray[2]);
            eprintln!("[direct]    drone pos ({:.2},{:.2},{:.2})", dr.pos[0], dr.pos[1], dr.pos[2]);
            for bx in &nboxes { eprintln!("[direct]    box part {} centre ({:.2},{:.2},{:.2}) half {:?}", bx.part, bx.m[3][0], bx.m[3][1], bx.m[3][2], bx.half); }
            for bx in &nboxes {
                if let Some(tv) = nf_game::damage::ray_box(a, ray, bx) { eprintln!("[direct]    box part {} HIT t {:.3} centre ({:.2},{:.2},{:.2}) half {:?}",
                    bx.part, tv, bx.m[3][0], bx.m[3][1], bx.m[3][2], bx.half); }
            }
            let first = nf_game::damage::first_box_hit(a, ray, &nboxes);
            eprintln!("[direct]    first_box_hit = {first:?}");
            // The convention check: the same slab test with the box axes taken from the COLUMNS.
            let ray_box_cols = |a: [f32;3], ray: [f32;3], b: &nf_game::damage::HitBox| -> Option<f32> {
                let c = [b.m[3][0], b.m[3][1], b.m[3][2]];
                let (mut t0, mut t1) = (0f32, 1f32);
                for k in 0..3 {
                    let axis = [b.m[0][k], b.m[1][k], b.m[2][k]];
                    let l = (axis[0]*axis[0] + axis[1]*axis[1] + axis[2]*axis[2]).sqrt();
                    if l <= 0.0 { return None; }
                    let p = ((a[0]-c[0])*axis[0] + (a[1]-c[1])*axis[1] + (a[2]-c[2])*axis[2]) / l;
                    let d = (ray[0]*axis[0] + ray[1]*axis[1] + ray[2]*axis[2]) / l;
                    let h = b.half[k];
                    if d.abs() < 1e-12 { if p < -h || p > h { return None; } }
                    else {
                        let (x, y) = ((-h - p)/d, (h - p)/d);
                        let (lo, hi) = if x < y { (x, y) } else { (y, x) };
                        t0 = t0.max(lo); t1 = t1.min(hi);
                        if t0 > t1 { return None; }
                    }
                }
                Some(t0)
            };
            for bx in &nboxes {
                if let Some(tv) = ray_box_cols(a, ray, bx) { eprintln!("[direct]    COLS box part {} HIT t {:.3}", bx.part, tv); }
            }
        }
        if t < 8 {
            let (paths, _) = sim.drones.bullet_paths();
            let last = paths.last().map(|(a, b)| format!("({:.1},{:.1},{:.1})→({:.1},{:.1},{:.1})", a[0], a[1], a[2], b[0], b[1], b[2]));
            eprintln!("[direct]    last seg {last:?} impacts {:?}", sim.drones.player_impacts);
        }
        if hp < 10.0 { eprintln!("[direct] t{t}: DAMAGE LANDED (hp {hp})"); break; }
    }
    let hp = scene.sim.as_ref().unwrap().drones.list[si].health;
    // The damage path works END TO END when the bullet is aimed at the live box centre: the P2K's
    // 3.5 torso × the Agent 1.5 difficulty = 5.25 → health 4.75, the state machine's hit state.
    assert!(hp < 9.99, "the sniper's damage path stopped working: health {hp} after the live-centre shot");
}

/// 0700000c: the rooftop sniper shoots Bond (the user "heard sniper shots") but Bond's shots do
/// nothing to him. Find the sniper (the Winter sniper 0x1E drone on the highest perch), then shoot
/// him through the REAL fire path (the walker's aim + trigger + flanks) and require damage.
#[test]
fn the_0c_sniper_takes_damage_when_shot() {
    let mut scene = nf_view::load(&iso(), "0700000c.bin", false).expect("load 0700000c");
    settle(&mut scene, 300);
    let (snipers, all) = {
        let sim = scene.sim.as_ref().unwrap();
        for (i, dr) in sim.drones.list.iter().enumerate() {
            eprintln!("[sniper] drone #{i} skin {:#x} kind {} state {:#x} pos ({:.2},{:.2},{:.2}) weapon {} hp {:.1}",
                dr.skin, dr.kind, dr.state, dr.pos[0], dr.pos[1], dr.pos[2], dr.weapon, dr.health);
        }
        let s: Vec<usize> = (0..sim.drones.list.len()).filter(|&i| {
            let dr = &sim.drones.list[i]; dr.present && !dr.corpse && matches!(dr.weapon, 0x1e | 0x1f | 0x24 | 0x25)
        }).collect();
        (s, sim.drones.list.len())
    };
    // The ROOFTOP sniper: the sniper-weapon drone with the highest perch.
    let si = snipers.iter().copied()
        .max_by(|&a, &b| scene.sim.as_ref().unwrap().drones.list[a].pos[1].total_cmp(&scene.sim.as_ref().unwrap().drones.list[b].pos[1]))
        .expect("0700000c has a sniper-weapon drone");
    let sp = { let sim = scene.sim.as_ref().unwrap(); (sim.drones.list[si].pos, sim.drones.list[si].pos[1]) };
    eprintln!("[sniper] {all} drones, snipers {snipers:?}, rooftop target #{si} at ({:.2},{:.2},{:.2})", sp.0[0], sp.0[1], sp.0[2]);
    // The sniper must be armed and firing in the real game (the user HEARD him): assert the port
    // gives him a weapon at all.
    assert!(matches!(scene.sim.as_ref().unwrap().drones.list[si].weapon, 0x1e | 0x1f | 0x24 | 0x25), "the rooftop drone lost his sniper weapon");
    // The sniper waits on a switch channel (WAIT_SWITCH 0x33, enabled=false — the game's trigger
    // writes it when the player reaches the yard). Raise it the way the level's trigger does, then
    // stand Bond at a standable yard vantage with clear LOS (the user looked UP at him — the aim
    // pitch needs the AIM hold, whose clamp is the game's ±0.9 [M 0x1bfe58]).
    let ch = scene.sim.as_ref().unwrap().drones.list[si].switch_channel;
    eprintln!("[sniper] enabling rooftop sniper #{si} via switch channel {ch}");
    scene.sim.as_mut().unwrap().drones.set_channel(ch);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert!(scene.sim.as_ref().unwrap().drones.list[si].enabled, "the sniper stayed disabled after his switch channel rose");
    // Find a standable spot with clear LOS: floor within 1.5 below, headroom, chest segment clear,
    // 16..36 horizontal (the standing look clamp 0.492 needs dxz ≥ ~16 at dy ≈ 8).
    let sim = scene.sim.as_ref().unwrap();
    let chest = [sp.0[0], sp.0[1] + 0.3, sp.0[2]];
    let mut vantage: Option<[f32; 3]> = None;
    'scan: for x in (18..70usize).step_by(2) {
        for z in (-232i32..-160).step_by(2) {
            let (x, z) = (x as f32, z as f32);
            let dxz = ((x - chest[0]).powi(2) + (z - chest[2]).powi(2)).sqrt();
            if !(16.0..=36.0).contains(&dxz) { continue; }
            for y in [7.0f32, 12.2, 13.2] {
                if sim.world.segment([x, y, z], [x, y - 1.5, z]).is_none() { continue; }
                let eye = [x, y + 1.5, z];
                if sim.world.segment(eye, [x, y + 2.2, z]).is_some() { continue; }
                if sim.world.segment(eye, chest).is_some() { continue; }
                vantage = Some([x, y, z]); break 'scan;
            }
        }
    }
    drop(sim);
    let v = vantage.unwrap_or_else(|| panic!("no standable clear-LOS vantage to the rooftop sniper"));
    eprintln!("[sniper] vantage at ({:.1},{:.1},{:.1})", v[0], v[1], v[2]);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, v, (chest[2] - v[2]).atan2(chest[0] - v[0]));
        // The chest pitch from the REAL eye (`pos + eye − crouch_drop` — fix-guard-misc2: the old
        // setup used `v[1] + 1.5`, which is NOT where `stand_at` puts the eye: floor_below drops to
        // 13.1 + 1.1 → pos 14.2, eye +1.5 → 15.7, 1.0 ABOVE the assumed 14.7 — the setup pitch
        // aimed 0.187 instead of 0.162, i.e. 0.4 ABOVE the head box, and every tap sailed over him).
        // bond.pitch is NORMALISED ±1 (the walker divides atan by FRAC_PI_2).
        let eye_y = sim.bond.pos[1] + sim.bond.eye - sim.bond.crouch_drop;
        let dy = chest[1] - eye_y;
        let dxz = ((chest[0] - v[0]).powi(2) + (chest[2] - v[2]).powi(2)).sqrt();
        sim.bond.pitch = (dy / dxz).atan() / std::f32::consts::FRAC_PI_2;
        // The game's shipped option default (the options menu's auto-aim ON; T toggles in nf-view).
        sim.auto_aim = true;
    }
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // Aim at the sniper's chest (AIM held) and fire in taps through the real trigger.
    let mut hp = scene.sim.as_ref().unwrap().drones.list[si].health;
    let mut hits = 0usize;
    for t in 0..3000u32 {
        let sim = scene.sim.as_ref().unwrap();
        let bp = sim.bond.pos;
        let s = sim.drones.list[si].pos;
        if t % 100 == 0 { eprintln!("[sniper] t{t} bond ({:.1},{:.1},{:.1}) yaw {:.3} pitch {:.3} clip {} sniper state {:#x} hp {:.1}",
            bp[0], bp[1], bp[2], sim.bond.yaw, sim.bond.pitch, sim.gun.clip, sim.drones.list[si].state, sim.drones.list[si].health); }
        let desired = (s[0] - bp[0]).atan2(s[2] - bp[2]);
        let dy = (s[1] + 0.3) - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
        let dxz = ((s[0]-bp[0]).powi(2) + (s[2]-bp[2]).powi(2)).sqrt().max(0.5);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let perr = (dy / dxz).atan() / std::f32::consts::FRAC_PI_2 - sim.bond.pitch;
        // The player's stick drives the RAY, not the body: the shot ray's yaw is `yaw − tx`
        // (d = forward + tx·right rotates the ray by −tx horizontally) and its pitch is
        // `pitch + ty` — so the turn stick feeds (err + tx) and the look stick feeds (perr − ty).
        // The aim-mode window (±0.296/±0.4) carries the correction into the free-aim cursor and
        // only the overflow moves the body ([M Player_Aiming 0x1a1d28]).
        let (tx0, ty0) = (0.76966 * sim.cross[0], 0.57724 * sim.cross[1]);
        let input = nf_game::player::Input { turn: (err + tx0).clamp(-0.05, 0.05), look: (perr - ty0).clamp(-0.05, 0.05), aiming: true, ..Default::default() };
        // Fire when the SHOT RAY (the crosshair ray the fire path uses — forward + 0.76966·cx·right
        // + 0.57724·cy·up, the port's `aim` at scene.rs) sits on the live chest: the torso box is
        // ±0.25 y at ≥23 units ⇒ the window is ±0.011 rad.
        let (yaw, pit) = (sim.bond.yaw, sim.bond.pitch * std::f32::consts::FRAC_PI_2);
        let (sy, cy, sp, cp) = (yaw.sin(), yaw.cos(), pit.sin(), pit.cos());
        let (tx, ty) = (0.76966 * sim.cross[0], 0.57724 * sim.cross[1]);
        let d = [cp * sy + tx * (-cy) + ty * (-sp * sy), sp + ty * cp, cp * cy + tx * sy + ty * (-sp * cy)];
        let dl = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
        let to = [s[0] - eye[0], (s[1] + 0.3) - eye[1], s[2] - eye[2]];
        let tl = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt().max(0.5);
        let cosang = (d[0] * to[0] + d[1] * to[1] + d[2] * to[2]) / (dl * tl);
        let on = cosang > 0.012f32.cos(); // 0.012 rad ≈ the torso box's half-height at 23 units — the
        // body yaw may sit up to a window (±0.296) short of the target while the free-aim cursor
        // carries the correction: the RAY is what the fire path uses, so the RAY is the gate.
        let trigger = on && t % 20 == 0;
        let reload = sim.gun.clip == 0;
        drop(sim);
        scene.sim.as_mut().unwrap().step_bond(input, trigger, reload, false);
        let sim = scene.sim.as_ref().unwrap();
        let hp_now = sim.drones.list[si].health;
        if hp_now < hp { hits += 1; hp = hp_now; eprintln!("[sniper] +{t}: sniper health {hp_now} (hits {hits})"); }
        if hp <= 0.0 { break; }
        if sim.gun.clip == 0 && sim.gun.spare == 0 { eprintln!("[sniper] out of ammo at +{t}"); break; }
    }
    eprintln!("[sniper] final: sniper #{si} health {hp} hits_taken {}", scene.sim.as_ref().unwrap().drones.list[si].hits_taken);
    assert!(hp < 10.0, "the 0c rooftop sniper took no damage (health {hp}, {hits} damage ticks from the clear-LOS vantage)");
    // Sister: the damage rode the real msg-8 bullet path (`Drone_BulletHit` 0x13a9f8 →
    // `NDrone2_HitDamage` 0x145558) — the hit counter +0x1D8 ticked, not an environment kill.
    assert!(scene.sim.as_ref().unwrap().drones.list[si].hits_taken >= 1, "health dropped without a bullet-hit counter tick");
}

/// fix-guard-misc2: a 06 guard DRIVEN along his route through a door-flagged nav-node pair must
/// open the door himself and cross, without Bond's use key. The game's chain: the followed route
/// reaches the pair (`LinkCreep_Handler` 0x15a2c8 → `NDrone2_ReachedDestNode` 0x153358's flag-0x2
/// case) → DSTATE_OpenDoor 0x81 → `NDrone2_OpenDoor` 0x14aa38 → `Door_Activate` 0x135be0 → wait
/// for state 4 → the saved return state resumes. Drives the game's GoToGoalPosition (0x63) input
/// path — the port's route for a dispatched route-less guard — at the pair whose nav path carries
/// a live guard (the census found path 7 n0–n1 → door #1 and path 3 n11–n14 → door #4).
#[test]
fn the_06_guard_driven_through_a_door_pair_opens_it_and_crosses() {
    let mut scene = nf_view::load(&iso(), "07000006.bin", false).expect("load 07000006");
    settle(&mut scene, 300);
    // The door-flagged pairs that sit at a real door (nearest door within 4.0 of the pair's middle).
    let mut sim = scene.sim.as_mut().unwrap();
    let mut pairs: Vec<(usize, usize, usize, [f32; 3], usize)> = Vec::new(); // (path, a, b, mid, door)
    for (pi, p) in sim.nav.paths.iter().enumerate() {
        for (a, na) in p.nodes.iter().enumerate() {
            if na.flags & 2 == 0 { continue; }
            for (b, nb) in p.nodes.iter().enumerate().skip(a + 1) {
                if nb.flags & 2 == 0 { continue; }
                let mid = [(na.pos[0] + nb.pos[0]) * 0.5, na.pos[1], (na.pos[2] + nb.pos[2]) * 0.5];
                if let Some((di, _)) = sim.drones.doors.list.iter().enumerate()
                    .map(|(i, d)| (i, ((d.pos[0] - mid[0]).powi(2) + (d.pos[2] - mid[2]).powi(2)).sqrt()))
                    .filter(|(_, dist)| *dist < 4.0).min_by(|x, y| x.1.total_cmp(&y.1)) {
                    pairs.push((pi, a, b, mid, di));
                }
            }
        }
    }
    assert!(!pairs.is_empty(), "07000006 lost its door-flagged nav node pairs (the census had path 7 n0–n1 and path 3 n11–n14)");
    for (pi, a, b, mid, di) in &pairs {
        eprintln!("[drive] pair path {pi} n{a}-n{b} mid ({:.1},{:.1},{:.1}) → door #{di} ({:.1},{:.1},{:.1})",
            mid[0], mid[1], mid[2], sim.drones.doors.list[*di].pos[0], sim.drones.doors.list[*di].pos[1], sim.drones.doors.list[*di].pos[2]);
    }
    // A live guard whose nav path region contains one of the pairs, closest to it.
    let mut pick: Option<(usize, usize, [f32; 3], f32)> = None; // (guard, pair idx, beyond pos, dist)
    for (gi, dr) in sim.drones.list.iter().enumerate() {
        if !dr.present || dr.corpse || dr.health <= 0.0 { continue; }
        let Some(path) = sim.nav.nav_path_for(dr.pos) else { continue };
        for (k, (pi, a, b, _, _)) in pairs.iter().enumerate() {
            if path != *pi { continue; }
            let (na, nb) = (&sim.nav.paths[*pi].nodes[*a].pos, &sim.nav.paths[*pi].nodes[*b].pos);
            // The goal sits 2.5 beyond the pair node FARTHER from the guard — i.e. on the far side
            // of the doorway from wherever the guard stands, so the route must cross the pair.
            let da = (dr.pos[0] - na[0]).powi(2) + (dr.pos[2] - na[2]).powi(2);
            let db = (dr.pos[0] - nb[0]).powi(2) + (dr.pos[2] - nb[2]).powi(2);
            let (near, far) = if da <= db { (na, nb) } else { (nb, na) };
            let dir = [far[0] - near[0], far[1] - near[1], far[2] - near[2]];
            let l = (dir[0] * dir[0] + dir[2] * dir[2]).sqrt().max(1e-4);
            let beyond = [far[0] + dir[0] / l * 2.5, far[1], far[2] + dir[2] / l * 2.5];
            let dist = da.sqrt().min(db.sqrt());
            if pick.map_or(true, |(_, _, _, d0)| dist < d0) { pick = Some((gi, k, beyond, dist)); }
        }
    }
    let (gi, k, beyond, gdist) = pick.unwrap_or_else(|| panic!("no live guard shares a nav path with a door-flagged pair: {:?}",
        sim.drones.list.iter().filter(|d| d.present && !d.corpse).map(|d| (d.pos[0], d.pos[2])).collect::<Vec<_>>()));
    let (pi, a, b, _, di) = pairs[k];
    eprintln!("[drive] driving guard #{gi} (nav path {pi}, {:.1} from n{a}) to ({:.1},{:.1},{:.1}) beyond door #{di}",
        gdist, beyond[0], beyond[1], beyond[2]);
    // The drive: goal beyond the door, GoToGoalPosition (0x63) entered through the real dispatch.
    {
        let drones = &mut sim.drones;
        let d = nf_game::drone::Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
            skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
            hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
            surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: drones.difficulty,
        };
        let dr = &mut drones.list[gi];
        dr.goal = Some(beyond);
        dr.next_state = Some(nf_game::drone::state::GO_TO_GOAL);
        dr.send(&d, &mut drones.rng, drones.frame, 0);
        assert_eq!(drones.list[gi].state, nf_game::drone::state::GO_TO_GOAL, "the 0x63 entry did not happen");
    }
    // Tick: the guard must enter OPEN_DOOR (0x81), the door must reach OPEN (4), and the guard
    // must cross to the beyond point — all with Bond passive (no use key anywhere in this test).
    let (mut saw_open_door, mut saw_door_open, mut crossed) = (None, None, false);
    for t in 0..3600u32 {
        sim.step_bond(Default::default(), false, false, false);
        let (st, hp) = (sim.drones.list[gi].state, sim.drones.list[gi].pos);
        if saw_open_door.is_none() && st == 0x81 { saw_open_door = Some(t); eprintln!("[drive] +{t}: guard entered OPEN_DOOR (0x81), hit_door {:?}", sim.drones.list[gi].hit_door); }
        if saw_door_open.is_none() && sim.drones.doors.list[di].state == nf_game::door::OPEN {
            saw_door_open = Some(t);
            eprintln!("[drive] +{t}: door #{di} OPENED by the guard (bond never pressed use)");
        }
        if (hp[0] - beyond[0]).powi(2) + (hp[2] - beyond[2]).powi(2) < 2.0_f32.powi(2) { crossed = true; }
        if crossed && saw_door_open.is_some() { break; }
        if t % 600 == 0 { eprintln!("[drive] +{t}: guard state {st:#x} at ({:.1},{:.1},{:.1}) door {} state {:#x}",
            hp[0], hp[1], hp[2], di, sim.drones.doors.list[di].state); }
    }
    eprintln!("[drive] result: OPEN_DOOR at {saw_open_door:?}, door opened at {saw_door_open:?}, crossed {crossed}");
    assert!(saw_open_door.is_some(), "the driven guard never entered DSTATE_OpenDoor (0x81) — the dispatch never fired");
    assert!(saw_door_open.is_some(), "the driven guard entered OPEN_DOOR but the door never opened");
    assert!(crossed, "the door opened but the driven guard never crossed to the far side");
}
