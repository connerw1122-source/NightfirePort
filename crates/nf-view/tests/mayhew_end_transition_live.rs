//! mayhew-end-transition: the END of 07000005 (Mayhew) — the end trigger's exit hold, the
//! frozen 300-frame fade, and the hand-off to 07000006.
//!
//! The chain [D docs/levels/07000005.md §3; the truck script 0x06000A22]: the walked hand-over
//! (the 750c mayhew-exit-walk route), the crouched ride, the park (ch 0x70) with the bed touch
//! (ch 0x71) and the gate guard at his post (ch 0x72) → mux #316 (out 0x65) → SP #320
//! (`0x06000A23`, mode 0x40 = self-trigger on ch 0x65) waits 300 script ticks → event 0x11
//! writes ch 0x1D [D the script's stream 1; M `Script_EventHandler` 0x1d94c0 case 0x11] → #118
//! (0xE8, gate 0x1D) auto-fires [M `Trigger_Update` 0x1b1a78:162-166 — no proximity check] →
//! `Trigger_Activate` 0x1b2318's tail: the channel writes, `Music_Event(3,1)`,
//! **`GS_PauseGame(1)`** [M 0x17d178], `ResetMap_LevelToLoad(0x07000006, 1, 0)` [M 0x1d3800],
//! **`GameFlow_PushState(300.0f, 7, 0xff)`** [M 0x1b2318:89-92] — the push converts to state 8
//! with `Camera_SetFade(6, 300, 1)` [M 0x1c9f38:18-24] — the trigger is spent, `Player_RamSave`.
//! State 8 holds while the fade runs, then pops onto state 3 (`ResetMap_Load`) [M 0x1c98c8:173-189].
//! The world freezes under it: the game's clock stops while paused [M 0x1c98c8:44-47], the
//! player input dies [M `psiInput_MapInputs` 0x10cd80:36], `Game_Run`'s world sections are
//! behind the pause gate [M 0x1c9790:30-38] — but the fade advances every frame [M 0x1c9790 tail]
//! and the scene keeps DRAWING [M 0x1c98c8 `LAB_001c9e28`: `Game_Run` runs every frame; state 8
//! is not in the skip-draw set {1,3,4,5,9}].
//!
//! The real capture agrees [L tests/traces/05-truck.trace]: the last live 07000005 tick (line
//! 37173) has drones mid-animation and the `ch 37173 29 0 1` marker (ch 0x1D going on); the
//! next captured line is already 07000006 — the capture hole between them IS the paused hold
//! (the trace captures unpaused sim ticks), then the load's four pre-place frames (lines
//! 37174-37177, bond 0.0) and the first in-game tick at the truck-ride start #496.
//!
//! The script event-0x12 loadmap does NOT hold [M 0x1d94c0:194-199 — `Player_RamSave` +
//! `ResetMap_LevelToLoad` only]: sister test 2 pins 07000006's dressing exit (event 0x12 →
//! 07000007) posting with no hold and no freeze.

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::{Input, RamSave};
use walker::Walker;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The whole end sequence through the real path (no teleports): the 750c walk to the gatehouse
/// edge, the hand-over, the crouched ride, the park — then the PIN: at the fire the exit hold
/// stands (GS_PauseGame + the 300-frame fade push [M 0x1b2318:90-92]), the world is FROZEN
/// under it while the fade counts down, the hold clears at 300 with the exit request intact,
/// and the carried load spawns Bond at the gate-0x1D start #496 in 07000006.
#[test]
fn the_end_hold_freezes_the_world_for_300_frames_then_the_next_level_loads() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(p[0] > 100.0 && p[0] < 110.0 && p[2] > 100.0 && p[2] < 115.0,
            "the cold start spawns at the gatehouse road (#26 [D]), got {p:?}");
        assert!(sim.drones.exit_hold.is_none(), "no hold at the level start");
    }
    w.wait(200);
    // The real-input walk to the gatehouse walkway edge, inside the ch-0x5C plate's XZ
    // (centre (98.49, 27.05, 106.36) [D the entity matrix]) — the 750c mayhew-exit-walk route.
    w.walk_nav_to([98.6, 26.0, 106.5], "to the gatehouse walkway edge", 1.2);
    let mut fired = false;
    for _ in 0..3600u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x5c) { fired = true; break; }
    }
    assert!(fired, "ch 0x5C fires while Bond stands at the gatehouse edge [D the truck script's stream 8 window]");
    // The hand-over puts Bond on the truck bed (ch 0x71); the game's ride stance is crouched
    // [D fix-truck's trace]; the crouched ride crosses the courtyard plates (ch 0x22 → the gate
    // guards spawn [D spawner #268]) and parks on ch 0x70 — mux #316 completes, SP #320 waits
    // 300 ticks, event 0x11 writes ch 0x1D, #118 fires.
    let mut input = Input::default();
    let mut crouched = false;
    let mut rode = false;
    let mut mux_seen = None;
    let mut fire = None;
    for _ in 0..6800u32 {
        {
            let sim = w.scene.sim.as_ref().unwrap();
            let on_bed = sim.drones.channels.on(0x71);
            let west = sim.bond.pos[0] < 90.0;
            if on_bed && west { rode = true; }
            if !crouched && on_bed && west && !sim.bond.crouched {
                crouched = true;
                input.crouch = true;
            } else {
                input.crouch = false;
            }
            // The park is LIVE here: mux 0x65 is on while SP #320 still waits — the world runs
            // (the real capture's guards idle-animate through these 300 ticks [L 05-truck.trace
            // frames 36874..37173]) and the hold must not exist before #118 fires.
            if mux_seen.is_none() && sim.drones.channels.on(0x65) {
                mux_seen = Some(w.total_ticks);
                assert!(rode, "the hand-over put Bond on the truck bed (ch 0x71) and the ride carried him west of the gatehouse");
                assert!(sim.drones.exit_hold.is_none(),
                    "no hold while SP #320 waits — the world is live until ch 0x1D fires #118 [M 0x1b1a78:162-166]");
            }
        }
        w.tick(input, false);
        if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { fire = Some(w.total_ticks); break; }
    }
    let fire = fire.expect("the crouched ride parks on ch 0x70 and the chain ends the level [D mux #316 → SP #320 → ch 0x1D → #118]");
    eprintln!("[05-end] ride done, exit fired at t {fire}");
    assert!(rode, "Bond rode the truck bed west [D ch 0x71 parented to the truck]");
    assert!(mux_seen.is_some(), "the park's mux 0x65 came on before the exit");
    // ─── THE PIN: at the fire the hold stands and the exit request is posted.
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let h = sim.drones.exit_hold
            .expect("Trigger_Activate ran GS_PauseGame(1) + GameFlow_PushState(300.0f, 7, 0xff) [M 0x1b2318:90-92]");
        assert_eq!(h.next, 0x0700_0006, "the hold carries #118's next level [D placement #118 p0 = 0x07000006]");
        assert_eq!(h.total, 300, "the push carried 300.0f [M 0x1b2318:92 — 0x43960000]");
        assert_eq!(h.fade_frames, 300, "the fade has all 300 frames left at the fire");
        assert_eq!(sim.drones.level_exit, Some(0x0700_0006),
            "the exit request is posted; the host drains it only after the hold clears [M 0x1c98c8 case 8]");
    }
    // The world freezes under the fade: Bond (on the parked truck), the sim's player anchor and
    // the game clock all stand still while the fade counts down
    // [M 0x1c98c8:44-47, 0x1c9790:30-38, 0x10cd80:36].
    let (bond0, anchor0, frame0) = {
        let sim = w.scene.sim.as_ref().unwrap();
        (sim.bond.pos, sim.drones.player_pos, sim.drones.frame)
    };
    for _ in 0..100 { w.tick(Default::default(), false); }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.bond.pos, bond0, "the world is frozen under the fade — Bond stands on the parked truck");
        assert_eq!(sim.drones.player_pos, anchor0, "the sim's player anchor is frozen under the fade");
        assert_eq!(sim.drones.frame, frame0, "the game clock stops while paused [M 0x1c98c8:44-47]");
        let h = sim.drones.exit_hold.expect("the hold still runs at fire+100");
        assert_eq!(h.fade_frames, 200, "the fade counted down 100 of its 300 frames (linear [M 0x1bbdd8])");
    }
    // The rest of the hold: 200 more ticks and it clears — state 8 pops onto state 3
    // [M 0x1c98c8:173-189] — with the exit request intact for the host's drain.
    for _ in 0..300 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.exit_hold.is_none() { break; }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.exit_hold.is_none(), "the hold cleared at 300 frames [M 0x1c98c8:173-189]");
        assert_eq!(sim.drones.level_exit, Some(0x0700_0006), "the loader waited under the fade — the exit request is intact");
    }
    // The hand-off: the host's drain takes this request, captures the RamSave (the game saves at
    // the fire [M 0x1b2318:96]; the world is frozen between, so the capture is identical) and
    // loads the next level warm [M ResetMap_LevelToLoad 0x1d3800 → ResetMap_Load 0x1d6360].
    let save = {
        let sim = w.scene.sim.as_ref().unwrap();
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
    };
    assert!(save.channels.get(0x1d).copied().unwrap_or(0) != 0,
        "the success channel rides the save [M 0x1b2318:64-88]");
    drop(w);
    let mut next = nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&save)).expect("load 07000006 warm");
    {
        let sim = next.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 57.18).abs() < 2.0 && (p[2] - 22.61).abs() < 2.0,
            "the carried gate ch 0x1D selects the truck-ride start #496 (57.18, 39.73, 22.61) [D; M Player_Start 0x190fd0], got {p:?}");
        assert!(sim.drones.exit_hold.is_none(), "the fresh level carries no hold");
    }
}

