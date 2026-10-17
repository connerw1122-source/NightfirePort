//! walkreal: 07000006 (Mayhew B, the castle interior to the party) walked start to finish with the
//! player's own movement — no teleports after the truck-ride spawn. The level has NO 0xE8
//! LOAD_LEVEL placement [D docs/levels/07000006.md §3]: the single exit is the walk-in LoadPoint
//! Touch #746 (0xEB, out ch 0x83) → ScriptPlayer #810 → the dressing cut-scene 0x06000071, whose
//! closing event 0xD [2,1] completes the level's ONLY objective ("Find a way inside to the party",
//! complete ch 0x02, revealed by ch 0x46 [L ee_start_05.bin record 1]) and whose final event
//! 0x12 [0x07000007] loads the party [M Script_EventHandler 0x1d94c0 case 0x12].
//!
//! The route (all anchors [D census]): spawn #496 (57.18, 39.73, 22.61, the ch-0x1D truck gate) →
//! the arrival plates (#743/#744/#745 → 0x51, #756/#757/#782 → 0xB9; OR #498 → the 0x46 reveal) →
//! the well courtyard (#65) → E-door #47 Door_Top_I1 → the guardroom FuseBox #747 (the stealth
//! route's optional Laser cut: group 0 → ch 57 searchlights-off + 103 [M FuseBox_Update
//! 0x1adfe8:110-115]) → E-door #59 Door_Top_I2 → the searchlight courtyard (entry plates
//! #525/#526 → 0x52; SP #808's boot NIS plays, skipped with the game's own skip action [M
//! SP_Update 0x1dbda8:129 → Script_FFwd 0x1dac78, which still runs every remaining event]) →
//! E-door #71 Door_Tunnel_G9 → the upper tunnel (y 42 → 49) → E-door #72 Door_Tunnel_G9_2 →
//! the dressing room (walk-in #765 → 177 → the inverter #749 clears the searchlight alarm) →
//! LoadPoint #746 → the dressing scene → 07000007.
//!
//! Cut-scene law: the walker's skip is the game's own (skip_request → Script::ffwd), and ffwd's
//! catch-up still fires every event, so the objective channel and the exit survive a skip [M
//! nis.rs Script::update's catch-up; pinned here by the ch-0x02 + exit asserts].
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::{Input, RamSave};
use walker::{dxz, Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn mission_row() -> usize {
    nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0006).expect("the 06 record")
}

/// The save the truck route's Trigger_Activate would make at 07000005's exit [M 0x1b2318:108-116]:
/// the carried gate ch 0x1D selects the truck-ride start #496, ch 0x45 (the 05 reveal) + 0x70
/// (the park) ride along.
fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

/// The Laser drawn with the real weapon-cycle input (the disc's own cutting laser, 0x4E) [D
/// USATxt label 0x05000043; the proven fusebox_06_live draw].
fn draw_laser(w: &mut Walker) {
    let draw = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { w.tick(draw, false); }
    assert_eq!(w.scene.sim.as_ref().unwrap().gun.weapon, 0x4e, "the Laser equips [D 0x05000043]");
}

