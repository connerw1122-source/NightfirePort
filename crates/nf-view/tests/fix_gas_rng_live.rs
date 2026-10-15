//! fix-gas-rng (G-033): the muzzle smoke puff's shared-RNG cost. `Player_MuzzleFlash` 0x1a4298's
//! smoke arm draws `Rand_FRand(2π)` (0x1a43ec — the puff quaternion's z) and `Gas_Init` 0x1aacc8
//! adds the spin `Rand_FRand(π/2·REC)` (0x1ab060), the sign coin `Rand_Rand(0x100)` (0x1ab08c,
//! negate at ≥ 0x80) and the kind-7/NULL-vel wind spread `Rand_FRand(0.2)` ×3 (0x1ab160/0x1ab184/
//! 0x1ab1a4) — exactly 6 shared-stream draws per muzzle puff. The game's fixed args for the puff
//! [M disasm 0x1a4404-0x1a442c]: scale 0.05 → 0.1, velocity pointer NULL, mesh 0x2000039, kind 7,
//! rgba 0x48403080, end 1 (the alpha end byte is 1), life 0x80 = 128 ticks (the stack word the
//! decompile drops).
//!
//! Pre-fix the port consumed only the 2π draw and built the puff from the counter hash with a
//! guessed 8-tick life — every shot left the sim's RNG stream 5 draws short of the game's and each
//! later AI roll shifted (G-033).
//!
//! Opt-in via NF_ISO. Fail-first: `…_advances_the_sim_rng_by_exactly_the_games_count` fails against
//! the pre-fix sources (the puff signature: life 8.0 ≠ the game's 128, counter-hash fields no
//! stream offset reproduces); the sister's `assert_eq!(delta, SMOKE_DRAWS)` fails there too (the
//! pre-fix delta was 0 — only the 2π draw rode the stream).
//!
//! Bond is teleported 40 units up over the courtyard before the shot so the round's first-tick
//! ray (the flight step `max(1.0)` unit) meets no geometry — an impact puff's own draws would sit
//! behind the muzzle draws on a 6-shifted stream and their 1-in-2 spawn coin could diverge between
//! the runs [M update_player_bullets; effect.rs's `rand(2)`].

use nf_game::drone::Rng;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    scene
}

/// Arms the gun for a clean shot (fire_modes3_live's harness), teleports Bond 40 units up (both
/// runs identically — the draws below cancel in the sister's delta) and aims level.
fn arm(scene: &mut nf_view::scene::Scene, clip: i16) {
    let sim = scene.sim.as_mut().unwrap();
    let start = sim.bond.pos;
    sim.bond.pos = [start[0], start[1] + 40.0, start[2]];
    sim.gun.reload_state = 0;
    sim.gun.fire_state = false;
    sim.gun.pending = 0;
    sim.gun.clip = clip.max(0) as u32;
    let w = sim.gun.weapon as usize;
    if let Some(c) = sim.drones.inventory.clip.get_mut(w) { *c = clip; }
}

/// How many draws `b` lies ahead of `a` on the shared stream. Every draw — `frand` or `rand` —
/// steps BOTH streams once, so the draw count is the step count and the values in between are
/// irrelevant; the first (x, y) state match is the answer.
fn steps_between(a: &Rng, b: &Rng) -> u32 {
    let mut t = a.clone();
    let mut k = 0;
    while (t.x != b.x || t.y != b.y) && k < 1_000_000 { t.frand(1.0); k += 1; }
    k
}

/// The muzzle puff's game stream cost: the 2π quaternion draw + the spin + the sign + the wind ×3
/// [M 0x1a43ec, 0x1ab060, 0x1ab08c, 0x1ab160/0x1ab184/0x1ab1a4].
const SMOKE_DRAWS: u32 = 6;

