//! Probes for fix-objective-drives: what the port's real writers do today on the four
//! campaign legs (09/0a/08/4a). Probe only — printed evidence for the runner drivers.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn wrap_pi(a: f32) -> f32 { a.rem_euclid(std::f32::consts::TAU + std::f32::consts::PI) - std::f32::consts::PI }

/// 0a: does the arrival NIS open the cab-mouth SS doors, can Bond walk out, and does the
/// security keyboard E-use fire SP #60 (out 0x3A)?
#[test]
fn probe_0a_doors_and_keyboard() {
    let iso = iso();
    let cold = nf_view::load(&iso, "0700000a.bin", false).expect("load 0a cold");
    let save = {
        let sim = cold.sim.as_ref().unwrap();
        nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
    };
    let mut sc = nf_view::load_with_carry(&iso, "0700000a.bin", false, Some(&save)).expect("load 0a warm");
    let sim = sc.sim.as_mut().unwrap();
    let (start, yaw) = sc.start.unwrap_or(([0.0, 2.0, 0.0], 0.0));
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, start, yaw);
    eprintln!("[0a] spawn {:?} disabled {}", sim.bond.pos, sim.disabled);
    let mut disabled_ticks = 0u32;
    for t in 0..1200u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.disabled { disabled_ticks += 1; }
        if disabled_ticks > 0 && !sim.disabled && t > 60 { eprintln!("[0a] NIS over at t{t} (disabled {disabled_ticks})"); break; }
    }
    eprintln!("[0a] after intro: ch47 {} pos {:?} disabled_tot {disabled_ticks}", sim.drones.channels.on(0x47) as u8, sim.bond.pos);
    let (sp_idx, sp_script, sp_mode, sp_mask, kp) = {
        let sp60 = sim.drones.scripts.players.iter().find(|p| p.out == 0x3a).expect("SP #60 (out 0x3a)");
        (sp60.placement, sp60.scripts[0], sp60.modes[0], sp60.mask, sp60.matrix[3])
    };
    eprintln!("[0a] SP #{sp_idx} script {sp_script:08x} modeA {sp_mode} mask {sp_mask:#x} at {kp:?}");
    let sp_state = |sim: &nf_view::scene::Sim| -> u16 {
        sim.drones.scripts.players.iter().find(|p| p.placement == sp_idx).map(|p| p.state).unwrap_or(0xff)
    };
    let leaves: Vec<_> = sim.drones.sss.list.iter().enumerate()
        .filter(|(_, s)| { let m = s.matrix[3]; (m[2] - 13.0).abs() < 2.0 && m[0] > -7.0 && m[0] < 1.0 })
        .map(|(i, s)| (i, s.channel, s.matrix[3])).collect();
    for (i, ch, m) in &leaves { eprintln!("[0a] SS #{i} ch{ch} at ({:.2},{:.2},{:.2})", m[0], m[1], m[2]); }
    // Walk out of the cab, north.
    let target = [-3.9f32, 0.0, 15.5];
    for t in 0..300u32 {
        let bp = sim.bond.pos;
        let desired = (target[0] - bp[0]).atan2(target[2] - bp[2]);
        let err = wrap_pi(desired - sim.bond.yaw);
        sim.step_bond(Input { turn: err.clamp(-0.08, 0.08), forward: 1.0, ..Default::default() }, false, false, false);
        if t % 60 == 0 { eprintln!("[0a] walk t{t} pos ({:.2},{:.2},{:.2})", bp[0], bp[1], bp[2]); }
        if (bp[2] - target[2]).abs() < 0.8 { eprintln!("[0a] WALKED OUT at t{t}"); break; }
    }
    eprintln!("[0a] after walk-out pos {:?}", sim.bond.pos);
    // The keyboard E-use: stand ~1.1 from SP #60's origin, face it, hold use.
    let stand = [kp[0], kp[1] - 0.3, kp[2] + 1.3];
    sim.bond = nf_game::player::Bond::new(stand, 0.0);
    for t in 0..300u32 {
        let bp = sim.bond.pos;
        let desired = (kp[0] - bp[0]).atan2(kp[2] - bp[2]);
        let err = wrap_pi(desired - sim.bond.yaw);
        let input = Input { turn: err.clamp(-0.08, 0.08), ..Default::default() };
        sim.step_bond(input, false, false, true);
        if t % 50 == 0 { eprintln!("[0a] E t{t} ch3a {} ch49 {} state {}", sim.drones.channels.on(0x3a) as u8, sim.drones.channels.on(0x49) as u8, sp_state(sim)); }
        if sim.drones.channels.on(0x49) { eprintln!("[0a] KEYBOARD FIRED at t{t}"); break; }
    }
    eprintln!("[0a] keyboard: ch3a {} ch49 {} SP state {}", sim.drones.channels.on(0x3a) as u8, sim.drones.channels.on(0x49) as u8, sp_state(sim));
}

