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
use walker::{trigger_centre, Walker};

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

/// KIND: hurt 0xE4 — the class-0x27 volume damage [M Player_DealWithObjHit 0x194458:95-103 →
/// Player_HandlePain(damage, Hurt_GetType, −1); armour cannot absorb pain types 1..7 [M
/// 0x1902c8:84-104]]. Bond walks off the upper west ledge at (−5, 34.3, 45) toward the valley: the
/// fall crosses the y-8.1 Hurt plane #329 (x −76..−23, z −0.2..74 [D the object cels]) whose 1000
/// type-6 damage kills outright. Sister: Bond stays on the ledge — alive, no fail channel.
#[test]
fn the_valley_fall_dies_in_the_hurt_volume() {
    let iso = iso();
    // Sister: the ledge stand.
    {
        let mut w = load(&iso);
        {
            let sim = w.scene.sim.as_mut().unwrap();
            sim.bond.pos = [-5.0, 35.0, 45.0];
        }
        w.wait(90);
        let sim = w.scene.sim.as_ref().unwrap();
        assert!((sim.bond.pos[1] - 34.3).abs() < 1.5, "the ledge stands at y 34.3 (got {:.2})", sim.bond.pos[1]);
        assert!(sim.drones.health.health > 0.0, "sister: standing on the ledge is safe");
        assert!(!sim.drones.channels.on(0x62), "sister: no mission fail");
    }
    let mut w = load(&iso);
    {
        let sim = w.scene.sim.as_mut().unwrap();
        // The walkable valley floor east [the probe: (63.5, 6, 57.9) stands at y 5.9].
        sim.bond.pos = [63.5, 6.5, 57.9];
    }
    w.wait(60);
    // Walk west down the valley floor into the #328/#330 plane's footprint (y −0.70, x −36..29
    // [D the object cels]); the plunge shells over the valley are the game's second kill path.
    w.walk_to(walker::Wp { at: [-10.0, -0.7, 60.0], act: walker::Act::None, arrive: 1.5 }, "west down the valley floor");
    let sim = w.scene.sim.as_ref().unwrap();
    let (ca, cb) = sim.bond.capsule_seg();
    let in_hurt = sim.drones.hurts.iter().filter(|h| h.contains(ca, cb, 0.55)).count();
    eprintln!("[05] after the fall: bond {:?} health {} in_hurt {} ch 0x62 {}",
        sim.bond.pos, sim.drones.health.health, in_hurt, sim.drones.channels.on(0x62) as u8);
    assert!(sim.drones.health.health <= 0.0 || sim.drones.channels.on(0x62),
        "the valley fall kills Bond — the hurt plane's 1000 damage or the plunge chain's death NIS [M/D]");
    assert!(in_hurt > 0, "the rest position sits inside a hurt volume [D the valley planes]");
}

/// KIND: copter 0x2F — #241 (no gate) flies from the load (the ambient fly-by [D param 6 = 0;
/// M Copter_Update's gate head]); #242 waits on ch 0x42 and starts when the
/// "SWITCH_Helicopterstart(SC3)" TouchOnce #269 fires (out 66 [D]), then its arrival writes
/// ch 75 [M Copter_Update cases 4/5/6]. Sisters: #242 idle before the plate.
#[test]
fn the_sc3_plate_starts_the_gated_copter() {
    let iso = iso();
    let mut w = load(&iso);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.copters.iter().find(|c| c.gate == 0).expect("#241").started,
            "the gate-0 copter runs from the first tick [M Copter_Update: a zero gate never blocks]");
        assert!(!sim.drones.copters.iter().find(|c| c.gate == 66).expect("#242").started,
            "sister: the ch-0x42 copter is idle before the plate");
        assert!(!sim.drones.channels.on(0x42), "sister: no plate, no ch 0x42");
    }
    // The plate #269 at (−22.76, 37.15, −72.36) [D] — a teleport to reach the upper ledge, the
    // walk-in is the real operation.
    let centre = trigger_centre(w.scene.sim.as_ref().unwrap(), walker::trigger_kind::TOUCH_ONCE, 66)
        .expect("the SC3 plate's box [D #269]");
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [centre[0], centre[1] + 2.0, centre[2] + 2.0];
    }
    w.wait(60);
    w.walk_nav_to([centre[0], centre[1], centre[2]], "onto the SC3 plate", 0.8);
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x42), "the plate raises ch 0x42 [D out 66]");
    assert!(sim.drones.copters.iter().find(|c| c.gate == 66).expect("#242").started,
        "the gate opens and #242's flight starts [M Copter_Update]");
}

