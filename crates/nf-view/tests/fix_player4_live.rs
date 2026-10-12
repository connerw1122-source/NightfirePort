//! fix-player4 repro tests: fists and melee, driven through the REAL path the viewer uses
//! (`nf_view::load` + `Sim::step_bond`).
//!
//! The evidence (fix-player3 gathered it; this session ports it):
//! - the fists (weapon 1) are ammo_type 0, clip 1, range +0x1C = 1.75, damage +0x0C = 5.0, speed 1.0,
//!   fire rate +0x40 = 30, fire anim +0x44 = 6, flags +0x68 = 0x880A2 (0x80000 = the random
//!   fire-script pick), +0xB0 fire script 0x060006DE, +0xB4 fire script 0x060006DD
//!   [L ee_start_05.bin record 1 @ 0x2bf25c].
//! - `NDrone2_DealWithObjHit` case 5 (0x13fff0:269-306): a PLAYER bullet's weapon decides the drone
//!   message — 1 (fists) / 0x4B / 0x4D → msg 6 (punch); 0x4A / 0x4C → msg 7 (taser); else msg 8
//!   (Drone_BulletHit). The port sent msg 8 for every weapon, so a punch was a normal bullet.
//! - `DroneFunc_HandleImpact` case 6 (0x1469b8:34-57) → `NDrone2_PunchImpact` (0x145f28): the punch
//!   applies its damage (`DoHitEffects` → `NDrone2_HitDamage`) and at health ≤ 0 enters state 0x42
//!   `NDrone2_DSTATE_KnockedOut_Anim` (0x172a30) — unless the bot-punch mode 0x1E (→ 0xF2, not ported).
//! - `KnockedOut_Anim` ENTER (0x172a30 disasm): obj+0xF0 |= 0x20 (corpse), alert status 3, health = 0,
//!   `CallAnim(0, 0x50, 0, next 0x47)`, Drone+0x4F8 |= 0x1000000, `DroneWeap_DropWeapon`, and
//!   `PlrStat_LogEnemyDisabled` when +0x44 == 1. EXIT: `SetDeathChannel` on every level except
//!   {0x07000009, 0x0700000A} (the `sltiu` at 0x172b14 is UNSIGNED [L disasm]).
//! - The KO flag +0x1000000 makes `DroneFunc_OnInitDeath` (0x145bd0:22,57-59) and
//!   `NDrone2_LogDroneDeath` (0x14ab00) skip `PlrStat_LogEnemyDispatched`: a KO'd guard counts as
//!   DISABLED, not killed.
//! - `Player_SetFiringAnim` (0x1a3c70:34): flag 0x80000 weapons (the fists) draw `Rand(100)` per shot
//!   and play the +0xB4 script on > 50 — the left/right punch alternation. Flag 0x40000 weapons
//!   strictly alternate via the BLData+0x958 toggle.
//!
//! Opt-in via `NF_ISO`.

use nf_game::drone::state;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    let intro: &[u32] = match level {
        "07000001.bin" => &[0x0600_0087],
        "07000002.bin" => &[0x0600_0084],
        _ => &[],
    };
    skip_active_cutscene(&mut scene, intro);
    scene
}

/// Skips the named level-start cut-scenes the way the game lets the player: hold the skip action
/// (`Scripts::skip_request` → `Script_FFwd` 0x1dac78) until the script is done.
fn skip_active_cutscene(scene: &mut nf_view::scene::Scene, intros: &[u32]) {
    if intros.is_empty() { return; }
    let pending = |sim: &nf_view::scene::Sim| sim.drones.scripts.players.iter().any(|p| intros.contains(&p.scripts[0]) && (matches!(p.state, 1 | 2 | 7) || p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)));
    for t in 0..600u32 {
        let sim = scene.sim.as_mut().unwrap();
        if t >= 2 && !pending(sim) && !sim.disabled { break; }
        sim.drones.scripts.skip_request = true;
        sim.step_bond(Default::default(), false, false, false);
    }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the level-start cut-scene never handed control back");
}

