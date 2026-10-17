//! pickups-g029 (GAPS G-029 open half): the three rules through the real paths —
//! 1. the pickup auto-switch option gate [M Player_EquipWeapon 0x1a8a58:76: the per-player
//!    `PlayerSetting+0xA` byte (`DAT_002a38d2[idx·0x158]`, `Input_Init` 0x17edd8 default 1,
//!    live 1 [L ee_start_05]) ∧ `Player_IsBetterWeapon` 0x1a6eb0 → `anim+0x63 = id`;
//! 2. the level-placed vs script-spawned pickup differences [M parsemap_create_dynamic_objects
//!    0x1d05ac vs Script_CreateEntity 0x1d8be0-0x1d8c20: the SAME record fields (+0x2c type,
//!    +0x30 id, +0x34 rounds, +0x38 switch, +0x3c sound, +0x40 respawn), rest = 1 and lifetime 0
//!    in both arms — the differences are the LABEL (the placement's +0x44 / param 6 rides;
//!    the script arm's delay slot `sw $zero, 0x20($sp)` passes 0) and the source of the record];
//! 3. the respawn tail [M Pickup_Handler 0x18e650:338-348 + Pickup_Update 0x18e148:182-189:
//!    respawn 0 → deleted, 0xFFFF → stays takeable, else hidden for `FRAME_RATE_INT · respawn
//!    · 10` ticks — SP map values and the MP placements carry theirs the same way].

use nf_game::player::Input;
use nf_view::dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

fn wrap_pi(mut a: f32) -> f32 {
    while a > std::f32::consts::PI { a -= std::f32::consts::TAU; }
    while a < -std::f32::consts::PI { a += std::f32::consts::TAU; }
    a
}

/// Steer Bond to an XZ point with the real input path (fix_auto_switch_live's walker).
fn steer_until(scene: &mut nf_view::scene::Scene, xz: [f32; 2], max: u32, mut until: impl FnMut(&nf_view::scene::Sim) -> bool) -> Option<u32> {
    for t in 0..max {
        let (desired, d, yaw) = {
            let sim = scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            ((xz[0] - p[0]).atan2(xz[1] - p[2]), dxz(p, [xz[0], p[1], xz[1]]), sim.bond.yaw)
        };
        let err = wrap_pi(desired - yaw);
        let input = Input { turn: err.clamp(-0.07, 0.07), forward: if d > 0.8 { 1.0 } else { 0.0 }, ..Default::default() };
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(input, false, false, false);
        if until(sim) { return Some(t); }
    }
    None
}

// ── 1. The option gate ──────────────────────────────────────────────────────────────────────────

/// The option gate [M 0x1a8a58:76]: with the per-player auto-switch option OFF, a better-ranked
/// drop still fills the inventory (the take is real) but never writes the switch request — Bond
/// keeps the held weapon. Then flipping the option ON (the game's default [M 0x17edd8, L]) lets
/// the same take through the draw machine — the gate, not the ranking, was the difference.
#[test]
fn the_auto_switch_option_gate_gates_the_switch() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let held = scene.sim.as_ref().unwrap().gun.weapon;
    let drop_at = {
        let p = scene.sim.as_ref().unwrap().bond.pos;
        [p[0] + 1.0, p[2] + 1.5]
    };
    // The SG5 (0x16) ranks 6th [L BestWeapon 0x2bede0] vs the spawn P2K (0x02) 16th — better.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.auto_switch = false; // the game's option byte, PlayerSetting+0xA = 0
        let p = sim.bond.pos;
        let m: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [p[0] + 1.0, p[1], p[2] + 1.5, 1.0]];
        let mut pickup = nf_game::pickup::Pickup::create_simple(&m, 0x16, 16, nf_game::pickup::kind::WEAPON, &sim.weapons, &sim.models).expect("the SG5 drop builds");
        pickup.state = 1; // resting at the drop point (the sg5-pickup scenario's pattern)
        sim.drones.pickups.push(pickup);
    }
    let taken = steer_until(&mut scene, drop_at, 900, |sim| {
        sim.drones.inventory.owned.get(0x16).copied().unwrap_or(false)
    });
    taken.expect("the option-off take never happened — the touch path broke");
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.gun.weapon, held, "option off: the held weapon must not move [M 0x1a8a58:76]");
        assert_eq!(sim.gun.switch_state, 0, "no draw cycle may run");
        assert!(sim.drones.inventory.owned.get(0x16).copied().unwrap_or(false), "the take itself is real");
        assert!(sim.drones.weapon_switch_request.is_none(), "no request may be pending");
    }
    // Flip the option ON (the game's default) — a better-ranked drop Bond does NOT own yet now
    // switches (a second SG5 would be the AMMO path — `Player_EquipAmmo` has no switch tail [M]).
    // The P2K (0x02) ranks 15th [L BestWeapon]; 0x12 ranks 12th — better. The take must be a real
    // Weapon equip: unowned (the 07000005 loadout) and the first-person model loaded (the skins
    // hashtable, [M 0x1a8a58's `hashtable_getitem` gate]).
    let (better_id, second) = {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.auto_switch = true; // the game's default: Input_Init 0x17edd8 writes 1 / [L ee_start_05 0x2a38d2]
        let better = [0x12u16, 0x0e, 0x0d, 0x0c, 0x0a, 0x08, 0x10, 0x04];
        let id = *better.iter().find(|&&w| {
            !sim.drones.inventory.owned.get(w as usize).copied().unwrap_or(true)
                && sim.weapons.records.get(w as usize).is_some()
                && sim.skins.contains_key(&sim.weapons.first_person(w as usize))
        }).expect("no better-ranked unowned weapon with a loaded first-person model");
        let p = sim.bond.pos;
        let m: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [p[0] + 1.0, p[1], p[2] + 1.5, 1.0]];
        let mut pickup = nf_game::pickup::Pickup::create_simple(&m, id, 16, nf_game::pickup::kind::WEAPON, &sim.weapons, &sim.models).expect("the second drop builds");
        pickup.state = 1;
        sim.drones.pickups.push(pickup);
        (id, [p[0] + 1.0, p[2] + 1.5])
    };
    let swapped = steer_until(&mut scene, second, 900, |sim| {
        sim.gun.weapon == better_id as u8 && sim.gun.switch_state == 0
    });
    assert!(swapped.is_some(), "option on: the better-ranked take must switch through the draw machine");
}

