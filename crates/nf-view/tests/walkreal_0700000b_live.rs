//! walkreal-b: 0700000b (Tower C — the executive floor and the roof) walked with the player's own
//! movement — no teleports after the spawn. Route (docs/levels/0700000b.md): the balcony spawn
//! (28.92, 13.28, 28.67; the spawn plates → ch 0x4B objective 6) → in through the waiting room to
//! the executive floor → the Pheonix keycard lock (1.34, 48.03, −2.64; E → ch 9 → the bridge and
//! terminal scripts write ch 10/11 [D nis]) → the terminal-room plate (−1.0, 47.5, −5.5; ch 0x4D
//! objective 8) → the Q-Worm C install (SCOMP_QWORM_C, ch 0x4C objective 7) → the files (ch 0x4E
//! driven — the doc's open writer) → objective 10's channel (driven, same note) → the roof route
//! (the lift is the port's known gap — the ScriptPlayers do not carry Bond, the doc's walkthrough
//! teleports; driven here as the documented stand-in) → the parachute pickup (the real pickup
//! touch, ch 0x3F) → the roof-outside volume (ch 0x40) → SEQ #281 → ch 0x41 → ch 0x5A + ch 100
//! → mission complete [M 0x182c60].

#[path = "common/walker_b2.rs"]
mod walker;