/// Point Bond's view at `to` from his eye (setup helper; `turn` stays 0 so the aim holds).
fn aim_at(sim: &mut nf_view::scene::Sim, to: [f32; 3]) {
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
}

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// A live armed guard (present, enabled, an enemy with a gun) — for the PUNCH. `drawn` is NOT
/// required: the game's bullet scan (`Collide_PickObj` via `NDrone2_DealWithObjHit`) has no drawn
/// check — a guard who ran at Bond and left the draw list is still punchable [M 0x13fff0 case 5].
fn first_guard(sim: &nf_view::scene::Sim, skip: usize) -> Option<usize> {
    sim.drones.list.iter().enumerate()
        .filter(|(i, dr)| *i >= skip && dr.present && dr.enabled && !dr.corpse && dr.health > 0.0 && dr.team == 1 && (2..0x47).contains(&dr.weapon))
        .map(|(i, _)| i).next()
}

/// A DRAWN armed guard — for the shooting sister (the shot must actually reach a guard Bond can see).
fn first_drawn_guard(sim: &nf_view::scene::Sim, skip: usize) -> Option<usize> {
    sim.drones.list.iter().enumerate()
        .filter(|(i, dr)| *i >= skip && dr.present && dr.enabled && dr.drawn && !dr.corpse && dr.health > 0.0 && dr.team == 1 && (2..0x47).contains(&dr.weapon))
        .map(|(i, _)| i).next()
}

/// Stand Bond `dist` units in front of the guard (−Z side) at floor height and aim at his chest.
fn face_guard(sim: &mut nf_view::scene::Sim, gi: usize, dist: f32) {
    let gp = sim.drones.list[gi].pos;
    let floor = sim.world.floor_below(gp, 6.0).unwrap_or(gp[1]);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [gp[0], floor + 1.1, gp[2] - dist], 0.0);
    aim_at(sim, [gp[0], gp[1] + 0.2, gp[2]]);
}

/// Switch weapons through the real cycle input and wait for the holster/draw cycle to finish. The
/// request is retried: `Player_WeaponChange` refuses switches while a view script (fire/reload) plays
/// [M 0x1a6f80:33-47], so a request sent on that tick is lost — the game reads the wheel again.
fn cycle_to(sim: &mut nf_view::scene::Sim, dir: i8, want: u8) {
    for _ in 0..8 {
        if sim.gun.weapon == want && !sim.gun.switching() { break; }
        sim.step_bond(Input { weapon_cycle: dir, ..Default::default() }, false, false, false);
        for _ in 0..240 {
            sim.step_bond(Default::default(), false, false, false);
            if !sim.gun.switching() && sim.gun.weapon == want { break; }
        }
    }
    assert_eq!(sim.gun.weapon, want, "the cycle reaches weapon {want}");
    tick_idle(sim, 4);
}

/// Switch to the fists through the real cycle input: from the P2K (weapon 2) one cycle step down
/// lands on weapon 1 (the cycle bounds are 1..0x47 [M Player_WeaponChange 0x1a6f80]).
fn select_fists(sim: &mut nf_view::scene::Sim) { cycle_to(sim, -1, 1); }

