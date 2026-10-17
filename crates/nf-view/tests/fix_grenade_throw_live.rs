//! fix-grenade-throw — the grenade's throw timing, the blast scale, the bounce sounds and the smoke,
//! driven through the REAL path (`nf_view::load` + `Sim::step_bond` with the trigger input).
//!
//! The grenade assets (the fp skin + the view scripts 0x0600049c/d/e) merge only for the weapons the
//! load pre-gives (`Wanted::Ids`, lib.rs weapon_wanted) — the tests load with the grenade in the give
//! list, the same path the dev menu's lazy give reload takes.
//!
//! Evidence:
//! - The throw's release frame: `weapon_data+0x44` = 26 for the grenades [L record 0x34]; the round
//!   spawns when the fire script's frame reaches it, latched at obj+0x95C [M Player_SetWeaponAnimObj
//!   case 9 0x1a2e38:416-420]. The fire script 0x0600049d is the 70-frame wind-up whose op-1 plays the
//!   pin sound (0x1e4) at its frame 16 [D 07000026.bin] — the throw comes AFTER the pin pull. The
//!   refire interval is `weapon_data+0x40` = 63 [L record 0x34], set at the shot [M 0x1a5460:652-654].
//! - The blast: `Explode_Create(f12 = weapon_data+0x08, f13 = f12, f14 = weapon_data+0x0C, script
//!   0x6000052, attacker = BU+0x30)` [M disasm 0x12ce1c-0x12ce2c]; `Explode_Create` stores param_1
//!   into `SCRIPTINFO+0x94` [M 0x17cbd0] and `Script_GetInterp` multiplies every key scale by
//!   `SCRIPTINFO+0x94` [M 0x1da158:117] — the blast script plays at scale = radius (8.0), and
//!   `Mat_Scale3f` leaves the matrix translation untouched [M 0x1e35f0].
//! - The bounce sound: `Bullet_CollisionHandler` 0x12ce80 plays 0x51c only when the bounce counter is
//!   EVEN and < 0xF (bounces 0,2,4,…,14 — 8 sounds max); the flash (0x70 & 0x2000) plays 0x15 and the
//!   smoke (0x70 & 0x20000) plays 0x13 on the FIRST bounce only. A rested grenade (the slow-bounce
//!   tail zeroes the speed and `Bullet_update`'s re-normalise keeps the velocity at zero, disasm
//!   0x12f020-0x12f030) never moves again — no collision, no sound [M 0x12ce80 tail + 0x12f040].
//! - The smoke grenade (0x36): the first fuse expiry pops `Occlude_Create(pos, 1200, 1)` (mesh
//!   0x2000273, a 1200-tick lifetime countdown [M 0x17ea80/0x17eba0]) + Sound 0x14 and re-arms the
//!   fuse +1200 [M 0x12e114-0x12e1b4]; its record is radius 0 / damage 0 [L] so nobody is damaged.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Load pre-merged for the grenade (the assets the level archive lacks), give it through the panel
/// (ownership + ammo, the user's path), and wait for the switch.
fn load_grenade(iso: &str, id: u8) -> nf_view::scene::Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[id]).expect("load");
    let (msg, switch_to) = nf_view::panel::apply(&mut scene, &q(&[("do", "weapon"), ("id", &id.to_string()), ("rounds", "5")]));
    eprintln!("[give] {id:#04x}: {msg} -> {switch_to:?}");
    assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id && scene.sim.as_ref().unwrap().gun.switch_state == 0 { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the grenade {id:#04x} is in hand");
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    scene
}

/// Sound ids the audio driver was asked to start this tick (`Cmd::Start`, `Drones::audio` — the way
/// the windowed loop's `sound_tick` builds the batch, matrix.rs's `sample`).
fn started_ids(scene: &mut nf_view::scene::Scene) -> Vec<u32> {
    let sim = scene.sim.as_mut().unwrap();
    let (pos, f) = (sim.bond.head(), sim.bond.look());
    let norm = { let l = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6); [f[2] / l, 0.0, -f[0] / l] };
    sim.drones.map_sounds.update(&mut sim.drones.audio, pos);
    sim.drones.audio.update(sim.sfx.as_ref(), (pos, norm, [0.0, 1.0, 0.0], f), &|_| false);
    sim.drones.audio.take_commands().into_iter()
        .filter_map(|c| match c { nf_game::sound::Cmd::Start { id, .. } if id != 0 => Some(id), _ => None })
        .collect()
}

