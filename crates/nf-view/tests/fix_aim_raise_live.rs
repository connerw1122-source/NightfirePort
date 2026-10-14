//! fix-aim-raise: the aim gate — the aim bit, the zoom start and the auto-aim lock wait for the
//! aim-raise script's last tick.
//!
//! The game: the aim button sets the aim bit, but a record with an aim-raise script (`weapon_data
//! +0xB8`) immediately clears it again and plays the raise at 1.25× in view state 0xF
//! [`Player_Weapon` 0x1a24c8:145-152]; `Player_SetWeaponAnimObj` case 0xf sets the bit only when
//! that script stops [0x1a2e38:554-561]. The zoom target (+0x8D4) leaves 1.0 on the bit's rise
//! [0x1a24c8:156-187] and `Player_AutoAim` 0x1a7c98 keeps scanning (the `Check_AutoAim` branch)
//! until the bit is set, easing the lock offsets toward the manual cursor only while aimed. The
//! port turned the aim camera, the free-aim window and the auto-aim ease on the instant the button
//! went down (fix-guard-misc2 "What's left"; fix-weapon-anims deviation 2 — that task shipped the
//! raise/lower view-model anims themselves, held on `s/103k2-fix-weapon-anims2`, so this branch
//! carries its machine inside the gate).
//!
//! Records [L weapon_data + the disc script census]: the Winter Covert Sniper 0x24 raises with
//! 0x060007b9 (24 frames → ceil(23/1.25) = 19 ticks at 1.25, no +0xBC — the lower plays it
//! REVERSED), the AIMS-20 0x1a with 0x060007bf (20 frames → 16 ticks), the Wolfram P2K 0x02 has no
//! raise script and aims at once [M 0x1a24c8:117-119].
//!
//! Every test drives the REAL input path (`nf_view::load_weapons` + the panel give + `Sim::step_bond`).

use nf_game::player::Input;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[0x02, 0x1a, 0x1b, 0x24]).expect("load 07000005");
    for _ in 0..7200 {
        if scene.sim.as_ref().map_or(false, |s| !s.disabled) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert!(scene.sim.as_ref().map_or(false, |s| !s.disabled), "07000005 never enabled (intro stuck)");
    scene
}

fn idle(scene: &Scene) -> bool {
    let s = scene.sim.as_ref().unwrap();
    s.gun.switch_state == 0 && s.gun.reload_state == 0 && !s.gun.fire_state
}

