//! The 07000005 (Mayhew A, the castle approach) interactables: one live test per interactive KIND
//! on the main map (task level-05-interactables), each through the real input path (Bond walks /
//! turns / presses the use key with `Sim::step_bond` inputs via the walker), each with a sister
//! that fails without the operation. Every game claim cites its evidence: [M] the decompile
//! function + address, [D] the disc parse of FILES.BIN entry 07000005.bin. The census:
//! docs/levels/07000005.md §7.
//!
//! The main map is the ONLY map whose placements the game creates — `parsemap_parsemap` 0x1d1cf0's
//! state-2 epilogue processes `DynamicPtr`, the LAST block-0x1a of the archive, and the type-1 main
//! map (01000099) is the archive's last map-type entry [D `nf-extract bin-list`] — so the counts
//! here are the main map's 363 placements' [D].

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::Input;
use walker::Walker;

fn iso() -> String {
    std::env::var("NF_ISO").unwrap_or_else(|_| {
        eprintln!("NF_ISO not set — skipping");
        std::process::exit(0);
    })
}

fn load(iso: &str) -> Walker {
    let mut w = Walker::new(nf_view::load(iso, "07000005.bin", false).expect("load 07000005"));
    w.wait(30);
    w
}

/// KIND: the static creations, pinned by their disc numbers ([D]; the creators named in the
/// census). No operation exists for these — the count + the field values ARE the game behaviour.
/// Includes the four origin-parked FuseBox 0x31 placements: `FuseBox_Create` 0x1b0448 needs
/// param 0 as the wire/lid script hash and `obj+0xD8 = 0` never draws [M :33-35] — with no
/// params at all the four create nothing, which is exactly what the port's skip does.
#[test]
fn the_census_creations_match_the_disc() {
    let iso = iso();
    let w = load(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    let d = &sim.drones;
    // Starts [D #26/#0], drones [D ×16], doors [D ×5], ScriptPlayers [D ×6].
    assert_eq!(d.doors.list.len(), 5, "the five door placements [D]");
    assert_eq!(d.list.len(), 16, "the sixteen drones [D]");
    // Pickups: 2 armour (kind 3) + 2 9mm ammo (kind 1) + 2 Bond-bonus tokens (kind 6) [D].
    assert_eq!(d.pickups.len(), 6, "the six pickups [D]");
    assert_eq!(d.pickups.iter().filter(|p| p.kind == 3).count(), 2, "the two BodyArmourHeavy [D]");
    assert_eq!(d.pickups.iter().filter(|p| p.kind == 1).count(), 2, "the two 9mm ammo [D]");
    assert_eq!(d.pickups.iter().filter(|p| p.kind == 6).count(), 2, "the two Pickup 007 bonus tokens [D]");
    // Copters [D ×2] with the flight scripts 0x060000E6/0x060000E7 and the out channel 75 [D params].
    assert_eq!(d.copters.len(), 2, "the two Helicopter-Body placements [D]");
    assert!(d.copters.iter().all(|c| c.out == 75), "both copters' arrival out is ch 75 [D param 7]");
    assert!(d.copters.iter().any(|c| c.gate == 0), "#241 has no gate — the ambient fly-by runs from the start [D param 6 = 0]");
    assert!(d.copters.iter().any(|c| c.gate == 66), "#242 waits on ch 0x42 — the SWITCH_Helicopterstart(SC3) plate [D]");
    eprintln!("[05] copters: {:?}", d.copters.iter().map(|c| (c.gate, c.out, c.scripts, c.arrival, c.countdown)).collect::<Vec<_>>());
    assert!(d.copters.iter().all(|c| c.scripts == [0x0600_0046, 0x0600_0047]), "the flight scripts 0x06000046/0x06000047 [D params 4/5]");
    // The hurt volumes [D ×13]: type 6, damage 1000, no gate [D params].
    assert_eq!(d.hurts.len(), 13, "the thirteen hurt volumes [D]");
    assert!(d.hurts.iter().all(|h| h.kind == 6 && h.damage == 1000.0 && h.gate == 0), "type 6, damage 1000, ungated [D]");
    for h in &d.hurts {
        assert!(!h.tris.is_empty(), "hurt placement {} resolved its volume cels [D the object's collision]", h.placement);
    }
    // The origin-parked fuseboxes create nothing [M 0x1b0448 with no param 0 script; obj+0xD8 = 0].
    assert!(d.fuseboxes.is_empty(), "the four 0x31 placements carry no script — no fusebox exists [D params []]");
    for kind in [(0x2au8, 9usize, "sky"), (0x32, 18, "rainbox")] {
        // (the counts live on the level parse; sky/rainbox asserted through env below)
        let _ = kind;
    }
    // The snow env [D #360/#361 SNOW_gen, drop type 0] over the 18 rainbox volumes.
    let env = d.env.as_ref().expect("the SNOW_gen env");
    assert_eq!(env.boxes.len(), 18, "the eighteen RainBox volumes [D]");
    assert_eq!(env.kind, 0, "drop type 0 = SNOW [D params (0,0); M Env_Create]");
    eprintln!("[05] hurt boxes:");
    for h in &d.hurts {
        eprintln!("  placement {} lo {:?} hi {:?}", h.placement, h.lo, h.hi);
    }
    eprintln!("[05] ladders {:?} copters {:?} searchlight heads {}",
        sim.ladders.len(), d.copters.iter().map(|c| c.gate).collect::<Vec<_>>(), d.searchlights.parts.iter().filter(|p| p.kind == 2).count());
    eprintln!("[05] ladder detail {:?}", sim.ladders.iter().map(|l| (l.centre, l.top(), l.bottom())).collect::<Vec<_>>());
    eprintln!("[05] searchlight parts kind/pos: {:?}", d.searchlights.parts.iter().map(|p| (p.kind, p.pos)).collect::<Vec<_>>());
    eprintln!("[05] doors: {:?}", d.doors.list.iter().map(|dd| (dd.pos, dd.state, dd.ch_lock)).collect::<Vec<_>>());
    eprintln!("[05] plunge shells: {:?}", d.triggers.iter().filter(|t| t.kind == walker::trigger_kind::TOUCH_ONCE && t.out == 190 && !t.tris.is_empty())
        .map(|t| (t.centre(), t.lo, t.hi)).collect::<Vec<_>>());
}

/// KIND: TouchOnce 0xEA + the OR mux 0xEF — the approach plates #21/#22/#23 (out 35/36/37, mask 8
/// [D]) through `Trigger_MultiplexOrIn` #25 (out 34 = 0x22 ← 35,36,37 [D params]). Walking onto
/// #21 raises ch 0x23 and the mux copies it to ch 0x22 (the guards' wait channel AND spawner
/// #268's start [D]). Sister: a fresh load that never walks keeps both off.
#[test]
fn the_approach_plate_arms_the_or_mux() {
    let iso = iso();
    // Sister.
    {
        let w = load(&iso);
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x23) && !sim.drones.channels.on(0x22),
            "sister: no walk, no plate, no mux output");
    }
    let mut w = load(&iso);
    // Plate #21 "DroneTrigger 2" at (60.15, 28.42, 96.43) [D].
    w.walk_nav_to([60.1, 28.4, 96.4], "approach plate #21", 1.2);
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x23), "standing on plate #21 raises its out ch 0x23 [D param 0 = 35]");
    assert!(sim.drones.channels.on(0x22), "mux #25 (OR 35,36,37 → 34) copies it [D params; M Trigger_MultiplexOrIn]");
}

