//! level-08-interactables: the Hurt volumes (placement 0xE4) on 07000008 — the level's kill floor.
//!
//! #42/#43/#44 are three 2-triangle quads at y −24.10 tiling the valley (x −55.8..286.1,
//! z −260.4..−110.9 [D the placement collision dumps]), type 6 (fall), 1000 damage, no gate,
//! who 0x18 = classes 3+2 (drones AND the player), one-shot 0 [D placements; M Hurt_Update
//! 0x17e5d0's state-0 who-mask law]. The port never created them before this fix; `hurt.rs`
//! builds and ticks them now (the row law, the who-mask law and the touch law are pinned in
//! `hurt.rs`'s unit tests).
//!
//! The real-path fall: Bond walks off the gondola boarding platform (x ≈ 111.4, z ≈ −127.2, the
//! carriage's board pose [D the ride script's stream-1 keys]) — the plunge TouchOnce #54's floor
//! quad fires the falling cut-scene (ch 0x7C, the game's F5 law) on the way down, and the death
//! NIS (SP #307, 0x06000991 — `Script_IsDeathNIS` [M nis.rs]) then HOLDS the body mid-air: the
//! port has no death after a death NIS (the game fails the mission [M 0x182c60:21-43]), so the
//! kill floor cannot be reached end-to-end through this path yet. The hold is pinned here as the
//! recorded symptom (docs/levels/07000008.md §7, the finding); the volumes stay armed below.
//!
//! Opt-in via NF_ISO.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn free_bond(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
}

/// The real walk off the boarding platform: the fall happens, the plunge fires, and the death NIS
/// holds the body — the port's recorded symptom that blocks the y −24.1 kill floor
/// (docs/levels/07000008.md §7). The volumes themselves stay armed with zero contacts while the
/// body is held above them (the who-mask needs a body contact [M 0x17e5d0 state 0]).
#[test]
fn the_real_walk_off_the_platform_falls_and_the_death_nis_holds_the_body() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    free_bond(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [111.4, 24.5, -127.2];
        sim.bond.yaw = std::f32::consts::PI; // face −z, down the cable chasm
        let start = sim.bond.pos;
        let mut crossed_plunge = false;
        let mut min_y = f32::MAX;
        for t in 1..=1400u32 {
            sim.step_bond(Input { forward: 1.0, ..Input::default() }, false, false, false);
            min_y = min_y.min(sim.bond.pos[1]);
            if sim.drones.channels.on(0x7c) { crossed_plunge = true; }
            if t % 200 == 0 { eprintln!("[t{t}] pos ({:.2},{:.2},{:.2}) plunge {crossed_plunge} grounded {}", sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2], sim.bond.grounded); }
        }
        assert!(sim.bond.pos[2] < start[2] - 8.0, "the walk carried Bond off the platform edge (z {:.2} → {:.2})", start[2], sim.bond.pos[2]);
        assert!(min_y < 20.0, "the fall descended past the platform lip (min y {min_y:.2})");
        assert!(crossed_plunge, "the plunge TouchOnce fired on the way down (the game's F5 law [D #54])");
        let held = (0..300).all(|_| { let p = sim.bond.pos; sim.step_bond(Input::default(), false, false, false); p == sim.bond.pos });
        assert!(held, "the death NIS holds the body (the port's missing post-death-NIS fail — the finding)");
        let hurt_hits: u32 = sim.hurts.iter().map(|h| h.hits).sum();
        assert_eq!(hurt_hits, 0, "the held body never reaches the y −24.1 kill floor (the volumes stay armed)");
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
        let hurt_hits: u32 = sim.hurts.iter().map(|h| h.hits).sum();
        assert_eq!(hurt_hits, 0, "no hurt contact while standing [M the who-mask needs a body contact]");
        assert!(sim.drones.health.health > 0.0, "Bond is unharmed on the platform");
        assert!(!sim.drones.channels.on(0x7c), "the plunge never fires from standing [D]");
    }
}
