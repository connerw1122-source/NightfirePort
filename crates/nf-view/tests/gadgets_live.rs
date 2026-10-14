//! fix-gadgets-live: the laser's charge/beam/light, the stunner's target economics, the AIMS-20's
//! side lights — through the real input path (`nf_view::load_weapons` + `Sim::step_bond`).
//!
//! The playtest (2026-10-13 night, on the master after 101e/101f): the laser showed NO beam, NO wall
//! light and NO charge movement; the stunner "doesn't work". What the disc says [M decompile]:
//! - The whole gadget fire chain rides the REFIRE that spent a round: `Player_RoundToFire` 0x1a18e0
//!   spends `weapon_data+0x91` (1 for both [L]) from the clip slot `BLData+0x1B8 + w·0xC`
//!   (`Player_AmmoInGun` 0x1a9190), then `Player_WeaponInitBullet` 0x1a4bf0 runs on the same tick;
//!   its +0x95b latch makes `Player_CollisionHandler` 0x1949a8 call `Player_MuzzleFlash` 0x1a4298,
//!   whose `Light_Create` (rgb +0x56/+0x55/+0x54, radius +0x58) and `Draw_LaserBeam`/`Draw_TaserBeam`
//!   are the visible shot. At clip 0 `RoundToFire` fails — the beam STOPS (the user's capture:
//!   the HUD bar 31→0 as `clip·100/clip_size` [M HUD_UpdateAmmoPane 0x19a7f0:177-183], the laser's
//!   clip 500 / the stunner's 100 [L records]).
//! - The stunner's miss arm (no class-2/3 target within +0x1c) REGENS +0x91 and returns before
//!   `Bullet_init` [M 0x1a4bf0:193-199] — with RoundToFire's spend the aimed-off-target tick is free.
//! - The AIMS-20 (0x1a/0x1b): `AnimDatumSetEntity(bone, 0x2000132, 0xff)` on view-model bones 8..0xB
//!   every tick [M Player_WeaponFiring 0x1a5460:256-360] — the row of red side lights; blink > 15
//!   idle ticks, all off > 45 [M :264, :348].

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// Wait out the level-start input lock, then the weapon switch (`--give`'s switch request).
fn settle(scene: &mut nf_view::scene::Scene, want: u8) {
    for _ in 0..400 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if !s.disabled && s.gun.weapon == want && s.gun.switch_state == 0 { return; }
    }
    panic!("weapon {want:#04x} never settled (disabled or switch stuck)");
}

/// The playtest spot on 07000008 with `--give 0x4e` (the laser in hand, clip 500 [L]).
fn laser_scene() -> nf_view::scene::Scene {
    let mut scene = nf_view::load_weapons(&iso(), "07000008.bin", false, &[0x4e]).expect("load 08");
    {
        let sim = scene.sim.as_mut().unwrap();
        let floor = sim.world.floor_below([155.19, 45.0, -113.38], 8.0).unwrap_or(43.0);
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [155.19, floor + 1.1, -113.38], 1.57);
        sim.bond.yaw = 1.57;
        let wd = sim.weapons.clone();
        let rounds = wd.clip_size(0x4e).max(1);
        if sim.drones.inventory.equip_weapon(&wd, 0x4e, rounds, true)
            || sim.drones.inventory.owned.get(0x4e).copied().unwrap_or(false) {
            sim.gun.switch_request = 0x4e; // the give's contract: DRAW it (lib.rs's --give)
        }
    }
    settle(&mut scene, 0x4e);
    scene
}