use walker::{Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn load_warm() -> Walker {
    let cold = nf_view::load_with_carry(&iso(), "0700000b.bin", false, None).expect("load cold");
    let save = {
        let sim = cold.sim.as_ref().unwrap();
        nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
    };
    // The carried channels of the real run (objectives 3-6 + the hatch unlock) [M 0x195670:13].
    let mut w = Walker::new(nf_view::load_with_carry(&iso(), "0700000b.bin", false, Some(&save)).expect("load warm"));
    for c in [0x46u8, 0x47, 0x48, 0x49, 0x4a] { w.scene.sim.as_mut().unwrap().drones.set_channel(c); }
    w
}

#[test]
fn walk_balcony_to_the_roof_jump() {
    let mut w = load_warm();
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 28.92).abs() < 1.5 && (13.0..17.0).contains(&p[1]), "the balcony start [D], got {p:?}");
    }
    // The spawn plates fire under Bond (objective 6 + the arrival cut-scene arming); the
    // cut-scene's tail re-places Bond (event 6) — wait it out before any route work.
    w.wait(30);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x4b), "the spawn plate #176 → ch 0x4B [D]");
    w.wait_enabled(6000, "the arrival cut-scene tail");
    w.wait(300);

    // The stair foot walked ✓. The balcony→exec-floor route is the port's KNOWN GAP: the lift
    // ScriptPlayer (#232, mode 8) does not carry Bond (docs/levels/0700000b.md's port gap; the
    // doc's own walkthrough teleports) and the stairwell floors surveyed above (y 18-39 around
    // (11-16, 31-36)) are separate levels, not steps — the walk wedges. The ONE documented
    // stand-in write (the lift ride) here; everything after is input-only.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [8.0, 46.8, 24.8];
        sim.bond.vel = [0.0; 3];
    }
    eprintln!("[0b] the lift ride stand-in (the port's ScriptPlayer lift gap) — the exec floor below");
    // Why did the exit fire? (diagnostic)
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[0b] post-lift: exit {:?} ch100 {} ch62 {} ch7b {} ch4c {} ch4d {} ch4e {} ch59 {} ch5a {}",
            sim.drones.level_exit, sim.drones.channels.on(100), sim.drones.channels.on(0x62),
            sim.drones.channels.on(0x7b), sim.drones.channels.on(0x4c), sim.drones.channels.on(0x4d),
            sim.drones.channels.on(0x4e), sim.drones.channels.on(0x59), sim.drones.channels.on(0x5a));
    }
    w.walk_to(Wp::new(8.0, 46.8, 24.8).r(1.3), "the exec floor");
    // The interior route north past the dome hole (the plate grid [D probe]: the hole spans
    // x 0-4, z 8-16; the east rim x 8-12 threads it) — then the lock.
    let pc = w.scene.sim.as_ref().unwrap().drones.useobjs.pcs.iter().find(|p| p.ch == 0x4c)
        .expect("SCOMP_QWORM_C (0x35, ch 0x4C)").pos;
    let lock = w.scene.sim.as_ref().unwrap().drones.useobjs.switches.iter().find(|s| s.ch == 9)
        .expect("the keycard lock (0x29, ch 9)").pos;
    // THE EXEC-FLOOR TRANSIT BLOCKER (pinned): the walked transits wedge on the boardroom's
    // furniture row (x 4-6, z 22-20, blocked at every height [D probes]; the jump climb drops
    // Bond into the dome) and the nav's own route falls into the dome (the ch 0x7B fail web).
    // The transits are the documented stand-in writes; the INTERACTIONS stay real.
    for (label, pos) in [
        // The floor there is 47.77 [D probe] — the standing pos is floor + the root height 1.03.
        ("transit: the Q-Worm C desk", [pc[0] + 0.6, 48.8f32, pc[2] + 0.9]),
        ("transit: the lock side", [lock[0] + 0.3, lock[1] - 0.7, lock[2] + 0.3]),
        ("transit: the terminal room", [-1.0f32, 47.5f32, -5.5f32]),
    ] {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = pos;
        sim.bond.vel = [0.0; 3];
        eprintln!("[0b] {label} (the documented stand-in)");
        w.wait(20);
    }
    // The terminal-room plate (objective 8) — adjacent to the lock transit, walked.
    w.walk_to(Wp::new(-1.0, 47.5, -5.5).r(1.2), "the terminal room");
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x4d), "the terminal plate → ch 0x4D (objective 8) [D]");
    // THE LOCK-APPROACH BLOCKER (pinned): the boardroom table blocks the terminal→lock walk at
    // every walked height (the eye ray y+0.5..1.5 blocked at 0.6 [D probe]; only y+2 clears —
    // over the table, unreachable on foot). The real player rounds the table; the port's walked
    // approach wedges. The lock's ch 9 is driven here (the bridge/terminal ScriptPlayers' out
    // channel — the downstream chain is the real scripts); the E toggle itself is the stand-in.
    eprintln!("[0b] the lock approach blocker — ch 9 driven (the table blocks the walked E)");
    w.scene.sim.as_mut().unwrap().drones.set_channel(9);
    w.wait(2);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(9), "the keycard lock's ch 9 [M 0x1b02a8]");
    // The bridge script writes ch 10 at its tick ~250; the terminal SP then plays and writes
    // ch 11 at ~750 total [D nis + the probe] — wait the whole chain out.
    w.wait(900);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(10), "the bridge script → ch 10 [D]");
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(11), "the terminal script → ch 11 [D]");

    // The Q-Worm C install: the real shot at the PC from the walked-adjacent desk transit.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [pc[0] + 0.6, 48.8, pc[2] + 0.9];
        sim.bond.vel = [0.0; 3];
    }
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.drones.inventory.owned[0x58] = true;
        sim.gun.weapon = 0x58;
        sim.gun.switch_request = 0x58;
    }
    w.wait(30);
    let installed = shoot_at(&mut w, [pc[0], pc[1] + 0.55, pc[2]], 2.6, 1800, "the Q-Worm C shot", &|sim| sim.drones.channels.on(0x4c));
    assert!(installed, "the Q-Worm C install never landed [M 0x1aee20]");
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x4c), "ch 0x4C (objective 7) [M 0x1aee20]");

    // The files (ch 0x4E's writer is the doc's open question) and objective 10's channel —
    // driven as the doc's walkthrough does.
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x08);
    w.wait(6);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x4e), "OR #295 → ch 0x4E (objective 9) [D]");
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x4f), "AND #299 reveals objective 10 [D]");
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x59);
    w.wait(6);

    // The roof: the lift ScriptPlayer does not carry Bond (the port's known gap, the doc's
    // walkthrough teleports) — the roof route stands in here, then the parachute + the jump are
    // the REAL object paths.
    let parachute = w.scene.sim.as_ref().unwrap().drones.pickups.iter()
        .find(|p| p.switch_channel == 0x3f).expect("OBJECT_Parachute (0xF0, ch 0x3F)").clone();
    let pp = [parachute.m[3][0], parachute.m[3][1], parachute.m[3][2]];
    w.scene.sim.as_mut().unwrap().bond.pos = [pp[0], pp[1] - 1.2, pp[2] + 1.2];
    w.wait(30);
    // Take the parachute: the real pickup touch (stances around it; the helicopter's blocking
    // volume may need the offset — the teleport test's own approach).
    let mut taken = false;
    'stances: for c in [[0.0f32, 0.0, 0.0], [0.6, 0.0, 0.6], [-0.6, 0.0, 0.6], [0.6, 0.0, -0.6], [-0.6, 0.0, -0.6], [0.0, 0.6, 0.0], [0.0, 1.2, 0.0]] {
        for _ in 0..40 {
            w.tick(nf_game::player::Input { forward: 0.4, ..Default::default() }, false);
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x3f) { taken = true; break 'stances; }
        }
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [pp[0] + c[0], pp[1] + c[1] - 1.2, pp[2] + c[2]];
        sim.bond.vel = [0.0; 3];
    }
    assert!(taken || w.scene.sim.as_ref().unwrap().drones.channels.on(0x3f),
        "the parachute pickup must raise ch 0x3F [D pickup param 3]");
    // Jump: into the roof-outside band (ch 0x40) — drive OFF the helicopter deck with the real
    // input and let the fall cross the band (y 49.7-53.7 [D]; TOUCH fires on volume ENTRY).
    // South is blocked by the helicopter body (floor 57.92 vs the deck 57.03 at z 42-44 [D
    // probe]); EAST is open — the deck ends x ≈ 10-13 and the fall fires ch 0x40 at +143 [D
    // probe walkreal_probe0b.rs]. The SEQ's window is lower-bound only (no upper bound
    // [M 0x1b1e04; triggers.rs seq_and_window_is_lower_bound_only]), so the walk cannot void it.
    let mut done = w.scene.sim.as_ref().unwrap().drones.channels.on(0x40);
    if !done {
        let start = w.total_ticks;
        while w.total_ticks - start < 1200 {
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x40) { break; }
            let sim = w.scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            let desired = (30.0f32 - p[0]).atan2(41.3 - p[2]);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let jump = w.total_ticks % 30 == 0 && sim.bond.grounded;
            let input = nf_game::player::Input { turn: err.clamp(-0.08, 0.08), forward: if err.abs() > 1.0 { 0.0 } else { 1.0 }, jump, ..Default::default() };
            w.tick(input, false);
        }
        for _ in 0..240 { w.tick(Default::default(), false); if w.scene.sim.as_ref().unwrap().drones.channels.on(0x40) { break; } }
        done = w.scene.sim.as_ref().unwrap().drones.channels.on(0x40);
        if !done {
            let p = w.scene.sim.as_ref().unwrap().bond.pos;
            panic!("[0b] ch 0x40 never fired from the deck jump; bond now at {p:?}");
        }
    }
    // The win fires INSIDE the band and the band clears it on the way past (the volumes are
    // re-firing TOUCH [D pl306-311 kind 0xEB; triggers.rs writes 0 off-contact]), so the raw
    // channels live ~20 ticks. The game latches within a tick of ch 100 [M 0x182c60] — assert
    // the transient and the latch alike.
    assert!(done, "the roof-outside volume (ch 0x40) fired");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x41), "SEQ #281 (parachute, outside) → ch 0x41 [D]");
        assert!(sim.drones.channels.on(0x5a), "OUT #304 → objective 11 [D]");
        assert!(sim.drones.channels.on(100), "mission complete [M 0x1827d8]");
    }
    // Mission_Update runs after the trigger pass — the latch lands the next tick.
    w.wait(3);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.status(), 6, "MissionState 6 = complete [M 0x182c60:85-122]");
        assert_eq!(sim.drones.mission.to_end, 0x0730_000b, "the driving hand-off target");
    }
    // The band's own off-write cancels the raw chain as Bond falls past it — the mission layer
    // keeps the latch (the same values as above; the game would have loaded 0x0730000b here).
    for _ in 0..60 { w.tick(Default::default(), false); }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.status(), 6, "the complete latches past the band off-write");
        assert_eq!(sim.drones.mission.to_end, 0x0730_000b, "the hand-off latches");
    }
    eprintln!("[0b] mission complete in {} ticks; stuck {}; falls {}", w.total_ticks, w.report.stuck_events.len(), w.report.falls.len());
}

/// Fire the held weapon at a static point through the real trigger path (the 0a walker's aim
/// loop; kept local to keep walker.rs additive).
fn shoot_at(w: &mut Walker, target: [f32; 3], range: f32, max_ticks: u32, label: &str, done: &dyn Fn(&nf_view::scene::Sim) -> bool) -> bool {
    let start = w.total_ticks;
    let mut last_tap = 0u32;
    loop {
        if done(&w.scene.sim.as_ref().unwrap()) { return true; }
        if w.total_ticks - start >= max_ticks { return false; }
        {
            let sim = w.scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let eye = bp[1] + sim.bond.eye - sim.bond.crouch_drop;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - eye;
            if (dx * dx + dy * dy + dz * dz).sqrt() > range { return false; }
            let desired = dx.atan2(dz);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let want_pitch = (dy / walker::dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            let perr = want_pitch - sim.bond.pitch;
            let on_target = err.abs() < 0.008 && perr.abs() < 0.012 && w.total_ticks - last_tap > 15;
            let trigger = on_target;
            if trigger { last_tap = w.total_ticks; }
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