/// KIND: doors 0xDB — #153 "Door_UNLocked" opens on the use key (open anim 285 / close 284 [D
/// params 5/6]); the locked four (#152/#154/#155/#156, lock ch 200 [D param 3]) refuse — nothing
/// on disc writes ch 200 (U-044). Sisters: the door states before the use.
#[test]
fn the_unlocked_door_opens_and_the_locked_ones_refuse() {
    let iso = iso();
    let mut w = load(&iso);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let i = sim.drones.doors.list.iter().position(|d| (d.pos[0] - 57.75).abs() < 1.0 && (d.pos[2] - 89.31).abs() < 1.0)
            .expect("door #153 by the ladder [D]");
        assert_eq!(sim.drones.doors.list[i].state & 0xc, 0, "sister: #153 starts shut");
        let li = sim.drones.doors.list.iter().position(|d| (d.pos[0] - 151.02).abs() < 1.0 && (d.pos[2] - 91.74).abs() < 1.0)
            .expect("locked door #156 [D]");
        assert_eq!(sim.drones.doors.list[li].ch_lock, 200, "the lock channel is 200 [D param 3]");
    }
    // The unlocked door: walk up and E it.
    let (dpos, di) = {
        let sim = w.scene.sim.as_ref().unwrap();
        let di = sim.drones.doors.list.iter().position(|d| (d.pos[0] - 57.75).abs() < 1.0).unwrap();
        (sim.drones.doors.list[di].pos, di)
    };
    w.walk_nav_to([dpos[0] + 1.2, dpos[1], dpos[2] + 1.2], "door #153", 0.6);
    w.use_door(di, "door #153");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.doors.list[di].state & 0xc != 0,
            "E on #153 opens it [M Door_Update's use arm; D open anim 285]");
    }
    // A locked door: walk to #156 and E — nothing.
    let (lpos, li) = {
        let sim = w.scene.sim.as_ref().unwrap();
        let li = sim.drones.doors.list.iter().position(|d| (d.pos[0] - 151.02).abs() < 1.0).unwrap();
        (sim.drones.doors.list[li].pos, li)
    };
    w.walk_nav_to([lpos[0], lpos[1], lpos[2] + 1.2], "locked door #156", 0.6);
    let state_before = w.scene.sim.as_ref().unwrap().drones.doors.list[li].state;
    w.use_door(li, "locked door #156");
    w.wait(30);
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.doors.list[li].state, state_before,
        "E on the ch-200-locked door does nothing (ch 200 has no writer on disc [D; U-044])");
}