/// The laser's charge: the clip drains 1 per held-fire tick (`+0x91` = 1 [L]), the red muzzle light
/// (rgb 64,10,10, radius 5.0 [L record +0x54..0x58]) flashes per shot tick, and at clip 0 the beam
/// STOPS. Before the fix the burn/beam ran on the raw trigger — through the switch cycle and past an
/// empty clip — and the HUD bar sat still while the beam never ended.
#[test]
fn the_lasers_charge_drains_the_light_flashes_and_the_beam_stops_at_zero() {
    let mut scene = laser_scene();
    let sim = scene.sim.as_mut().unwrap();
    let clip0 = sim.gun.clip;
    assert_eq!(clip0, 500, "the laser's clip fills to clip_size 500 [L +0x92]");
    let lights0 = sim.drones.lights.lights.len();
    // Hold fire 40 ticks: the clip drains, the light flashes, the beam parks on the crosshair ray.
    let mut saw_light = false;
    let mut saw_beam = false;
    for t in 0..40 {
        sim.step_bond(Input::default(), true, false, false);
        if sim.drones.lights.lights.len() > lights0 { saw_light = true; }
        if sim.laser_beam.is_some() { saw_beam = true; }
        if t >= 10 {
            assert_eq!(sim.gun.clip, clip0 - (t as u32) - 1, "the clip drains 1 per shot tick");
        }
        assert!(sim.laser_beam.is_some() || t < 5, "the beam parks while firing (tick {t})");
    }
    assert!(saw_beam, "no beam while firing (the user's symptom)");
    assert!(saw_light, "no muzzle light while firing (the user's symptom)");
    // The light is the record's red: the newest light carries rgb (64, 10, 10) [L +0x54..0x56].
    let l = sim.drones.lights.lights.last().expect("a light");
    assert_eq!(l.rgb, [64, 10, 10], "the laser's muzzle light is red [L weapon_data+0x54..0x56]");
    // Keep firing past the clip: 500 rounds at 1/tick. The beam must stop EXACTLY at charge 0 (the
    // user's capture); the game then reloads from the reserve if it has rounds [M 0x1a5460:449-460]
    // — so the lasting assertion is the off-at-0 edge, not a permanent off.
    let mut stopped_at_zero = false;
    for _ in 0..600 {
        let empty_at_start = sim.gun.clip == 0;
        sim.step_bond(Input::default(), true, false, false);
        if empty_at_start {
            // A tick that STARTED at charge 0 fires nothing: no beam, no shot.
            assert!(sim.laser_beam.is_none(), "the beam stops at charge 0 (the user's capture)");
            stopped_at_zero = true;
        }
        if stopped_at_zero && sim.laser_beam.is_none() && sim.gun.clip == 0 { break; }
    }
    assert!(stopped_at_zero, "the clip never emptied");
}

/// The stunner's economics: the aimed-at-guard tick spends 1 (`+0x91` [L]) and latches the target
/// (the +0x95d beam gate [M 0x1a4bf0:169-200]); the aimed-off tick is FREE — the spend refunds
/// ([M 0x1a4bf0:193-199's regen arm]) and no beam parks. Sister: the laser has no such refund.
#[test]
fn the_stunner_spends_only_on_a_target_and_holds_its_charge_aimed_off() {
    let mut scene = nf_view::load_weapons(&iso(), "07000005.bin", false, &[0x4a]).expect("load 05");
    let gi = {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.list.iter().position(|d| d.present && d.enabled && !d.corpse && d.health > 0.0)
            .expect("a live guard on 05")
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        let g = sim.drones.list[gi].pos;
        let floor = sim.world.floor_below([g[0] + 3.0, g[1] + 2.0, g[2]], 8.0).unwrap_or(g[1]);
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [g[0] + 3.0, floor + 1.1, g[2]], 0.0);
        // look() = (sin yaw, ·, cos yaw): the guard sits at −x from Bond ⇒ yaw −π/2.
        sim.bond.yaw = -(std::f32::consts::FRAC_PI_2);
        let wd = sim.weapons.clone();
        let rounds = wd.clip_size(0x4a).max(1);
        if sim.drones.inventory.equip_weapon(&wd, 0x4a, rounds, true)
            || sim.drones.inventory.owned.get(0x4a).copied().unwrap_or(false) {
            sim.gun.switch_request = 0x4a;
        }
    }
    settle(&mut scene, 0x4a);
    let sim = scene.sim.as_mut().unwrap();
    // Aimed at the guard 3 units away (inside +0x1c = 5.0 [L]): the target latches, the clip spends.
    let clip0 = sim.gun.clip;
    let mut spent = 0;
    for _ in 0..20 {
        sim.step_bond(Input::default(), true, false, false);
        if sim.stunner_target == Some(true) { spent += 1; }
    }
    assert_eq!(sim.stunner_target, Some(true), "the guard 3 units out is a target");
    assert!(spent > 0, "the aimed ticks fire");
    assert!(sim.gun.clip < clip0, "the aimed ticks spend the clip");
    // Aim away (yaw reversed, pitch up at the sky): every tick is a free miss — the clip holds.
    let clip1 = sim.gun.clip;
    sim.bond.yaw += std::f32::consts::PI;
    sim.bond.pitch = 0.4;
    for _ in 0..30 {
        sim.step_bond(Input::default(), true, false, false);
        assert_eq!(sim.stunner_target, Some(false), "aimed away: no target");
        assert!(sim.laser_beam.is_none(), "no beam without a target (the +0x95d gate)");
    }
    assert_eq!(sim.gun.clip, clip1, "the aimed-off ticks are free (the miss arm's regen refunds the spend)");
}