/// The walk from the spawn to the dressing room's LoadPoint. `cut` does the stealth route's
/// fusebox Laser cut in the guardroom (group 0 → ch 57 + 103); without it the searchlights stay
/// live and the courtyard may raise the ch-55 alarm — neither blocks the walk (the alarm is not a
/// fail channel on this level [L record 1: fail 0xffffffff; M 0x1827d8:98-107]).
fn walk_to_the_loadpoint(w: &mut Walker, cut: bool) -> [f32; 3] {
    // THE DISMOUNT: the truck-ride spawn #496 stands on the WineTruck's bed/dock (Bond ~1.1 above
    // the ground; the probe: east/south walk free, west/north pin against the bed rails at
    // z≈20.5 / the cab at x≈55.5 [D #336 WineTruck @(64.48, 38.00, 28.06)]). The player walks off
    // the bed's open south side — the burst probe drops grounded at z≈29.
    w.walk_nav_to([60.0, 38.3, 31.5], "off the truck, south", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.bond.pos[2] > 28.0, "the dismount clears the truck's south side, got {:?}", sim.bond.pos);
    }
    // The arrival plates' OR #498 reveal (ch 0x46) fires at the spawn; the driver-greeting plates
    // #756/#757/#782 (49.9–57.1, 39.0, 22.5–29.8) wake the friendly driver on the way.
    w.walk_nav_to([45.0, 38.3, 24.0], "across the greeting plates", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x46), "the arrival plates' OR #498 reveals the objective (ch 0x46) [D #498 out 0x46]");
    }
    // SW to the well courtyard — the walked lane (the nav graph has no coverage here; every
    // anchor below was walked by the probe): the dock's west end, down the west lane, around the
    // well's west rim (the direct line to the well pins at z≈20.5, x≈31 — the well's north
    // fence [D floor grid]), then south around the garden wall at x≈22.5 (z 4..8) to the
    // guardhouse courtyard. The well bucket #65 (31.82, 39.79, 17.26) is passed on the east.
    w.walk_nav_to([24.3, 38.3, 27.6], "the dock's west end", 1.6);
    w.walk_nav_to([24.6, 38.5, 21.5], "down the west lane", 1.6);
    w.walk_nav_to([26.5, 39.1, 13.1], "the well courtyard's west side", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(dxz(p, [26.5, 39.1, 13.1]) < 3.0, "the well courtyard's west side reached, got {p:?}");
    }
    // The garden hedges wedge nav-assisted legs (the global roadmap routes east into the
    // z≈8.8 hedge); the probe's walked lane crosses the hedge band at x≈24.4, stands at the
    // x≈22.6 hedge face, then turns southeast around its south end to the guardhouse courtyard.
    w.walk_to(Wp::new(23.2, 39.1, 7.0), "the hedge gap");
    w.walk_to(Wp::new(27.2, 39.1, -2.4), "around the hedge end");
    w.walk_to(Wp::new(33.7, 39.1, -8.9), "the guardhouse court");
    w.walk_to(Wp::new(36.0, 39.1, -13.4), "before door #47");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(dxz(p, [36.0, 39.1, -13.4]) < 3.0, "door #47's approach reached, got {p:?}");
    }
    {
        let (pos, di) = walker::door_near(w.scene.sim.as_ref().unwrap(), [36.5, 40.0, -16.25])
            .expect("door #47 Door_Top_I1 in the doors list [D census]");
        eprintln!("[06] door {di} at {pos:?}");
        w.use_door(di, "door #47");
    }
    // Through #47 into the guardroom; stand at the fusebox nook (#747 at (28.00, 40.00, −18.15)).
    w.walk_nav_to([29.9, 40.5, -17.5], "the guardroom fusebox nook", 1.4);
    if cut {
        draw_laser(w);
        let wire = {
            let sim = w.scene.sim.as_ref().unwrap();
            let fb = sim.drones.fuseboxes.first().expect("the 06 fusebox #747 [D census]");
            fb.alive_wires().filter(|(g, _, _)| *g == 0)
                .filter_map(|(_, _, o)| nf_game::drone::Drones::script_obj_pos(&sim.drones.scripts, &sim.data(), o))
                .next().expect("the group-0 (searchlight) bundle is registered at load [M FuseBox_Register 0x1b0970]")
        };
        eprintln!("[06] burning the group-0 wire at {wire:?}");
        assert!(w.shoot_point(wire, "the fusebox laser cut",
            |sim| sim.drones.channels.on(57) || sim.drones.channels.on(103)),
            "the Laser cuts the searchlight bundle [M 0x1adfe8:110-115]");
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(57), "ch 57 — the searchlights-off gate — is on [D #747 p1]");
        assert!(sim.drones.channels.on(103), "the bundle's second word ch 103 fires [D #747 p3]");
    }
    // West out of the guardroom through the auto-closing door #59 Door_Top_I2 (27.88, 40.00, −23.00).
    w.walk_nav_to([27.9, 40.0, -21.0], "before door #59", 1.4);
    {
        let (pos, di) = walker::door_near(w.scene.sim.as_ref().unwrap(), [27.88, 40.0, -23.0])
            .expect("door #59 Door_Top_I2 in the doors list [D census]");
        eprintln!("[06] door {di} at {pos:?}");
        w.use_door(di, "door #59");
    }
    // West up the steps to the searchlight courtyard mouth (the entry plates #525/#526 → 0x52;
    // SP #808's boot NIS fires and the walker skips it with the game's own skip action).
    w.walk_nav_to([8.4, 42.0, -12.0], "the searchlight courtyard mouth", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[06] at the mouth: ch52 {} ch55 {}", sim.drones.channels.on(52) as u8, sim.drones.channels.on(55) as u8);
    }
    // South across the courtyard to the tunnel door #71 Door_Tunnel_G9 (3.75, 42.00, −51.13).
    w.walk_nav_to([4.4, 42.0, -48.5], "down the searchlight courtyard", 1.6);
    {
        let (pos, di) = walker::door_near(w.scene.sim.as_ref().unwrap(), [3.75, 42.0, -51.13])
            .expect("door #71 Door_Tunnel_G9 in the doors list [D census]");
        eprintln!("[06] door {di} at {pos:?}");
        w.use_door(di, "door #71");
    }
    // Up the tunnel (y 42 → 49) to the upper door #72 Door_Tunnel_G9_2 (4.00, 49.00, −64.88).
    w.climb_stairs([4.0, 49.0, -62.5], "up the tunnel", 1.6, 4000);
    {
        let (pos, di) = walker::door_near(w.scene.sim.as_ref().unwrap(), [4.0, 49.0, -64.88])
            .expect("door #72 Door_Tunnel_G9_2 in the doors list [D census]");
        eprintln!("[06] door {di} at {pos:?}");
        w.use_door(di, "door #72");
    }
    // Into the dressing room; return the LoadPoint's centre for the caller's final approach.
    let lp = {
        let sim = w.scene.sim.as_ref().unwrap();
        sim.drones.triggers.iter().find(|t| t.out == 0x83)
            .expect("the LoadPoint #746 (out 0x83) exists [D census]").centre()
    };
    w.walk_nav_to([24.0, 46.0, -58.0], "into the dressing room", 1.6);
    lp
}