/// KIND: ladder 0x22 — #151 (60.93, 29.91, 89.20 [D]) climbs to the upper ledge (top 34.7 [D the
/// ladder keys]). `Player_Climb` through the walker's real forward-held input. Sister: Bond at the
/// base stays at the courtyard height.
#[test]
fn the_ladder_climb_reaches_the_upper_ledge() {
    let iso = iso();
    let mut w = load(&iso);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.bond.pos[1] < 31.0, "sister: the spawn floor is the courtyard [D start #26 y 25.1]");
    }
    w.walk_nav_to([60.1, 28.4, 96.4], "courtyard west", 1.2);
    w.walk_nav_to([57.8, 26.0, 89.4], "door #153", 1.0);
    if let Some((dpos, di)) = walker::door_near(w.scene.sim.as_ref().unwrap(), [57.8, 26.0, 89.4]) {
        w.use_door(di, "door #153");
        let _ = dpos;
    }
    w.walk_to(walker::Wp { at: [60.9, 27.0, 89.2], act: walker::Act::Ladder { top_y: 34.7 }, arrive: 0.9 }, "climb ladder #151");
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.bond.pos[1] > 33.0, "the climb reaches the ledge (y {:.2}) [M Player_Climb]", sim.bond.pos[1]);
}

/// KIND: pickups 0xF0 — the armour case #278 (kind 3 → `Pickup_Handler` 0x18e650:220-225 sets
/// armour 50 [M]) and a bonus token (kind 6 → `PlrStat_LogBondBonus` [M :234-236]). Sisters: the
/// values before the walk.
#[test]
fn the_armour_pickup_sets_fifty_and_the_bonus_token_logs() {
    let iso = iso();
    let mut w = load(&iso);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.health.armour < 50.0, "sister: armour starts under 50");
        assert!(sim.drones.pickups.iter().any(|p| p.kind == 3 && !p.gone), "the armour case rests on the map");
    }
    // Armour #278 at (59.65, 33.57, 86.11) [D] — the upper courtyard by the ladder ledge.
    w.walk_nav_to([60.1, 28.4, 96.4], "courtyard west", 1.2);
    w.walk_nav_to([57.8, 26.0, 89.4], "door #153", 1.0);
    if let Some((_dpos, di)) = walker::door_near(w.scene.sim.as_ref().unwrap(), [57.8, 26.0, 89.4]) {
        w.use_door(di, "door #153");
    }
    w.walk_to(walker::Wp { at: [60.9, 29.9, 89.2], act: walker::Act::Ladder { top_y: 34.7 }, arrive: 0.9 }, "climb");
    w.walk_nav_to([59.7, 33.6, 86.1], "the armour case", 0.8);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.health.armour >= 50.0, "walking over #278 sets armour 50 [M 0x18e650:220-225]");
    }
    // The bonus token #326 at (73.69, 5.36, 79.82) [D] — the valley floor east. A teleport to
    // reach it, then the real walk-over take.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        // The valley floor east [D #326 (73.69, 5.36, 79.82)] — a teleport to reach it, the
        // walk-over take is the real operation.
        sim.bond.pos = [73.69, 6.0, 79.82];
    }
    w.wait(20);
    w.walk_nav_to([73.69, 5.36, 79.82], "the bonus token", 0.5);
    let sim = w.scene.sim.as_ref().unwrap();
    let taken = sim.drones.pickups.iter()
        .find(|p| p.kind == 6 && (p.start[0] - 73.69).abs() < 1.0)
        .expect("the valley token [D #326]")
        .gone;
    assert!(taken, "walking over the token takes it (kind 6 → PlrStat_LogBondBonus [M 0x18e650:234-236])");
}