/// Gives `id` through the panel and rides the switch to the fully idle machine.
fn give_and_settle(scene: &mut Scene, id: u8, rounds: i32) {
    let (status, sw) = nf_view::panel::apply(scene, &[
        ("do".to_string(), "weapon".to_string()),
        ("id".to_string(), format!("0x{id:02x}")),
        ("rounds".to_string(), rounds.to_string()),
    ]);
    assert_eq!(sw, Some(id), "give refused: {status}");
    for _ in 0..60 {
        if scene.sim.as_ref().unwrap().gun.weapon == id { break; }
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the switch never landed");
    for _ in 0..7200 {
        let s = scene.sim.as_ref().unwrap();
        if idle(scene) && s.view_script == s.weapons.idle_script(id as usize) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    let s = scene.sim.as_ref().unwrap();
    assert!(idle(scene) && s.view_script == s.weapons.idle_script(id as usize),
        "never settled to idle (script {:#x} switch {} reload {} fire {})",
        s.view_script, s.gun.switch_state, s.gun.reload_state, s.gun.fire_state);
}

/// The raise tick count from the script's frame count: ceil((len−1)/1.25) at the game's 1.25×.
fn raise_ticks(scene: &Scene, w: u8) -> u32 {
    let s = scene.sim.as_ref().unwrap();
    let h = s.weapons.raise_script(w as usize);
    let len = s.scripts.get(&h).unwrap_or_else(|| panic!("raise script {h:#x} missing")).length as f32;
    ((len - 1.0) / 1.25).ceil() as u32
}

/// The sniper: zoom and free-aim begin ON the raise's last tick, not on the press. The raise is
/// 19 ticks (0x060007b9, 24 frames @1.25); the zoom target leaves 1.0 and the free-aim window
/// clamps exactly on tick 19.
#[test]
fn the_snipers_zoom_and_free_aim_begin_on_the_raises_last_tick() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    // The record's own raise facts [L weapon_data 0x2bf150 + 0x24 = 0x060007b9, no +0xBC].
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.weapons.raise_script(0x24), 0x0600_07b9, "the sniper's raise script (+0xB8)");
        assert_eq!(s.weapons.lower_script(0x24), 0, "the sniper has no +0xBC — the lower plays +0xB8 reversed");
        assert_eq!(s.scripts.get(&0x0600_07b9).unwrap().length, 24, "the raise's frame count");
    }
    give_and_settle(&mut scene, 0x24, 3);
    let n = raise_ticks(&scene, 0x24);
    assert_eq!(n, 19, "the sniper's raise: ceil((24-1)/1.25) = 19 ticks");
    // Hold aim + zoom + a left free-aim push from tick 1. The push would clamp the cursor to
    // -0.296 the tick the free-aim window opens; before that the window stays shut and the
    // cursor never leaves 0 (the hip branch just halves it).
    let input = Input { aiming: true, zoom: true, turn: 0.76966 * 0.5, ..Default::default() };
    let mut aimed_tick = 0;
    for t in 1..=n + 5 {
        scene.sim.as_mut().unwrap().step_bond(input.clone(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if s.aiming { aimed_tick = t; break; }
        // During the raise: the aim bit is held OFF, the zoom target stays 1.0 (the zoom has not
        // started) and the free-aim window never opened (the cursor is still at rest).
        assert!(!s.aiming, "tick {t}: the aim bit set before the raise's last tick ({n})");
        assert_eq!(s.zoom_target, 1.0, "tick {t}: the zoom target moved during the raise");
        assert_eq!(s.zoom, 1.0, "tick {t}: the zoom moved during the raise");
        assert_eq!(s.cursor[0], 0.0, "tick {t}: the free-aim window opened during the raise");
        assert_eq!(s.view_script, 0x0600_07b9, "tick {t}: the raise script is not playing");
    }
    assert_eq!(aimed_tick, n + 1, "the aim bit must set exactly on the raise's last tick — the press tick plus the {n} advance ticks (got tick {aimed_tick})");
    {
        let s = scene.sim.as_ref().unwrap();
        assert!(s.zoom_target > 1.0, "the zoom target left 1.0 on the raise's last tick");
        assert!(s.zoom > 1.0, "the zoom started on the raise's last tick");
        assert!((s.cursor[0] - (-0.296)).abs() < 1e-4, "the free-aim window clamps on the raise's last tick, cursor {}", s.cursor[0]);
    }
    // Sister: while aimed, the window keeps pushing the view — the overflow (the 0.5 push wants
    // cursor -0.5, clamped at -0.296) holds the cursor on its bound.
    scene.sim.as_mut().unwrap().step_bond(input.clone(), false, false, false);
    let s = scene.sim.as_ref().unwrap();
    assert!(s.aiming && (s.cursor[0] - (-0.296)).abs() < 1e-4, "the clamped window holds while aimed");
}

/// The AIMS-20: the same gate at its own numbers — raise 0x060007bf (20 frames → 16 ticks).
#[test]
fn the_aims20_zoom_and_free_aim_begin_on_the_raises_last_tick() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.weapons.raise_script(0x1a), 0x0600_07bf, "the AIMS-20's raise script (+0xB8)");
        assert_eq!(s.scripts.get(&0x0600_07bf).unwrap().length, 20, "the AIMS raise's frame count");
    }
    give_and_settle(&mut scene, 0x1a, 3);
    let n = raise_ticks(&scene, 0x1a);
    assert_eq!(n, 16, "the AIMS-20's raise: ceil((20-1)/1.25) = 16 ticks");
    let input = Input { aiming: true, zoom: true, turn: 0.76966 * 0.5, ..Default::default() };
    let mut aimed_tick = 0;
    for t in 1..=n + 5 {
        scene.sim.as_mut().unwrap().step_bond(input.clone(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if s.aiming { aimed_tick = t; break; }
        assert!(!s.aiming, "tick {t}: the aim bit set before the raise's last tick ({n})");
        assert_eq!(s.zoom_target, 1.0, "tick {t}: the zoom target moved during the raise");
        assert_eq!(s.cursor[0], 0.0, "tick {t}: the free-aim window opened during the raise");
        assert_eq!(s.view_script, 0x0600_07bf, "tick {t}: the raise script is not playing");
    }
    assert_eq!(aimed_tick, n + 1, "the aim bit must set exactly on the raise's last tick — the press tick plus the {n} advance ticks (got tick {aimed_tick})");
    let s = scene.sim.as_ref().unwrap();
    assert!(s.zoom_target > 1.0 && s.zoom > 1.0, "the zoom started on the raise's last tick");
    assert!((s.cursor[0] - (-0.296)).abs() < 1e-4, "the free-aim window clamps on the raise's last tick");
}

/// The P2K has no raise script: it aims on the press tick — the bit, the zoom gate and the
/// free-aim window all open at once [M 0x1a24c8:117-119]. Sister of the gated weapons above.
#[test]
fn the_p2k_aims_immediately() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.weapons.raise_script(0x02), 0, "the P2K carries no aim-raise script");
    }
    give_and_settle(&mut scene, 0x02, 41);
    scene.sim.as_mut().unwrap().step_bond(Input { aiming: true, turn: 0.76966 * 0.5, ..Default::default() }, false, false, false);
    let s = scene.sim.as_ref().unwrap();
    assert!(s.aiming, "the P2K aims on the press tick");
    assert!((s.cursor[0] - (-0.296)).abs() < 1e-4, "the P2K's free-aim window opens on the press tick");
    // Sister: the scoped AIMS-20 under the same single-tick input does NOT aim yet (its raise is
    // still playing) — the two records differ exactly by the raise script's presence.
    let mut scene = load(&iso);
    give_and_settle(&mut scene, 0x1a, 3);
    scene.sim.as_mut().unwrap().step_bond(Input { aiming: true, turn: 0.76966 * 0.5, ..Default::default() }, false, false, false);
    let s = scene.sim.as_ref().unwrap();
    assert!(!s.aiming, "the AIMS-20 must not aim on the press tick");
    assert_eq!(s.cursor[0], 0.0, "the AIMS-20's free-aim window stays shut on the press tick");
}