// ── 3. The respawn tail ─────────────────────────────────────────────────────────────────────────

/// SP respawn on a real level [D the census]: 07000004 places weapon 0x12 with respawn 2 —
/// the game hides it for `FRAME_RATE_INT · 2 · 10` = 1200 ticks (20 s) after the take [M
/// 0x18e650:338-348 + 0x18e148:182-189]. Through the real take path: teleport next to it (the
/// ai_traces_live position pattern), walk the last step, take, watch the wait, and take again
/// after it returns.
#[test]
fn a_respawning_placed_pickup_hides_and_returns_on_07000004() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000004.bin", false).expect("load 07000004");
    let (spot, respawn_ticks) = {
        let sim = scene.sim.as_ref().unwrap();
        let p = sim.drones.pickups.iter().find(|p| p.kind == 0 && p.weapon == 0x12 && p.respawn == 2 && !p.gone)
            .expect("07000004 places weapon 0x12 with respawn 2 [D the census]");
        let c = p.centre();
        ([c[0], c[1] + 0.2, c[2]], 60 * p.respawn as u32 * 10)
    };
    eprintln!("[respawn] the 0x12 pickup at {spot:?}, wait {respawn_ticks} ticks");
    // Onto the pickup (the established position pattern; the touch path itself is untouched).
    scene.sim.as_mut().unwrap().bond.pos = spot;
    // Take: the pickup hides into state 2 with the full wait set [M the respawn tail].
    let taken = steer_until(&mut scene, [spot[0], spot[2]], 300, |sim| {
        sim.drones.pickups.iter().any(|p| p.kind == 0 && p.weapon == 0x12 && p.state == 2)
    });
    let at = taken.expect("the respawn pickup was never taken — the touch path broke");
    eprintln!("[respawn] taken at +{at}");
    // The level places THREE 0x12 pickups [D the census] — track the taken one by index.
    let slot = scene.sim.as_ref().unwrap().drones.pickups.iter()
        .position(|p| p.kind == 0 && p.weapon == 0x12 && p.state == 2)
        .expect("the taken pickup is in state 2");
    {
        let sim = scene.sim.as_ref().unwrap();
        let p = &sim.drones.pickups[slot];
        assert!(p.hidden, "hidden while waiting (obj+0xF0 & 0x10) [M]");
        assert_eq!(p.state, 2);
        assert!(!p.gone, "respawn != 0 is never deleted [M 0x18e650:338-348]");
        assert_eq!(p.respawn_ticks, respawn_ticks, "the full FRAME_RATE_INT · respawn · 10 wait [M]");
        assert!(sim.drones.inventory.owned.get(0x12).copied().unwrap_or(false), "the weapon went into the inventory");
    }
    // The wait: tick with idle input until the counter is at its last tick, still hidden; the
    // next tick restores (respawn_wait decrements then restores at 0 [M 0x18e148:182-189]).
    loop {
        if scene.sim.as_ref().unwrap().drones.pickups[slot].respawn_ticks <= 1 { break; }
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    }
    {
        let p = &scene.sim.as_ref().unwrap().drones.pickups[slot];
        assert!(p.hidden && p.state == 2, "still hidden on the last wait tick");
    }
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    {
        let p = &scene.sim.as_ref().unwrap().drones.pickups[slot];
        assert!(!p.hidden && p.state == 1, "restored after the wait [M 0x18e148:182-189]");
        eprintln!("[respawn] returned to rest");
    }
    // And it is takeable again: Bond is still standing on the spot — the touch re-fires. Keep him
    // alive through the 20 s wait (the level's guards shoot; the game's MP respawn scenario has
    // living players — the alive gate `health > 0` [M 0x18e148:203] is the only poke).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = spot;
        sim.drones.health.health = 100.0;
    }
    let retaken = steer_until(&mut scene, [spot[0], spot[2]], 300, |sim| {
        sim.drones.pickups.iter().any(|p| p.kind == 0 && p.weapon == 0x12 && p.state == 2)
    });
    assert!(retaken.is_some(), "the returned pickup must be takeable again (the MP behaviour the rule exists for)");
}

