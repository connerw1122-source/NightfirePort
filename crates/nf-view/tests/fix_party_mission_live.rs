//! fix-party-mission repro/verification tests (07000007 party + 07000008 gondola), driven through the
//! REAL path (`nf_view::load` + `Sim::step_bond`).
//!
//! Playtest 2026-10-12 items this file pins:
//! 1. the start guards shoot Bond (they must not until he is armed/sees a weapon);
//! 2. punching a guard fails the mission at once; shooting a guest does not;
//! 3. punching a guest T-poses the other guests;
//! 4. the mission music keeps playing over the cut-scenes;
//! 5. objective banners show "NEW OBJECTIVE" without the objective text and run during cut-scenes;
//! 6. the guards are not in their proper places after the Kimiko cut-scene.

use nf_game::drone::state;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Probe: what do the drones at the party start look like (states, teams, health)?
#[test]
fn probe_party_start_guards() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    // Trace alert growth for the two entrance guards (skins 0x5000013/14 near the spawn).
    for t in 0..3200u32 {
        sim.step_bond(Input::default(), false, false, false);
        if t % 100 == 0 {
            let g: Vec<(usize, f32, u8)> = sim.drones.list.iter().enumerate()
                .filter(|(_, d)| d.present && d.team == 1)
                .filter(|(_, d)| ((d.pos[0] - 41.0).powi(2) + (d.pos[2] + 71.0).powi(2)).sqrt() < 6.0)
                .map(|(i, d)| (i, d.alert, d.state)).collect();
            eprintln!("[party] t={t} disabled={} entrance-guards {:?}", sim.disabled, g);
        }
        if !sim.disabled && t > 100 { break; }
    }
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    tick_idle(sim, 200);
    for (i, d) in sim.drones.list.iter().enumerate() {
        if !d.present { continue; }
        let dist = ((d.pos[0] - sim.bond.pos[0]).powi(2) + (d.pos[2] - sim.bond.pos[2]).powi(2)).sqrt();
        eprintln!("[party] drone {i}: skin {:#x} state {:#04x} team {} alert {:.2} hp {:.0} kind {} dtype {} mode {} mn {} ma {} alt_st {:#x} 2nd_ch {} started {} pos {:?} dist {:.1}",
            d.skin, d.state, d.team, d.alert, d.health, d.kind, d.dtype, d.mode, d.mode_normal, d.mode_alert, d.alt_state, d.second_channel, d.started, d.pos, dist);
    }
    eprintln!("[party] bond at {:?} weapon {}", sim.bond.pos, sim.gun.weapon);
    eprintln!("[party] health {:.1} ch0x47 {}", sim.drones.health.health, sim.drones.channels.on(0x47));
}

/// ── Item 1: the entrance bouncers must not shoot unarmed Bond at the party start ────────────────
/// Playtest 2026-10-12: "At start of this mission, enemies, the guards, are shooting at me. They
/// shouldn't be shooting at me at all. It's only the ones in the beginning."
///
/// The game's gate: CivilianGuard UPDATE (0x1603a8 case 3) attacks only when
/// `NDrone2_NearArmedPerson` 0x1787c8 (prop 0x3c + Bond in sight `+0x228 & 4` +
/// `DroneWeap_OpponentHasWeapon` 0x17a4b8 → `ObjectIsArmed` 0x178768 on Bond's class byte) or
/// `ObjectIsArmed(Bond) && (+0x228 & 4)` — Bond at the party start holds the FISTS: InitWeapon
/// 0x1a7ef8 case 0x7000007 writes +0x63 = 1 and WeaponSelect 0x1a2cc0 copies the request into the
/// class byte (+0x62) at the switch's start, so `ObjectIsArmed` (classes 2..0x47 / 0x6a..0x71) is
/// false for the whole pre-hall phase. The bouncers BondTalk instead.
#[test]
fn the_party_entrance_guards_never_shoot_unarmed_bond_at_the_start() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    // The fists switch request lands within ~100 ticks (walk_07000007); tick well past it.
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 1, "Bond holds the fists at the party start [M 0x1a7ef8 case 0x7000007]");
    // Sister: nobody else drew either — the party is calm.
    let combat: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && d.started && !d.corpse)
        .filter(|(_, d)| matches!(d.state, 0x56..=0x7e | 0x85..=0x99))
        .map(|(i, _)| i).collect();
    assert!(combat.is_empty(), "no drone is in combat at the party start, got {combat:?} states {:?}",
        combat.iter().map(|&i| sim.drones.list[i].state).collect::<Vec<_>>());
    assert_eq!(sim.drones.health.health, 100.0,
        "Bond takes no damage from the party guards [playtest: they shot him]");
}

/// Probe: bouncer props (0x3c, 0x1f, 0x33) + sight variables.
#[test]
fn probe_bouncer_props() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 150);
    let data = sim.data();
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if dr.present && dr.team == 1 && dr.mode == 0x13 {
            eprintln!("[party] bouncer {i}: kind {} p3c {} p1f {} p33 {} p51 {} seen {} los {} bond_armed {}",
                dr.kind, dr.prop(&data, 0x3c) as u8, dr.prop(&data, 0x1f) as u8,
                dr.prop(&data, 0x33) as u8, dr.prop(&data, 0x51) as u8,
                dr.seen as u8, dr.los as u8, dr.bond_armed as u8);
        }
    }
}
