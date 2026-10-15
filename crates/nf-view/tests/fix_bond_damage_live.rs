//! fix-bond-damage: Bond taking damage — the armour absorb, the red flash, the pain sounds,
//! the melee hit-push, and the per-level difficulty factors. Every number is the game's:
//!
//! - `Player_HandlePain` 0x1902c8: the difficulty factor (`ReadTuningVars` 0x1d3a80), armour absorbs
//!   first (:100-104), the pain grunt 0x88 on a 1-in-4 `Rand_Rand(4)` roll (:110-119), the red flash
//!   `BL+0x968` += damage·21.333 clamped (:126-130).
//! - `Player_DealWithObjHit` 0x194458:229-236: every bullet hit on Bond plays `Sound_Play(100, 0x412)`
//!   while the weapon record's `+0x70 & 0x180 == 0`; :237-256 the melee records (flag `+0x10 & 1`)
//!   play `Sound_Play3D(100, −1, −1, 0x171, Bond)` and write the hit-push `BL+0x910` (0x65 → −0.1,
//!   0x66/99 → +0.1).
//! - `Player_Update` 0x193600 @0x1937a4-0x1937d8 (disasm): `BL+0x910 ×= 0.9` every tick;
//!   @0x193ccc-0x193cf0: the move vec's Y `+= BL+0x910`, then `obj+0x50 += move` — the hit-push is a
//!   per-tick displacement that decays.
//! - `Drone_ModBulletDamage` 0x139f50:19-21: the melee records 0x63..0x66 return their damage
//!   UNMODDED, so a staged record bullet carries the game's exact damage.
//!
//! Opt-in via `NF_ISO` (the damage_factor test runs headless).

use nf_game::bullet::{Bullet, PlayerHealth};
use nf_game::drone::Rng;
use nf_game::sound::Cmd;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Flushes the audio pool (the render side's `audio.update`) and returns the ids started since the
/// last flush — a node only becomes a `Cmd::Start` there [M Sounds::update].
fn flush_started(sim: &mut nf_view::scene::Sim) -> Vec<u32> {
    let listener = (sim.bond.pos, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    sim.drones.audio.update(None, listener, &|_| true);
    let ids = sim.drones.audio.out.iter().filter_map(|c| match c {
        Cmd::Start { id, .. } => Some(*id),
        _ => None,
    }).collect();
    sim.drones.audio.out.clear();
    ids
}

/// Loads 07000005 and parks Bond at `pos` facing +Z (yaw 0).
fn load_at(pos: [f32; 3]) -> nf_view::scene::Scene {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, pos, 0.0);
    sim.drones.health = PlayerHealth::default();
    scene
}

/// Stages a guard bullet two units in front of Bond's EYE (yaw 0 → +Z ahead; the standing eye rides
/// 0.7 above the feet — BL+0x908 [L bldata.md, M 0x18f5d8:117-128]) and steps once.
fn shoot_from_front(scene: &mut nf_view::scene::Scene, damage: f32, weapon: u16) {
    let p = scene.sim.as_ref().unwrap().bond.pos;
    scene.sim.as_mut().unwrap().drones.bullets.push(Bullet {
        pos: [p[0], p[1] + 0.7, p[2] + 2.0],
        dir: [0.0, 0.0, -1.0],
        speed: 15.0, travelled: 0.0, range: 100.0,
        damage, owner: usize::MAX, state: 0, weapon, flyby: false, bounces: 0,
    });
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
}

/// A guard's bullet hit on Bond without armour: the game's numbers — health −(damage × 0.6)
/// [L 07000005 difficulty 1 = 0.6, work/ee_start_05 + ReadTuningVars 0x1d3a80 case 0x7000005-08],
/// the flash at 127 end-of-tick (the pain writes 128, the pane's −1/tick fade [M 0x19b9c8] runs in
/// the same tick), both-sides direction bits (the shot straight at the eye [M 0x194458:204-226]) and
/// `Sound_Play(100, 0x412)` (the record's `+0x70 & 0x180 == 0` [L weapon_data 0x2c: 0x4]
/// [M 0x194458:229-236]).
#[test]
fn guard_bullet_hit_without_armour_costs_the_game_s_numbers_and_plays_0x412() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([128.0, 2.0, 128.0]);
    // The 0x2c guard rifle's record damage is 100.0 [L weapon_data 0x2c +0x0C]; the staged bullet
    // carries the game's pre-mod number (the port mods at fire time, the game at hit time — same
    // value; the ASSERTION is the armour/factor arithmetic).
    shoot_from_front(&mut scene, 100.0, 0x2c);
    let started = flush_started(scene.sim.as_mut().unwrap());
    let sim = scene.sim.as_ref().unwrap();
    assert!((sim.drones.health.health - 40.0).abs() < 1e-3,
        "100 × 0.6 = 60 health, got {}", sim.drones.health.health);
    assert!((sim.drones.health.flash - 127.0).abs() < 1e-3,
        "the flash: 128 written, −1 by the pane's fade the same tick, got {}", sim.drones.health.flash);
    assert!(sim.drones.health.dir_bits != 0,
        "the hit sets direction bits (the exact both-sides rule is pinned by hud_playtest's unit test), got 0");
    assert!(started.contains(&0x412),
        "the bullet hit must start the hurt sound 0x412 [M 0x194458:229-236]; started {started:?}");
}

