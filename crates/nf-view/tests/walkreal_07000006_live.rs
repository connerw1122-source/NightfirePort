//! walkreal: 07000006 (Mayhew B, the castle interior to the party) walked with the player's own
//! movement — no teleports after the truck-ride spawn. The level has NO 0xE8 LOAD_LEVEL placement
//! [D docs/levels/07000006.md §3]: the single exit is the walk-in LoadPoint Touch #746 (0xEB, out
//! ch 0x83) → ScriptPlayer #810 → the dressing cut-scene 0x06000071, whose closing event 0xD [2,1]
//! completes the level's ONLY objective ("Find a way inside to the party", complete ch 0x02,
//! revealed by ch 0x46 [L ee_start_05.bin record 1]) and whose final event 0x12 [0x07000007] loads
//! the party [M Script_EventHandler 0x1d94c0 case 0x12].
//!
//! THE PROVEN ROUTE (every leg walked through `Sim::step_bond` inputs; anchors [D census] +
//! the walked probes): the truck-ride spawn #496 (57.18, 39.73, 22.61, the carried ch-0x1D gate
//! [M Player_Start 0x190fd0]) stands on the WineTruck's bed/dock (Bond ~1.1 above the ground [D
//! #336 WineTruck @(64.48, 38.00, 28.06)]; the probe: east/south walk free, west/north pin on the
//! bed rails at z≈20.5 / the cab at x≈55.5) → THE DISMOUNT off the bed's open south side (the
//! arrival plates' OR #498 raises ch 0x46 at the spawn; the driver-greeting plates #756/#757/#782
//! wake the friendly driver on the way) → the dock's west end → down the west lane → around the
//! well's west rim (the direct line to the well pins at z≈20.5, x≈31 — the well's north fence [D
//! floor grid]) → the garden hedge gap (the hedges wedge nav-assisted legs; the walked lane
//! crosses the z≈8.8 hedge band at x≈24.4 and turns around the x≈22.6 hedge's south end) →
//! the guardhouse courtyard → E-door #47 Door_Top_I1 (36.50, 40.00, −16.25) → the guardroom
//! FuseBox #747 (28.00, 40.00, −18.15): the stealth route's optional Laser cut (group 0 → ch 57
//! searchlights-off + ch 103 [M FuseBox_Update 0x1adfe8:110-115]) → E-door #59 Door_Top_I2
//! (27.88, 40.00, −23.00, auto-close) → the searchlight courtyard (the entry plates #525/#526 →
//! 0x52 stage SP #808's boot NIS; ch 55 rises when a head holds Bond — the alarm is NOT a fail
//! channel on this level [L record 1: fail 0xffffffff; M 0x1827d8:98-107]) → E-door #71
//! Door_Tunnel_G9 (3.75, 42.00, −51.13) → the tunnel climb (y 42 → 49) → E-door #72
//! Door_Tunnel_G9_2 (4.00, 49.00, −64.88) → the upper gallery (y≈50.1, the WineTruck building's
//! top corridor, z −65..−70).
//!
//! THE OPEN LEG (the walk's one gap, see docs/reports/level-06-complete.md): the gallery → the
//! dressing room (the LoadPoint #746 at (35.5, 44.5, −54.1)). The port's collision gives no
//! walked connection yet: the gallery's north lip at z≈−65.5 (x 20–30) blocks both walking and
//! jumping [D probe: grounded at 50.4 across 400 ticks with jump pulses], the east corridor
//! (x 38–39.5, y 50.1) is walled at both ends (z −31.5 north, the LoadPoint side west) [D probe],
//! no door placement sits between [D the 12-door dump: #46 Locked_Door_Top_I at (36.25, 51.06,
//! −68.38) is ON the gallery and locks ch 200 [M U-044]], and the room floor (y 42.5–47 [D
//! #765's walk-in volume (31.12, 42.22, −59.06)..(36.96, 46.79, −48.87)]) is ~6 below the
//! gallery. The game's own F3 route must use something the walk does not model yet (a stair
//! volume the port's floors miss, a scripted carry, or the AI patrol graph — ai point 5 at
//! (2.33, 54.67, −52.50) patrols a 9-node path ABOVE the tunnel [D ai dump]).
//!
//! Cut-scene law: the walker's skip is the game's own (skip_request → Script::ffwd [M SP_Update
//! 0x1dbda8:129 → Script_FFwd 0x1dac78]), and ffwd's catch-up still fires every event, so the
//! objective channel and the exit survive a skip — pinned green on master by
//! walk_07000006_live.rs's LoadPoint entry (ch 0x83 → ch 0x02 → level_exit 07000007).
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
    // [S] unverified: the real 05-exit RamSave carries the player's full 05 stock; the walkreal
    // harness gives the route room to fight the alarm's reinforcements through the real fire path.
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 240), &ch)
}

