//! G-035 remainder (`fix-doors-remainder`): the use pick against the door's collision mesh
//! (`Player_Activate` 0x1a5170), the door labels 0x2000003 / 0x200000b (`Door_Update` 0x1363c8 /
//! `Check_Target` 0x1a7810 + `Door_DoMsg` 0x135f18), and the `Collide_Update` proxy path of
//! `Door_ReactToHit` (0x1ea398 → 0x135980). Opt-in: NF_ISO.

#[path = "common/walker.rs"]
mod walker;
use walker::Walker;

use nf_game::door;
use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn norm_h(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[2] * v[2]).sqrt();
    if l < 1e-4 { [0.0, 0.0, 0.0] } else { [v[0] / l, 0.0, v[2] / l] }
}

/// Teleport Bond to `pos`, aim his yaw/pitch at `target`, settle, then pulse E one tick per 6.
fn aim_and_use(w: &mut Walker, pos: [f32; 3], target: [f32; 3], ticks: u32) {
    let sim = w.scene.sim.as_mut().unwrap();
    sim.bond.pos = pos;
    for _ in 0..30 { w.tick(Input::default(), false); } // settle on the floor
    for k in 0..ticks {
        let sim = w.scene.sim.as_ref().unwrap();
        let desired = (target[0] - sim.bond.pos[0]).atan2(target[2] - sim.bond.pos[2]);
        let err = desired - sim.bond.yaw;
        let err = if err > std::f32::consts::PI { err - 2.0 * std::f32::consts::PI } else if err < -std::f32::consts::PI { err + 2.0 * std::f32::consts::PI } else { err };
        let eye = [sim.bond.pos[0], sim.bond.pos[1] + sim.bond.eye - sim.bond.crouch_drop, sim.bond.pos[2]];
        let to = sub(target, eye);
        let want_pitch = (to[1] / to[0].hypot(to[2]).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
        let perr = want_pitch - sim.bond.pitch;
        w.tick(Input { turn: err.clamp(-0.08, 0.08), look: perr.clamp(-0.06, 0.06), ..Default::default() }, k % 6 == 0);
    }
}

/// The first door the mesh pick can act on: a loaded shape, unlocked, no approach gate (07000001's doors are all kind-2 slides).
fn pickable_door(w: &Walker) -> Option<usize> {
    let sim = w.scene.sim.as_ref().unwrap();
    sim.drones.doors.list.iter().position(|d| {
        d.glist != 0 && d.ch_lock == 0 && d.approach == 0
            && sim.world.has_shape(d.glist)
    })
}

/// (a) The use pick is the game's mesh pick: one unit ahead of the head, radius 1, against the
/// door's leaf — a press 2.3 units out on the leaf normal reaches 1.3 from the plane and must NOT
/// open (the pre-fix stand-in opened everything inside 2.5 of the placement); at 1.2 it must.
#[test]
fn the_use_pick_respects_the_leaf_mesh() {
    if std::env::var_os("NF_ISO").is_none() { eprintln!("skip: NF_ISO not set"); return; }
    let mut w = Walker::new(nf_view::load(&iso(), "07000001.bin", false).unwrap());
    w.wait_enabled(3000, "level start");
    w.wait(30);
    let di = pickable_door(&w).expect("07000001 has a shape-backed door");
    let (centre, nrm) = {
        let sim = w.scene.sim.as_ref().unwrap();
        let d = &sim.drones.doors.list[di];
        (d.sph.0, [d.matrix[0][0], d.matrix[0][1], d.matrix[0][2]])
    };
    let n = norm_h(nrm);
    let pos = centre;

    // 2.3 out: the sphere (r 1 at head+1) stays 0.3 short of the leaf. The old stand-in (2.5
    // placement distance) opened the door here — the assertion that fails pre-fix.
    aim_and_use(&mut w, [pos[0] + n[0] * 2.3, pos[1], pos[2] + n[2] * 2.3], centre, 30);
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.doors.list[di].state, door::CLOSED,
        "E at 2.3 from the leaf must not reach it (the 1.0-radius sphere at head+1 falls 0.3 short)");
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.doors.list[di].pos01, 0.0, "the leaf never moved");

    // Sister: 1.2 out — the use sphere overlaps the leaf by 0.8; the door opens.
    aim_and_use(&mut w, [pos[0] + n[0] * 1.2, pos[1], pos[2] + n[2] * 1.2], centre, 30);
    let st = w.scene.sim.as_ref().unwrap().drones.doors.list[di].state;
    assert!(st & 0xc != 0, "E at 1.2 opens the door (state {st:#x})");
}