/// Sister: with armour, the armour absorbs first [M 0x1902c8:100-104]: a 60-effective hit against
/// armour 50 empties the armour (50) and costs 10 health. The hurt sound still plays.
#[test]
fn armour_absorbs_first_then_health() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([128.0, 2.0, 128.0]);
    scene.sim.as_mut().unwrap().drones.health.armour = 50.0;
    shoot_from_front(&mut scene, 100.0, 0x2c);
    let started = flush_started(scene.sim.as_mut().unwrap());
    let sim = scene.sim.as_ref().unwrap();
    assert!((sim.drones.health.armour - 0.0).abs() < 1e-3,
        "the armour absorbs the first 50, got {}", sim.drones.health.armour);
    assert!((sim.drones.health.health - 90.0).abs() < 1e-3,
        "the remaining 10 reach health, got {}", sim.drones.health.health);
    assert!(started.contains(&0x412), "the hurt sound plays with armour too");
}

/// Sister: the gate is the weapon record's `+0x70 & 0x180` — the 0x37 grenade carries 0x100
/// [L weapon_data 0x37: 0x18300], so its hit never plays 0x412 [M 0x194458:229-236].
#[test]
fn the_0x412_gate_follows_the_weapon_record_s_flags() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([128.0, 2.0, 128.0]);
    shoot_from_front(&mut scene, 10.0, 0x37);
    let started = flush_started(scene.sim.as_mut().unwrap());
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.health.health < 100.0, "the hit still lands");
    assert!(!started.contains(&0x412),
        "a 0x37 hit (flags 0x18300 & 0x180 = 0x100) must NOT play 0x412; started {started:?}");
}

/// The pain grunt: `Player_HandlePain` 0x1902c8:110-119 rolls `Rand_Rand(4)` on the shared RNG and
/// plays `Sound_Play3D(100, −1, −1, 0x88, Bond)` on 0. The roll is deterministic, so a run of hits
/// grunts some but not all of the time.
#[test]
fn the_pain_grunt_rolls_one_in_four_on_the_shared_rng() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([128.0, 2.0, 128.0]);
    let mut grunts = 0;
    let hits = 24;
    for _ in 0..hits {
        scene.sim.as_mut().unwrap().drones.health.health = 100.0; // stay alive
        shoot_from_front(&mut scene, 1.0, 0x2c);
        grunts += flush_started(scene.sim.as_mut().unwrap()).iter().filter(|&&id| id == 0x88).count();
    }
    assert!(grunts >= 1, "no 0x88 pain grunt in {hits} hits — the Rand_Rand(4) roll never fired");
    assert!(grunts < hits, "every hit grunted — the roll is not 1-in-4");
}

/// The melee hit-push [M 0x194458:237-256]: the 0x65 record (flag `+0x10 & 1` set [L weapon_data
/// 0x65: 0x1]) writes `BL+0x910 = −0.1` and plays `Sound_Play3D(0x171, Bond)`; the push decays
/// ×0.9/tick [M 0x193600 @0x1937a4-0x1937d8]. The melee damage is the record's 15.0 unmodded
/// [M 0x139f50:19-21] → 15 × 0.6 = 9 health. Sister: 0x67 (flag 0 [L]) sets no push and no 0x171.
#[test]
fn melee_hits_play_0x171_and_set_the_decaying_hit_push() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([128.0, 2.0, 128.0]);
    shoot_from_front(&mut scene, 15.0, 0x65);
    let started = flush_started(scene.sim.as_mut().unwrap());
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!((sim.drones.health.health - 91.0).abs() < 1e-3,
            "the melee damage is the record's 15 × 0.6 = 9, got {}", sim.drones.health.health);
        assert!(started.contains(&0x171),
            "the melee hit must start the impact sound 0x171 [M 0x194458:255]; started {started:?}");
        assert!(started.contains(&0x412), "the melee hit also plays the hurt sound (0x65 flags 0x400 & 0x180 = 0)");
        assert!((sim.drones.health.hit_push - (-0.1)).abs() < 1e-6,
            "weapon 0x65 writes BL+0x910 = −0.1, got {}", sim.drones.health.hit_push);
    }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    assert!((scene.sim.as_ref().unwrap().drones.health.hit_push - (-0.09)).abs() < 1e-6,
        "the push decays ×0.9 per tick [M 0x193600 @0x1937d8], got {}",
        scene.sim.as_ref().unwrap().drones.health.hit_push);
    // Sister: the 0x67 record's flag byte is 0x10 — bit 0 clear [L weapon_data 0x67: 0x10].
    let mut scene = load_at([128.0, 2.0, 128.0]);
    shoot_from_front(&mut scene, 50.0, 0x67);
    let started = flush_started(scene.sim.as_mut().unwrap());
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.health.health < 100.0, "the 0x67 hit still lands");
    assert!(!started.contains(&0x171), "0x67's flag bit 0 is clear — no 0x171");
    assert_eq!(sim.drones.health.hit_push, 0.0, "0x67 sets no hit-push");
}