/// ── Item 1: the knock-out punch ─────────────────────────────────────────────────────────────────
/// A guard punched with the fists must enter the KO state 0x42 (`KnockedOut_Anim`) and never the
/// death state 0x44 — the punch bullet routes msg 6 (`NDrone2_DealWithObjHit` 0x13fff0 case 5) and
/// `NDrone2_PunchImpact` (0x145f28) sends health ≤ 0 to 0x42, not Death_Anim. The KO counts as
/// DISABLED (`PlrStat_LogEnemyDisabled` 0x172ad8), never DISPATCHED (the +0x1000000 flag makes
/// `OnInitDeath` 0x145bd0 skip it), and the guard drops his gun (`DroneWeap_DropWeapon` 0x172ac0).
#[test]
fn punching_a_guard_knocks_him_out_without_killing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);

    // Sister (the contrast) FIRST, while Bond still holds the P2K: a guard SHOT from 3 units dies through
    // Death_Anim and counts as dispatched, not disabled.
    let Some(sgi) = first_drawn_guard(sim, 0) else { panic!("07000005 starts armed, drawn guards") };
    let wpn = sim.drones.list[sgi].weapon;
    let base = sim.weapons.base(wpn as usize);
    face_guard(sim, sgi, 3.0);
    tick_idle(sim, 10);
    let mut shot_died = false;
    for t in 0..600 {
        face_guard(sim, sgi, 3.0);
        sim.step_bond(Default::default(), t % 9 == 0, false, false);
        if sim.drones.list[sgi].health <= 0.0 || sim.drones.list[sgi].corpse { shot_died = true; break; }
        if first_drawn_guard(sim, 0) != Some(sgi) { break; } // he died and left the filter
    }
    assert!(shot_died, "the P2K kills the guard from 3 units");
    let shot_dr = &sim.drones.list[sgi];
    assert!(shot_dr.dispatched_logged, "a shot guard logs EnemyDispatched [M 0x145bd0:57-59]");
    assert!(!shot_dr.disabled_logged, "a shot guard is not 'disabled' [M 0x172ac8 is KO-only]");
    assert!(!shot_dr.knocked_out, "a shot guard never carries the KO flag");
    assert!(!shot_dr.log.iter().any(|(_, s)| *s == state::KNOCKED_OUT_ANIM), "no KO state on a shot guard");
    let shot_states: Vec<u8> = shot_dr.log.iter().map(|(_, s)| *s).collect();
    eprintln!("[punch] shot guard {sgi} (weapon {wpn:#x}, base {base:#x}) dispatched, states {shot_states:?}");

    // The punch: Bond switches to the fists and punches the next guard until he goes down.
    select_fists(sim);
    let Some(gi) = first_guard(sim, 0) else {
        for (i, dr) in sim.drones.list.iter().enumerate() {
            if dr.present { eprintln!("[punch] drone {i}: present {} en {} drawn {} corpse {} hp {} team {} wpn {:#x} state {:#x} pos {:?}", dr.present, dr.enabled, dr.drawn, dr.corpse, dr.health, dr.team, dr.weapon, dr.state, dr.pos); }
        }
        panic!("a second armed guard exists for the punch");
    };
    let punch_states_before = sim.drones.list[gi].log.len();
    face_guard(sim, gi, 1.15);
    tick_idle(sim, 6);
    let mut ko = false;
    let mut punches = 0u32;
    let mut last_fire = 0i32;
    for t in 0..1500u32 {
        face_guard(sim, gi, 1.15);
        // Punch once per fire interval (fire rate +0x40 = 30 [L]); hold-release so each press is fresh.
        let press = last_fire < 0 || t as i32 - last_fire >= 34;
        if press { last_fire = t as i32; punches += 1; }
        sim.step_bond(Default::default(), press, false, false);
        let dr = &sim.drones.list[gi];
        if dr.knocked_out || dr.log[punch_states_before..].iter().any(|(_, s)| *s == state::KNOCKED_OUT_ANIM) { ko = true; break; }
        if dr.log[punch_states_before..].iter().any(|(_, s)| *s == state::DEATH_ANIM) {
            panic!("the punch KILLED the guard (state 0x44) — the port sent msg 8 instead of msg 6");
        }
        if !dr.present { break; }
    }
    assert!(ko, "the guard never entered the KO state after {punches} punches (health {})",
        sim.drones.list[gi].health);
    let dr = &sim.drones.list[gi];
    assert!(dr.knocked_out, "the KO flag +0x1000000 is set [M 0x172ab0-c4]");
    assert!(!dr.log.iter().any(|(_, s)| *s == state::DEATH_ANIM), "Death_Anim 0x44 never entered — the guard is not dead");
    assert!(dr.disabled_logged, "PlrStat_LogEnemyDisabled ran (team 1) [M 0x172ac8-dc]");
    assert!(!dr.dispatched_logged, "a KO'd guard is NOT dispatched [M 0x145bd0:22 KO-flag skip]");
    assert_eq!(dr.health, 0.0, "the KO enter zeroes health [M 0x172a88]");
    let ko_states: Vec<u8> = dr.log.iter().map(|(_, s)| *s).collect();
    // The KO drop: DroneWeap_DropWeapon at KO enter — the same one-visible-gun drop as death.
    tick_idle(sim, 30);
    let mine: Vec<&nf_game::pickup::Pickup> = sim.drones.pickups.iter().filter(|p| p.from == Some(gi)).collect();
    let visible: Vec<&nf_game::pickup::Pickup> = mine.iter().filter(|p| !p.hidden).copied().collect();
    assert_eq!(visible.len(), 1, "the KO'd guard drops exactly his gun (visible kind 0); all: {:?}",
        mine.iter().map(|p| (p.weapon, p.kind, p.hidden)).collect::<Vec<_>>());
    assert_eq!(visible[0].kind, nf_game::pickup::kind::WEAPON, "the KO drop is kind 0 [M 0x17a2a0]");
    let ko_drop = visible[0].weapon;
    eprintln!("[punch] guard {gi} KO after {punches} punches, states {ko_states:?}, drop {ko_drop}");
}

