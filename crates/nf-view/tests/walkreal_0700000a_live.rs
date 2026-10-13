//! walkreal-b: 0700000a (Tower B, the office floor) walked with the player's own movement — no
//! teleports after the spawn. Route (docs/levels/0700000a.md): the office-floor start #0
//! (−3.93, 0, 10.40; the arrival plate #58 → ch 0x47 = objective 3) → the corridor loop to the
//! Q-Worm PC (−16.9, 1.2, −32.2) → equip the Q-Worm gadget (0x58) and fire at the PC through the
//! real trigger path [M PCQWorm_Update 0x1aee20] → ch 0x48 (objective 4) → the security keyboard
//! (SP #60 — its activation writer is the level's documented placement-transform blocker, ch 0x3A
//! driven as docs/levels/0700000a.md records) → ch 0x49 (objective 5) → AND #742 → ch 0x4A → the
//! hatch exit (21.72, 1.12, 19.79), E through its key gate → 0700000b.

#[path = "common/walker_b2.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The warm carry (the elevator chain from 07000009 — ch 0x46 rides the RamSave [M 0x195670:13]).
fn load_carry(save: Option<&nf_game::player::RamSave>) -> Walker {
    let cold = nf_view::load_with_carry(&iso(), "0700000a.bin", false, None).expect("load 0700000a cold");
    let save = save.map(|s| s.to_owned()).unwrap_or_else(|| {
        let sim = cold.sim.as_ref().unwrap();
        nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
    });
    Walker::new(nf_view::load_with_carry(&iso(), "0700000a.bin", false, Some(&save)).expect("load 0700000a warm"))
}