#[test]
fn a_muzzle_puff_advances_the_sim_rng_by_exactly_the_games_count() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    arm(&mut scene, 41);
    let sim = scene.sim.as_mut().unwrap();
    let pre = sim.drones.rng.clone();
    let puffs0 = sim.drones.gas.puffs.len();
    sim.step_bond(Default::default(), true, false, false); // the shot
    let post = sim.drones.rng.clone();
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.shot_tick, "the press fired this tick");
    assert_eq!(steps_between(&pre, &post) > 0, true, "the tick drew from the shared stream");
    let _ = post;
    // The puff exists with the game's fixed args [M 0x1a4404-0x1a442c]. The pool already ran one
    // update when the tick ends, so the read shows life 128−1 and scale 0.05+rate; the searchlight
    // site's kind-7 puff carries the same 0.05 start but a 0.25 end — scale_rate separates them.
    let rate = (0.1 - 0.05) / 128.0;
    let puff = sim.drones.gas.puffs.iter().rev()
        .find(|p| p.kind == 7 && p.scale_rate == rate && p.life == 127.0)
        .expect("the muzzle smoke puff exists (pre-fix: life 8.0, counter-hash fields)");
    assert!(puff.scale > 0.05 && puff.scale < 0.051, "the puff grew one tick from the 0.05 start");
    // The pool's update already faded the start rgba one tick; the rates are untouched and exact.
    let inv = 1.0f32 / 128.0;
    assert_eq!(puff.rgba_rate, [
        (0.0 - 0x48 as f32) * inv, (0.0 - 0x40 as f32) * inv,
        (0.0 - 0x30 as f32) * inv, (1.0 - 0x80 as f32) * inv,
    ], "the end-1 bytes reach the alpha rate (the game's reciprocal form)");
    assert!(puff.rgba[3] < 0x80 as f32, "the puff faded one tick");
    assert_eq!(puff.mesh, nf_game::gas::SMOKE_QUAD);
    assert_eq!(sim.drones.gas.puffs.iter().filter(|p| p.kind == 7 && p.scale_rate == rate).count(), 1,
        "one muzzle puff for one shot (Bond is in the sky: no impact puffs behind it)");
    // The 6 game draws, replayed on the pre-shot snapshot after k other draws the tick made before
    // the puff (the gun's accuracy/spread draws, the light, the flight-step draw…), reproduce the
    // puff's fields bit for bit. (Each draw advances the stream once, so k filler draws land on
    // the same state regardless of their kind.) Pre-fix the fields came from a counter hash: no k
    // works. The kind-7 update the pool ran never touches spin/vel, so the read fields are raw.
    let rec = 1.0f32 / 60.0;
    let expect = |r: &mut Rng| {
        let _quat_z = r.frand(6.2831854820251465); // 0x1a43ec, 0x40c90fdb
        let spin = r.frand(std::f32::consts::FRAC_PI_2 * rec); // 0x1ab060
        let sign = r.rand(0x100);                              // 0x1ab08c
        let (rx, ry, rz) = (r.frand(0.2), r.frand(0.2), r.frand(0.2)); // 0x1ab160/184/1a4
        (if sign >= 0x80 { -spin } else { spin },
         [nf_formats::ps2float::mul(rx - 0.1, rec), nf_formats::ps2float::mul(ry, rec), nf_formats::ps2float::mul(rz - 0.1, rec)])
    };
    let mut hit = None;
    for k in 0..1024u32 {
        let mut r = pre.clone();
        for _ in 0..k { r.frand(1.0); }
        let (spin, vel) = expect(&mut r);
        if spin.to_bits() == puff.spin.to_bits() && vel.iter().zip(puff.vel.iter()).all(|(a, b)| a.to_bits() == b.to_bits()) {
            hit = Some(k);
            break;
        }
    }
    let k = hit.expect("the puff's fields = the game's 6 draws at a fixed stream offset (pre-fix: a counter hash, no such k)");
    eprintln!("[gas-rng] the 6 smoke draws sit {k} draws into the shot tick");
}

#[test]
fn the_no_smoke_gate_makes_no_puff_and_no_smoke_draws() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // Sister: the identical shot with the weapon's no-smoke bit set (weapon_data+0x68 & 0x2000,
    // Player_MuzzleFlash 0x1a4298:92's skip arm) — no puff, and the tick draws exactly 6 fewer
    // than the smoke run's: the smoke path's advance is exactly the game's 6 and nothing else
    // moved (the bit is read nowhere else on this path [M the decompile grep: only MuzzleFlash
    // 0x1a4298 and the drone fire-script op 0x1c1b38 read 0x2000]; every other draw cancels).
    let mut smoke = load(&iso);
    let mut quiet = load(&iso);
    quiet.sim.as_mut().unwrap().weapons.records[2][0x69] |= 0x20; // 0x2000 in the LE word at 0x68
    for sc in [&mut smoke, &mut quiet] { arm(sc, 41); }
    let (pre_s, pre_q) = (smoke.sim.as_ref().unwrap().drones.rng.clone(), quiet.sim.as_ref().unwrap().drones.rng.clone());
    smoke.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    quiet.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    let (post_s, post_q) = (smoke.sim.as_ref().unwrap().drones.rng.clone(), quiet.sim.as_ref().unwrap().drones.rng.clone());
    assert!(smoke.sim.as_ref().unwrap().shot_tick && quiet.sim.as_ref().unwrap().shot_tick, "both presses fired");
    let s = smoke.sim.as_ref().unwrap();
    let rate = (0.1 - 0.05) / 128.0;
    assert_eq!(s.drones.gas.puffs.iter().filter(|p| p.kind == 7 && p.scale_rate == rate).count(), 1, "the smoke run made the puff");
    let q = quiet.sim.as_ref().unwrap();
    assert!(q.drones.gas.puffs.is_empty(), "the gated run made no puffs at all (Bond in the sky)");
    let total_s = steps_between(&pre_s, &post_s);
    let total_q = steps_between(&pre_q, &post_q);
    assert_eq!(total_s - total_q, SMOKE_DRAWS, "the puff costs exactly the game's 6 draws (pre-fix: 0)");
    eprintln!("[gas-rng] shot tick draws: smoke {total_s}, gated {total_q}, delta {}", total_s - total_q);
}