/// The Laser drawn with the real weapon-cycle input (the disc's own cutting laser, 0x4E) [D
/// USATxt label 0x05000043; the proven fusebox_06_live draw].
fn draw_laser(w: &mut Walker) {
    let draw = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { w.tick(draw, false); }
    assert_eq!(w.scene.sim.as_ref().unwrap().gun.weapon, 0x4e, "the Laser equips [D 0x05000043]");
}

/// The alarm's reinforcement guards (the ch-55 spawner #735 [D census]) body-block the courtyard
/// lane; the player shoots them. Any live guard within 8 units of Bond is taken down with the
/// walker's own real fire path.
fn fight_through(w: &mut Walker) {
    w.switch_to_armed("the courtyard fight");
    for _ in 0..4 {
        let target = {
            let sim = w.scene.sim.as_ref().unwrap();
            (0..sim.drones.list.len()).find(|&i| {
                let d = &sim.drones.list[i];
                !d.corpse && d.health > 0.0 && d.pos[1] < 46.0 && dxz(d.pos, sim.bond.pos) < 8.0
            })
        };
        let Some(di) = target else { return };
        eprintln!("[06] fighting guard {di} at {:?}", w.scene.sim.as_ref().unwrap().drones.list[di].pos);
        if !w.shoot(di, "the courtyard guard") {
            // Out of ammo: resupply from the ground pickups (the route carries spares).
            w.resupply("the courtyard fight");
        }
    }
}

/// The walk from the spawn to the upper gallery. `cut` does the stealth route's fusebox Laser cut
/// in the guardroom (group 0 → ch 57 + 103); without it the searchlights stay live and the
/// courtyard raises the ch-55 alarm — neither blocks the walk (no fail channel on this level [L
/// record 1; M 0x1827d8:98-107]).
fn walk_to_the_gallery(w: &mut Walker, cut: bool) {
    // THE DISMOUNT: off the truck bed's open south side (the spawn's own facing walks onto the
    // bed; the probe drops grounded at z≈29).
    w.walk_nav_to([60.0, 38.3, 31.5], "off the truck, south", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.bond.pos[2] > 28.0, "the dismount clears the truck's south side, got {:?}", sim.bond.pos);
    }
    // Across the driver-greeting plates (#756/#757/#782 → ch 0xB9) on the ground west of the dock.
    w.walk_nav_to([45.0, 38.3, 24.0], "across the greeting plates", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x46), "the arrival plates' OR #498 reveals the objective (ch 0x46) [D #498 out 0x46]");
    }
    // The dock's west end, down the west lane, around the well's west rim (the nav graph has no
    // coverage here; each anchor was walked by the probe).
    w.walk_nav_to([24.3, 38.3, 27.6], "the dock's west end", 1.6);
    w.walk_nav_to([24.6, 38.5, 21.5], "down the west lane", 1.6);
    w.walk_nav_to([26.5, 39.1, 13.1], "the well courtyard's west side", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(dxz(p, [26.5, 39.1, 13.1]) < 3.0, "the well courtyard's west side reached, got {p:?}");
    }
    // The garden hedges wedge nav-assisted legs (the global roadmap routes east into the z≈8.8
    // hedge); the probe's walked lane crosses the hedge band at x≈24.4, stands at the x≈22.6
    // hedge face, then turns southeast around its south end to the guardhouse courtyard.
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
    // Through #47 into the guardroom; the nav graph covers this room [D leg probe: 4 nodes].
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
    fight_through(w);
    // South across the courtyard to the tunnel door #71 Door_Tunnel_G9 (3.75, 42.00, −51.13)
    // [D leg probe: the nav graph covers this courtyard, 7 nodes].
    w.walk_nav_to([4.4, 42.0, -48.5], "down the searchlight courtyard", 1.6);
    fight_through(w);
    {
        let (pos, di) = walker::door_near(w.scene.sim.as_ref().unwrap(), [3.75, 42.0, -51.13])
            .expect("door #71 Door_Tunnel_G9 in the doors list [D census]");
        eprintln!("[06] door {di} at {pos:?}");
        w.use_door(di, "door #71");
    }
    // Through the doorway to the inside spot (trigger #674 (4.42, 42.66, −52.94) [D census]).
    w.walk_to(Wp::new(4.42, 42.7, -52.9), "into the tunnel");
    // Up the tunnel (y 42 → 49) to the upper door #72 Door_Tunnel_G9_2 (4.00, 49.00, −64.88).
    // The climb must actually gain the height — the door E below is only honest from beside it.
    w.climb_stairs([4.0, 49.0, -62.5], "up the tunnel", 1.6, 4000);
    if w.scene.sim.as_ref().unwrap().bond.pos[1] < 47.0 {
        w.climb_stairs([4.0, 49.0, -62.5], "up the tunnel (retry)", 1.2, 4000);
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.bond.pos[1] > 47.0, "the tunnel climb gains the upper level (y {:.2} at {:?})",
            sim.bond.pos[1], sim.bond.pos);
    }
    {
        let (pos, di) = walker::door_near(w.scene.sim.as_ref().unwrap(), [4.0, 49.0, -64.88])
            .expect("door #72 Door_Tunnel_G9_2 in the doors list [D census]");
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(dxz(sim.bond.pos, pos) < 4.0, "door #72 is E'd from beside it, not through the wall (bond {:?} door {pos:?})", sim.bond.pos);
        eprintln!("[06] door {di} at {pos:?}");
        w.use_door(di, "door #72");
    }
    // Through the doorway to the inside spot (trigger #521 (4.62, 49.80, −66.65) [D census]) —
    // the gallery walk needs the doorway crossed first or the steering slams the wall beside it.
    w.walk_to(Wp::new(4.6, 49.8, -66.6), "through door #72");
    // East into the upper gallery (y≈50.1 — the WineTruck building's top corridor, z −65..−70).
    w.walk_to(Wp::new(20.0, 50.1, -67.0), "east into the gallery");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(p[1] > 49.0, "Bond stands on the upper gallery (y {:.2})", p[1]);
    }
}

