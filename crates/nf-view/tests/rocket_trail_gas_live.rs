//! rocket-trail-gas — the gas puffs one rocket shot spawns, against the game.
//!
//! The game's truth for the 'Unguided' launcher 0x2d [L ee_start_05.bin @ 0x2bf150, record 0x2d]:
//! `+0x68 = 0x00014000` (no `0x2000` — the muzzle smoke runs [M `Player_MuzzleFlash` 0x1a4298:91-102]),
//! `+0x6c = 0x0001a200` (bit 0x2000 fly-light + bit 0x10000 swoosh ribbon; NONE of `Bullet_DoTrails`
//! 0x12d890's gas bits 0x2/0x20/0x80000 [M decomp :144-199]), `+0x70 = 0x00000004` (no timer arm, no
//! impact-gas bit). So one 0x2d shot spawns exactly ONE `Gas_Init` puff — the muzzle smoke
//! `Gas_Init(0.05 → 0.1, 0x02000039, kind 7, rgba 0x48403080 → bytes (0,0,0,1), life 0x80)` [M
//! 0x1a43f8/0x1a4440/0x1a442c] — and the flight itself spawns none: the thin trail is the swoosh
//! RIBBON (0x2000481, a sprite strip `Swoosh_Create` feeds each tick, not a gas puff; the system is
//! not ported — the report documents the gap). Pre-fix the port spawned one cyan stand-in puff per
//! flying tick for every bit-0x10000 record — the port-only grey haze down the 08 corridor this
//! task removes.

use nf_game::player::Input;
use nf_game::gas::{GasPool, SMOKE_QUAD};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Give a weapon through the dev panel (the user's `--give` path) and wait for the switch.
fn give_and_switch(scene: &mut nf_view::scene::Scene, id: u8) {
    let (msg, switch_to) = nf_view::panel::apply(scene, &q(&[("do", "weapon"), ("id", &id.to_string()), ("rounds", "5")]));
    eprintln!("[give] {id:#04x}: {msg} -> {switch_to:?}");
    assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id && scene.sim.as_ref().unwrap().gun.switch_state == 0 { return; }
    }
    panic!("the switch to {id:#04x} never completed");
}

/// The 0x2d 'Unguided' record is the Sentinel 0x2c's ○ mode twin (weapon_data[0x2c]+0x05 = +1
/// [L ee_start_05.bin]; the direct number-key select lands on the base, as the MGL's 0x2b).
/// `fire_mode_switch` [M gun.rs, Player_Weapon 0x1a24c8:204-260]: give the base, press ○, wait.
fn give_sentinel_and_switch_to_unguided(scene: &mut nf_view::scene::Scene) {
    give_and_switch(scene, 0x2c);
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x2d, "○ on the Sentinel requests the 'Unguided' twin 0x2d");
    }
    for _ in 0..200 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.gun.weapon == 0x2d && sim.gun.switch_state == 0 { return; }
    }
    panic!("the switch to the 'Unguided' twin 0x2d never completed");
}

/// The muzzle puff's exact game data, checked against the pool's puff [M 0x1a4298 call + the
/// dropped stack life; M 0x1a43f8 `li $v0, 0x80`, 0x1a4440 delay-slot `li $t3, 1`]. The pool's
/// update may have run 1-2 ticks before the test reads it, so every field carries a ±2-tick
/// window around the birth value.
fn assert_muzzle_puff(g: &nf_game::gas::Gas) {
    let rate = 1.0 / 128.0;
    assert_eq!(g.mesh, SMOKE_QUAD, "the muzzle puff is the shared smoke quad [M 0x1a4434]");
    assert_eq!(g.kind, 7, "move kind 7 [M 0x1a4434 `li $t1, 7`]");
    assert!(g.scale >= 0.05 && g.scale <= 0.05 + 2.0 * ((0.1 - 0.05) * rate) + 1e-6, "start scale 0.05 [M 0x1a440c], got {}", g.scale);
    assert!((g.scale_rate - (0.1 - 0.05) * rate).abs() < 1e-9, "end scale 0.1 over life 128 [M 0x1a441c + the dropped 0x80], got {}", g.scale_rate);
    for (i, start) in [0x48 as f32, 0x40 as f32, 0x30 as f32].iter().enumerate() {
        let start = *start;
        assert!(g.rgba[i] <= start + 1e-4 && g.rgba[i] >= start - 2.0 * (start * rate) - 1e-4,
            "rgba[{i}] starts at {start} and only fades [M 0x1a4438], got {}", g.rgba[i]);
    }
    let a0 = 0x80 as f32;
    assert!(g.rgba[3] <= a0 + 1e-4 && g.rgba[3] >= a0 - 2.0 * ((a0 - 1.0) * rate) - 1e-4,
        "alpha starts 0x80, rate (1−0x80)/128 toward the packed end 1 = bytes (0,0,0,1) [M 0x1a4440], got {}", g.rgba[3]);
    assert!((g.rgba_rate[0] - (0.0 - 0x48 as f32) * rate).abs() < 1e-4
         && (g.rgba_rate[1] - (0.0 - 0x40 as f32) * rate).abs() < 1e-4
         && (g.rgba_rate[2] - (0.0 - 0x30 as f32) * rate).abs() < 1e-4
         && (g.rgba_rate[3] - (1.0 - 0x80 as f32) * rate).abs() < 1e-4,
        "the fade runs start → packed end 1 [M 0x1a4440], got {:?}", g.rgba_rate);
    assert!(g.life <= 128.0 && g.life >= 128.0 - 2.0, "the dropped 11th arg = 0x80 ticks [M 0x1a43f8/0x1a442c], got {}", g.life);
}