/// ── The throw timing: the release frame and the refire interval ─────────────────────────────────
/// The pin sound (the fire script's op-1, frame 16 [D 07000026.bin]) precedes the spawn (frame 26
/// [L +0x44; M 0x1a2e38:416-420]); the held refire honours +0x40 = 63 [L; M 0x1a5460:652-654].
#[test]
fn the_frag_throws_on_the_release_frame_and_refires_at_the_interval() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_grenade(&iso, 0x34);
    scene.sim.as_mut().unwrap().bond.pitch = -0.9; // throw at the floor ahead
    let mut spawn_ticks = Vec::new();
    let mut pin_sound_tick = None;
    let mut fired_tick = None;
    for t in 0..400u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), true, false, false); // hold the trigger the whole time
        let ids = started_ids(&mut scene);
        let sim = scene.sim.as_ref().unwrap();
        if fired_tick.is_none() && sim.gun.fire_state { fired_tick = Some(t); }
        let n = sim.drones.grenades.len();
        if n > spawn_ticks.len() { spawn_ticks.push(t); }
        if pin_sound_tick.is_none() && ids.contains(&0x1e4) { pin_sound_tick = Some(t); }
    }
    eprintln!("first fire tick {fired_tick:?}, pin sound at {pin_sound_tick:?}, spawns {spawn_ticks:?}");
    assert!(!spawn_ticks.is_empty(), "a grenade spawned while holding the trigger");
    let first = spawn_ticks[0];
    let pin = pin_sound_tick.expect("the pin sound played (the fire script's op-1 frame 16)");
    // fix-frag-throw-timing: the release is the +0xB4 script's playhead reaching +0x44 = 26 [M 0x1a2e38 case 0xc],
    // i.e. after the 70-frame fire script has played out at 1.5x — the pin sound (the fire script's op-1 frame 16)
    // still precedes it, by a wider margin now.
    assert!(pin < first, "the pin sound (frame 16) precedes the throw (the +0xB4 playhead 26) [D script; L +0x44]");
    if spawn_ticks.len() > 1 {
        let gap = spawn_ticks[1] - spawn_ticks[0];
        eprintln!("held-trigger refire gap {gap}");
        assert!(gap >= 63, "the held refire honours the interval (+0x40 = 63 [L record]), got {gap}");
        assert!(gap < 130, "the next grenade is not 'forever' away [L +0x40 = 63, script 70], got {gap}");
    } else {
        panic!("a second grenade never spawned while holding for 400 ticks");
    }
}

/// ── The blast effect plays at the game's scale: SCRIPTINFO+0x94 = radius ─────────────────────────
/// `Explode_Create` gets f12 = weapon_data+0x08 (the radius) [M disasm 0x12ce1c] and stores it in
/// SCRIPTINFO+0x94 [M 0x17cbd0]; `Script_GetInterp` multiplies the key scales by it [M 0x1da158:117].
/// The port used radius·0.0667 (obj+0xE8's value — a field nothing visual reads) — 15× too small:
/// the user's "The explosion is very small and not accurate to the game."
#[test]
fn the_frag_blast_effect_plays_at_the_game_scale() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_grenade(&iso, 0x34);
    scene.sim.as_mut().unwrap().bond.pitch = -0.9;
    let mut blast = None;
    for _ in 0..400u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), true, false, false);
        if !scene.sim.as_ref().unwrap().drones.explosions.is_empty() { blast = Some(scene.sim.as_ref().unwrap().drones.explosions.clone()); break; }
    }
    let ex = blast.expect("the frag exploded at its fuse");
    assert!((ex[0].1 - 8.0).abs() < 1e-3, "the blast radius is weapon_data+0x08 = 8.0 [L], got {}", ex[0].1);
    assert!((ex[0].2 - 100.0).abs() < 1e-3, "the blast damage is weapon_data+0x0C = 100.0 [L], got {}", ex[0].2);
    // The blast script's player: scale = SCRIPTINFO+0x94 = the radius [M 0x17cbd0 + 0x1da158:117].
    let scale = scene.sim.as_ref().unwrap().drones.scripts.players.iter()
        .filter(|p| p.scripts[0] == 0x0600_0052).map(|p| p.scale).max_by(|a, b| a.total_cmp(b))
        .unwrap_or(-1.0);
    assert!((scale - 8.0).abs() < 1e-3, "the blast script plays at scale = radius (8.0 [L]), got {scale}");
}