/// Sister [M 0x1d94c0:194-199]: the script event-0x12 loadmap does NOT hold — 07000006's
/// dressing exit (LoadPoint #746 → ch 0x83 → SP #810 `0x06000071` → event 0x12 → 07000007
/// [D the placement; the walk_07000006 route]) posts `level_exit` with no hold, and the world
/// keeps ticking after it (no GS_PauseGame on the 0x12 path). Fails if anyone wires the hold
/// into the script-load path.
#[test]
fn the_script_loadmap_exit_does_not_hold() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The truck route's arrival: the carried gate ch 0x1D selects start #496 [D; M Player_Start].
    let mut ch = nf_game::triggers::Channels::default();
    for &c in &[0x1d, 0x45, 0x70u32] { ch.set(c, 1, 0); }
    let save = RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch);
    let mut scene = nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&save)).expect("load 07000006");
    {
        let sim = scene.sim.as_mut().unwrap();
        // Wait out the level-start NIS freeze.
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        assert!(!sim.disabled, "Bond is free after the intro");
        // The arrival plates (the driver greeting) — walk a little at the spawn (the walked
        // arrival legs live in walk_07000006_live; this pin is the 0x12 arm itself).
        let fwd = Input { forward: 1.0, ..Default::default() };
        for _ in 0..60 { sim.step_bond(fwd, false, false, false); }
    }
    // The LoadPoint trigger #746 (out 0x83): stand in its volume — the player walks into the
    // LoadPoint and the cut-scene takes over (the walk_07000006 pattern).
    let lp = scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.out == 0x83)
        .expect("the LoadPoint trigger (out 0x83) exists").clone();
    {
        let sim = scene.sim.as_mut().unwrap();
        let c = lp.centre();
        sim.bond.pos = [c[0], c[1] + 1.0, c[2]];
    }
    for _ in 0..2000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.level_exit.is_some() { break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0007),
        "the dressing scene's closing event 0x12 loads 07000007 [M 0x1d94c0 case 0x12]");
    assert!(sim.drones.exit_hold.is_none(),
        "the script loadmap does NOT push state 7 — no hold [M 0x1d94c0:194-199: Player_RamSave + ResetMap_LevelToLoad only]");
    let frame0 = sim.drones.frame;
    drop(sim);
    for _ in 0..5 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.frame > frame0,
        "the world keeps ticking after a 0x12 exit (no pause on the script-load path) [M 0x1d94c0:194-199]");
}