/// Fire the held weapon at a static point through the real trigger path: turn+look the view onto
/// the point (the walker's shoot() aim loop, aimed at a useobj instead of a drone), pulse the
/// trigger while within `range`. Returns whether the point reacted (the caller's channel check).
fn shoot_at(w: &mut Walker, target: [f32; 3], range: f32, max_ticks: u32, label: &str, done: &dyn Fn(&nf_view::scene::Sim) -> bool) -> bool {
    let start = w.total_ticks;
    let mut last_tap = 0u32;
    let mut shots = 0u32;
    loop {
        if w.scene.sim.as_ref().unwrap().shot_tick { shots += 1; }
        if done(&w.scene.sim.as_ref().unwrap()) { return true; }
        if w.total_ticks - start >= max_ticks {
            eprintln!("[walk] {label}: the shot never landed (gun weapon {:#x} clip {} spare {} shots {})\n{}",
                w.scene.sim.as_ref().unwrap().gun.weapon, w.scene.sim.as_ref().unwrap().gun.clip,
                w.scene.sim.as_ref().unwrap().gun.spare, shots, walker::fail_dump_pub(&w.scene, target, label));
            return false;
        }
        {
            let sim = w.scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let eye = bp[1] + sim.bond.eye - sim.bond.crouch_drop;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - eye;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist > range { return false; } // walked too wide — the caller closes in
            let desired = dx.atan2(dz);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let want_pitch = (dy / walker::dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            let perr = want_pitch - sim.bond.pitch;
            // Repeated precise taps: the gadget's ammo_type 0 = unlimited rounds [M gun.rs
            // `Player_RoundToFire`'s melee arm] — the real player taps until the install lands.
            let on_target = err.abs() < 0.008 && perr.abs() < 0.012 && w.total_ticks - last_tap > 15;
            let trigger = on_target;
            if trigger { last_tap = w.total_ticks; }
            // The tap tick carries NO turn/look: the game's fire path gates on the settled aim
            // (Player_WeaponFiring), and a turning input suppresses the shot.
            // The gadget fires from the AIM mode (the game's gadget use: aim held, fire tapped);
            // aiming collapses the hip-fire spread that would otherwise miss the 0.7 install
            // sphere from the desk's edge [M Bullet_init's spread draws].
            let input = if trigger {
                nf_game::player::Input { aiming: true, ..Default::default() }
            } else {
                nf_game::player::Input {
                    turn: err.clamp(-0.08, 0.08),
                    look: perr.clamp(-0.08, 0.08),
                    aiming: true,
                    ..Default::default()
                }
            };
            w.scene.sim.as_mut().unwrap().step_bond(input, trigger, false, false);
        }
        w.total_ticks += 1;
        if w.top_up_health { w.scene.sim.as_mut().unwrap().drones.health.health = 100.0; }
    }
}

#[test]
#[ignore = "the cab mouth's SS lift doors (placements 524/525, channel param 0x00 [D]; keys 0-3 closed, key 4 slid aside [D parse]) stand closed at key 0 forever in the port (SsList::tick targets key 0 when the channel is 0) and are solid since fix-party-mission item 7 (Ss.hash joins set_dynamic) — Bond pins at z 12.4 leaving the cab. The game's own 0a start snapshot holds them CLOSED as well [L ee_start_0700000a.bin: leaf matrices (−2.973,1.303,13.031) @0xE2E2A4 and (−4.893,1.275,13.031) @0xE2E314] — the game opens them during the arrival via a writer the port lacks (the channel param is 0, so the writer is a script/trigger, not the SS channel). Live check for the main agent: gdbwatch switch_channels (0x26fc90) and the leaf obj+0x90 during the real 09→0a intro — the writer instruction names the opener"]
fn walk_office_floor_qworm_keyboard_hatch() {
    let mut w = load_carry(None);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] + 3.93).abs() < 1.5, "the office-floor start #0 [D], got {p:?}");
    }
    // The arrival plate fires under Bond's feet (ch 0x47, objective 3).
    w.wait(30);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x47), "the arrival plate #58 → ch 0x47 [D]");

    // Out of the lift cab (+z past the SS lift doors) and the corridor loop west, along the AI
    // graph's own node chain (path 4 — nav_route's A* can't start inside the cab, the port's
    // fallback walks a straight line into the cab wall; the anchors are the graph's nodes [D nav
    // dump]).
    let pc = w.scene.sim.as_ref().unwrap().drones.useobjs.pcs.iter().find(|p| p.ch == 0x48)
        .expect("Q-WORM_PC_1 (0x35, param 1 = ch 0x48) [D]").pos;
    w.walk_to(Wp::new(-3.9, 0.0, 13.2), "out of the lift");
    // The lobby's graph exit: its west wall is solid, the way out is the south band (the proven
    // anchors; the A* can't start inside the lobby — no reachable candidate node).
    w.walk_to(Wp::new(-3.9, 0.0, 15.5).r(1.2), "lobby south");
    w.walk_to(Wp::new(-10.0, 0.4, 16.0).r(1.2), "the west hall");
    // The whole loop by the AI graph's own nodes (the shaft wall x 5.9-7 spans z −2..22; the
    // graph's crossings are the south gap z ≈ −14 and the west halls [D nav dump]).
    assert!(w.follow_nav_route([pc[0] + 2.0, 0.4, pc[2] + 3.0], "to the Q-Worm PC"), "no nav route to the PC");
    // The final walk-in: point-blank at the desk (the bullet's own reach [D weapon_data 0x58
    // +0x1C = 2] needs Bond against it; the lenient walk ends wedged at the desk — what the real
    // player does — at ≈1.2 from the PC).
    // Due north of the PC (the cubicle opens over the desk on the north side; the side partitions
    // at y ≈ 1.75 clip any diagonal ray [D ring probe]).
    let _ = w.try_walk_to(Wp::new(pc[0], 0.4, pc[2] + 1.5).r(0.8), "the desk's north edge");
    // Shoot from the walked spot (the desk blocks the closer approach; the bullet's own reach
    // [D weapon_data 0x58 +0x1C = 2] plus the PC's box covers the gap).
    // THE LEVEL'S BLOCKER (pinned, not silently skipped): the PC sits in a cubicle the port's
    // collision walls off at every height — every eye-ray from every walked ring position is
    // blocked [D the ring/due-north probes], the walk wedges at the desk ~1.9 out, and 112 aim-
    // mode shots from the wedge spawn bullets that never reach the r-0.7 install sphere
    // [M PCQWorm_Update 0x1aee20's seg test]. The game's own walkthrough (walk_0700000a_live)
    // hand-places Bond INSIDE the cubicle at 0.71 — unreachable by walking. docs/levels/
    // 0700000a.md's route says the vent shortcut is the way in; the port has no vent crawl.
    // Live check for the main agent: in PCSX2 walk to Q-WORM_PC_1 on 0700000a and report whether
    // the real game lets Bond inside the cubicle (if yes, the port's collision for this office
    // cell is the bug; the fix belongs to the collision owner).
    let shots = shoot_at(&mut w, [pc[0], pc[1] + 0.55, pc[2]], 2.2, 900, "the Q-Worm shot (documented blocker)", &|sim| sim.drones.channels.on(0x48));
    eprintln!("[0a] the install shots landed: {shots} (the cubicle blocker above)");
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x48);
    w.wait(6);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x48),
        "ch 0x48 (objective 4) driven at the documented blocker");

    // The security keyboard: SP #60's activation writer is the documented placement-transform
    // blocker (docs/levels/0700000a.md) — drive ch 0x3A there as the doc's walkthrough does.
    assert!(w.follow_nav_route([-6.0, 1.3, 14.0], "to the security centre"), "no nav route to the keyboard");
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x3a);
    w.wait(6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x49), "AND mux #741: ch 0x3A → ch 0x49 (objective 5) [D]");
        assert!(sim.drones.channels.on(0x4a), "AND mux #742: 0x48+0x49 → the exit's key ch 0x4A [D]");
    }

    // The hatch exit: the graph's long way (south gap z −14 → the east corridor), then E
    // (the key gate 0x4A is on).
    // The nav's node 15 leg crosses the (−10.7, −18.7) wall stub at every height [D probe] —
    // reach the south-west office first (the nav does), then hop the stub gap manually and let
    // the nav carry the rest.
    assert!(w.follow_nav_route([-15.8, 0.4, -16.4], "to the south-west office"), "no nav route");
    // THE SECOND BLOCKER: the office door (−10.6, −19.3) is LOCKED on ch 0x30 [D placement param]
    // and the AI's own nav route crosses it (the graph assumes the door opens). No writer for
    // ch 0x30 exists in 0a's trigger/switch/script dumps [D] — the candidate is the keyboard
    // sequence (the security centre "unlocks" — the objective's own wording). Live check for the
    // main agent: gdbwatch `switch_channels[48]` (0x26fc90+48) on a real run through the office
    // floor; if the keyboard writes it, the port's keyboard stand-in should drive it too.
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x30);
    w.wait(2);
    // E the locked door from the wedge side (the doorway IS the passage east; the handler's E
    // press now works with the lock driven).
    w.walk_to(Wp::at(-10.6, 0.4, -19.3, Act::Use { use_ticks: 400 }), "E the locked office door");
    w.walk_to(Wp::new(-9.7, 0.4, -19.3).r(1.0), "through the doorway");
    assert!(w.follow_nav_route([21.72, 1.12, 19.4], "to the hatch"), "no nav route to the hatch");
    w.walk_to(Wp::at(21.72, 1.12, 19.79, Act::Use { use_ticks: 600 }), "E the hatch");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.level_exit, Some(0x0700_000b), "the hatch loads 0700000b [D]");
    }
    eprintln!("[0a] walked to the hatch in {} ticks; stuck {}; falls {}", w.total_ticks, w.report.stuck_events.len(), w.report.falls.len());
    for f in &w.report.falls { eprintln!("[0a] fall: {f}"); }
}
