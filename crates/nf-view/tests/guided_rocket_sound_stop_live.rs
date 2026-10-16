//! guided-rocket-sound-stop live tests: every voice the Sentinel's guided launch starts is either
//! gone at the detonation or riding the rocket — none stays "where I was standing".
//!
//! Playtest 1015-1738 (07000005): "when the rocket finishes, I can still hear a sound being played
//! from where I was standing". s/rocket-sound-leak fixed the launch whoosh 0x10e (looped [L
//! SFXOutputData @0x2c94a0 entry 0x10e looped byte 1, inner 5 / outer 30]; handle BU+0xF0 [M
//! Bullet_init 0x12c5a0:277-282]; moved with the rocket per tick [M Bullet_update 0x12e7b0:98-108];
//! stopped at the delete [M Bullet_Delete 0x12e660:13-16 — disasm 0x12e680 `lw a0,0xf0(s2)` →
//! `jal Sound_Stop` → `sw zero,0xf0(s2)`], the same pass that restores the guided camera :22-31
//! `BL+0x878 = 0`, `BL+0x944 = 0xf`). This file pins the FULL sound surface of the guided launch,
//! swept at the instruction level (work/elf/ACTION.ELF, jal into the Sound_* functions):
//!
//! - the launch sound and the flight loop are ONE sound — the looped 0x10e whoosh; nothing else in
//!   `Bullet_init`/`Bullet_update`/`Bullet_Delete`/`Player_WeaponInitBullet` 0x1a4bf0 (the fire
//!   path, zero sound calls) starts a voice with a handle;
//! - the camera ride itself starts NOTHING: `GT_TakeControl` 0x181740 / `GT_LoseControl` 0x181408
//!   have no `jal` into any `Sound_*`; the ride's only sounds are `GT_Update` 0x180498's
//!   action-9 submunition fire — 0x4e7 / 0x10a at :346/:518, both UNLOOPED ([L] entry looped byte
//!   0), one-shots that reap themselves (the port zeroes the ride input [M 0x193600 case 10], so
//!   it starts neither);
//! - the detonation one-shots (`Bullet_CollisionHandler` 0x12ce80's 0x51c/0x13/0x15,
//!   `Bullet_Delete`'s 0x28d — the grapple arm only) are unlooped [L].
//!
//! So the game's contract: exactly one live voice from the launch — the whoosh — and it tracks the
//! ROCKET (not Bond), dying with the bullet's delete, i.e. during the 15-tick camera restore,
//! BEFORE the hand-back completes. Both tests fire through the real path (`load_dev("07000005.bin")`
//! → the Sentinel 0x2c given → `step_bond` fire → scene.rs's `fire_player` → the nf-audio driver).
//! RED on the pre-fix behaviour (fire-and-forget play3d, `snd: 0` on the bullet): the voice never
//! moves off the launch spot and outlives the delete.
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

fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

/// The EE pool's live 0x10e node count (`Sounds::live_ids`, duplicates counted).
fn ee_whoosh_nodes(scene: &nf_view::scene::Scene) -> usize {
    scene.sim.as_ref().unwrap().drones.audio.live_ids().iter().filter(|&&id| id == 0x10e).count()
}
/// The driver's live 0x10e item count (`Driver::ids`, one entry per item).
fn driver_whoosh_items(sound: &nf_audio::sfx::Sound) -> usize {
    sound.driver.ids().iter().filter(|&&id| id == 0x10e).count()
}

/// THE flight-loop contract: while the guided rocket flies, the launch whoosh's tracked position
/// moves WITH THE ROCKET (the game's per-tick `Sound_SetPosition(handle, obj)` [M Bullet_update
/// 0x12e7b0:104]) and NOT with Bond — it leaves the launch spot behind at the rocket's flight step.
/// Pre-fix (fire-and-forget) the voice sat at the muzzle forever: the playtest's "a sound being
/// played from where I was standing".
#[test]
fn the_flight_loop_tracks_the_rocket_not_bond() {
    let Some(mut scene) = load("07000005.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    // The Sentinel 'Guided' 0x2c: the only `+0x6C & 4` camera-bit carrier, ammo type 0x0e [L
    // ee_start_05.bin weapon_data @0x2bf150: +0x90 = 0x0e].
    if !equip(&mut scene, 0x2c) { panic!("the Sentinel (0x2c) could not be given on 07000005 (dev merge gap — 100a)"); }
    let mut sound = nf_view::build_sound(&mut scene).expect("the level has a bank");
    until_drawn(&mut scene, &mut sound, 0x2c, 600);

    // Fire through the real path and take the launch spot from the rocket's spawn position
    // (the whoosh's `from` is the same point [M Bullet_init 0x12c5a0 plays at the shooter]).
    let launch: [f32; 3];
    loop {
        scene.sim.as_mut().unwrap().step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        tick(&mut sound, &mut scene);
        if let Some(b) = scene.sim.as_ref().unwrap().drones.player_bullets.first() {
            launch = b.pos;
            break;
        }
    }

    // While it flies: every sampled tick the whoosh is on the rocket (one-tick lag ≈ 0.2525 u at
    // the game's flight step — the SetPosition runs at the tick HEAD, before the move), and by the
    // last sample it has LEFT the launch spot far behind (the voice does not stay with Bond).
    let mut samples = 0usize;
    let mut last_from_launch = 0.0f32;
    for _ in 0..200 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        tick(&mut sound, &mut scene);
        let sim = scene.sim.as_ref().unwrap();
        if let (Some(w), Some(b)) = (sound.driver.item_pos(0x10e), sim.drones.player_bullets.first().map(|b| b.pos)) {
            let drift = dist2(w, b);
            assert!(drift < 1.0, "the whoosh drifted {drift:.2} u² off the rocket ({w:?} vs {b:?}) — the voice is not riding the missile");
            last_from_launch = dist2(w, launch);
            samples += 1;
        }
        if sim.drones.player_bullets.is_empty() { break; }
    }
    assert!(samples >= 2, "the flight ended before two follow samples (the rocket died too fast for the ride check)");
    // 2.0 u² = ~1.4 u: 6 flight ticks at 0.2525 u/tick. A voice stuck at the launch spot (the old
    // behaviour) holds last_from_launch ≈ 0 and fails here.
    assert!(last_from_launch > 2.0,
        "after {samples} flight samples the whoosh still sits {last_from_launch:.2} u² from the launch spot — it tracks Bond, not the rocket");
}

