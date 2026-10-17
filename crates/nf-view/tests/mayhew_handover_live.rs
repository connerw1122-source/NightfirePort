//! mayhew-exit-walk: 07000005 (Mayhew A) — the walk to the exit before the mid cut-scene.
//!
//! The level's truck-exit chain [D docs/levels/07000005.md §3; the truck script 0x06000A22]:
//! SP #37 (mode 1) plays the truck script at level start; stream 8 creates the script TouchOnce
//! ch 0x5C at t 1882 and kills it at t 2312 (0x908) [D script decode; the sim trace: the trigger
//! appears between the t 2100/2400 samples] — a static plate whose world centre is
//! (98.49, 27.05, 106.36) (the gatehouse walkway edge above the arch road; the walked lane
//! there is y ≈ 25.9). When Bond touches it, SP #38 (script 0x060008C8, 143 ticks, camera true,
//! Bond skin 0x05000007 on anim 0x06000A24 [D the script's stream 1]) plays — the mid cut-scene
//! that hands Bond onto the waiting truck — and the truck rides on: the bed touch ch 0x71
//! (script stream 7, parented to the truck) parents Bond; the park plate ch 0x70 (stream 6);
//! the gate-guard ch 0x72 (placement #318); mux #316 = AND(0x70, 0x71, 0x72) → SP #320
//! "EndLevel" (0x06000A23, event 0x11 at t 300 [D]) → ch 0x1D → #118 loads 07000006.
//!
//! The user's report ("mayhew is bugged with walking to the exit as well before the mid
//! cutscene") is this chain: walked Bond must reach the gatehouse edge INSIDE the plate's
//! live window. The probe on master (2026-10-17): the real-input walker arrives at the edge
//! (99.69, 25.85, 106.98) at t ≈ 373, waits, ch 0x5C fires while the window is open, SP #38
//! re-places Bond on the truck bed and ch 0x71 follows the ride. There is NO Mayhew drone on
//! 07000005 (16 drones, all guards 0x0500002e/2f/30 [D nf-extract drones]) — "Mayhew" is the
//! level's own name (Mayhew A).
//!
//! Sister: Bond who never walks to the gatehouse never sees the hand-over — ch 0x5C stays
//! low through the whole window and the level does not exit.

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn the_walk_to_the_gatehouse_fires_the_handover_window_and_rides_out() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"));
    // The cold start #26 (gate 0) [D]: (104.95, 25.12, 108.83) — the gatehouse road.
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(p[0] > 100.0 && p[0] < 110.0 && p[2] > 100.0 && p[2] < 115.0,
            "the cold start spawns at the gatehouse road (#26 [D]), got {p:?}");
        assert!(!sim.disabled, "no intro blocks this level's start");
    }
    w.wait(200);
    // The real-input walk (the walker drives step_bond with Input only) to the gatehouse
    // walkway edge — INSIDE the ch-0x5C plate's XZ (centre (98.49, 27.05, 106.36) [D the
    // entity matrix], extent ±1.95/±1.75 [D the TouchplateBig glist box]) — while the truck
    // script's window is still ahead of us (the plate lives t 1882..2312 [D stream 8]).
    w.walk_nav_to([98.6, 26.0, 106.5], "to the gatehouse walkway edge", 1.2);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let b = sim.bond.pos;
        let d = ((b[0] - 98.49).powi(2) + (b[2] - 106.36).powi(2)).sqrt();
        assert!(d < 2.0, "Bond reaches the plate's XZ within the window (t {}), d = {d:.2} at {b:?}",
            w.total_ticks);
        assert!(w.total_ticks < 1882, "the walk arrives before the plate even exists (t {})",
            w.total_ticks);
        assert!(!sim.drones.channels.on(0x5c), "the plate is not up yet [D t 1882]");
    }
    // Wait at the edge through the window: the plate appears, fires, the hand-over scene
    // plays (Bond disabled during the cut-scene, then re-placed on the truck bed).
    let mut fired_at = None;
    for t in 0..3600u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0x5c) { fired_at = Some(w.total_ticks); break; }
    }
    let fired_at = fired_at.expect("ch 0x5C fires while Bond stands at the gatehouse edge [D stream 8's window]");
    eprintln!("[05] ch 0x5C fired at t {fired_at}");
    assert!(fired_at >= 1882 && fired_at <= 2400,
        "the fire lands inside the script's own window (t 1882..2312 [D]), got {fired_at}");
    // The hand-over + the ride: Bond ends up parented to the truck (ch 0x71); the game's ride
    // stance is crouched [D fix-truck's trace] — the walked Bond crouches once the hand-over
    // re-enables him — and the crouched ride parks on the ch-0x70 plate, the chain ends the
    // level into 07000006 [D mux #316 → SP #320 → ch 0x1D → #118].
    let mut rode = false;
    let mut crouched = false;
    let mut input = nf_game::player::Input::default();
    let mut exited = None;
    for _ in 0..6800u32 {
        let sim = w.scene.sim.as_ref().unwrap();
        let on_bed = sim.drones.channels.on(0x71);
        let west = sim.bond.pos[0] < 90.0;
        if !crouched && on_bed && west && !sim.bond.crouched {
            crouched = true;
            input.crouch = true;
        } else {
            input.crouch = false;
        }
        drop(sim);
        w.tick(input, false);
        let sim = w.scene.sim.as_ref().unwrap();
        if on_bed && west { rode = true; }
        if let Some(l) = sim.drones.level_exit { exited = Some(l); break; }
    }
    assert!(rode, "the hand-over puts Bond on the truck bed (ch 0x71) and the ride carries him west of the gatehouse");
    assert_eq!(exited, Some(0x0700_0006),
        "the crouched ride parks on ch 0x70 and the chain ends the level into 07000006 [D docs/levels/07000005.md §3]");
}

/// Sister: Bond who stays at the spawn never fires the hand-over — the plate's window opens
/// and closes (t 1882..2312 [D]) with ch 0x5C low the whole time, and the level does not exit.
#[test]
fn the_handover_never_fires_for_bond_who_stays_at_the_spawn() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..3000u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x5c),
        "the ch-0x5C hand-over never fires without the walk [D the plate is player-mask 8]");
    assert!(sim.drones.level_exit.is_none(), "the truck chain does not end the level from the spawn");
}