/// ── One 0x2d shot on 08 spawns the game's puffs and only them ──────────────────────────────────
/// The muzzle smoke is the shot's ONLY gas puff [M Player_MuzzleFlash — the flight's `Bullet_DoTrails`
/// arms all read bits 0x2/0x20/0x80000 the 0x2d record doesn't carry (L sweep), and the impact arm
/// reads +0x70 & 0x2000 — 0x2d's +0x70 = 0x4]. Pre-fix the flight spawned a cyan stand-in puff per
/// tick (scale 0.3 → 0.6, kind 5, life 6) — this test fails on that tree.
#[test]
fn one_unguided_rocket_shot_spawns_only_the_games_puffs() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    give_sentinel_and_switch_to_unguided(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
        sim.bond.pitch = 0.0; // level: down the corridor, the flight the side-by-side shows
    }
    let mut armed = false;
    let mut flight_ticks = 0u32;
    let mut max_puffs = 0usize;
    let mut saw_muzzle_puff = false;
    let mut impact = false;
    for t in 0..450u32 {
        let had = scene.sim.as_ref().unwrap().drones.player_bullets.len();
        let trigger = !armed;
        scene.sim.as_mut().unwrap().step_bond(Input::default(), trigger, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.player_bullets.len() > had { armed = true; } // the round is out: release
        // The gas pool this tick: at most the one muzzle puff, and it must carry the muzzle data.
        let puffs = &sim.drones.gas.puffs;
        max_puffs = max_puffs.max(puffs.len());
        for g in puffs {
            // The old stand-in was kind 5 / cyan [0x30,0xd0,0xff,0x90] / scale 0.3 — none of that.
            assert_eq!(g.kind, 7, "a non-muzzle puff in the pool: kind {} scale {} rgba {:?}", g.kind, g.scale, g.rgba);
            assert!(g.rgba[1] < 0x50 as f32, "a cyan-ish puff (the old stand-in's rgba {:?})", g.rgba);
            if !saw_muzzle_puff { assert_muzzle_puff(g); saw_muzzle_puff = true; }
        }
        if armed && !sim.drones.player_bullets.is_empty() { flight_ticks += 1; }
        if !sim.drones.explosions.is_empty() { impact = true; break; }
        let _ = t;
    }
    assert!(saw_muzzle_puff, "the muzzle smoke puff spawned (the 0x2d's +0x68 = 0x14000 has no 0x2000 skip [L; M 0x1a4298])");
    assert!(flight_ticks >= 5, "the rocket must fly a few ticks for the flight window to mean anything (got {flight_ticks})");
    assert!(impact, "the round reached its blast (the flight resolved)");
    assert_eq!(max_puffs, 1, "exactly ONE live puff at any tick — the game spawns no flight gas [M 0x12d890 + L 0x2d sweep]");
}

/// Sister: the guided Sentinel 0x2c (the same swoosh family, +0x6c = 0x00012204 [L]) — its flight
/// spawns no gas either; only the muzzle puff exists.
#[test]
fn the_guided_sentinel_flight_spawns_no_gas_either() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    give_and_switch(&mut scene, 0x2c);
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
        sim.bond.pitch = 0.0;
    }
    let mut armed = false;
    let mut max_puffs = 0usize;
    let mut flight_ticks = 0u32;
    for _ in 0..450u32 {
        let had = scene.sim.as_ref().unwrap().drones.player_bullets.len();
        scene.sim.as_mut().unwrap().step_bond(Input::default(), !armed, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.player_bullets.len() > had { armed = true; }
        max_puffs = max_puffs.max(sim.drones.gas.puffs.len());
        for g in &sim.drones.gas.puffs {
            assert_eq!(g.kind, 7, "a non-muzzle puff in the pool: kind {} scale {}", g.kind, g.scale);
        }
        if armed && !sim.drones.player_bullets.is_empty() { flight_ticks += 1; }
        if !sim.drones.explosions.is_empty() { break; }
    }
    assert!(flight_ticks >= 5, "the Sentinel round must fly (got {flight_ticks})");
    assert!(max_puffs <= 1, "no flight gas for the 0x2c either [L 0x2c = 0x00012204], max {max_puffs}");
}