/// (b) The door labels: the aim hint 0x200000b once per unlocked door (latched), never for a
/// locked one, and the unlock note 0x2000003 on the lock channel's rising edge.
#[test]
fn door_labels_fire_per_the_decompile_rules() {
    if std::env::var_os("NF_ISO").is_none() { eprintln!("skip: NF_ISO not set"); return; }
    let mut w = Walker::new(nf_view::load(&iso(), "07000001.bin", false).unwrap());
    w.wait_enabled(3000, "level start");
    w.wait(30);
    let di = pickable_door(&w).expect("a shape-backed door");
    let (centre, nrm) = {
        let sim = w.scene.sim.as_ref().unwrap();
        let d = &sim.drones.doors.list[di];
        (d.sph.0, [d.matrix[0][0], d.matrix[0][1], d.matrix[0][2]])
    };
    let n = norm_h(nrm);
    let pos = centre;
    let hint = w.scene.sim.as_ref().unwrap().lang.as_ref().unwrap().bind(0x2000_000b).unwrap_or("").trim().to_string();
    assert!(!hint.is_empty(), "0x200000b binds in USATxt.dat");

    // Aim at the door from 2.0: the ray (head + 50·look) meets the leaf at ~1.0 < 5.0 → the hint.
    aim_and_use(&mut w, [pos[0] + n[0] * 2.0, pos[1], pos[2] + n[2] * 2.0], centre, 20);
    assert!(w.scene.sim.as_ref().unwrap().hud.msgs.iter().any(|m| m.text == hint),
        "aiming at an unlocked door shows the {hint:?} hint");
    let seen = w.scene.sim.as_ref().unwrap().hud.msgs.iter().filter(|m| m.text == hint).count();
    aim_and_use(&mut w, [pos[0] + n[0] * 2.0, pos[1], pos[2] + n[2] * 2.0], centre, 20);
    let now = w.scene.sim.as_ref().unwrap().hud.msgs.iter().filter(|m| m.text == hint).count();
    assert_eq!(now, seen, "Door_DoMsg latches per door (DI+0x74) — no second hint");

    // A locked door gives NO hint (Door_DoMsg: the lock channel blocks). Lock it on channel 200
    // (unused by this level's scripts) — another door, this one is open by now.
    let dj = {
        let sim = w.scene.sim.as_ref().unwrap();
        (0..sim.drones.doors.list.len())
            .position(|j| j != di && sim.drones.doors.list[j].glist != 0
                                && sim.drones.doors.list[j].ch_lock == 0
                && sim.world.has_shape(sim.drones.doors.list[j].glist))
    }.expect("a second shape-backed door");
    let (pos2, nrm2) = {
        let sim = w.scene.sim.as_ref().unwrap();
        let d = &sim.drones.doors.list[dj];
        (d.pos, [d.matrix[0][0], d.matrix[0][1], d.matrix[0][2]])
    };
    let n2 = norm_h(nrm2);
    let centre2 = [pos2[0], pos2[1] + 1.0, pos2[2]];
    w.scene.sim.as_mut().unwrap().drones.doors.list[dj].ch_lock = 200;
    let msgs_before = w.scene.sim.as_ref().unwrap().hud.msgs.iter().filter(|m| m.text == hint).count();
    aim_and_use(&mut w, [pos2[0] + n2[0] * 2.0, pos2[1], pos2[2] + n2[2] * 2.0], centre2, 20);
    let msgs_after = w.scene.sim.as_ref().unwrap().hud.msgs.iter().filter(|m| m.text == hint).count();
    assert_eq!(msgs_after, msgs_before, "a locked door shows no aim hint");

    // The lock channel's rising edge unlocks for good and shows 0x2000003 (`Door_Update`).
    let note = w.scene.sim.as_ref().unwrap().lang.as_ref().unwrap().bind(0x2000_0003).unwrap_or("").trim().to_string();
    assert!(!note.is_empty(), "0x2000003 binds in USATxt.dat");
    w.scene.sim.as_mut().unwrap().drones.channels.v[200] = 1;
    for _ in 0..10 { w.tick(Input::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.hud.msgs.iter().any(|m| m.text == note), "the unlock edge shows 0x2000003");
    assert!(sim.drones.doors.list[dj].state & 0xc != 0, "the unlocked door self-activates");
    assert_eq!(sim.drones.doors.list[dj].ch_lock, 0, "the door clears its lock channel");
}

/// (c) The proxy path: a closing door that meets Bond reverses (the anti-crush) — `Collide_Update`
/// post-walk fills the records, `Door_ReactToHit` (0x135980) consumes them the same frame.
#[test]
fn a_closing_door_reverses_off_bond() {
    if std::env::var_os("NF_ISO").is_none() { eprintln!("skip: NF_ISO not set"); return; }
    let mut w = Walker::new(nf_view::load(&iso(), "07000001.bin", false).unwrap());
    w.wait_enabled(3000, "level start");
    w.wait(30);
    let di = pickable_door(&w).expect("a shape-backed door");
    let (centre, nrm) = {
        let sim = w.scene.sim.as_ref().unwrap();
        let d = &sim.drones.doors.list[di];
        (d.sph.0, [d.matrix[0][0], d.matrix[0][1], d.matrix[0][2]])
    };
    let n = norm_h(nrm);
    let pos = centre;

    // Open it, then step back out of the blocking zone so the 360-tick auto-close fires.
    aim_and_use(&mut w, [pos[0] + n[0] * 1.2, pos[1], pos[2] + n[2] * 1.2], centre, 30);
    let opened = { let s = w.scene.sim.as_ref().unwrap().drones.doors.list[di].state; s & 0xc != 0 };
    assert!(opened, "the door opened for the close test");
    for _ in 0..380 { w.tick(Input::default(), false); }
    let st = w.scene.sim.as_ref().unwrap().drones.doors.list[di].state;
    assert!(st & 0x12 != 0, "the auto-close ran (state {st:#x})");

    // Step into the doorway DURING the closing sweep: the leaf meets Bond's sphere and the
    // proxy path reverses the door to OPENING. Pre-fix the door closed through him (state 2).
    let mut reversed = false;
    for k in 0..60u32 {
        let sim = w.scene.sim.as_mut().unwrap();
        if k == 3 { sim.bond.pos = [pos[0] + n[0] * 0.5, sim.bond.pos[1], pos[2] + n[2] * 0.5]; }
        let _ = sim;
        w.tick(Input::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.doors.list[di].state & 0x8 != 0 { reversed = true; break; }
    }
    assert!(reversed, "the closing leaf met Bond and Door_ReactToHit reopened the door");
}

/// The census: every shape-backed, unlocked, approach-open door on every level bin opens to E at
/// its approach (the game's pick). Failures are listed with their door and level for the report.
#[test]
fn census_every_door_opens_at_its_approach() {
    if std::env::var_os("NF_ISO").is_none() { eprintln!("skip: NF_ISO not set"); return; }
    let mut total = 0u32;
    let mut opened = 0u32;
    let mut fails: Vec<String> = Vec::new();
    for code in 1..=0x1b {
        let name = format!("0700{code:04}.bin");
        let Ok(scene) = nf_view::load(&iso(), &name, false) else { continue };
        let mut w = Walker::new(scene);
        w.wait_enabled(1500, &name);
        w.wait(20);
        let doors: Vec<(usize, [f32; 3], [f32; 3])> = {
            let sim = w.scene.sim.as_ref().unwrap();
            sim.drones.doors.list.iter().enumerate()
                .filter(|(_, d)| d.glist != 0 && sim.world.has_shape(d.glist) && d.ch_lock == 0 && d.locked_flag == 0 && d.approach == 0)
                .map(|(i, d)| (i, d.pos, [d.matrix[0][0], d.matrix[0][1], d.matrix[0][2]]))
                .collect()
        };
        for (i, pos, nrm) in doors {
            total += 1;
            let n = norm_h(nrm);
            let centre = [pos[0], pos[1] + 1.0, pos[2]];
            aim_and_use(&mut w, [pos[0] + n[0] * 1.2, pos[1], pos[2] + n[2] * 1.2], centre, 24);
            let st = w.scene.sim.as_ref().unwrap().drones.doors.list[i].state;
            if st & 0xc != 0 { opened += 1; } else { fails.push(format!("{name} door {i} at {:.1},{:.1},{:.1} state {st:#x}", pos[0], pos[1], pos[2])); }
            // Reset the door so the next candidate starts clean (close it the same way).
            aim_and_use(&mut w, [pos[0] + n[0] * 1.2, pos[1], pos[2] + n[2] * 1.2], centre, 12);
        }
        eprintln!("[census] {name}: {} doors so far, {} opened", total, opened);
    }
    eprintln!("[census] TOTAL {total} doors, {opened} opened, {} failed: {fails:#?}", fails.len());
    assert!(fails.is_empty(), "doors that would not open at their approach: {fails:#?}");
}