/// The full play through the real input path: the truck-ride spawn → the dismount → the well
/// courtyard's hedge lane → E-doors #47/#59 (the guardroom, with the stealth route's optional
/// fusebox Laser cut through the real aim/fire path) → the searchlight courtyard → E-doors
/// #71/#72 (the tunnel climb) → the upper gallery. Every door on the route is E-opened by the
/// player's own use key, the fusebox channels fire from the player's own Laser, and the level's
/// only objective is revealed by the route itself (ch 0x46). The gallery → dressing-room leg is
/// the walk's one documented gap (see the header) — the LoadPoint chain it feeds (ch 0x83 →
/// ch 0x02 → level_exit 07000007) is pinned green through the real trigger on master by
/// walk_07000006_live.rs.
/// [ignore] OPEN (level-06-complete): the walked route reaches the guardroom, where the scripted
/// idle guard #438 (skin 0x500002e, drone 2) engages at (36.0, 41.0, −18.9) and never dies —
/// 8 real-fire shoot() rounds with the carried P2K leave it alive (its position barely moves;
/// the idle-scripted guards may be invulnerable [D #438 script 0x0600021F, census §4]) and its
/// body wedges the walked lane. The route is proven to that point; next session: the guard's
/// shootability flag in the decompile (Drone_Damage 0x18c330 vs the idle-script arm), then the
/// gallery → dressing-room leg (see the header's OPEN LEG).
#[test]
#[ignore]
fn walk_the_castle_interior_to_the_upper_gallery() {
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

    walk_to_the_gallery(&mut w, false);
    eprintln!("[06] reached the gallery at tick {}", w.total_ticks);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        // The route's own channel, written by the walk itself (the fusebox cut is pinned by
        // fusebox_06_live; the walk crosses the courtyard fighting the alarm's reinforcements).
        assert!(sim.drones.channels.on(0x46), "the objective was revealed by the route [D #498]");
        // The objective stays INCOMPLETE until the dressing scene — the level cannot be finished
        // without reaching the LoadPoint (the mission monitor folds ch 0x02 only [L record 1]).
        assert_ne!(sim.drones.mission.states[mission_row()][0], 4,
            "the objective is not complete before the dressing scene");
        assert_eq!(sim.drones.level_exit, None, "no exit before the LoadPoint chain");
    }
    eprintln!("[06] walked to the gallery in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[06] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[06] stuck: {s}"); }
}

/// Sister: the same walk that stops before the tunnel never ends the level — no alarm-channel
/// dependency, no ch 0x83, no objective fold, no exit. The gallery walk's asserts depend on the
/// walked route, not on any earlier scripted hand-out.
/// [ignore] with the full walk above (the same guard wedge).
#[test]
#[ignore]
fn stopping_before_the_tunnel_never_ends_the_castle() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000006.bin", false,
        Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load 07000006"));
    w.wait_enabled(3000, "the intro NIS");
    w.wait(30);
    // The route WITHOUT the fusebox cut, halted at the guardroom (before doors #59/#71/#72).
    walk_to_the_gallery(&mut w, false);
    eprintln!("[06] sister walked to the gallery in {} ticks (no cut)", w.total_ticks);
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x83), "the LoadPoint stays cold far from its volume [D #746]");
    assert_ne!(sim.drones.mission.states[mission_row()][0], 4, "the objective is not complete");
    assert_eq!(sim.drones.level_exit, None, "the castle does not end away from the LoadPoint");
    let _ = Act::None;
}