/// The full play: the truck-ride spawn → the guardroom stealth cut → the searchlight courtyard →
/// the tunnel → the dressing room LoadPoint → the dressing cut-scene (skipped with the game's own
/// skip action, which still fires every event) → the objective completes and the level ends into
/// 07000007, exactly the way the game's own script ends it.
#[test]
fn walk_the_castle_interior_to_the_party() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000006.bin", false,
        Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load 07000006"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 57.18).abs() < 2.0 && (p[2] - 22.61).abs() < 2.0,
            "the carried ch-0x1D gate selects the truck-ride spawn #496 [M Player_Start 0x190fd0], got {p:?}");
    }
    // The level-start NIS: waited out through the game's own skip action.
    w.wait_enabled(3000, "the intro NIS");
    w.wait(30);

    let lp = walk_to_the_loadpoint(&mut w, true);
    eprintln!("[06] at the dressing room tick {}; LoadPoint at {lp:?}", w.total_ticks);

    // Walk INTO the LoadPoint volume: the walk-in raises ch 0x83, SP #810 starts the dressing
    // scene, event 4 disables Bond; the walker skips the scene with the game's own skip action
    // and the catch-up fires the closing events (0xD [2,1] objective + 0x12 exit).
    w.walk_to(Wp { at: lp, act: Act::None, arrive: 1.2 }, "into the LoadPoint #746");
    for t in 0..2600u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { break; }
        if t == 2500 { eprintln!("[06] the dressing scene never ended (ch83 {} ch2 {})",
            w.scene.sim.as_ref().unwrap().drones.channels.on(0x83) as u8,
            w.scene.sim.as_ref().unwrap().drones.channels.on(2) as u8); }
    }
    // The monitor folds the objective on the tick AFTER the script's channel write [M
    // Mission_MonitorObjectives 0x1827d8] — a few ticks, NOT 240 (the ch-100 banner hold).
    for _ in 0..4 { w.tick(Default::default(), false); }

    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x83), "the LoadPoint walk-in fires the dressing watch channel [D #746 out 0x83]");
    assert_eq!(sim.drones.mission.states[mission_row()][0], 4,
        "'Find a way inside to the party' completes on the scene's 0xD [2,1] → ch 0x02 [L record 1]");
    assert_eq!(sim.drones.level_exit, Some(0x0700_0007),
        "the dressing script's 0x12 [0x07000007] ends the level into the party [M 0x1d94c0 case 0x12]");
    eprintln!("[06] walked the interior in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[06] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[06] stuck: {s}"); }
}

/// Sister: the same walk that stops short of the LoadPoint never ends the level — the objective
/// stays incomplete and no exit appears. The full play's asserts above depend on actually walking
/// into the dressing room's volume, not on anything earlier on the route.
#[test]
fn stopping_before_the_loadpoint_never_ends_the_castle() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000006.bin", false,
        Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load 07000006"));
    w.wait_enabled(3000, "the intro NIS");
    w.wait(30);
    // The route up to the tunnel door — deliberately WITHOUT the fusebox cut (the searchlights
    // may raise the ch-55 alarm; the walk still gets through: no fail channel on this level).
    let _ = walk_to_the_loadpoint(&mut w, false);
    // Halt in the upper room, well short of the dressing room's LoadPoint volume.
    w.walk_nav_to([15.0, 47.5, -60.0], "the upper room, short of the LoadPoint", 1.6);
    for _ in 0..600 { w.tick(Default::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x83), "the LoadPoint stays cold short of its volume [D #746]");
    assert_ne!(sim.drones.mission.states[mission_row()][0], 4, "the objective is not complete");
    assert_eq!(sim.drones.level_exit, None, "the castle does not end away from the LoadPoint");
    eprintln!("[06] sister done in {} ticks", w.total_ticks);
}
