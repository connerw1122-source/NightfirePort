//! rocket-sound-leak live tests: the guided (AT-420 Sentinel 0x2c) rocket's launch whoosh 0x10e.
//!
//! Playtest 1015-1738: after a guided rocket finishes, "I can still hear a sound being played from
//! where I was standing when I shot the rocket, and it doesn't stop, and it can stack up the more
//! times I shoot rockets".
//!
//! The game's lifecycle [M]: `Bullet_init` 0x12c5a0:277-282 plays the whoosh 0x10e for every
//! ammo-type-0x0e weapon and stores its handle at BU+0xF0; `Bullet_update` 0x12e7b0:98-108 moves the
//! voice with the rocket every tick (`Sound_SetPosition`) and clears the handle when the driver
//! finished it; `Bullet_Delete` 0x12e660:13-16 stops it on the object's delete (disasm 0x12e680
//! `lw a0, 0xf0(s2)` → `jal Sound_Stop` → `sw zero, 0xf0(s2)`); the guided camera's restore rides the
//! same delete pass. 0x10e is LOOPED ([L ACTION.ELF SFXOutputData @0x2c94a0 entry 0x10e: looped 1,
//! inner 5 / outer 30]) — the EE pool frees a looped node only on `Sound_Stop`, so the port's old
//! fire-and-forget `play3d` left one looping whoosh at the muzzle per shot, stacking per rocket.
//!
//! The requests flow through the real path: `nf_view::load_dev` → `Sim::step_bond` (draw / fire) →
//! `Sounds::update` (the EE DYNAMICSOUNDS pool) → `nf_view::sound_tick` → the nf-audio driver.
//! Assertions are numeric: the driver's live item count for 0x10e (one during each flight, ZERO after
//! each death — pre-fix it stacked k after k rockets and never dropped), and the item's tracked
//! position follows the bullet (`Driver::item_pos` vs `sim.drones.player_bullets[0].pos`).
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn load(archive: &str) -> Option<nf_view::scene::Scene> {
    // Never through the speakers (the weapon-sound tests' rule): NF_SILENT makes `build_sound`
    // skip the real output device. SAFETY: set before any audio thread starts.
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let iso = std::env::var("NF_ISO").ok()?;
    Some(nf_view::load_dev(&iso, archive, false).expect("level loads (dev: foreign weapon scripts merge)"))
}

/// One game tick through the real sound path, then the headless stand-in for the SPU's end reports
/// (with no output device the port's voices never report their end, so mark them all ended; one-shots
/// are reaped by the next `SFXUpdate` exactly as on hardware, looped samples never end [M sound.md §6]
/// — a looped item only ever dies on the EE's `SFXRemove`, which is exactly the signal under test).
fn tick(sound: &mut nf_audio::sfx::Sound, scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    nf_view::sound_tick(sound, sim);
    let all: Vec<u32> = (0..nf_audio::sfx::HW_VOICES as u32).collect();
    let mut out = Vec::new();
    sound.driver.update(&mut out, &all);
}

fn equip(scene: &mut nf_view::scene::Scene, id: usize) -> bool {
    let sim = scene.sim.as_mut().unwrap();
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(id).max(1);
    if sim.drones.inventory.equip_weapon(&wd, id, rounds, true) {
        sim.drones.inventory.equip_ammo(&wd, id, (rounds * 3).max(rounds) * 4);
        sim.gun.switch_request = id as u8;
        true
    } else { false }
}

/// Wait for the draw to finish (the weapon-sound tests' rule: the weapon byte is the requested one,
/// the switch state machine and the reload states are clear).
fn until_drawn(scene: &mut nf_view::scene::Scene, sound: &mut nf_audio::sfx::Sound, want: usize, max: usize) {
    for t in 0..max {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        tick(sound, scene);
        let gun = &scene.sim.as_ref().unwrap().gun;
        if t >= 2 && gun.weapon as usize == want && gun.switch_state == 0 && gun.reload_state == 0 { break; }
    }
}

/// The EE pool's live 0x10e node count (`Sounds::live_ids`, duplicates counted) — the leak's other
/// half: a looped node sits in the pool until `Sound_Stop`.
fn ee_whoosh_nodes(scene: &nf_view::scene::Scene) -> usize {
    scene.sim.as_ref().unwrap().drones.audio.live_ids().iter().filter(|&&id| id == 0x10e).count()
}
/// The driver's live 0x10e item count (`Driver::ids`, one entry per item — duplicates are the stack).
fn driver_whoosh_items(sound: &nf_audio::sfx::Sound) -> usize {
    sound.driver.ids().iter().filter(|&&id| id == 0x10e).count()
}

