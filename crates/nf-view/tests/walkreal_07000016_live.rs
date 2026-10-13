//! walkreal-c: 07000016 (the missile base launch centre) walked with the player's own movement —
//! no `bond.pos` writes after the spawn. The route (docs/levels/07000016.md; the walkreal_probe_c
//! dump): the arrival (5.17, 0.90, −65.30, the carried 07000014/15 chain) → the spawn plate #252
//! fires ch 0x8e (completes the carried record 21) → the south corridor (doors [7]/[0]/[1]/[3]/[2]
//! — [0]/[3]/[2] are locked by ch 0x24/0x27/0x28, whose writers are the mask-0x04 activate
//! triggers beside them; the stuck-mitigation's E press presses them) → the silo plate #253 fires
//! ch 0x8f (completes "Reach Drake's launch silo", reveals the Kiko objective) → the south room's
//! ch-0x81 touch trigger at (2.4, 1.0, −176.4) fires the launch chain [D] (the cinema plays OUT —
//! no skip: its stream writes ch 185 → SP #272 → ch 240/242 → the spawners) → the chaser (Kiko,
//! #261) spawns and is defeated with Bond's own fire → ch 0x90 → AND #271 → ch 100 → MissionState
//! 6 → the driving hand-off 0x7300017 [M 0x182c60:111-112].
//!
//! Enemies are DISABLED for the walk only in the walkreal sense: damage is suppressed
//! (top_up_health, the walker's allowance). Kiko dies to Bond's own bullets. Opt-in via NF_ISO.

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::{Input, RamSave};
use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    // A fresh Bond with only the carried gates: the save Trigger_Activate would make [M 0x1b2318:97-116].
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

