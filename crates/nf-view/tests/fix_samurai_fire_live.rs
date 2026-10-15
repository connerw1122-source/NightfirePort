//! fix-samurai-fire: the Phoenix Samurai (0x32 Overcharge / 0x33 Beam — a LASER RIFLE, not a sword)
//! fires nothing on 16 of 18 levels (weapon-matrix4 red #1).
//!
//! The game's path [M]: the mode press lands the twin through `Player_WeaponHasAmmo` 0x1a16c0, whose
//! clip half is `Player_AmmoInGun` 0x1a9190 — a twin (+0x05 != 0) reads the BASE's clip slot (record
//! +0x02) unless the +0x90 bytes differ. The Samurai's give fills clip[0x33] (the base 0x33's slot;
//! both records' +0x90 = 0x1f [L ee_start_05.bin @ 0x2bf150]), so the Overcharge twin's dry check
//! must read [0x33] — reading its own (always 0) made the mode press forever inert and every fire
//! cell empty. The overcharge bolt spawns when the 380-frame charge script 0x060007c2 crosses the
//! release frame +0x44 = 280 [M Player_SetWeaponAnimObj case 9 0x1a2e38:404-421; the first pull
//! spends the 100-round clip through Player_RoundToFire 0x1a18e0's twin-slot read], flies at 4.25
//! u/tick (range 500, radius 8, damage 256 [L +0x20/+0x1C/+0x08/+0x0C]) and blasts on ANY
//! non-ricochet contact — the small-blast arms are the collision handler's shared tail [M
//! Bullet_CollisionHandler 0x12ce80:207-216, `+0x70 & 0x100000`, only 0x32 carries it (L sweep)].
//! The beam (0x33) spawns on the fire tick every 30 ticks [M 0x1a5460:640-698: +0x44 == 0 → the
//! press-tick spawn; +0x40 = 30], spends 12/shot [L +0x91], and both rounds carry the `+0x6C &
//! 0x10000` Swoosh ribbon [M Bullet_init 0x12c5a0:116-133 — texture 0x20005cd for base 0x33, the
//! live capture's "a short cyan beam"].
//!
//! Every test drives the REAL input path (`nf_view::load_weapons` + the panel give + `Sim::step_bond`).

use nf_game::player::Input;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[0x32, 0x33]).expect("load 07000005");
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

/// Gives 0x32 through the panel (the give fills the BASE's clip slot [M Player_EquipWeapon
/// 0x1a8a58]), selects the base 0x33 (owned[] is per base — the direct twin select is refused
/// [M 0x1a8a58:39-41]) and settles to the idle view.
fn give_and_reach_base(scene: &mut Scene) {
    let (status, sw) = nf_view::panel::apply(scene, &[
        ("do".to_string(), "weapon".to_string()),
        ("id".to_string(), "0x32".to_string()),
    ]);
    assert_eq!(sw, Some(0x32), "give refused: {status}");
    for _ in 0..400 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x33 { break; }
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x33), ..Default::default() }, false, false, false);
    }
    for _ in 0..7200 {
        let s = scene.sim.as_ref().unwrap();
        if idle(scene) && s.view_script == s.weapons.idle_script(0x33) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    let s = scene.sim.as_ref().unwrap();
    assert!(idle(scene) && s.view_script == s.weapons.idle_script(0x33), "the base 0x33 never settled");
}

/// The mode press from the base lands the Overcharge twin 0x32: the dry check reads the BASE's
/// clip slot [M Player_AmmoInGun 0x1a9190]. FAILED BEFORE THE FIX: the press was inert (the port's
/// has_ammo read the twin's own, never-filled slot) — the Samurai was unreachable by any player
/// route and every fire cell measured empty (weapon-matrix4 red #1, 16 of 18 levels).
#[test]
fn the_mode_press_reaches_the_overcharge_twin() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_reach_base(&mut scene);
    // The record facts [L ee_start_05.bin @ 0x2bf150].
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.weapons.base(0x32), 0x33, "0x32 bases to 0x33");
        assert_eq!(s.weapons.ammo_type(0x32), s.weapons.ammo_type(0x33), "the +0x90 bytes match — the shared slot");
        assert_eq!(s.drones.inventory.clip.get(0x33).copied(), Some(100), "the give filled the BASE's slot");
        assert_eq!(s.drones.inventory.clip.get(0x32).copied(), Some(0), "the twin's own slot is empty");
    }
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    for _ in 0..7200 {
        let s = scene.sim.as_ref().unwrap();
        if s.gun.weapon == 0x32 && idle(&scene) && s.view_script == s.weapons.idle_script(0x32) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x32, "the mode press must land the Overcharge twin 0x32");
    // Sister: the press goes straight back to the base (0x32's twin byte +1 → 0x33 [L +0x05]).
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    for _ in 0..7200 {
        let s = scene.sim.as_ref().unwrap();
        if s.gun.weapon == 0x33 && idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x33, "the mode press returns to the base 0x33");
    // Sister 2: the plain twins' mode press is unaffected by the shared-slot read — the P2K's
    // 0x02 → 0x03 still lands (the fire_modes3 suite re-runs its own numbers too).
}

