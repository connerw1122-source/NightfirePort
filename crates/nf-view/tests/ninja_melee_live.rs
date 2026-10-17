//! ninja-melee (queue 728): the 07000004 assassin runs the game's melee chain against Bond.
//!
//! The audit gap ③ acceptance: a headless chase test asserts a sword/flip state is entered when
//! dist < 2, and a star projectile spawns in the NoRoute arm. The state-id ↔ function table is
//! `NDrone2_StateFuncs` [L ee_snapshot 0x29b300]: 0xab GetClose, 0xac Sword, 0xad Somersault,
//! 0xae Backflip, 0xaf Sideflip (the ±3 router), 0xb0/0xb1 the flips, 0xb2 StandFire, 0xb3 NoRoute.
use nf_game::kiko as kiko_states;
use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn free(sim: &mut nf_view::scene::Sim, ticks: u32) -> bool {
    for _ in 0..ticks { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { return true; } }
    !sim.disabled
}

/// A scripted fight: Bond stands inside the melee band and the assassin works the chain
/// (07000004, the level that places the ninja [D 07000004.md — the dtype-0xf assassin at
/// (−2.64, 5.50, 30.08), the story scene at Bond's spot]).
#[test]
fn the_assassin_gets_close_and_swings() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000004.bin", false).expect("load 07000004");
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim, 1800), "Bond is free after the intro");
    let ni = sim.drones.list.iter().position(|d| d.present && d.dtype == 0xf).expect("the assassin exists [D]");
    // Bond beside the assassin (the story-scene spot [D 07000004.md]) and alive throughout.
    scene.sim.as_mut().unwrap().bond = nf_game::player::Bond::new([-3.87, 5.44, 30.30], 0.0);
    let mut saw = std::collections::BTreeSet::new();
    let mut projectiles = 0usize;   // bullets seen in flight, any tick
    let mut hits = 0usize;          // the ninja's landed damage
    for _ in 0..900u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.health.health = 100.0; // the star throws must not end the probe
        sim.step_bond(Input::default(), false, false, false);
        let dr = &sim.drones.list[ni];
        projectiles += sim.drones.bullets.iter().filter(|b| b.owner == ni).count();
        hits += sim.drones.player_hits.iter().filter(|(_, o, _)| *o == ni).count();
        if kiko_states::NINJA_FIGHT_STATES.contains(&dr.state) { saw.insert(dr.state); }
    }
    let sim = scene.sim.as_ref().unwrap();
    let ninja = &sim.drones.list[ni];
    // The close band reached: the run-in 0xab or the sword 0xac (the audit's "sword/flip state
    // entered when dist < 2") [M 0x16cb70 case 3 / 0x16bb50 case 1].
    assert!(saw.contains(&kiko_states::NINJA_GET_CLOSE) || saw.contains(&kiko_states::NINJA_SWORD),
        "the assassin closes to the melee band, saw {saw:02x?} (log tail {:?})",
        &ninja.log[ninja.log.len().saturating_sub(24)..]);
    // A strike or evasion followed the game's transition out of 0xab (≤2 → 0xac [M 0x16cb70:40-45])
    // or out of 0xac (the slash done → 0xa7 [M 0x16d100 case 1 CallAnim next]).
    let seq: Vec<u8> = ninja.log.iter().map(|(_, s)| *s).collect();
    let close_then_sword = seq.windows(2).any(|w| w[0] == kiko_states::NINJA_GET_CLOSE && w[1] == kiko_states::NINJA_SWORD);
    let sword_then_attack = seq.windows(2).any(|w| w[0] == kiko_states::NINJA_SWORD && w[1] == kiko_states::NINJA_ATTACK);
    let evasion = saw.contains(&kiko_states::NINJA_SOMERSAULT) || saw.contains(&kiko_states::NINJA_BACKFLIP)
        || saw.contains(&kiko_states::NINJA_SIDEFLIP_LEFT) || saw.contains(&kiko_states::NINJA_SIDEFLIP_RIGHT)
        || saw.contains(&kiko_states::NINJA_SIDEFLIP);
    assert!(close_then_sword || sword_then_attack || evasion,
        "the game's transitions over the scripted fight, seq {saw:02x?} log {seq:02x?}");
    // The star throws are DroneWeap_Fire(1) with the ninja's weapon [M 0x16cb70:72 tail]: the
    // shots discharge (the projectile is a Bullet with the ninja's placement weapon — 0x12 here
    // [D 07000004 placement]) and land at point-blank (`dist < 3` always hits, fire_weapon).
    assert!(!ninja.shots.is_empty(), "the ninja's throws discharge, shots {:?}", ninja.shots.len());
    // Point-blank the thrown projectile reaches Bond inside its spawn tick (dist < 3 always hits,
    // fire_weapon), so the flight is observed through the damage it lands, not the bullets list.
    assert!(hits > 0, "the melee-band fight lands the game's damage, player_hits");
    let _ = (projectiles, sim);
}

/// The NoRoute arm: entered directly, it throws on ENTER and on every Update (the star spray
/// [M 0x16d878 case 1 + case 3]), and its TIMER_C (+30) returns to 0xa7 when a route exists again
/// [M 0x16d878 case 0xc].
#[test]
fn no_route_throws_stars_and_recovers() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000004.bin", false).expect("load 07000004");
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim, 1800), "Bond is free after the intro");
    let ni = sim.drones.list.iter().position(|d| d.present && d.dtype == 0xf).expect("the assassin exists [D]");
    scene.sim.as_mut().unwrap().drones.list[ni].next_state = Some(kiko_states::NINJA_NO_ROUTE);
    scene.sim.as_mut().unwrap().drones.health.health = 100.0;
    let before = scene.sim.as_ref().unwrap().drones.list[ni].shots.len();
    let mut thrown = 0usize;
    for t in 0..40u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.health.health = 100.0;
        sim.step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let ninja = &sim.drones.list[ni];
        thrown += sim.drones.bullets.iter().filter(|b| b.owner == ni).count();
        if t == 0 {
            assert!((ninja.alert - 1.0).abs() < 1e-4, "NoRoute sets alert 1.0 [M 0x16d878 case 1]");
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(thrown > 0 || sim.drones.list[ni].shots.len() > before,
        "the NoRoute arm spawns the star projectile ({thrown}, shots {} → {})", before, sim.drones.list[ni].shots.len());
    let st = sim.drones.list[ni].state;
    // TIMER_C +30 with a route again → 0xa7 [M 0x16d878 case 0xc]; the 0xa7 ENTER re-routes by
    // distance inside the same send, so the fight chain state (not NoRoute) is the observable.
    assert!(st != kiko_states::NINJA_NO_ROUTE && kiko_states::NINJA_FIGHT_STATES.contains(&st),
        "TIMER_C +30 leaves NoRoute into the attack chain, state {st:#x}");
}
