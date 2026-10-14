//! fix-silencer-modes (102b): the P2K silencer (twin 0x02 'Semi' <-> 0x03 'Silenced') and the SSG's
//! semi-auto pace, from the 1013-1825 playtest on 07000006/07000005.
//!
//! - "swap out, swap back, it didn't stay": the game remembers the equipped twin in
//!   `BLData+0x1BC + base·0xC` (written by `Player_WeaponSelect` 0x1a2cc0:15-16/27-28 on every
//!   same-base switch) and `Player_WeaponChange` 0x1a6f80:179-198 hands the TWIN back on the next
//!   cycle to the family (the toggle-back +0x930 gates on the BASE's selectable/owned and restores
//!   the remembered id verbatim, line 113). The port landed on the bare 0x02 every time.
//! - "the animation played, then the silencer disappeared (bullets stayed silenced)": the twins
//!   share the FP skin 0x05000088, idle 0x06000825 and fire 0x06000823 [L weapon_data +0xDC/+0xA0/
//!   +0xB0]; the silencer visual is the +0xC8 variant-swap pair (0x06000828 attach / 0x06000829
//!   detach [L]) animating the part bones — 43's local translation runs (0.29, 0.33, −1.60) in the
//!   shared idle [L port pose code] vs (0.00, 0.09, 0.14) mounted at the attach's hold. The game's
//!   persistent mount is a draw-side gate this session did not decode — live checks in the report.
//! - "infinite ammo: the reserves filled, but the magazine didn't stay full" + "semi-auto doesn't
//!   shoot fast enough on the SSG": one bug. The dev cheat topped the inventory every 30 frames but
//!   left the live magazine drained between tops, and the 5-round sniper (0x24, clip 5, refire
//!   +0x40 = 120 [L weapon_data]) kept tripping the auto-reload. With the clip pinned every tick
//!   (dev only) the refire is the table's.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_05(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the intro never released Bond");
    scene
}

/// Attach the silencer through the real switch path (a same-base request 0x02 → 0x03 plays the
/// +0xC8 attach at 1.3×, state 8 [M Player_WeaponSelect 0x1a2cc0:18-29 — the twins share the FP
/// model 0x05000088 [L]]) and wait it out.
fn attach_silencer(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.gun.weapon, 2, "Bond starts on the P2K [L]");
    sim.gun.switch_request = 3;
    for _ in 0..400 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.gun.weapon, 3, "the silenced twin is the live weapon after the attach");
    assert_eq!(sim.gun.switch_state, 0, "the variant-swap anim completed");
    assert_eq!(sim.gun.twin_skip[2], 1, "the twin marker BL+0x1BC[0x02] = 1 [M 0x1a2cc0:15-16]");
}

/// The cycle to another weapon and back hands the SILENCED twin back: `Player_WeaponChange`'s walk
/// offers `cand + twin_skip[cand]` first and lands on it while it has ammo [M 0x1a6f80:180-190].
#[test]
fn the_silenced_twin_survives_a_weapon_swap_round_trip() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_05(&iso);
    // Own the SG5 (0x16) with ammo so the cycle has somewhere to go.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.inventory.owned[0x16] = true;
        sim.drones.inventory.clip[0x16] = 30;
        let at = sim.weapons.records[0x16][0x90] as usize;
        if at < sim.drones.inventory.carried.len() { sim.drones.inventory.carried[at] = 150; }
    }
    attach_silencer(&mut scene);
    // Cycle UP (to the SG5) through the real input path, wait the swap out, then cycle BACK.
    for &(dir, ticks) in &[(1i8, 300usize), (-1i8, 400usize)] {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.switch_state = 0; // the previous swap settled above
        let input = Input { weapon_cycle: dir, ..Input::default() };
        for _ in 0..ticks { sim.step_bond(input, false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.gun.weapon, 3,
        "cycling back to the P2K family hands the SILENCED twin [M 0x1a6f80:180-190; L the 1013-1825 playtest] (got {:#x})", sim.gun.weapon);
    // Sister: the fire-mode toggle from the silenced twin returns to the bare 0x02 and CLEARS the
    // marker (the same-base request 0x03 → 0x02 writes (0x02 − 0x02) = 0 [M 0x1a2cc0:15-16]).
    let mut scene = scene;
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.switch_request = 2;
    for _ in 0..400 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.gun.weapon, 2, "the fire-mode toggle returns to 'Semi'");
    assert_eq!(sim.gun.twin_skip[2], 0, "the marker clears when the base is equipped [M 0x1a2cc0:15-16]");
}

/// With the dev panel's infinite ammo the CURRENT weapon's clip stays at its `weapon_data+0x92`
/// size every tick: the sniper (0x24, clip 5, refire 120 [L weapon_data]) never trips the
/// auto-reload, and its held-trigger cadence is the table's ~121 ticks.
#[test]
fn dev_infinite_ammo_keeps_the_snipers_magazine_full() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_05(&iso);
    let mut dev = <nf_view::dev::Dev as Default>::default();
    dev.infinite_ammo = true;
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x24; sim.gun.switch_request = 0x24; sim.gun.switch_state = 0; sim.gun.timer = 0.0;
        sim.drones.inventory.owned[0x24] = true;
        sim.drones.inventory.clip[0x24] = 5;
        let at = sim.weapons.records[0x24][0x90] as usize;
        if at < sim.drones.inventory.carried.len() { sim.drones.inventory.carried[at] = 100; }
        let c = sim.drones.inventory.clip[0x24];
        sim.gun.clip = c.max(0) as u32;
    }
    let mut min_clip = u32::MAX;
    let mut shots = Vec::new();
    let mut reload_ticks_seen = 0usize;
    for t in 0..900u32 {
        // Semi-auto: one shot per press — tap every 121 ticks (the table's refire 120).
        scene.sim.as_mut().unwrap().step_bond(Input::default(), t % 121 == 0, false, false);
        dev.after_tick(&mut scene);
        let sim = scene.sim.as_ref().unwrap();
        min_clip = min_clip.min(sim.gun.clip);
        if sim.shot_tick { shots.push(t); }
        if sim.gun.reload_state != 0 { reload_ticks_seen += 1; }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.weapons.records[0x24][0x92], 5, "the sniper's clip size is 5 [L weapon_data +0x92]");
    assert_eq!(min_clip, 5, "the magazine stays full every tick [L the 1013-1825 playtest: 'the magazine didn't stay full']");
    assert_eq!(reload_ticks_seen, 0, "no auto-reload ever starts with the clip pinned");
    let gaps: Vec<i64> = shots.windows(2).map(|a| a[1] as i64 - a[0] as i64).collect();
    assert!(gaps.iter().all(|&g| (118..=135).contains(&g)),
        "the tapped refire is the table's +0x40 = 120 frames + the tap quantization [M 0x1a5460:661-663] (gaps {gaps:?}, shots {})", shots.len());
    assert!(shots.len() >= 6, "the sniper actually fires through the run (shots {:?})", shots);
}
