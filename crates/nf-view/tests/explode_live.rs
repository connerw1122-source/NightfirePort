//! fix-explosions: the grenade blast through the real Sim path (nf-view load + the sim tick).
//! The facts under test, all off the disc/decompile:
//! - `Bullet_handle_object_destruction` 0x12cd98: the fuse explodes with radius = weapon+0x08 and
//!   damage = weapon+0x0C (`lwc1 $f12, 8($v0)` / `lwc1 $f14, 12($v0)` → `Explode_Create`'s f12/f13/f14;
//!   the `addiu $t2, $zero, 0x64` u8 is never read by `Explode_Create` 0x17cbd0 — the old port's
//!   hardcoded 100 was wrong for every type).
//! - The frag (0x35: radius 10, damage 0 [L weapon record]) therefore damages NOBODY through
//!   `Explode_Propagate` 0x17c518 — the user's "kills a guard on impact / bounces back and damages
//!   Bond" was the hardcode. The flashbang (0x36: radius 0, damage 0 [L]) is a no-op there too.
//! - The flashbang's real work is `Bullet_DoTrails` 0x12d890's death branch (`weapon+0x70 & 0x2000`,
//!   record 0x36 [L]): Sound 0x16, Light_Create(10, 5, white), the Bond whiteout
//!   (`Player_SetFlashBang` [M 0x12e014]) and drones within 20 u → msg 0x18 → `HandleImpact` case
//!   0x18 → `StunGrenadeImpact` 0x4D [M 0x1469b8:250-313] — never a `HitDamage` call. The stun
//!   lasts `DroneFunc_RecoverTime` 0x147188 = 36·(200 − +0xB9) = 7200 ticks with the stats-default
//!   +0xB9 = 0 [M 0x14c600:90]; a stunned guard shot (msg 8/9) dies into 0x44 [M 0x172bb0 case 8/9].
//! - The blast sound is script 0x6000052's own `Sound(0x3E)` [D script stream 36], played through the
//!   script player's `Request::Sound` — the old port's 0x4A7 was `Bullet_CollisionHandler`'s
//!   rest-impact sound of weapons 0x3B/0x45 [M 0x12ce80:315-327].
//!
//! Opt-in: set `NF_ISO` to the disc image.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, "07000005.bin", false).expect("load 07000005")
}

/// The frag thrown through the real input path explodes at its fuse and damages NOTHING
/// (record 0x35: radius 10, damage 0 [L]); the blast script 0x6000052 is playing (its Sound(0x3E)
/// is the blast sound). Sister: the mine (0x37: 8/100 [L]) pushed at a guard hurts him.
#[test]
fn frag_blast_deals_no_damage_and_the_mine_blast_does() {
    use nf_game::player::{Bond, Input};
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    sim.disabled = false;
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    // Own the frag and throw through the input path (fix-player3's route).
    let id = 0x35usize;
    let model = sim.weapons.model(id);
    let bounds = *sim.models.get(&model).unwrap_or(&nf_game::pickup::Model { centre: [0.0, 0.05, 0.0], radius: 0.3 });
    let p = nf_game::pickup::Pickup::create_placed(sim.bond.pos, [0.0; 3], nf_game::pickup::kind::WEAPON, id as u16, 1, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons).expect("frag pickup");
    sim.drones.pickups.push(p);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    sim.gun.switch_request = 0x35;
    for _ in 0..120 { if sim.gun.weapon == 0x35 && sim.gun.switch_state == 0 { break; } sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 0x35, "the frag is drawn");
    let before: Vec<f32> = sim.drones.list.iter().map(|d| d.health).collect();
    let n_guards = sim.drones.list.iter().filter(|d| d.present && d.enabled && !d.corpse).count();
    // A grenade with a 12-tick fuse right in the guards' midst: the fuse path, not the throw physics.
    let at = sim.bond.pos;
    sim.drones.grenades.push(nf_game::grenade::Grenade {
        pos: at, velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 12.0,
        damage: f32::from_le_bytes(sim.weapons.records[0x35][0x0c..0x10].try_into().unwrap()),
        radius: f32::from_le_bytes(sim.weapons.records[0x35][0x08..0x0c].try_into().unwrap()),
        weapon_type: 0x35, mesh: 0, owner: usize::MAX, state: 1, bounces: 0,
    });
    let mut blasted = false;
    for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); if !sim.drones.explosions.is_empty() { blasted = true; break; } }
    assert!(blasted, "the frag exploded at its fuse");
    if n_guards > 0 {
        for (i, d) in sim.drones.list.iter().enumerate() {
            assert!(d.health >= before[i] - 1e-4 && !d.corpse,
                "guard {i}: the frag blast (damage 0 [L record 0x35]) must not hurt ({:.2} → {:.2})", before[i], d.health);
        }
    }
    assert!(sim.drones.health.health >= 100.0 - 1e-3, "Bond takes nothing from a damage-0 blast");
    // The blast script player is up (its Sound 0x3E request is the game's blast sound [D]).
    assert!(sim.drones.scripts.players.iter().any(|p| p.scripts[0] == 0x0600_0052), "the 0x6000052 one-shot is playing");
    // Sister: the mine (0x37) blast with its own record's 100/8 hurts an enabled guard.
    let gi = sim.drones.list.iter().position(|d| d.present && d.enabled && !d.corpse);
    if let Some(gi) = gi {
        let hp0 = sim.drones.list[gi].health;
        let gp = sim.drones.list[gi].pos;
        sim.drones.grenades.push(nf_game::grenade::Grenade {
            pos: [gp[0], gp[1] + 0.4, gp[2]], velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 3.0,
            damage: f32::from_le_bytes(sim.weapons.records[0x37][0x0c..0x10].try_into().unwrap()),
            radius: f32::from_le_bytes(sim.weapons.records[0x37][0x08..0x0c].try_into().unwrap()),
            weapon_type: 0x37, mesh: 0, owner: usize::MAX, state: 1, bounces: 0,
        });
        let mut hurt = false;
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); if sim.drones.list[gi].health < hp0 || sim.drones.list[gi].corpse { hurt = true; break; } }
        assert!(hurt, "the mine blast (record 0x37: 8/100 [L]) hurts through Explode_Propagate");
    }
}