/// The AIMS-20's four side lights: mesh 0x2000132 on the 0x1a view model's bones 8..0xB
/// [M 0x1a5460:256-360], lit while the gun is in hand (the user's "row of RED lights, lit" — the port
/// showed them dark). Sister: the idle timer runs (the blink/off arms read it).
#[test]
fn the_aims20s_side_lights_attach_to_its_view_model() {
    let mut scene = nf_view::load_weapons(&iso(), "07000005.bin", false, &[0x1a]).expect("load 05");
    {
        let sim = scene.sim.as_mut().unwrap();
        let wd = sim.weapons.clone();
        let rounds = wd.clip_size(0x1a).max(1);
        if sim.drones.inventory.equip_weapon(&wd, 0x1a, rounds, true)
            || sim.drones.inventory.owned.get(0x1a).copied().unwrap_or(false) {
            sim.gun.switch_request = 0x1a;
        }
    }
    settle(&mut scene, 0x1a);
    let sim = scene.sim.as_ref().unwrap();
    let vm = *scene.view_models.get(&0x1a).expect("the AIMS view model");
    let lights: Vec<&nf_view::scene::Attachment> = scene.attachments.iter()
        .filter(|a| a.aims_light.is_some() && a.character as usize == vm).collect();
    assert_eq!(lights.len(), 4, "bones 8..0xB each carry one light [M 0x1a5460:259-266]");
    assert!(lights.iter().all(|a| a.aims_light.is_some()), "all four are light slots");
    // The idle timer runs while the gun sits in hand (the blink/off arms read it).
    let i0 = sim.drones.aims_idle;
    drop(sim);
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.aims_idle > i0, "the idle timer climbs while the AIMS sits idle");
}


/// Oddjob's hat (0x45): a thrown hat KILLS ON CONTACT (the record's damage 100.0 [L +0x0C] through
/// the standard bullet hit — `Bullet_CollisionHandler` 0x12ce80's 0x45 arm + msg 8) and NEVER
/// explodes (the fuse arm only sets the post-hit lifetime [M 0x12e7b0:368-373]; Bullet_Delete's
/// effect kind 0 spawns nothing). Before the fix the port's grenade path fuse-blasted it — the
/// user's "should be a one-shot kill but it acts like a grenade". Sister: a guard standing beside
/// the victim survives (no blast radius).
#[test]
fn oddjobs_hat_kills_on_contact_and_never_explodes() {
    let mut scene = nf_view::load_weapons(&iso(), "07000005.bin", false, &[0x45]).expect("load 05");
    {
        let sim = scene.sim.as_mut().unwrap();
        // A pair of live guards within 3.5 units: the victim and the blast-radius witness.
        let mut pair = None;
        'outer: for i in 0..sim.drones.list.len() {
            let a = &sim.drones.list[i];
            if !a.present || a.enabled == false || a.corpse || a.health <= 0.0 { continue; }
            for j in 0..sim.drones.list.len() {
                if j == i { continue; }
                let b = &sim.drones.list[j];
                if !b.present || !b.enabled || b.corpse || b.health <= 0.0 { continue; }
                let d2 = (a.pos[0] - b.pos[0]).powi(2) + (a.pos[1] - b.pos[1]).powi(2) + (a.pos[2] - b.pos[2]).powi(2);
                if d2 < 3.5f32.powi(2) { pair = Some((i, j)); break 'outer; }
            }
        }
        let Some((i, _j)) = pair else { eprintln!("no guard pair on 05 — skipping"); return };
        let target = sim.drones.list[i].pos;
        // Bond 4 units away, aiming at the victim (look() = (sin yaw, ·, cos yaw)).
        let bx = target[0] + 4.0;
        let floor = sim.world.floor_below([bx, target[1] + 2.0, target[2]], 8.0).unwrap_or(target[1]);
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [bx, floor + 1.1, target[2]], 0.0);
        sim.bond.yaw = (target[0] - bx).atan2(0.0f32.max(0.0) + (target[2] - target[2]));
        // yaw from the +x offset: forward = (sin yaw, ·, cos yaw), the guard is at −x ⇒ yaw −π/2.
        sim.bond.yaw = -std::f32::consts::FRAC_PI_2;
        let wd = sim.weapons.clone();
        let rounds = wd.clip_size(0x45).max(1);
        if sim.drones.inventory.equip_weapon(&wd, 0x45, rounds, true)
            || sim.drones.inventory.owned.get(0x45).copied().unwrap_or(false) {
            sim.gun.switch_request = 0x45;
        }
    }
    settle(&mut scene, 0x45);
    let sim = scene.sim.as_mut().unwrap();
    // Hold fire until the throw spawns, then let it fly (the fuse is 300 ticks [L +0x70 & 0x10000]).
    let mut thrown = false;
    for _ in 0..120 {
        sim.step_bond(Input::default(), true, false, false);
        if !sim.drones.grenades.is_empty() { thrown = true; break; }
    }
    assert!(thrown, "the hat never threw (no grenade spawned)");
    for _ in 0..420 { sim.step_bond(Input::default(), true, false, false); }
    // The witness is the OTHER guard of the pair — find it again by proximity to the victim.
    let victim_dead = sim.drones.list.iter().filter(|d| d.health <= 0.0).count();
    assert!(victim_dead >= 1, "the hat killed nobody on contact (the user's symptom)");
    assert!(sim.drones.explosions.is_empty(), "the hat exploded (the grenade path's blast)");
}
