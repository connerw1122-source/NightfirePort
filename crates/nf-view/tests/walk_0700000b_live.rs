//! Headless NF_ISO walkthrough for 0700000b (Tower C — the executive floor and the roof) — the
//! real Sim path (`nf_view::load_with_carry` + `Sim::step_bond`). The chain
//! (docs/levels/0700000b.md): the arrival plates → ch 0x4B (objective 6); the Pheonix keycard
//! lock (0x29 Switch, ch 9) → the bridge script writes ch 10 → the terminal script writes ch 11
//! [D nis 0x06000037/0x06000035]; the Q-Worm gadget shot at SCOMP_QWORM_C (0x35, param 1 = ch
//! 0x4C) installs the virus [M PCQWorm_Update 0x1aee20]; the terminal-room plate → ch 0x4D
//! (objective 8); the files → ch 0x4E (OR mux #295); the parachute pickup (0xF0, param 3 = ch
//! 0x3F) then the roof-outside volume (ch 0x40) → SEQ #281 → ch 0x41 → OUT #304 → ch 0x5A +
//! ch 100 → mission complete → `LevelToEndTo(0x0700000b)` = **0x0730000b** [M 0x182c60:99-101].

use nf_game::player::RamSave;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_carry(iso: &str, level: &str, save: Option<&RamSave>) -> nf_view::scene::Scene {
    nf_view::load_with_carry(iso, level, false, save).expect("load_with_carry")
}

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

fn step_at(scene: &mut nf_view::scene::Scene, pos: [f32; 3], yaw: f32, use_key: bool) {
    step_look(scene, pos, yaw, 0.0, use_key)
}

fn step_look(scene: &mut nf_view::scene::Scene, pos: [f32; 3], yaw: f32, pitch: f32, use_key: bool) {
    if let Some(sim) = scene.sim.as_mut() {
        sim.bond.pos = pos;
        sim.bond.yaw = yaw;
        // pitch is in half-π units (`Bond::look` scales it [M player.rs]); take radians in.
        sim.bond.pitch = pitch / std::f32::consts::FRAC_PI_2;
        sim.step_bond(Default::default(), false, false, use_key);
    }
}

/// The warm save arriving from 0700000a: the office floor left ch 0x46 (the elevator), 0x47/0x48/
/// 0x49 (objectives 3–5) and 0x4A (the hatch unlock, objective 6's reveal) on — the carried
/// channel block of a real run ([M 0x195670:13]).
fn save_from_0a(iso: &str) -> RamSave {
    let mut scene = load_carry(iso, "0700000a.bin", None);
    let sim = scene.sim.as_mut().unwrap();
    for c in [0x46u8, 0x47, 0x48, 0x49, 0x4a] { sim.drones.set_channel(c); }
    let sim = scene.sim.as_ref().unwrap();
    RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
}

/// Smoke + arrival: the balcony start, 6 objective-6 fires on the spawn plate, the arrival
/// cut-scene plates raise ch 0x33, and 600 ticks run clean with no level end.
#[test]
fn arrival_and_smoke() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let save = save_from_0a(&iso);
    let mut scene = load_carry(&iso, "0700000b.bin", Some(&save));
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.level, 0x0700_000b);
        assert!((sim.bond.pos[0] - 28.92).abs() < 1.0 && (13.0..17.0).contains(&sim.bond.pos[1]),
            "Bond should stand at the balcony start #162, not {:?}", sim.bond.pos);
        assert_eq!(sim.drones.list.len(), 10, "10 drone placements [D survey]");
    }
    tick(&mut scene, 30);
    // The arrival plates #165/#247/#284 sit a few steps inside from the spawn point — walk in.
    step_at(&mut scene, [26.5, 14.5, 32.0], 0.0, false);
    tick(&mut scene, 5);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x4b), "the spawn plate #176 completes objective 6");
        assert!(sim.drones.channels.on(0x33), "the arrival plates #165/#247/#284 arm the intro script");
    }
    tick(&mut scene, 570);
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.level_exit.is_none(), "an idle run must not end the level (the mission needs objectives 7-11)");
    assert!(!sim.drones.channels.on(0x62), "no fail without a cause");
}