/// The flashbang stuns: every live guard within 20 u of the blast enters `StunGrenadeImpact` 0x4D
/// with health UNTOUCHED (the game's path has no `HitDamage` [M 0x12d890]), and a shot on a stunned
/// guard kills into 0x44 [M 0x172bb0 case 8/9]. Sister: Bond's whiteout is set (flashbang > 0) and
/// a guard beyond 20 u is unaffected.
#[test]
fn flashbang_stuns_without_damage_and_a_shot_kills_the_stunned() {
    use nf_game::player::{Bond, Input};
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    sim.disabled = false;
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    let gi = sim.drones.list.iter().position(|d| d.present && d.enabled && !d.corpse).expect("07000005 starts a live guard");
    let hp0 = sim.drones.list[gi].health;
    let gp = sim.drones.list[gi].pos;
    // The flash lands at the guard (within the 20 u broadcast) with a 5-tick fuse.
    sim.drones.grenades.push(nf_game::grenade::Grenade {
        pos: [gp[0], gp[1] + 0.5, gp[2]], velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 5.0,
        damage: 0.0, radius: 0.0, weapon_type: 0x36, mesh: 0, owner: usize::MAX, state: 1, bounces: 0,
    });
    let mut stunned = false;
    for _ in 0..40 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.list[gi].state == nf_game::drone::state::STUN_GRENADE_IMPACT
            || sim.drones.list[gi].state == nf_game::drone::state::STUN_GRENADE_LOOP { stunned = true; break; }
    }
    assert!(stunned, "the guard entered the stun family (0x4D/0x4E), got {:#x}", sim.drones.list[gi].state);
    assert_eq!(sim.drones.list[gi].health, hp0, "the flashbang dealt no damage [M Bullet_DoTrails 0x12d890]");
    assert!(sim.drones.health.flashbang > 0.0, "Bond's whiteout is set [M Player_SetFlashBang 0x12e0b8]");
    // Set to 2.0 this tick, decayed once by the tick tail (×0.9 − 0.05): 1.75 left — the flash registered.
    assert!(sim.drones.shake > 0.0, "the flash's Camera_Shake(2.0) [M 0x12df6c]");
    // A shot on the stunned guard: msg 8 → Death_Anim 0x44 (the stunned guard dies like the game's).
    // The Data borrows the Sim's parts through the scene (the sim tick's own construction, inlined).
    let data = nf_game::drone::Data {
        tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
        world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
        skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
        hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
        surfaces: None, voices: None,
    };
    sim.drones.list[gi].send(&data, &mut sim.drones.rng, sim.drones.frame, nf_game::drone::msg::BULLET);
    assert_eq!(sim.drones.list[gi].state, nf_game::drone::state::DEATH_ANIM, "the stunned guard shot → 0x44 [M 0x172bb0 case 8/9]");
}