/// THE hand-back contract: the whoosh dies with the BULLET's delete — the game stops it in
/// `Bullet_Delete` [M 0x12e660:13-16], the same pass that starts the guided camera's 15-tick
/// restore ([M :22-31 `BL+0x944 = 0xf`]) — so by the FIRST restore tick the voice is gone from
/// both layers, before the hand-back completes; and after the hand-back no voice the launch
/// started is still playing anywhere near the launch spot (the playtest symptom, swept over EVERY
/// live driver item, not just 0x10e — if any other launch-started voice leaked, this catches it).
#[test]
fn the_voice_dies_at_the_delete_not_at_the_handback() {
    let Some(mut scene) = load("07000005.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    if !equip(&mut scene, 0x2c) { panic!("the Sentinel (0x2c) could not be given on 07000005 (dev merge gap — 100a)"); }
    let mut sound = nf_view::build_sound(&mut scene).expect("the level has a bank");
    until_drawn(&mut scene, &mut sound, 0x2c, 600);

    // The driver's ids BEFORE the shot — the sweep at the end forgives only these (ambient beds
    // that were already live); anything new near the launch spot after the hand-back is a leak.
    let pre_fire_ids: std::collections::HashSet<u32> = sound.driver.ids().into_iter().collect();

    // Fire.
    let launch: [f32; 3];
    loop {
        scene.sim.as_mut().unwrap().step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        tick(&mut sound, &mut scene);
        if let Some(b) = scene.sim.as_ref().unwrap().drones.player_bullets.first() { launch = b.pos; break; }
    }

    // Fly to the death: the first tick whose sim has no bullets left is the DELETE tick (the flush
    // ran the game's delete pass at the end of it [M Bullet_Delete 0x12e660]).
    let mut deleted = false;
    let mut restore_seen = false;
    for _ in 0..3400 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        tick(&mut sound, &mut scene);
        if scene.sim.as_ref().unwrap().drones.player_bullets.is_empty() {
            deleted = true;
            // The camera restore is still counting ([M 0x12e660:28 `+0x944 = 0xf`, 15 ticks]) —
            // the hand-back has NOT completed, and the voice must already be gone.
            if scene.sim.as_ref().unwrap().missile_cam.is_some() {
                restore_seen = true;
                assert_eq!(driver_whoosh_items(&sound), 0,
                    "a live 0x10e item stayed in the driver after the bullet's delete, while the camera restore is still counting — the stop moved off the delete");
                assert_eq!(ee_whoosh_nodes(&scene), 0,
                    "a 0x10e node stayed live in the EE pool after the bullet's delete — the stop never ran at the delete");
            }
            break;
        }
    }
    assert!(deleted, "the guided rocket never died");
    // The ordering pin: the delete tick came while the camera was still up — the restore window
    // existed, so the assertions above were not skipped vacuously.
    assert!(restore_seen, "the delete tick and the hand-back coincided — the restore window never existed");

    // The hand-back completes (the restore elapses, missile_cam None — the game's substate-10 exit).
    let mut handed_back = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        tick(&mut sound, &mut scene);
        if scene.sim.as_ref().unwrap().missile_cam.is_none() { handed_back = true; break; }
    }
    assert!(handed_back, "the guided camera never handed back");

    // THE sweep (the playtest symptom, any id): no voice the launch started is still playing near
    // where Bond was standing. Every live driver item that was not live before the shot must sit
    // away from the launch spot; 2.0 u² = ~1.4 u around it.
    let mut checked = 0usize;
    for id in sound.driver.ids() {
        if pre_fire_ids.contains(&id) { continue; }
        if let Some(p) = sound.driver.item_pos(id) {
            let d2 = dist2(p, launch);
            assert!(d2 > 2.0,
                "a live voice id {id:#x} still plays {d2:.2} u² from the launch spot after the detonation and the hand-back — the leak");
            checked += 1;
        }
    }
    // And the strict 0x10e form, both layers.
    assert_eq!(driver_whoosh_items(&sound), 0, "a 0x10e item survived the whole ride in the driver");
    assert_eq!(ee_whoosh_nodes(&scene), 0, "a 0x10e node survived the whole ride in the EE pool");
    let _ = checked;
}