/// Bond with his EYE (pos + 0.7 [M player.rs EYE]) level with the lock, close enough that the use
/// sphere (r 1.0 at eye + look [M 0x1a5170]) covers it.
fn use_at_lock(scene: &mut nf_view::scene::Scene, lock: [f32; 3]) {
    step_look(scene, [lock[0] + 0.3, lock[1] - 0.7, lock[2] + 0.3], (-0.3f32).atan2(-0.3), 0.0, true);
}

/// The Q-Worm install through the real fire path: stand within the shot's 2-unit range
/// (weapon_data[0x58]+0x1C [D probe]) with the eye's ray on the PC, draw, fire.
fn fire_qworm_at(scene: &mut nf_view::scene::Scene, pc: [f32; 3]) {
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.inventory.owned[0x58] = true;
        sim.gun.weapon = 0x58;
        sim.gun.switch_request = 0x58;
    }
    // The eye (pos + 0.7 [M EYE]) must sit level with the PC, inside the shot's 2-unit range
    // (weapon_data[0x58]+0x1C [D probe]) with a clear line: try candidate stances until one holds
    // (the exec-office furniture shoves Bond off some of them).
    let candidates: [[f32; 3]; 8] = [
        [0.5, -0.7, 0.5], [-0.5, -0.7, 0.5], [0.5, -0.7, -0.5], [-0.5, -0.7, -0.5],
        [0.0, -0.7, 0.9], [0.0, -0.7, -0.9], [0.9, -0.7, 0.0], [-0.9, -0.7, 0.0],
    ];
    for c in candidates {
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond.pos = [pc[0] + c[0], pc[1] + c[1], pc[2] + c[2]];
            sim.bond.yaw = (-c[0]).atan2(-c[2]);
            sim.bond.pitch = 0.0;
        }
        tick(scene, 20);
        let (eye, ok) = {
            let sim = scene.sim.as_ref().unwrap();
            let eye = sim.bond.head();
            let d = ((eye[0] - pc[0]).powi(2) + (eye[1] - pc[1]).powi(2) + (eye[2] - pc[2]).powi(2)).sqrt();
            let clear = sim.world.segment(eye, pc).is_none();
            (eye, d < 1.9 && clear)
        };
        if ok {
            {
                let sim = scene.sim.as_mut().unwrap();
                sim.bond.yaw = (pc[0] - eye[0]).atan2(pc[2] - eye[2]);
                sim.bond.pitch = ((pc[1] - eye[1]) / (pc[0] - eye[0]).hypot(pc[2] - eye[2])).atan() / std::f32::consts::FRAC_PI_2;
            }
            for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false); }
            tick(scene, 90);
            return;
        }
    }
}

/// The keycard chain: E at the Pheonix keycard lock (0x29 Switch, ch 9, at (1.34, 48.03, −2.64))
/// toggles ch 9 [M Switch_Activate 0x1b02a8]; the bridge ScriptPlayer (out ch 9) plays and its
/// script writes ch 10 at tick 246, then the terminal ScriptPlayer (out ch 10) writes ch 11 at
/// tick 485 [D nis 0x06000037/0x06000035].
#[test]
fn keycard_lock_chain() {
    let Some(iso) = iso() else { return };
    let save = save_from_0a(&iso);
    let mut scene = load_carry(&iso, "0700000b.bin", Some(&save));
    let lock = scene.sim.as_ref().unwrap().drones.useobjs.switches.iter().find(|s| s.ch == 9)
        .expect("the Pheonix keycard lock switch (0x29, ch 9) exists").pos;
    // Bond on the executive floor looking down at the lock (its centre is below the eye).
    use_at_lock(&mut scene, lock);
    tick(&mut scene, 5);
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(9), "the keycard lock toggles ch 9 [M 0x1b02a8]");
    // The bridge script writes ch 10 at its tick 246; the terminal script ch 11 at 485.
    tick(&mut scene, 300);
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(10), "the bridge script wrote ch 10 [D 0x06000037]");
    tick(&mut scene, 500);
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(11), "the terminal script wrote ch 11 [D 0x06000035]");
    // Sister: E away from the lock leaves ch 9 off.
    let mut scene2 = load_carry(&iso, "0700000b.bin", Some(&save));
    step_at(&mut scene2, [lock[0], lock[1] - 0.5, lock[2] + 1.0], 0.0, false);
    tick(&mut scene2, 5);
    assert!(!scene2.sim.as_ref().unwrap().drones.channels.on(9), "no E → no toggle");
}

