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
    assert!(!opened.is_empty(), "no door opened without Bond's use key — the guards never open their route's doors");
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
            first: true, dead: false, bounces: 0, homing: false, target: None, guided_cam: false,
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
        // The chest pitch, set once (the stand_at setup) — the look loop only holds it.
        // bond.pitch is NORMALISED ±1 (the walker divides atan by FRAC_PI_2).
        let dy = chest[1] - (v[1] + 1.5);
        let dxz = ((chest[0] - v[0]).powi(2) + (chest[2] - v[2]).powi(2)).sqrt();
        sim.bond.pitch = (dy / dxz).atan() / std::f32::consts::FRAC_PI_2;
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
        let input = nf_game::player::Input { turn: err.clamp(-0.05, 0.05), aiming: true, ..Default::default() };
        let on = err.abs() < 0.03 && perr.abs() < 0.02; // pitch is setup-fixed; ±0.02 ≈ ±0.03 rad
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
}