/// 0a: what blocks the Q-Worm shot? Segment casts from the walked ring toward the PC.
#[test]
fn probe_0a_pc_rays() {
    let iso = iso();
    let mut sc = nf_view::load(&iso, "0700000a.bin", false).expect("load 0a");
    let sim = sc.sim.as_mut().unwrap();
    let pc = sim.drones.useobjs.pcs.iter().find(|p| p.ch == 0x48).expect("Q-Worm PC");
    eprintln!("[0a] PC at {:?} e_usable {} armed {}", pc.pos, pc.e_usable, pc.armed);
    let pcp = pc.pos;
    for (label, from) in [
        ("desk-north-1.5", [pcp[0], 0.4, pcp[2] + 1.5]),
        ("desk-north-1.9", [pcp[0], 0.4, pcp[2] + 1.9]),
        ("desk-north-2.5", [pcp[0], 0.4, pcp[2] + 2.5]),
        ("north-east-2.0", [pcp[0] + 1.4, 0.4, pcp[2] + 1.4]),
        ("north-west-2.0", [pcp[0] - 1.4, 0.4, pcp[2] + 1.4]),
    ] {
        for eye_h in [1.65f32, 1.3, 1.0] {
            let from3 = [from[0], from[1] + eye_h, from[2]];
            let to = [pcp[0], pcp[1] + 0.2, pcp[2]];
            let hit = sim.world.segment_hit_scoped(None, from3, to);
            eprintln!("[0a] {label} eye{eye_h} -> {} {:?}", hit.is_some(), hit.map(|(t, p)| (t, [p[0], p[1], p[2]])));
        }
    }
}

/// 08: the gondola chain with teleports (probe): plate #237 → 0x49, the cable-car floor → ch 4,
/// mux ch 1 (with 0x4A stand-in) → SP #295 ride → ch 83 → SP #296 → ch 5 → copter → 0x4B.
#[test]
fn probe_08_gondola_chain() {
    let iso = iso();
    let mut sc = nf_view::load(&iso, "07000008.bin", false).expect("load 08");
    let sim = sc.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
    let c = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::TOUCH_ONCE && t.out == 0x49).map(|t| t.centre()).expect("plate 237");
    sim.bond.pos = [c[0], c[1], c[2]];
    for _ in 0..2000 { sim.step_bond(Input::default(), false, false, false); if sim.drones.channels.on(0x49) && !sim.disabled { break; } }
    eprintln!("[08] plate: ch49 {} disabled {}", sim.drones.channels.on(0x49) as u8, sim.disabled);
    for _ in 0..4000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    let floor = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::TOUCH && t.out == 4).map(|t| t.centre()).expect("cable-car floor touch");
    eprintln!("[08] floor touch at {:?}", floor);
    sim.bond = nf_game::player::Bond::new([floor[0], floor[1] + 0.2, floor[2]], 0.0);
    sim.drones.channels.set(0x4a, 1, sim.drones.frame); // the briefcase stand-in (probe only)
    let mut seen = [false; 4];
    for t in 0..3600u32 {
        sim.step_bond(Input::default(), false, false, false);
        for (k, (ch, tag)) in [(0usize, (83u32, "ch83")), (1, (5, "ch5")), (2, (0x4b, "ch4b")), (3, (100, "ch100"))] {
            if sim.drones.channels.on(ch) && !seen[k] { eprintln!("[08] t{t} {tag} ON"); seen[k] = true; }
        }
        if t % 200 == 0 {
            let carriage = sim.drones.scripts.entity_matrix((0x0600_004c, 1));
            eprintln!("[08] t{t} ch4 {} ch1 {} bond {:?} grounded {} carriage {:?} disabled {}",
                sim.drones.channels.on(4) as u8, sim.drones.channels.on(1) as u8, sim.bond.pos, sim.bond.grounded,
                carriage.map(|m| m[3]), sim.disabled);
        }
        if sim.drones.channels.on(0x4b) { break; }
    }
    for p in sim.drones.scripts.players.iter().filter(|p| p.scripts[0] == 0x0600_004c || p.scripts[0] == 0x0600_0078) {
        eprintln!("[08] SP #{} script {:08x} out {:#x} mask {:#x} state {} deleted {}", p.placement, p.scripts[0], p.out, p.mask, p.state, p.deleted);
    }
    eprintln!("[08] end ch4b {} ch100 {} exit {:?} bond {:?}", sim.drones.channels.on(0x4b) as u8, sim.drones.channels.on(100) as u8, sim.drones.level_exit, sim.bond.pos);
}