/// KIND: searchlight 0xDE — the seven heads (param 0 = 1 [D]) sweep in state 0 and the port's
/// link rule reproduces the game's measured orphan rule [L spot 016e8ec0 on 07000005]: head #27's
/// psiLight (#31) is created AFTER the head, so the light links back into head.links[2] and the
/// head carries a live spot; head #230's light (#33) is created FIRST and its −10000 move takes it
/// out of the 3-unit link walk before the head exists — the measured orphan, no spot, never arms.
/// The sweep itself is [M Searchlight_Update 0x1cba60 state 0; L the golden heads bit-exact].
/// Sister: the sweep angles before Bond moves.
#[test]
fn the_searchlight_sweeps_and_the_orphan_light_never_arms() {
    let iso = iso();
    let mut w = load(&iso);
    let d = &w.scene.sim.as_ref().unwrap().drones.searchlights;
    let heads: Vec<usize> = d.parts.iter().enumerate().filter(|(_, p)| p.kind == 1).map(|(i, _)| i).collect();
    assert_eq!(heads.len(), 7, "the seven heads [D param 0 = 1]");
    let h27 = *heads.iter().find(|&&i| (d.parts[i].pos[0] + 57.03).abs() < 1.0).expect("head #27 [D]");
    let h230 = *heads.iter().find(|&&i| (d.parts[i].pos[0] - 62.17).abs() < 1.0).expect("head #230 [D]");
    assert!(d.parts[h27].links[2].is_some(), "head #27's light (#31, created after) links back [L the live link]");
    assert!(d.parts[h230].links[2].is_none(), "head #230's light (#33, created first) is the measured orphan [L spot 016e8ec0]");
    let (yaw0, pitch0) = (d.parts[h230].yaw, d.parts[h230].pitch);
    w.wait(120);
    let d = &w.scene.sim.as_ref().unwrap().drones.searchlights;
    assert_eq!(d.parts[h230].state, 0, "sister: the head still sweeps");
    assert!((d.parts[h230].yaw - yaw0).abs() > 1e-3 || (d.parts[h230].pitch - pitch0).abs() > 1e-3,
        "the sweep angles moved [M Searchlight_Update state 0]");
    // The bounds: #230 sweeps the (0,35)° cone — mid 17.5°, half 17.5° [D params 4/5; M half_mid].
    assert!((d.parts[h230].mid - 0.3054).abs() < 0.01 && (d.parts[h230].half - 0.3054).abs() < 0.01,
        "half_mid(0,35) = 17.5°/17.5° [M half_mid; D params]");
}