/// The per-level difficulty factors (`ReadTuningVars` 0x1d3a80): the port had only the 0x1-4 and
/// 0x5-8 groups; the game tunes five more, and difficulty 0 reads `Plr_DMod_Normal`
/// [M 0x1902c8:29-48]. All values [M 0x1d3a80:37-42, 50-55, 97-99, 126-127+143, 168-173, 180-185,
/// 190-197] / boot defaults [L ACTION.ELF 0x30cc50: 0.75/1.0/1.25].
#[test]
fn damage_factor_matches_readtuningvars_per_level_group() {
    // [M 0x1d3a80 case 0x7000009-b: 0.6/0.8/1.0]
    assert!((nf_game::bullet::damage_factor(0x0700_0009, 1) - 0.6).abs() < 1e-6);
    assert!((nf_game::bullet::damage_factor(0x0700_000b, 2) - 0.8).abs() < 1e-6);
    assert!((nf_game::bullet::damage_factor(0x0700_000a, 3) - 1.0).abs() < 1e-6);
    // [M 0x1d3a80 case 0x700000c-d: 0.4/0.7/1.2]
    assert!((nf_game::bullet::damage_factor(0x0700_000c, 1) - 0.4).abs() < 1e-6);
    assert!((nf_game::bullet::damage_factor(0x0700_000d, 2) - 0.7).abs() < 1e-6);
    assert!((nf_game::bullet::damage_factor(0x0700_000c, 3) - 1.2).abs() < 1e-6);
    // [M 0x1d3a80 case 0x7000011-13,4a: 0.5/0.6/1.0]
    assert!((nf_game::bullet::damage_factor(0x0700_0011, 1) - 0.5).abs() < 1e-6);
    assert!((nf_game::bullet::damage_factor(0x0700_004a, 2) - 0.6).abs() < 1e-6);
    // [M 0x1d3a80 case 0x7000014-16: 0.4/0.6/1.0]
    assert!((nf_game::bullet::damage_factor(0x0700_0014, 1) - 0.4).abs() < 1e-6);
    // [M 0x1d3a80 case 0x700001b: 0.6/0.7/1.0]
    assert!((nf_game::bullet::damage_factor(0x0700_001b, 2) - 0.7).abs() < 1e-6);
    // Unmapped levels keep the ELF boot defaults [L 0x30cc50-58: 0.75/1.0/1.25].
    assert!((nf_game::bullet::damage_factor(0x0700_000e, 1) - 0.75).abs() < 1e-6);
    assert!((nf_game::bullet::damage_factor(0x0700_000e, 3) - 1.25).abs() < 1e-6);
    // Difficulty 0 reads Plr_DMod_Normal [M 0x1902c8:29-48 (the `else` arm)].
    assert!((nf_game::bullet::damage_factor(0x0700_0009, 0) - 0.8).abs() < 1e-6);
    // Difficulty 4 doubles [M 0x1902c8:43-45].
    assert!((nf_game::bullet::damage_factor(0x0700_0005, 4) - 2.0).abs() < 1e-6);
    // The already-pinned groups stay [L progress_live: 07000005 = 0.6/0.7/1.0].
    assert!((nf_game::bullet::damage_factor(0x0700_0005, 1) - 0.6).abs() < 1e-6);
    // Sister: the shared RNG's Rand_Rand(4) contract the grunt uses.
    let mut r = Rng::new(7);
    let first = r.rand(4);
    assert!(first < 4, "Rand_Rand(4) returns 0..4, got {first}");
}
