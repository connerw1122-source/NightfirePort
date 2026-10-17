//! level-08-interactables: the Hurt volumes (placement 0xE4) on 07000008 — the level's kill floor.
//!
//! #42/#43/#44 are three 2-triangle quads at y −24.10 tiling the valley (x −55.8..286.1,
//! z −260.4..−110.9 [D the placement collision dumps]), type 6 (fall), 1000 damage, no gate,
//! who 0x18 = classes 3+2 (drones AND the player), one-shot 0 [D placements; M Hurt_Update
//! 0x17e5d0's state-0 who-mask law]. The port never created them before this fix: a body falling
//! into the valley took nothing from the volumes.
//!
//! The real path: Bond walks off the gondola boarding platform (x ≈ 112, z ≈ −127, y ≈ 24–27
//! [D the ride script's stream-1 carriage keys + the plunge AABB note in docs/levels/07000008.md])
//! into the chasm — the plunge TouchOnce #54's floor quad at y 17.5 fires the falling cut-scene
//! (ch 0x7C, the game's own F5 law) on the way down, and the hurt quads take the 1000 at the
//! bottom. The sister stands on the platform: no contact, no damage, no plunge.
//!
//! Opt-in via NF_ISO.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn free_bond(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
}

/// Walking off the boarding platform into the chasm: the fall reaches the y −24.1 kill floor and
/// the hurt volumes apply their 1000 (type 6) — the same tick family the game's collide pass
/// drives [M Hurt_Update 0x17e5d0 + Hurt_GetDamage 0x17ea18].
#[test]
fn the_valley_kill_floor_takes_the_falling_bond() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    free_bond(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        // The lower station's platform edge: start #8's own spot (118.7, 35.0, −113.2) [D #8] —
        // z −113.2 stands over the hurt zone (z −110.9..−260.4 [D #42-44 bounds]). WALK off the
        // edge (the real operation; the fall is the level's response).
        sim.bond.pos = [118.7, 34.6, -113.4];
        sim.bond.yaw = std::f32::consts::PI; // face −z, over the valley
        let mut crossed_plunge = false;
        let mut hurt_hits = 0u32;
        let mut min_health = f32::MAX;
        eprintln!("[start] disabled {} grounded {} sub {} pos {:?}", sim.disabled, sim.bond.grounded, sim.bond.substate, sim.bond.pos);
        for t in 1..=4800u32 {
            // Walk input: forward (the run flag false — a plain walk off the edge).
            sim.step_bond(Input { forward: 1.0, ..Input::default() }, false, false, false);
            if t <= 20 { eprintln!("[t{t}] pos {:?} vel {:?} grounded {} disabled {}", sim.bond.pos, sim.bond.vel, sim.bond.grounded, sim.disabled); }
            let h = sim.drones.hurts.iter().map(|h| h.hits).sum::<u32>();
            if h != hurt_hits { hurt_hits = h; eprintln!("[t{t}] hurt hit #{h} at y {:.2}", sim.bond.pos[1]); }
            if sim.drones.channels.on(0x7c) { crossed_plunge = true; }
            min_health = min_health.min(sim.drones.health.health);
            if t % 200 == 0 { eprintln!("[t{t}] pos ({:.2},{:.2},{:.2}) health {:.1} hurt {hurt_hits} plunge {crossed_plunge} grounded {}", sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2], sim.drones.health.health, sim.bond.grounded); }
            if hurt_hits > 0 && min_health <= 0.0 { break; }
        }
        eprintln!("[final] y {:.2} health {:.1} hurt {hurt_hits} plunge {crossed_plunge}", sim.bond.pos[1], sim.drones.health.health);
        assert!(hurt_hits > 0, "the falling body took the kill floor's contact [D #42-44 at y −24.1]");
        assert!(crossed_plunge, "the fall crossed the plunge shell on the way down (the game's F5 law)");
        assert!(min_health <= 0.0, "1000 type-6 damage kills [M Hurt_GetDamage 0x17ea18]");
    }
}

/// Sister: standing on the boarding platform touches nothing — the volumes stay armed, health is
/// untouched, the plunge never fires (the game's player does not die on the platform [D
/// docs/levels/07000008.md §6 — the platform sits inside #54's AABB touching neither quad]).
#[test]
fn standing_on_the_platform_touches_no_kill_floor() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    free_bond(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [111.4, 24.5, -127.2];
        sim.bond.yaw = std::f32::consts::PI;
        for t in 1..=1200u32 {
            sim.step_bond(Input::default(), false, false, false); // no input: Bond stands
            if t % 300 == 0 { eprintln!("[t{t}] y {:.2} health {:.1}", sim.bond.pos[1], sim.drones.health.health); }
        }
        let hurt_hits: u32 = sim.drones.hurts.iter().map(|h| h.hits).sum();
        assert_eq!(hurt_hits, 0, "no hurt contact while standing [M the who-mask needs a body contact]");
        assert!(sim.drones.health.health > 0.0, "Bond is unharmed on the platform");
        assert!(!sim.drones.channels.on(0x7c), "the plunge never fires from standing [D]");
    }
}