/// THE repro: fire 3 guided rockets (one per ride — the trigger is dead while the camera rides the
/// missile [M Player_Update's substate-10 gate]), let each die, and after every death assert NO
/// leftover 0x10e anywhere: not in the driver (`Driver::ids`) and not in the EE pool
/// (`Sounds::live_ids`). Pre-fix each rocket left a permanent looping whoosh at the muzzle and the
/// counts stacked 1, 2, 3; post-fix the count returns to 0 after every death. During each flight the
/// whoosh is exactly one live item, and its tracked position follows the bullet [M Bullet_update
/// 0x12e7b0:104's `Sound_SetPosition`].
#[test]
fn three_guided_rockets_leave_no_looping_whoosh_behind() {
    let Some(mut scene) = load("07000005.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    // The Sentinel 'Guided' 0x2c: the only `+0x6C & 4` camera-bit carrier, ammo type 0x0e [L
    // ee_start_05.bin weapon_data @0x2bf150: +0x90 = 0x0e].
    if !equip(&mut scene, 0x2c) { panic!("the Sentinel (0x2c) could not be given on 07000005 (dev merge gap — 100a)"); }
    let mut sound = nf_view::build_sound(&mut scene).expect("the level has a bank");
    until_drawn(&mut scene, &mut sound, 0x2c, 600);

    for rocket in 0..3 {
        // Hold fire until the shot leaves (the bullet list gains the rocket).
        let mut fired = false;
        for _ in 0..40 {
            scene.sim.as_mut().unwrap().step_bond(Input { fire: true, ..Input::default() }, true, false, false);
            tick(&mut sound, &mut scene);
            if !scene.sim.as_ref().unwrap().drones.player_bullets.is_empty() { fired = true; break; }
        }
        assert!(fired, "rocket {rocket}: the Sentinel never fired");
        // The launch whoosh starts and is exactly ONE live item (pre-fix this held for the first
        // rocket too — the stacking shows on rockets 2 and 3, which the tail of this loop asserts).
        let mut started = false;
        for _ in 0..10 {
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
            tick(&mut sound, &mut scene);
            if driver_whoosh_items(&sound) == 1 { started = true; break; }
        }
        assert!(started, "rocket {rocket}: the launch whoosh 0x10e never started a live driver item");
        assert_eq!(ee_whoosh_nodes(&scene), 1, "rocket {rocket}: the EE pool holds exactly one whoosh node in flight");

        // The whoosh follows the rocket [M Bullet_update 0x12e7b0:104]: sample the tracked position
        // and the bullet's position on the way out; each tick the whoosh moves with the rocket
        // (ramping from the quarter speed to 0.2525 u/tick [L] — rocket-ramp) and never drifts off it.
        let (mut p0, mut b0): (Option<[f32; 3]>, Option<[f32; 3]>) = (None, None);
        let mut samples = 0;
        for _ in 0..200 {
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
            tick(&mut sound, &mut scene);
            let sim = scene.sim.as_ref().unwrap();
            if let (Some(w), Some(b)) = (sound.driver.item_pos(0x10e), sim.drones.player_bullets.first().map(|b| b.pos)) {
                if let (Some(w0), Some(b0)) = (p0, b0) {
                    let drift = (w[0] - b[0]).powi(2) + (w[1] - b[1]).powi(2) + (w[2] - b[2]).powi(2);
                    let moved = (w[0] - w0[0]).powi(2) + (w[1] - w0[1]).powi(2) + (w[2] - w0[2]).powi(2);
                    // A stuck whoosh moves 0; the rocket's slowest tick is its quarter-speed start
                    // (0.0625 u = 0.0039 u², then +2%/tick [M Bullet_DoTrails 0x12d890:134-138] —
                    // rocket-ramp), so 0.03 u/tick (0.0009 u²) separates the two.
                    assert!(moved > 0.0009, "rocket {rocket}: the whoosh never moved (tracked {w:?} vs {w0:?}) — the game's Sound_SetPosition per tick is missing");
                    assert!(drift < 25.0, "rocket {rocket}: the whoosh drifted {drift:.1} u² off the rocket ({w:?} vs {b:?})");
                    samples += 1;
                }
                p0 = Some(w); b0 = Some(b);
            }
            if scene.sim.as_ref().unwrap().drones.player_bullets.is_empty() { break; }
        }
        assert!(samples >= 2, "rocket {rocket}: the flight ended before two follow samples (the rocket died too fast for the ride check)");

        // Let it die: the ride ends, the 15-tick restore elapses [M Bullet_Delete 0x12e660:28's
        // `+0x944 = 0xf`], the EE stop flows and the driver's Remove lands. The bound is the flight
        // time (the 0.2525 u/tick over the 200-unit range ≈ 790 ticks [L record +0x20; the
        // fix-sentinel-camera-play scale]) plus margin — the range death is silent, so the wait adds
        // no blast near Bond.
        let mut died = false;
        for _ in 0..3400 {
            let sim = scene.sim.as_ref().unwrap();
            if sim.drones.player_bullets.is_empty() && sim.missile_cam.is_none() { died = true; break; }
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
            tick(&mut sound, &mut scene);
        }
        assert!(died, "rocket {rocket}: the guided ride never ended");
        // THE assertion (the playtest symptom): nothing left behind, in either layer.
        assert_eq!(driver_whoosh_items(&sound), 0,
            "rocket {rocket}: a live looping 0x10e item stayed in the driver after the rocket died — the leak");
        assert_eq!(ee_whoosh_nodes(&scene), 0,
            "rocket {rocket}: a 0x10e node stayed live in the EE pool after the rocket died — the stop never ran");
    }
}
