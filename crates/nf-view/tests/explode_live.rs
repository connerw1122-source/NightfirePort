//! fix-explosions2: the grenade blast through the real Sim path (nf-view load + the sim tick).
//! The facts under test, all off the disc/decompile:
//! - `Bullet_DoTrails` 0x12d890's timer-death branch (gate `weapon+0x70 & 0x200`, asm 0x12dd2c-0x12dd44)
//!   routes on the weapon FLAGS: `& 0x2000` → the flash branch (only the stun grenade 0x35 among the
//!   grenades carries the bit [L sweep of all 115 records]); else `& 0x20000` → the smoke branch (0x36
//!   [L 0x20280]); else → `Explode_Create(wd+8, wd+8, wd+0xC)` [asm 0x12e1c0].
//! - The disc name labels (Txt_BindLabel of weapon_data+0x38, resolved against USATxt.dat with the
//!   game's own fixup algorithm) name the records [L]: 0x34 'Fragmentation Grenade' = radius 8 /
//!   damage 100 (the real frag kill), 0x35 'Stun Grenade' = 10/0, 0x36 'Smoke Grenade' = 0/0,
//!   0x37 'Remote Mine' = 8/100. fix-explosions had the mapping shifted by one id (it read 0x35 as
//!   the frag → "the frag damages nobody" — wrong: the frag is 0x34 and it kills).
//! - The flash branch has NO damage and NO 0x6000052 script: Sound 0x16, Light_Create(10, 5, white),
//!   the Bond whiteout (`Player_SetFlashBang` [M 0x12e014]) and drones within 20 u → msg 0x18 →
//!   `HandleImpact` case 0x18 → `StunGrenadeImpact` 0x4D [M 0x1469b8:250-313] — broadcast only for a
//!   PLAYER thrower in SP [M 0x12e094: `MPSettings._388_4_ == 0 && thrower+0xFF != 3` skips]. The stun
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

/// The frag (0x34 'Fragmentation Grenade': radius 8, damage 100 [L record]) thrown with a short fuse
/// at a guard KILLS him through `Explode_Propagate` 0x17c518 case 2 (`Drone_ExplosiveHit` → msg 9) —
/// the game's ordinary frag kill; Bond, far from the blast, keeps his 100. The blast script 0x6000052
/// is playing (its Sound(0x3E) is the blast sound [D]). Sister: the stun grenade (0x35) at the same
/// spot touches NOBODY's health (the flash branch has no damage [M 0x12d890]).
#[test]
fn frag_blast_kills_the_guard_next_to_it_and_the_stun_touches_nobody() {
    use nf_game::player::{Bond, Input};
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    sim.disabled = false;
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    // Own the frag through the real take path and draw it (the pickup's weapon id = the disc's own
    // id 0x34 — the labels above name it).
    let id = 0x34usize;
    let model = sim.weapons.model(id);
    let bounds = *sim.models.get(&model).unwrap_or(&nf_game::pickup::Model { centre: [0.0, 0.05, 0.0], radius: 0.3 });
    let p = nf_game::pickup::Pickup::create_placed(sim.bond.pos, [0.0; 3], nf_game::pickup::kind::WEAPON, id as u16, 1, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons, &std::collections::HashMap::new()).expect("frag pickup");
    sim.drones.pickups.push(p);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    sim.gun.switch_request = 0x34;
    for _ in 0..120 { if sim.gun.weapon == 0x34 && sim.gun.switch_state == 0 { break; } sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 0x34, "the frag (0x34) is drawn");
    // Sanity: the routing bit sits on 0x35, NOT on the frag [L sweep — the asm reads +0x70 at 0x12dd60].
    let f70 = |w: usize| u32::from_le_bytes(sim.weapons.records[w][0x70..0x74].try_into().unwrap());
    assert_eq!(f70(0x34) & 0x2000, 0, "the frag carries no flash bit");
    assert_eq!(f70(0x35) & 0x2000, 0x2000, "the stun grenade carries the flash bit");
    // The frag lands at a live guard with a 12-tick fuse: the fuse path, not the throw physics.
    let gi = sim.drones.list.iter().position(|d| d.present && d.enabled && !d.corpse).expect("07000005 starts a live guard");
    let hp0 = sim.drones.list[gi].health;
    let gp = sim.drones.list[gi].pos;
    sim.drones.grenades.push(nf_game::grenade::Grenade {
        pos: [gp[0], gp[1] + 0.4, gp[2]], velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 12.0,
        damage: f32::from_le_bytes(sim.weapons.records[0x34][0x0c..0x10].try_into().unwrap()),
        radius: f32::from_le_bytes(sim.weapons.records[0x34][0x08..0x0c].try_into().unwrap()),
        weapon_type: 0x34, mesh: 0, owner: usize::MAX, state: 1, bounces: 0, smoke_popped: false,
    });
    let mut blasted = false;
    for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); if !sim.drones.explosions.is_empty() { blasted = true; break; } }
    assert!(blasted, "the frag exploded at its fuse");
    assert!(sim.drones.list[gi].corpse || sim.drones.list[gi].health < hp0,
        "the frag blast (0x34: 8/100 [L]) kills/hurts the guard at the blast ({} → {})", hp0, sim.drones.list[gi].health);
    assert!(sim.drones.health.health >= 100.0 - 1e-3, "Bond, away from the blast, keeps his 100");
    // The blast script player is up (its Sound 0x3E request is the game's blast sound [D]).
    assert!(sim.drones.scripts.players.iter().any(|p| p.scripts[0] == 0x0600_0052), "the 0x6000052 one-shot is playing");
    // Sister: the stun grenade (0x35) at a live guard's feet — health UNTOUCHED (the flash branch
    // carries no damage [M 0x12d890; the sweep above shows 0x35 routes there]).
    let gi2 = sim.drones.list.iter().position(|d| d.present && d.enabled && !d.corpse && d.health > 0.0);
    if let Some(gi2) = gi2 {
        let hp0 = sim.drones.list[gi2].health;
        let gp = sim.drones.list[gi2].pos;
        sim.drones.grenades.push(nf_game::grenade::Grenade {
            pos: [gp[0], gp[1] + 0.4, gp[2]], velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 12.0,
            damage: f32::from_le_bytes(sim.weapons.records[0x35][0x0c..0x10].try_into().unwrap()),
            radius: f32::from_le_bytes(sim.weapons.records[0x35][0x08..0x0c].try_into().unwrap()),
            weapon_type: 0x35, mesh: 0, owner: usize::MAX, state: 1, bounces: 0, smoke_popped: false,
        });
        let mut hurt = false;
        for _ in 0..40 {
            sim.step_bond(Input::default(), false, false, false);
            if sim.drones.list[gi2].health < hp0 { hurt = true; break; }
        }
        assert!(!hurt, "the stun grenade (0x35) deals no damage (flash branch [M 0x12d890]) ({} → {})", hp0, sim.drones.list[gi2].health);
    }
}