/// 4a: the lift ride from the spawn exactly as the runner would play it (no walker, cut-scenes
/// played in full): the platform carries Bond down, the landing NIS, the script end → ch 0x4a.
#[test]
fn probe_4a_ride() {
    let iso = iso();
    let mut sc = nf_view::load(&iso, "0700004a.bin", false).expect("load 4a");
    let sim = sc.sim.as_mut().unwrap();
    let (start, yaw) = sc.start.unwrap_or(([0.0, 2.0, 0.0], 0.0));
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, start, yaw);
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    let y0 = sim.bond.pos[1];
    let mut disabled_seen = 0u32;
    for t in 0..5600u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.disabled { disabled_seen += 1; }
        if t % 300 == 0 {
            eprintln!("[4a] t{t} y {:.2} (boarded {y0:.2}) grounded {} disabled_tot {disabled_seen} ch4d {} ch4a {} health {:.0} states {:?}",
                sim.bond.pos[1], sim.bond.grounded as u8, sim.drones.channels.on(0x4d) as u8,
                sim.drones.channels.on(0x4a) as u8, sim.drones.health.health, sim.drones.mission.states[17]);
        }
        if sim.drones.channels.on(0x4a) { eprintln!("[4a] ch 0x4a ON at t{t}"); break; }
    }
    eprintln!("[4a] end y {:.2} grounded {} disabled_tot {disabled_seen} ch4a {} states {:?} pos {:?}",
        sim.bond.pos[1], sim.bond.grounded as u8, sim.drones.channels.on(0x4a) as u8,
        sim.drones.mission.states[17], sim.bond.pos);
}


/// 0a: all script players + the first script events (does an arrival intro run?).
#[test]
fn probe_0a_sp_list() {
    let iso = iso();
    let mut sc = nf_view::load(&iso, "0700000a.bin", false).expect("load 0a");
    let sim = sc.sim.as_mut().unwrap();
    for p in sim.drones.scripts.players.iter() {
        eprintln!("[0a-sp] #{} scripts {:08x}/{:08x} modes {:?} out {:#x} mask {:#x} state {} at {:?}",
            p.placement, p.scripts[0], p.scripts[1], p.modes, p.out, p.mask, p.state, p.matrix[3]);
    }
    for _ in 0..400 { sim.step_bond(Input::default(), false, false, false); }
    eprintln!("[0a-sp] after 400 ticks: ch3a {} disabled {}", sim.drones.channels.on(0x3a) as u8, sim.disabled);
}

/// 0a: the cab-mouth SS doors' mask/state — are they E-openable (mask 4, armed)?
#[test]
fn probe_0a_ss_masks() {
    let iso = iso();
    let mut sc = nf_view::load(&iso, "0700000a.bin", false).expect("load 0a");
    let sim = sc.sim.as_mut().unwrap();
    for (i, s) in sim.drones.sss.list.iter().enumerate() {
        let m = s.matrix[3];
        if (m[2] - 13.0).abs() < 2.0 && m[0] > -7.0 && m[0] < 1.0 {
            eprintln!("[0a-ss] SS #{i} ch{} mask {:#x} state {} keys {} at {:?}", s.channel, s.mask, s.state, s.keys.len(), m);
        }
    }
}
