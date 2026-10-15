//! fix-level-0c: the hatch chain (the user's soft lock, playtest 2026-10-12). Evidence base
//! (docs/levels/0700000c.md §3/§6 + the disc placements):
//! - The display vent: `ventmesh-front` #138 (class 0xE2, param 3 = ch 18, 6 keys — the hatch
//!   slides from y 16.40 down to 14.76), four burnable hinges `hingelock1/3` #139–142 (class 0x2E
//!   = `Lock_Create` 0x1ae1f8, out chs 14/15/16/17 [D #139: param 5]), `VentBlockingPoly` #179
//!   (0xE2, ch 18, 2 keys — rises y 16.30 → 20.00, rate 10), and the AND mux #505 (type 3, out
//!   0x12 = 18, list [14,15,16,17] [D triggers]).
//! - The warehouse vent: `ventmesh` #180 (0xE2, ch 84, 6 keys), `VentLatch` #182 (0x2E, out 84),
//!   `VentmeshBlockingPloy` #184 (0xE2, ch 84, 2 keys).
//! - `Lock_Update` 0x1ae3d8:194 writes the lock's out channel on burn-through; the laser contact
//!   (the beam's segment-vs-sphere pass) is fix-gadgets'; this file drives the HATCH side: the
//!   mux, the SimpleScripts (`SS_Update` 0x1a9948's playhead on param 3's channel) and the
//!   opening's collision.
//!
//! Opt-in via `NF_ISO`.

use nf_game::door::interp;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load() -> nf_view::scene::Scene {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..100 { sim.step_bond(Input::default(), false, false, false); }
    scene
}

/// The display vent's opening: x 4.5..7.1 (the hinges), y 15.7..16.87, the plane at z −112.25.
/// A ray through the opening's centre, aimed through the plane.
const RAY_FROM: [f32; 3] = [5.78, 16.3, -111.7];
const RAY_TO: [f32; 3] = [5.78, 16.3, -113.0];

fn ray_hits(sim: &nf_view::scene::Sim) -> bool {
    sim.world.segment_hit(RAY_FROM, RAY_TO).is_some()
}

/// Burn (or not) the locks whose out channel is in `outs`, then tick and return (ch18, ss138_t,
/// ss179_t, ray_blocked).
fn setup_and_burn(scene: &mut nf_view::scene::Scene, outs: &[u16], ticks: u32) -> (bool, f32, f32, bool) {
    let sim = scene.sim.as_mut().unwrap();
    for k in 0..sim.drones.locks.len() {
        if outs.contains(&sim.drones.locks[k].out) { sim.drones.locks[k].burn(&mut sim.drones.channels, 1); }
    }
    for _ in 0..ticks { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let ss138 = sim.drones.sss.of_placement(138).expect("ventmesh-front SS exists [D #138]");
    let ss179 = sim.drones.sss.of_placement(179).expect("VentBlockingPoly SS exists [D #179]");
    (sim.drones.channels.on(18), sim.drones.sss.list[ss138].t, sim.drones.sss.list[ss179].t, ray_hits(sim))
}

/// The user's soft lock, reproduced: with the hinges intact the opening is SHUT — the ray through
/// it hits the hatch/blocker meshes at their key-0 poses, and ch 18 is off (it can only rise
/// through the AND mux #505 of the four hinges' burn channels [D #505]).
#[test]
fn the_vent_is_shut_while_the_hinges_hold() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load();
    let (ch18, t138, t179, blocked) = setup_and_burn(&mut scene, &[], 60);
    assert!(blocked, "the closed vent must block a ray through the opening (hatch+blocker at key 0)");
    assert!(!ch18, "ch 18 stays off while the hinges hold");
    assert_eq!((t138, t179), (0.0, 0.0), "neither SS has played");
}