// GAP (walkreal-c): the corridor walk stalls at door[7] (4.3,−0.1,−74.2, unlocked, flags 0x3, locked_flag 0) — the E reaches (use-sphere d² 3.36 < 6.25) and Doors::activate passes its guards, yet the leaf never moves (14 use_door calls, 840 ticks). A kind-2 slide at 1.0 BELOW the approach floor; door[0] on 07000015 (same kind, same height) opens. Root cause not found in the session — live check requested (docs/reports/walkreal-c.md).
#[test]
#[ignore = "the walk's reached blocker — see docs/reports/walkreal-c.md; un-ignore when the blocker lands"]
fn walk_arrival_to_the_chasers_defeat() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000016.bin", false, Some(&carry(&[0x47, 0x48, 0x4a, 0x54, 0xce, 0x4b]))).unwrap());
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(p[0] > 3.0 && p[0] < 7.0 && p[1] > -1.0 && p[1] < 2.0 && p[2] > -67.0 && p[2] < -63.0,
            "the arrival spawns at the Omega exit's far side (#1 [D]), got {p:?}");
    }
    w.wait_enabled(3000, "the launch-centre arrival");
    w.wait(200);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x8e), "the spawn plate #252 fires ch 0x8e [D]");
        assert!(sim.drones.mission.states[21].iter().all(|&s| s == 4), "the carried record 21 (07000015) fully completes");
        assert_eq!(sim.drones.mission.states[22][0], 3, "'Reach Drake's launch silo' announces [L record 16 obj 1]");
    }

    // The south corridor. Doors: [7] (−74.2, unlocked), [0] (−114.2, lock 0x24), [1] (−124.2,
    // unlocked), [3] (−141.9, lock 0x27), [2] (−152.3, lock 0x28). The locked ones' writers are
    // the mask-0x04 activate triggers beside them ((3.4,0.8,−141.5) → 0x27, (3.4,0.8,−151.9) →
    // 0x28) — the player's E on the panel; the stuck-mitigation presses E at each closed door.
    let route: &[(&str, walker::Wp)] = &[
        ("the corridor south", walker::Wp::new(4.3, 0.6, -72.5).r(1.2)),
        ("door[7] through", walker::Wp::new(3.5, -0.1, -78.0).r(1.2)),
        ("the descent", walker::Wp::new(2.5, -0.1, -95.0).r(1.2)),
        ("door[0] through", walker::Wp::new(2.0, -0.1, -119.0).r(1.2)),
        ("door[1] through", walker::Wp::new(2.0, -0.1, -132.0).r(1.2)),
        ("door[3] through", walker::Wp::new(2.0, -0.1, -147.0).r(1.2)),
        ("door[2] through", walker::Wp::new(2.2, -0.1, -151.5).r(1.2)),
    ];
    w.walk_route(route);

    // The silo plate #253 (0.45-4.62, −0.20..1.88, −154.93..−152.85 [D]): walk onto its slab.
    w.walk_to(walker::Wp::new(2.5, 0.3, -153.9).r(1.0), "the silo plate");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x8f), "the silo plate #253 fires ch 0x8f [D]");
        assert_eq!(sim.drones.mission.states[22][0], 4, "the monitor completes 'Reach Drake's launch silo'");
        assert_eq!(sim.drones.mission.states[22][1], 3, "ch 0x8f reveals 'Prevent Kiko from launching into space' [L record 16 obj 2]");
    }

    // The launch chain's head: the ch-0x81 BOND touch trigger at (2.4, 1.0, −176.4) [D — the
    // walkreal_probe_c dump; the doc's "no writer" scan missed the touch trigger]. Walk south
    // through door[4] (2.5, 1.0, −166.4, lock 0xb4 — its writer is the mask-0x04 trigger at
    // (3.5, 1.0, −165.7), the E panel) into the south room.
    w.walk_to(walker::Wp::new(2.5, 1.0, -169.0).r(1.3), "door[4] through");
    w.walk_to(walker::Wp::new(2.4, 1.0, -175.0).r(1.2), "the south room");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x81), "the touch trigger at (2.4,1.0,−176.4) fires ch 0x81 [D]");
    }

    // The chain PLAYS OUT (no skip): SP #262's pack cinema 0x06000631 (750 ticks) writes ch 185
    // at t748 → SP #272 "ShuttleLaunch" → ch 240/242 → spawners #257/#293 [D]. Bond stands while
    // the cinema runs (the real player watches it).
    w.skip_cutscenes = false;
    let chaser = {
        let mut found = None;
        for _ in 0..6000u32 {
            w.tick(Input::default(), false);
            let sim = w.scene.sim.as_ref().unwrap();
            if let Some(i) = sim.drones.list.iter().position(|d| d.present && !d.corpse && d.skin & 0xffff == 0x1e && d.dtype == 0xf && d.death_channel == 0x90) {
                found = Some(i);
                break;
            }
        }
        w.skip_cutscenes = true;
        found.expect("the launch chain spawns the chaser (spawner #257, group 6 [D])")
    };
    eprintln!("[walk] the chaser spawned at {:?}", w.scene.sim.as_ref().unwrap().drones.list[chaser].pos);

    // Walk back to the silo rim so the chaser is in view, then the defeat with Bond's own fire.
    // (She runs the ninja chain [M 0x14c938 case 0xf → 0xd]; the walk to her current spot, then
    // shoot() re-aims every tick — she may be moving.)
    let cpos = w.scene.sim.as_ref().unwrap().drones.list[chaser].pos;
    let _ = w.try_walk_to(walker::Wp::new(cpos[0], cpos[1] + 6.0, cpos[2] + 4.0).r(2.0), "the chaser's rim");
    assert!(w.shoot(chaser, "the chaser"), "the chaser dies to Bond's fire");

    // ch 0x90 → AND #271 → ch 100 → Mission_Update state 6 [M 0x1827d8:155-160, 0x182c60:85-122].
    let mut complete = false;
    for _ in 0..900u32 {
        w.tick(Input::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.mission.status() == 6 { complete = true; break; }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x90), "the chaser's defeat raises ch 0x90 [S kiko.rs]");
        assert!(sim.drones.channels.on(100), "AND #271 [ch 0x90] → ch 100 [D #271]");
        assert!(complete, "the final mission completes (MissionState 6 [M 0x182c60:85-116])");
        assert_eq!(sim.drones.mission.to_end, 0x0730_0017, "the driving hand-off [M 0x182c60:111-112]");
        assert!(sim.drones.mission.states[21].iter().all(|&s| s == 4), "the carried 07000015 record stays complete");
    }
}