/// The Q-Worm C install: Bond fires the Q-Worm gadget (weapon 0x58) at SCOMP_QWORM_C (0x35,
/// param 1 = ch 0x4C) — the install writes ch 0x4C (objective 7) [M PCQWorm_Update 0x1aee20].
#[test]
fn qworm_c_install() {
    let Some(iso) = iso() else { return };
    let save = save_from_0a(&iso);
    let mut scene = load_carry(&iso, "0700000b.bin", Some(&save));
    let pc = scene.sim.as_ref().unwrap().drones.useobjs.pcs.iter().find(|p| p.ch == 0x4c)
        .expect("SCOMP_QWORM_C (0x35, param 1 = ch 0x4C) exists").pos;
    fire_qworm_at(&mut scene, pc);
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(0x4c), "the Q-Worm C install completed objective 7");
}

/// The terminal room + the files + the ending: the terminal plate → ch 0x4D (objective 8); the
/// files' channel (driven where the doc records the open writer) → OR #295 → ch 0x4E (objective
/// 9); ch 0x4F = AND(0x4C, 0x4E) reveals objective 10; objective 10's channel (driven, same
/// note); the parachute pickup taken (the real pickup path, ch 0x3F) then the roof-outside
/// volume (ch 0x40) within the SEQ's 4-tick slack → ch 0x41 → ch 0x5A + ch 100 → the mission
/// ends to the driving level 0x0730000b [M 0x182c60:99-101]. Sisters: without the parachute the
/// jump path cannot complete (SEQ #281 needs ch 0x3F first), and a fail (ch 0x62) ends to
/// 0x07000048 instead.
#[test]
fn files_parachute_and_mission_complete() {
    let Some(iso) = iso() else { return };
    let save = save_from_0a(&iso);
    let mut scene = load_carry(&iso, "0700000b.bin", Some(&save));
    // The arrival: the spawn plate #176 needs Bond to stand in it once (the real game's first
    // moments) before the walkthrough teleports between rooms.
    tick(&mut scene, 10);
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(0x4b), "objective 6 fires on arrival");
    // Objective 8: the Secure Terminal Room plate #163.
    step_at(&mut scene, [-1.0, 47.5, -5.5], 0.0, false);
    tick(&mut scene, 5);
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(0x4d), "the terminal-room plate completes objective 8");
    // Objective 9: the files channel (docs/levels/0700000b.md: the writer is the level's open
    // question — the test drives it at the same point the game's terminal sequence would).
    // Objective 7 first: the Q-Worm C install (the AND #299 needs both channels).
    let pc = scene.sim.as_ref().unwrap().drones.useobjs.pcs.iter().find(|p| p.ch == 0x4c)
        .expect("SCOMP_QWORM_C exists").pos;
    fire_qworm_at(&mut scene, pc);
    tick(&mut scene, 5);
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(0x4c), "objective 7 via the Q-Worm C");
    scene.sim.as_mut().unwrap().drones.set_channel(0x08);
    tick(&mut scene, 5);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x4e), "OR #295: ch 0x08 completes objective 9");
        assert!(sim.drones.channels.on(0x4f), "AND #299: objectives 7+9 done reveal objective 10");
    }
    // Objective 10 (same open-writer note): Escape to the roof.
    scene.sim.as_mut().unwrap().drones.set_channel(0x59);
    tick(&mut scene, 5);
    // Objective 11: the parachute pickup (2.53, 58.45, 42.34) — the real pickup path writes ch
    // 0x3F on the take [M 0x18e650:330-333] — then the roof-outside volume within the SEQ slack.
    let parachute = scene.sim.as_ref().unwrap().drones.pickups.iter()
        .find(|p| p.switch_channel == 0x3f).expect("OBJECT_Parachute (0xF0, param 3 = ch 0x3F) exists");
    let pp = parachute.m[3];
    drop(parachute);
    // Take the parachute: try stances around it until the touch lands (the helicopter's blocking
    // poly shoves/refuses some of them).
    for c in [[0.0f32, -0.4, 0.0], [0.8, -0.4, 0.0], [0.0, -0.4, 0.8], [0.8, -1.2, 0.8], [-0.8, -0.4, 0.0], [0.0, -1.4, 0.0]] {
        step_at(&mut scene, [pp[0] + c[0], pp[1] + c[1], pp[2] + c[2]], 0.0, false);
        tick(&mut scene, 3);
        if scene.sim.as_ref().unwrap().drones.channels.on(0x3f) { break; }
    }
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(0x3f), "taking the parachute raised ch 0x3F");
    // Jump: into the roof-outside band (ch 0x40) within the SEQ's 4-tick slack [M 0x1b1e04].
    step_at(&mut scene, [2.5, 52.9, 42.0], 0.0, false);
    tick(&mut scene, 30);
    {
        let sim = scene.sim.as_ref().unwrap();
        if std::env::var_os("NF_QWORMDBG").is_some() {
            for c in [0x46u8, 0x47, 0x48, 0x49, 0x4b, 0x4c, 0x4d, 0x4e, 0x59, 0x5a, 0x61, 100] {
                eprintln!("[dbg] ch {c:#04x} = {}", sim.drones.channels.on(c as u32));
            }
        }
        assert!(sim.drones.channels.on(0x40), "the roof-outside volume fired");
        assert!(sim.drones.channels.on(0x41), "SEQ #281 (parachute, outside) → ch 0x41");
        assert!(sim.drones.channels.on(0x5a), "OUT #304: objective 11 complete");
        assert!(sim.drones.channels.on(100), "the mission-complete channel [M 0x1827d8:155-160]");
        // The mission layer: MissionState 6 (complete) and the hand-off target — 0x0730000b is a
        // DRIVING level the on-foot port cannot load, so the banner ends with the decision (the
        // same skip the port logs [M 0x182c60:99-101; the LevelEnd drain]).
        assert_eq!(sim.drones.mission.status(), 6, "MissionState 6 = complete [M 0x182c60:85-122]");
        assert_eq!(sim.drones.mission.to_end, 0x0730_000b, "the driving hand-off target");
        assert_eq!(sim.drones.mission.states.iter().flatten().filter(|&&s| s == 4).count(), 11,
            "all 11 tower objectives complete");
    }
    // Sister 1: no parachute → no complete (the SEQ's first input never fires).
    let mut scene2 = load_carry(&iso, "0700000b.bin", Some(&save));
    let sim2 = scene2.sim.as_mut().unwrap();
    sim2.drones.set_channel(0x59);
    sim2.bond.pos = [2.5, 52.9, 42.0];
    drop(sim2);
    tick(&mut scene2, 10);
    let sim2 = scene2.sim.as_ref().unwrap();
    assert!(sim2.drones.channels.on(0x40) && !sim2.drones.channels.on(0x41),
        "outside without the parachute: ch 0x40 but the SEQ stays off");
    assert_ne!(sim2.drones.level_exit, Some(0x0730_000b));
    assert_ne!(sim2.drones.mission.status(), 6, "without the parachute the mission stays open");
    // Sister 2: the fall-fail web — ch 0x7B (the outside-below chain) starts the death
    // cut-scene SP #325 (0x0600098A [D nis]). The death→mission-fail hand-off is the game's
    // Player_CheckForDeath path (Mission_Update's ch-0x62 arm, 0x182c60:21-43) — not ported
    // (031-fix-mission's file; the doc records it), so this sister asserts the ported half:
    // the scene runs and the level does NOT silently complete.
    let mut scene3 = load_carry(&iso, "0700000b.bin", Some(&save));
    scene3.sim.as_mut().unwrap().drones.set_channel(0x7b);
    tick(&mut scene3, 20);
    let scene3ref = scene3.sim.as_ref().unwrap();
    let played = scene3ref.drones.scripts.players.iter()
        .any(|p| p.out == 0x7b && p.state >= 2 && !p.deleted);
    assert!(played, "the fall cut-scene SP #325 runs on ch 0x7B");
    assert_ne!(scene3ref.drones.level_exit, Some(0x0730_000b), "the fall never completes the mission");
}