// ── 4. Placed vs script, through the real loaders ───────────────────────────────────────────────

/// The two arms' differences on the disc's own data [D the census; M 0x1d05ac vs 0x1d8be0]:
/// - 07000008's safe script 0600004c spawns the rocket launcher (kind 0, id 0x2c, rounds → the
///   level override 4) and its ammo with **respawn 0xFFFF** (stays takeable forever) and **label 0**
///   — the script arm passes the label as a hardcoded 0 [M 0x1d8c1c delay slot];
/// - 0700000b's PLACED kind-4 item carries the placement's label 0x2000036 [M 0x1d05e4: label =
///   placement+0x44] — the same field the script arm never reads.
/// Both arms agree on the rest: rest = 1 (state 1), lifetime 0, the same param mapping.
#[test]
fn script_pickups_carry_respawn_and_no_label_while_placed_ones_carry_the_label() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // 07000008: the script reservations (built at load from the level's own scripts).
    let scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_ref().unwrap();
    let rocket = sim.drones.pickups.iter().enumerate()
        .find(|(_, p)| p.gone && p.kind == 0 && p.weapon == 0x2c)
        .map(|(i, p)| (i, p.clone()))
        .expect("07000008's safe script reserves the rocket pickup [D the census: script 0600004c]");
    let ammo = sim.drones.pickups.iter()
        .find(|p| p.gone && p.kind == 1 && p.weapon == 0x2c)
        .expect("the safe script's kind-1 ammo pile");
    eprintln!("[placed-vs-script] rocket slot {} respawn {:#x} label {:#x} state {}",
        rocket.0, rocket.1.respawn, rocket.1.label, rocket.1.state);
    assert_eq!(rocket.1.respawn, 0xFFFF, "the script's respawn word rides (stays takeable forever) [M 0x1d8bfc]");
    assert_eq!(rocket.1.label, 0, "the script arm passes label 0 [M 0x1d8c1c]");
    assert_eq!((rocket.1.state, rocket.1.lifetime), (1, 0), "rest = 1, lifetime 0 in the script arm [M 0x1d8c10/0x1d8c18]");
    assert_eq!(ammo.respawn, 0xFFFF, "sister: the ammo pile rides the same respawn word");
    assert_eq!(ammo.label, 0, "sister: no label");
    assert_eq!(rocket.1.rounds, 4, "the level-conditional rounds override applies to script creates too [M 0x18dda0]");
    // 0700000b: the PLACED kind-4 item with the placement's label [M 0x1d05e4].
    let scene = nf_view::load(&iso, "0700000b.bin", false).expect("load 0700000b");
    let sim = scene.sim.as_ref().unwrap();
    let item = sim.drones.pickups.iter()
        .find(|p| p.kind == 4 && p.label != 0)
        .expect("0700000b places the kind-4 item with label 0x2000036 [D the census]");
    assert_eq!(item.label, 0x2000036, "the placed pickup carries the placement's label word [M 0x1d05e4]");
    assert_eq!(item.weapon, 0x47, "id 0x47 [D the census]");
}

/// The census [D the slim ISO's story archives]: every story level's placed 0xF0 placements with
/// respawn (param 5) ≠ 0 or a label (param 6) ≠ 0, and every script obj_type-0xF0 Entity with
/// its params — the real-level material the other tests pin. Run with --nocapture to read it.
#[test]
fn g029_census_of_placed_and_script_pickups() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso_path).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    for id in nf_game::mission::LEVELS.iter().map(|l| l.level) {
        let name = format!("{id:08x}.bin");
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)) else { continue };
        let Ok(data) = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64) else { continue };
        let Ok(level) = nf_formats::level::Level::parse(&data) else { continue };
        let mm = level.main_map().unwrap_or(0);
        for p in level.maps[mm].placements.iter().filter(|p| p.flags & 0xff == 0xf0) {
            let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|(_, v)| *v).unwrap_or(0);
            let (type_, idw, respawn, label) = (param(0), param(1), param(5), param(6));
            if respawn != 0 || label != 0 {
                eprintln!("[census] {name} PLACED type {type_:#x} id {idw:#x} respawn {respawn} label {label:#x}");
            }
        }
        let mut hashes: Vec<_> = level.nis.keys().copied().collect();
        hashes.sort_unstable();
        for hash in hashes {
            let nis = &level.nis[&hash];
            for st in nis.streams.iter() {
                let cmds = st.decode();
                if let Some(nf_formats::nis::Cmd::Entity { obj_type: 0xf0, params, .. }) = cmds.iter().find(|c| matches!(c, nf_formats::nis::Cmd::Entity { obj_type: 0xf0, .. })) {
                    let p = |k: usize| params.get(k).copied().unwrap_or(0);
                    eprintln!("[census] {name} SCRIPT {hash:08x} params {:?} (respawn = {})", params, p(5));
                }
            }
        }
    }
}