/// The stun grenade (0x35) stuns: every live guard within 20 u of the blast enters
/// `StunGrenadeImpact` 0x4D with health UNTOUCHED (the game's path has no `HitDamage` [M 0x12d890]),
/// and a shot on a stunned guard kills into 0x44 [M 0x172bb0 case 8/9]. Sister: Bond's whiteout is
/// set (flashbang > 0) and the camera shake is the branch's fixed 2.0 [M 0x12df6c].
#[test]
fn stun_grenade_stuns_without_damage_and_a_shot_kills_the_stunned() {
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
    // The flash lands at the guard (within the 20 u broadcast) with a 5-tick fuse. The weapon is
    // 0x35 — its `+0x70 & 0x2000` [L 0x2280] routes the death into the flash branch.
    sim.drones.grenades.push(nf_game::grenade::Grenade {
        pos: [gp[0], gp[1] + 0.5, gp[2]], velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 5.0,
        damage: 0.0, radius: 0.0, weapon_type: 0x35, mesh: 0, owner: usize::MAX, state: 1, bounces: 0, smoke_popped: false,
    });
    let mut stunned = false;
    for _ in 0..40 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.list[gi].state == nf_game::drone::state::STUN_GRENADE_IMPACT
            || sim.drones.list[gi].state == nf_game::drone::state::STUN_GRENADE_LOOP { stunned = true; break; }
    }
    assert!(stunned, "the guard entered the stun family (0x4D/0x4E), got {:#x}", sim.drones.list[gi].state);
    assert_eq!(sim.drones.list[gi].health, hp0, "the stun grenade dealt no damage [M Bullet_DoTrails 0x12d890]");
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
        surfaces: None, voices: None, difficulty: 1 
    };
    sim.drones.list[gi].send(&data, &mut sim.drones.rng, sim.drones.frame, nf_game::drone::msg::BULLET);
    assert_eq!(sim.drones.list[gi].state, nf_game::drone::state::DEATH_ANIM, "the stunned guard shot → 0x44 [M 0x172bb0 case 8/9]");
}