/// ── Item 2: the punch alternation ───────────────────────────────────────────────────────────────
/// `Player_SetFiringAnim` (0x1a3c70:34): the fists (flag 0x80000) draw `Rand(100)` per shot and play
/// the +0xB4 script (0x060006DD) when it is > 50, else +0xB0 (0x060006DE) — the left/right punches.
/// The test punches 10 times through the input path, predicts each pick from a clone of the shared
/// RNG taken just before the press, and asserts both scripts appear.
#[test]
fn the_punch_fire_script_alternates_left_and_right() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    const FIRE: u32 = 0x0600_06de;
    const FIRE_ALT: u32 = 0x0600_06dd;
    let mut scene = load(&iso, "07000005.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    // Both scripts are in the level archive (the fists' +0xB0/+0xB4 [L ee_start_05.bin record 1]).
    assert!(sim.scripts.contains_key(&FIRE), "the +0xB0 punch script is in 07000005");
    assert!(sim.scripts.contains_key(&FIRE_ALT), "the +0xB4 punch script is in 07000005");
    select_fists(sim);

    // Punch in the open air (no guard near Bond): the fire script plays regardless of a hit.
    let mut seen: Vec<u32> = Vec::new();
    let mut mismatches: Vec<(usize, bool, u32)> = Vec::new();
    let mut last_fire: i32 = -100;
    for t in 0..1500u32 {
        if seen.len() >= 10 { break; }
        let press = t as i32 - last_fire >= 36;
        if press {
            last_fire = t as i32;
            // The game's pick is the next shared-RNG draw (nothing in the player block draws before
            // `Player_SetFiringAnim` on a fresh punch with no target).
            let mut expect_rng = sim.drones.rng.clone();
            let alt = expect_rng.rand(100) > 50;
            let before = sim.view_script;
            sim.step_bond(Default::default(), true, false, false);
            if sim.view_script != before && matches!(sim.view_script, FIRE | FIRE_ALT) {
                seen.push(sim.view_script);
                let want = if alt { FIRE_ALT } else { FIRE };
                if sim.view_script != want { mismatches.push((seen.len(), alt, sim.view_script)); }
            }
            continue;
        }
        sim.step_bond(Default::default(), false, false, false);
    }
    assert_eq!(seen.len(), 10, "10 punches fired; got {seen:?}");
    assert!(mismatches.is_empty(), "every pick follows Rand(100) > 50 [M 0x1a3c70:34]; mismatches {mismatches:?}");
    assert!(seen.contains(&FIRE), "the +0xB0 punch plays; seen {seen:?}");
    assert!(seen.contains(&FIRE_ALT), "the +0xB4 punch plays (the alternation); seen {seen:?}");
    eprintln!("[punch] scripts {seen:?}");
}