/// Holding the Overcharge charges the 280-frame release script and the bolt flies and blasts on
/// contact. FAILED BEFORE THE FIX: the mode press was inert (no 0x32 to hold) — and the bolt's
/// blast never fired on a flesh contact (the port's small-blast gate sat only on its world arm,
/// while the game's gates are the handler's shared tail [M 0x12ce80:186-231]).
#[test]
fn the_overcharge_bolt_flies_and_blasts_on_contact() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_reach_base(&mut scene);
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    for _ in 0..7200 {
        let s = scene.sim.as_ref().unwrap();
        if s.gun.weapon == 0x32 && idle(&scene) && s.view_script == s.weapons.idle_script(0x32) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x32, "0x32 reached");
    // Hold the trigger through the full charge (280 frames at 1.0 = 280 ticks) and past the spawn.
    let mut spawned_tick = 0;
    let mut ex0 = scene.sim.as_ref().unwrap().drones.explosions.len();
    let mut blast_tick = 0;
    for t in 1..=460 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        let s = scene.sim.as_ref().unwrap();
        if spawned_tick == 0 && (s.shot_tick || !s.drones.player_bullets.is_empty()) { spawned_tick = t; }
        if blast_tick == 0 && s.drones.explosions.len() > ex0 {
            blast_tick = t;
            let e = *s.drones.explosions.last().unwrap();
            assert_eq!(e.4, 0x32, "the blast is the Samurai bolt's (weapon 0x32)");
            assert!((e.1 - 8.0).abs() < 1e-3 && (e.2 - 256.0).abs() < 1e-3,
                "the blast carries the record's radius 8 / damage 256 [L +0x08/+0x0C]");
        }
        ex0 = s.drones.explosions.len();
        if t < 280 {
            assert!(!s.shot_tick && s.drones.player_bullets.is_empty(),
                "tick {t}: the bolt spawned before the release frame 280 [M 0x1a2e38:404-421]");
        }
    }
    assert_eq!(spawned_tick, 280, "the bolt spawns exactly when the script crosses the release frame +0x44 = 280 (got tick {spawned_tick})");
    assert!(blast_tick > 0, "the bolt never blasted on contact — the charged shot must explode where it lands");
    assert!(blast_tick > spawned_tick && blast_tick <= spawned_tick + 150,
        "the blast lands within the bolt's flight (spawn {spawned_tick}, blast {blast_tick})");
}

/// The Beam mode fires on the press tick every 30 ticks, spends 12 charge per shot and leaves the
/// cyan swoosh trail behind the round [M 0x1a5460:640-698; Bullet_init 0x12c5a0:116-133]. FAILED
/// BEFORE THE FIX: the trail did not exist (the swoosh system was never ported), so the beam was
/// invisible — the live capture's "a short cyan beam from the barrel".
#[test]
fn the_beam_draws_its_trail_to_the_wall() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_reach_base(&mut scene);
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x33, "the Beam base 0x33 held");
    {
        let s = scene.sim.as_ref().unwrap();
        let w = s.weapons.records.get(0x33).unwrap();
        assert_eq!(u16::from_le_bytes([w[0x44], w[0x45]]), 0, "the Beam has no release frame — the press-tick spawn [M 0x1a5460:688-690]");
        assert_eq!(w[0x91], 12u8, "the Beam spends 12 per shot [L +0x91]");
        assert_eq!(u16::from_le_bytes([w[0x40], w[0x41]]), 30, "the Beam's auto interval is 30 ticks [L +0x40]");
        assert!(u32::from_le_bytes(w[0x6c..0x70].try_into().unwrap()) & 0x1_0000 != 0, "the Beam round carries the Swoosh bit [L +0x6C = 0x10000]");
    }
    // Hold the trigger 100 ticks: shots on ticks 1/31/61/91 (the interval 30), 12 charge each.
    let mut shot_ticks = Vec::new();
    let mut saw_trail = false;
    let mut clip_seen = Vec::new();
    for t in 1..=100 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        let s = scene.sim.as_ref().unwrap();
        if s.shot_tick { shot_ticks.push(t); }
        if !s.drones.gas.puffs.is_empty() { saw_trail = true; }
        if t == 12 || t == 42 { clip_seen.push(s.drones.inventory.clip.get(0x33).copied().unwrap_or(-1)); }
    }
    assert_eq!(shot_ticks, vec![1, 31, 61, 91], "the beam fires on the press tick then every 30 ticks [M 0x1a5460]");
    assert_eq!(clip_seen, vec![88, 76], "12 charge per shot [L +0x91] (100 → 88 → 76 → …)");
    assert!(saw_trail, "the swoosh trail never drew — the beam must draw to the wall [M 0x12c5a0:116-133]");
    // Sister: the trail puffs ride the round — they exist only while a round is alive or just died
    // (the gas pool's 6-tick life), so a held beam keeps some alive the whole hold.
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    assert!(!scene.sim.as_ref().unwrap().drones.gas.puffs.is_empty(), "the held beam keeps its trail alive");
}