/// ── A resting grenade stops playing bounce sounds ────────────────────────────────────────────────
/// The slow-bounce tail zeroes the bullet's speed and `Bullet_update`'s re-normalise keeps a rested
/// bullet's velocity at zero — it never moves, never collides, never sounds again [M 0x12ce80 tail +
/// 0x12f020-0x12f030]. The port's velocity kept re-accumulating gravity while at rest, so the
/// grenade re-collided EVERY tick: the user's "once it settles it makes a rapid sound as if it's
/// rapidly contacting the ground". The bounce sound itself is gated: the counter even and < 0xF
/// [M 0x12ce80] — at most 8 sounds, all within the first moments after the impacts.
#[test]
fn a_resting_grenade_stops_playing_bounce_sounds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_grenade(&iso, 0x34);
    scene.sim.as_mut().unwrap().bond.pitch = -0.9;
    let mut bounce_ticks: Vec<u32> = Vec::new();
    let mut spawn = None;
    for t in 0..400u32 {
        // Hold the trigger only until the first grenade is out: the count below is ONE grenade's.
        let trigger = spawn.is_none();
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), trigger, false, false);
        let ids = started_ids(&mut scene);
        if spawn.is_none() && !scene.sim.as_ref().unwrap().drones.grenades.is_empty() { spawn = Some(t); }
        if ids.contains(&0x51c) { bounce_ticks.push(t); }
    }
    eprintln!("spawn {spawn:?}, bounce-sound ticks {bounce_ticks:?}");
    spawn.expect("a grenade spawned");
    assert!(bounce_ticks.len() <= 8, "the counter gate allows at most 8 bounce sounds (even counters < 0xF) [M 0x12ce80], got {}", bounce_ticks.len());
    // The rest: once the grenade has settled (the last bounce-sound tick + 30), silence.
    if let Some(&last) = bounce_ticks.last() {
        let quiet_from = last + 30;
        for _ in quiet_from..quiet_from + 120 {
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
            let ids = started_ids(&mut scene);
            assert!(!ids.contains(&0x51c), "a resting grenade plays no bounce sounds [M 0x12ce80 tail + 0x12f020]");
        }
    } else {
        panic!("no bounce sounds at all — the throw never reached the floor");
    }
}

/// ── The smoke grenade pops its cloud and damages no one ─────────────────────────────────────────
/// 0x36's record is radius 0 / damage 0 [L]; its death branch pops `Occlude_Create(pos, 1200, 1)` +
/// Sound 0x14 and re-arms the fuse +1200 [M 0x12e114-0x12e1b4]. The port played the pop but spawned
/// no visual: the user's "Smoke grenade didn't work."
#[test]
fn the_smoke_grenade_pops_a_cloud_and_damages_no_one() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_grenade(&iso, 0x36);
    scene.sim.as_mut().unwrap().bond.pitch = -0.9;
    let mut popped = None;
    let mut pop_sound = None;
    for t in 0..600u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), true, false, false);
        let ids = started_ids(&mut scene);
        if popped.is_none() && scene.sim.as_ref().unwrap().drones.grenades.iter().any(|g| g.smoke_popped) {
            popped = Some(t);
            // The cloud's countdown starts at the pop: assert it while it is fresh.
            let clouds = &scene.sim.as_ref().unwrap().drones.smoke_clouds;
            assert!(!clouds.is_empty(), "the smoke cloud exists as a visual [M 0x17ea80]");
            assert!((clouds[0].ticks as i32 - 1200).abs() <= 2, "the cloud lives 1200 ticks [M 0x17eba0], got {}", clouds[0].ticks);
            if ids.contains(&0x14) { pop_sound = Some(t); }
            break;
        }
    }
    let pop = popped.expect("the smoke grenade popped its cloud at the first fuse expiry [M 0x12e114]");
    assert_eq!(pop_sound, Some(pop), "the pop sound 0x14 rides the same tick [M 0x12e1a0]");
    // No blast, no damage: the record is 0/0 and the smoke branch never reaches Explode_Create [M 0x12e118].
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert!(scene.sim.as_ref().unwrap().drones.explosions.is_empty(), "the smoke grenade exploded");
}
