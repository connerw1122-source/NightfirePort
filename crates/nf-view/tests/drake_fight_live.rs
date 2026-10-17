//! drake-fight-1b: the 0700001b final fight's mechanics — the missile launch sequence
//! (`Create_SpaceMissile` 0x1a9ef0 / `SpaceMissile_Update` 0x1aa118's five states), the
//! `MissileDeploy` bytes (0x30cd70), and the win/fail conditions (AND #139 → ch 9, the
//! state-3 abort → ch 199 → `Mission_Update` 0x1827d8:113-115).
//!
//! Drives the REAL path (`nf_view::load` + `Sim::step_bond`). The launch-order channel 19 is
//! driven directly: it is AND #202's output (ch 0xc1 & 0xc2 → 19 [D #202]) and 0xc2 has **no
//! writer** in the level's 77 scripts or any placement [D sweep 2026-10-17] — the same shape as
//! fix_finale_live's ch 0x0a drive (the gate whose chain head the disc data cannot reach).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn free(sim: &mut nf_view::scene::Sim, ticks: u32) -> bool {
    for _ in 0..ticks { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { return true; } }
    !sim.disabled
}

/// The launch sequence's phases on the game's trigger, then the disarm branch:
/// ch 19 → state 0 arms every missile (`MissileDeploy[k] = 1`, the p3 seeds are all < the 512.0
/// step [M 0x1aa1ec-238]) → the p0 scripts write their gate channels 31..38 at t359 (missile 4's
/// 06000a30 writes ch 192 at t1 instead [D]) → the MissileDissarm SPs #213–220 (gates 31..38 [D])
/// play the console scripts, which write the console channels 1..8 at t682 [D] → each p0 end →
/// state 2 takes the disarm branch (`switch_channels[p4]` set: `MissileDeploy[k] = 2`, obj+0xf6 = 0
/// [M 0x1aa25c-338]) → state 3 writes the per-missile channels 21..28 [M 0x1aa354-398] → AND #139
/// (21 & 22 & 27 → ch 9 [D #139]) completes "Disable the missiles" [L record 1b].
/// Sister: ch 199 (the abort channel) never rises while every missile disarms [M state 3's 0xf6==0
/// arm writes p6, not p5]. Missile 4's console chain is driven with ch 4 [S]: its gate 34's only
/// writer is 06000928 (SP #200, gated on ch 191 ← SP #199 out, gated on 190) and 190 has no writer
/// in the level's scripts [D sweep] — the drive stands in for that unproven chain head.
#[test]
fn the_missile_launch_sequence_reaches_each_phase_and_disarms() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 0700001b");
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 3000), "Bond is free after the shuttle arrival");
        assert!(sim.drones.space_missiles.len() == 8, "the level carries the eight kind-0x39 missiles [D #129/#131–137]");
        // The launch order (AND #202's output; the test drives it — see the module doc).
        sim.drones.channels.set(19, 1, sim.drones.frame);
    }
    // Phase 1 — state 0 arms on the first gated tick (every p3 seed 0..105 < the 512.0 step).
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.missile_deploy.iter().all(|&b| b == 1),
            "state 0 → MissileDeploy[k] = 1 for all eight [M 0x1aa210-238], got {:?}", sim.drones.missile_deploy);
        assert!(sim.drones.space_missiles.iter().all(|m| m.state == 1), "every missile is in state 1 [M 0x1aa21c]");
    }
    // Phase 2 — the p0 scripts' gate writes at t359 [D]; missile 4 wrote ch 192 at its t1 [D].
    for _ in 0..370 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        for g in [31u32, 32, 33, 35, 36, 37, 38] {
            assert!(sim.drones.channels.on(g), "the p0 script's t359 gate write ch {g} [D]");
        }
        assert!(sim.drones.channels.on(192), "missile 4's 06000a30 writes ch 192 at t1 [D]");
    }
    // The console chain: the port's SPs #213–220 play on gates 31..38, the console scripts write
    // ch 1..8 at their t682 [D] (gate 34's ch 34 rise — SP #216 — is disc-positive in the port).
    let mut console = Vec::new();
    for _ in 0..2000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        for k in 1..=8u32 { if sim.drones.channels.on(k) && !console.contains(&k) { console.push(k); } }
        if console.len() == 8 { break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(console.len(), 8, "the console scripts write ch 1..8 at t682 [D], got {console:?}");
    }
    // Phase 3 — each p0 end (1800..3300 [D]) → state 2 → the disarm branch (ch[k] set) →
    // MissileDeploy[k] = 2; the state-3 p2 wait [S 1131] → ch 21..28; AND #139 → ch 9.
    let mut disarmed = false;
    for _ in 0..7000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.missile_deploy.iter().all(|&b| b == 2) { disarmed = true; break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(disarmed, "state 2's disarm branch sets MissileDeploy[k] = 2 [M 0x1aa2cc-338], got {:?}", sim.drones.missile_deploy);
        assert!(sim.drones.space_missiles.iter().all(|m| m.launch_flag == 0), "obj+0xf6 = 0 on the disarm path [M 0x1aa2bc]");
    }
    let mut ch9 = false;
    for _ in 0..3000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.channels.on(9) { ch9 = true; break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(ch9, "state 3 writes ch 21..28 [M 0x1aa354-398] → AND #139 (21&22&27) → ch 9 [D #139]");
        for c in [21u32, 22, 27] {
            assert!(sim.drones.channels.on(c), "AND #139's input ch {c} [D p6]");
        }
        // Sister: ch 24 rose (missile 4's state 3) and is then cleared by the speech script
        // 060009a4's own ch-24 stream [D decode: "ch 24 @t1200/1510-ish"] — the game's AND watches
        // only 21/22/27, so the clear is disc behaviour, not a port bug.
        assert!(!sim.drones.channels.on(199), "sister: the abort channel stays clear on the disarm path [M 0x1aa354 vs 0x1aa360]");
        // The objective monitor folds ch 9 into objective 1 the tick after (the monitor runs at the
        // top of the tick [M 0x1827d8; fix-objective-banner]).
        for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.mission.states.iter().flatten().any(|&s| s == 4),
            "objective 1 'Disable the missiles' completes [L record 1b: text 0x04000059, ch 9]");
    }
}