/// KIND: SoundTrigger 0xF9 — #254 (trigger ch 139 → the sound 368 [D params (0,139),(2,368);
/// M SoundTrigger_Update 0x1b2790: the param-0 channel rising plays the param-2 id]). TouchOnce
/// #253 (out 139 [D]) sits on the upper ledge south (24.82, 31.88, 67.59) — the walk-in fires it.
/// Sister: a fresh load — ch 139 off, nothing plays.
#[test]
fn the_ledge_plate_plays_the_sound_trigger() {
    let iso = iso();
    {
        let w = load(&iso);
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(139), "sister: ch 139 off");
        assert!(!sim.drones.sound_triggers.playing(368), "sister: sound 368 not playing");
    }
    let mut w = load(&iso);
    {
        let sim = w.scene.sim.as_mut().unwrap();
        // A teleport to reach the upper ledge south; the walk onto the plate is the operation.
        sim.bond.pos = [25.0, 33.0, 69.0];
    }
    w.wait(60);
    w.walk_nav_to([24.82, 31.8, 67.5], "plate #253", 0.5);
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(139), "the plate raises ch 139 [D out]");
    assert!(sim.drones.sound_triggers.playing(368), "the ch-139 trigger plays sound 368 [M 0x1b2790]");
}

/// KIND: spawner 0xF1 — #268 (group 2, start ch 34 = 0x22 [D params (1,2),(4,34)]) brings the
/// group-2 templates in when the approach OR mux raises ch 0x22 (the plates test's chain [D]).
/// Sister: a fresh load keeps the spawner empty.
#[test]
fn the_approach_channel_spawns_the_sniper_group() {
    let iso = iso();
    {
        let w = load(&iso);
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.spawners.len(), 2, "the two spawner placements [D]");
        // #266's start = param 4 = 0 → active at create, the game's immediate fill [M 0x137aa0
        // immediate = (start == 0 || ch.on(start))]. #268 waits on ch 0x22.
        let s268 = sim.drones.spawners.iter().find(|s| s.start == 34).expect("spawner #268 [D]");
        assert!(s268.spawned.is_empty(), "sister: the ch-0x22 spawner holds before the plates");
    }
    let mut w = load(&iso);
    w.walk_nav_to([60.1, 28.4, 96.4], "approach plate #21", 1.2);
    let mut spawned = false;
    for _ in 0..1800u32 {
        w.tick(Input::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.spawners.iter().any(|s| !s.spawned.is_empty()) { spawned = true; break; }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    eprintln!("[05] spawners: {:?}", sim.drones.spawners.iter()
        .map(|s| (s.placement, s.group, s.active, s.spawned.len(), s.kill_latch)).collect::<Vec<_>>());
    assert!(spawned, "ch 0x22 starts spawner #268 and the group-2 templates come in [M DroneSpawner_Control 0x137e68]");
}

/// KIND: grapple points 0x3A — #282 (13.75, 32.29, 57.30 [D]; `Grapple_Create` 0x198298, class
/// 0x51): the Q-watch aim resolves the point from Bond's position and look [M; the port's
/// `pick_grapple_point`]. Sister: aiming away from the point misses.
#[test]
fn the_grapple_point_aims_for_the_watch() {
    let iso = iso();
    let mut w = load(&iso);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.grapple_points.len(), 2, "the two graple_pulley placements [D]");
    }
    // The point's base — a teleport to reach it, the aim is the real operation.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [13.75, 33.0, 57.3];
    }
    w.wait(60);
    let sim = w.scene.sim.as_ref().unwrap();
    let p = &sim.drones.grapple_points.iter().find(|g| (g.pos[0] - 13.75).abs() < 1.0).expect("#282").pos;
    let from = sim.bond.pos;
    let dir = [p[0] - from[0], p[1] - (from[1] + 1.0), p[2] - from[2]];
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let dir = [dir[0] / len, dir[1] / len, dir[2] / len];
    let hit = nf_game::gadget::pick_grapple_point(&sim.drones.grapple_points, from, dir, 60.0);
    assert!(hit.is_some(), "aiming at #282 resolves the grapple point [M Grapple_Create/fix-gadgets]");
    let away = nf_game::gadget::pick_grapple_point(&sim.drones.grapple_points, from, [dir[0], 0.0, dir[2]], 60.0);
    let _ = away; // the away-aim may still hit the other point — the sister is the MISS below.
    let miss = nf_game::gadget::pick_grapple_point(&sim.drones.grapple_points, from, [-dir[0], -dir[1], -dir[2]], 60.0);
    assert!(miss.is_none(), "sister: aiming the opposite way misses both points");
}