/// Releasing the aim drops the bit and the zoom AT ONCE (the game clears the bit in
/// `Player_Weapon` 0x1a24c8:46 and the zoom target follows [M :161]), then the lower plays the
/// raise script REVERSED (no +0xBC on the sniper [M :128-138]) before the idle returns.
#[test]
fn aim_release_drops_the_bit_then_plays_the_reverse_lower() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_settle(&mut scene, 0x24, 3);
    let n = raise_ticks(&scene, 0x24);
    // Raise fully (the press tick + the n advance ticks — the last one finishes it).
    for _ in 0..n + 1 {
        scene.sim.as_mut().unwrap().step_bond(Input { aiming: true, zoom: true, ..Default::default() }, false, false, false);
    }
    assert!(scene.sim.as_ref().unwrap().aiming, "not aimed after the raise");
    assert!(scene.sim.as_ref().unwrap().zoom > 1.0, "not zoomed after the raise");
    // The release tick: the bit and the zoom target drop immediately; the lower (the raise
    // script reversed) is playing.
    scene.sim.as_mut().unwrap().step_bond(Input { aiming: false, zoom: true, ..Default::default() }, false, false, false);
    {
        let s = scene.sim.as_ref().unwrap();
        assert!(!s.aiming, "the aim bit must drop on the release tick");
        assert_eq!(s.zoom_target, 1.0, "the zoom target returns to 1.0 on the release tick");
        assert_eq!(s.view_script, 0x0600_07b9, "the lower plays the raise script");
        assert!(s.view_speed < 0.0, "the lower plays it REVERSED (no +0xBC on the sniper)");
    }
    // The lower takes the same 19 ticks; then the idle returns and stays.
    for t in 1..=n + 3 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        assert!(!s.aiming, "lower tick {t}: the aim bit rose during the lower");
        if t < n { assert_eq!(s.view_script, 0x0600_07b9, "lower tick {t}: the lower left early"); }
    }
    let s = scene.sim.as_ref().unwrap();
    assert_eq!(s.view_script, s.weapons.idle_script(0x24), "the idle did not return after the lower");
}

/// The auto-aim lock waits for the raise: `Player_AutoAim` 0x1a7c98 keeps SCANNING while the bit
/// is clear (its `Check_AutoAim` branch — with nothing in the cone it clears the lock pointer) and
/// only eases the lock toward the manual cursor once aimed (the bit set). Probe: seed a lock
/// pointer the scan can never re-locate before each raise tick — the scan clears it, the ease
/// preserves it. It survives only from the raise's last tick on.
#[test]
fn the_auto_aim_lock_eases_only_from_the_raises_last_tick() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_settle(&mut scene, 0x24, 3);
    scene.sim.as_mut().unwrap().auto_aim = true; // the game's default option [L ee_start_05 PlayerSetting+1]
    let n = raise_ticks(&scene, 0x24);
    let input = Input { aiming: true, ..Default::default() };
    for t in 1..=n + 1 {
        // Seed a lock pointer the scan can never re-lock (no drone has that index).
        scene.sim.as_mut().unwrap().drones.aim_target = Some(usize::MAX);
        scene.sim.as_mut().unwrap().step_bond(input.clone(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if t <= n {
            assert!(!s.aiming, "tick {t}: aimed too early");
            assert_eq!(s.drones.aim_target, None,
                "tick {t}: the ease branch ran during the raise — the scan must keep scanning until the bit sets");
        } else {
            assert!(s.aiming, "tick {t}: not aimed on the raise's last tick");
            assert_eq!(s.drones.aim_target, Some(usize::MAX),
                "tick {t}: the scan still ran on the raise's last tick — the lock must ease from here");
        }
    }
    // Sister: the eased lock survives while held (the ease branch never clears the pointer).
    scene.sim.as_mut().unwrap().step_bond(input, false, false, false);
    assert_eq!(scene.sim.as_ref().unwrap().drones.aim_target, Some(usize::MAX), "the lock did not survive while aimed");
}