/// Aim Bond's view (the beam's ray) at `target` from his head — the port's look convention
/// (`Bond::look`: pitch ±1 → ±π/2, yaw around +z).
fn aim_at(sim: &mut nf_view::scene::Sim, target: [f32; 3]) {
    let h = sim.bond.head();
    let d = [target[0] - h[0], target[1] - h[1], target[2] - h[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() * 2.0 / std::f32::consts::PI;
}

/// The fix, through the REAL beam: Bond stands on the lower walkway before the display vent with
/// the Laser out (0x4E — the disc's beam weapon, name 0x05000043 "Laser" [L USATxt + weapon_data
/// +0x6C = 0x900; the old 0x50 id was the Grapple]); `Player_LaserPointer`
/// 0x1a65d8's beam (the sim's per-tick `gadget::laser_tick`) burns each hinge it is aimed at
/// (`Lock_Update` 0x1ae3d8 case 0), the AND mux #505 raises ch 18, the SSs play to their last
/// keys (the hatch slides away, the blocker rises out), and the opening is PASSABLE.
#[test]
fn burning_the_four_hinges_opens_the_display_vent() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load();
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.weapon = 0x4e;
    sim.gun.switch_request = 0x4e;
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [6.0, 13.8, -110.5], 0.0);
    let hinges: Vec<[f32; 3]> = sim.drones.locks.iter().filter(|l| (14..=17).contains(&l.out)).map(|l| l.pos).collect();
    assert_eq!(hinges.len(), 4, "the four hinge locks exist [D #139-142]");
    'outer: for h in &hinges {
        for _ in 0..60 {
            aim_at(sim, *h);
            // Fire HELD: the burn runs only while the fire is held (fix-gadgets2, `Player_WeaponFiring` 0x1a5460:442).
            sim.step_bond(Input::default(), true, false, false);
            if sim.drones.locks.iter().filter(|l| (14..=17).contains(&l.out) && l.burned).count() == 4 { break 'outer; }
        }
    }
    let burned = sim.drones.locks.iter().filter(|l| (14..=17).contains(&l.out) && l.burned).count();
    // The rate word is the whole sweep in ticks [M SS_Create 0x1a9c58:51]: #138 rate 50 over 5
    // key-units = 50 ticks + the wrap's overshoot; give 300.
    for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let ss138 = sim.drones.sss.of_placement(138).expect("ventmesh-front SS exists [D #138]");
    let ss179 = sim.drones.sss.of_placement(179).expect("VentBlockingPoly SS exists [D #179]");
    let blocked = ray_hits(sim);
    let ch18 = sim.drones.channels.on(18);
    assert_eq!(burned, 4, "the aimed beam burns all four hinges [M 0x1ae3d8 case 0]");
    assert!(ch18, "the AND mux #505 raised ch 18 [D #505 list 14,15,16,17]");
    // The end-of-sweep truth [M SS_Update 0x1a9948:119-128]: the playhead WRAPS to 0 and the state
    // goes dead (param 1 rearm = 1 [D #138/#179]) — what persists is the POSE at the last key
    // (states 0/0x40 never interp). The old scrub model clamped t at 5.0 forever; the game's
    // observable is the posed matrix (the hatch open, the blocker up) — asserted bit-exact.
    for (k, name) in [(ss138, "ventmesh-front"), (ss179, "VentBlockingPoly")] {
        let ss = &sim.drones.sss.list[k];
        let last_k = (ss.keys.len() - 1) as u16;
        let last = last_k as f32;
        let open = interp(&ss.keys, 0, last_k, last, ss.spline).expect("the last key interp");
        assert_eq!(ss.matrix, open, "{name}'s posed matrix must hold the last (open) key after the sweep");
        assert_eq!(ss.state, 0, "{name} is dead after its sweep (rearm = 1 [D]) [M 0x1a9948:121-125]");
        assert_eq!(ss.t, 0.0, "{name}'s playhead wrapped to 0 [M 0x1a9948:119-120]");
    }
    assert!(!blocked, "the open vent lets the ray through — the soft lock is gone");
}

/// Sister: a hinge burn's channel alone is not enough — the AND needs all four; three hinges
/// leave ch 18 off and the opening shut.
#[test]
fn three_hinges_are_not_enough() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load();
    let (ch18, _t138, _t179, blocked) = setup_and_burn(&mut scene, &[14, 15, 16], 60);
    assert!(!ch18, "the AND mux holds ch 18 with a hinge unburnt [D #505]");
    assert!(blocked, "the vent still blocks");
}

/// The warehouse vent: the single `VentLatch` #182 burn (out 84 [D #182 param 5]) plays
/// `ventmesh` #180 and clears `VentmeshBlockingPloy` #184 (both ch 84) — the second way down.
#[test]
fn the_warehouse_vent_latch_opens_its_hatch() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load();
    let sim = scene.sim.as_mut().unwrap();
    let k = sim.drones.locks.iter().position(|l| l.out == 84).expect("the VentLatch lock exists [D #182]");
    sim.drones.locks[k].burn(&mut sim.drones.channels, 1);
    for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(84), "the latch burn wrote ch 84 [M 0x1ae3d8:194]");
    let ss180 = sim.drones.sss.of_placement(180).expect("ventmesh SS [D #180]");
    let ss184 = sim.drones.sss.of_placement(184).expect("VentmeshBlockingPloy SS [D #184]");
    // The end-of-sweep truth [M SS_Update 0x1a9948:119-128] — the playhead wraps to 0, the state
    // dies (rearm = 1 [D #180/#184]) and the POSE holds the last key (the hatch open, the blocker
    // cleared). The old t==5.0/1.0 pins were the scrub model's clamped playhead, not the game's.
    for (k, name) in [(ss180, "ventmesh"), (ss184, "VentmeshBlockingPloy")] {
        let ss = &sim.drones.sss.list[k];
        let last_k = (ss.keys.len() - 1) as u16;
        let open = interp(&ss.keys, 0, last_k, last_k as f32, ss.spline).expect("the last key interp");
        assert_eq!(ss.matrix, open, "{name}'s posed matrix must hold the last (open) key after the sweep");
        assert_eq!(ss.state, 0, "{name} is dead after its sweep [M 0x1a9948:121-125]");
        assert_eq!(ss.t, 0.0, "{name}'s playhead wrapped to 0 [M 0x1a9948:119-120]");
    }
}