/// The launch (abort) branch and the fail condition: with ch 19 driven and missile 4's console
/// channel unreachable (gate 34's writer sits behind the unproven 190/191 chain [D sweep]),
/// missile 4's state 2 takes the launch branch (`MissileDeploy[3] = 3`, obj+0xf6 = 1 [M
/// 0x1aa2d4-338]) and its state 3 writes ch 199 [M 0x1aa360-3c4] — the mission fails with
/// FailLabel 0x0200_005a [M 0x1827d8:113-115]. Sister: the disarmed missiles still write their
/// done channels (21..28 minus the AND's inputs stay level-true to the state machine).
#[test]
fn the_missile_abort_path_fails_the_mission_on_ch_199() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 0700001b");
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 3000), "Bond is free");
        sim.drones.channels.set(19, 1, sim.drones.frame);
    }
    // Test setup: remove the MissileDissarm SPs #213–220 (0-based 212..219) so the console
    // channels 1..8 never rise — the proven precondition for the [M] state-2 launch branch
    // (`switch_channels[p4]` clear). The SPs themselves are the disarm chain under test A.
    scene.sim.as_mut().unwrap().drones.scripts.players.retain(|p| !(212..=219).contains(&p.placement));
    // Missile 4 reaches state 2 with ch 4 clear → the launch branch.
    let mut launched = false;
    for _ in 0..2000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.missile_deploy[3] == 3 { launched = true; break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(launched, "missile 4's state 2 takes the launch branch (ch 4 unreachable [D sweep]) → MissileDeploy[3] = 3 [M]");
        assert_eq!(sim.drones.space_missiles.iter().find(|m| m.index == 4).unwrap().launch_flag, 1, "obj+0xf6 = 1 on the launch path [M 0x1aa31c]");
    }
    // The p1 wait [S 1001] → state 3 writes ch 199 [M] → the mission's fail hook [M 0x1827d8:113-115].
    let mut failed = false;
    for _ in 0..3000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.mission.fail_label == 0x0200_005a { failed = true; break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(199), "state 3's abort arm writes ch[p5] = 199 [M 0x1aa360-3c4]");
        assert!(failed, "the mission fails on ch 199 with FailLabel 0x0200_005a [M 0x1827d8:113-115]");
        // Sister: a disarmed missile's state 3 writes its done channel (21: p0 3300 + the p2 wait),
        // never 199 — wait out missile 1's state-3 window.
        let mut sister = false;
        for _ in 0..3000u32 {
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
            if scene.sim.as_ref().unwrap().drones.channels.on(21) { sister = true; break; }
        }
        assert!(sister, "the disarmed missiles' done channels rise on the same run [M 0x1aa354-398]");
    }
}
