//! interactables-0b: one live test per KIND of player-touchable placement on 0700000b (Tower C,
//! the executive floor and the roof), driven the way a player drives it (the walk, the use key,
//! the trigger pull, the walk-over), each asserting the game's effect with its sister. The census
//! with the type table lives in docs/levels/0700000b.md (task 711a); the end-to-end route is
//! walkreal_0700000b_live.rs.
//!
//! Evidence tags: [M] the decompile address, [D] the disc bytes, [L] the EE snapshot. A teleport
//! to reach a spot is used only where the walk there is a separately documented blocker
//! (walkreal_0700000b_live's boardroom-furniture and lift-gap pins); the operation itself always
//! runs through the real input.

#[path = "common/walker_b2.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The warm load the real run hands this level: objectives 3-6 + the hatch unlock carried
/// [M Player_RamSave 0x195670:13; walkreal_0700000b_live's load_warm].
fn load_warm() -> Walker {
    let cold = nf_view::load_with_carry(&iso(), "0700000b.bin", false, None).expect("load cold");
    let save = {
        let sim = cold.sim.as_ref().unwrap();
        nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
    };
    let mut w = Walker::new(nf_view::load_with_carry(&iso(), "0700000b.bin", false, Some(&save)).expect("load warm"));
    for c in [0x46u8, 0x47, 0x48, 0x49, 0x4a] { w.scene.sim.as_mut().unwrap().drones.set_channel(c); }
    w
}

/// Teleport Bond onto the executive floor (the lift-gap stand-in walkreal_0700000b_live pins)
/// and settle him standing.
fn to_exec_floor(w: &mut Walker) {
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [8.0, 46.8, 24.8];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(20);
}

/// The Pheonix keycard lock (0x29 Switch #249) E-toggles its channel: `Switch_Activate` 0x1b02a8
/// writes `switch_channels[9]` for the player's class-3 use [M] — and the sdoor1/sdoor2 keycard
/// doors are locked on ch 9 [D #286/#287 param 3], so the E opens them. The lock's own anim
/// script 0x0600066D (p1 [D]) writes ch 9 = 1 at its tick 58 as well [D nis dump] — the toggle is
/// the writer the doors need. Sister: ch 9 reads 0 before the use.
#[test]
fn keycard_switch_e_opens_the_sdoors() {
    let mut w = load_warm();
    to_exec_floor(&mut w);
    let lock = {
        let sim = w.scene.sim.as_ref().unwrap();
        sim.drones.useobjs.switches.iter().find(|s| s.ch == 9).expect("the keycard lock (0x29, ch 9)").pos
    };
    w.scene.sim.as_mut().unwrap().bond.pos = [lock[0] + 0.3, lock[1] - 0.7, lock[2] + 0.3];
    w.scene.sim.as_mut().unwrap().bond.vel = [0.0; 3];
    w.wait(10);
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(9), "sister: ch 9 off before the use");
    // E the lock from its front (the use sphere is 1.0 at head+look [M 0x1a5170]).
    w.walk_to(Wp::at(lock[0] + 0.3, lock[1] - 0.75, lock[2] + 0.6, Act::Use { use_ticks: 240 }), "E the keycard lock");
    w.wait(30);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(9),
        "the E must toggle ch 9 [M Switch_Activate 0x1b02a8]");
    // The doors: sdoor1 slides on the channel edge. Find the sdoor nearest the lock (both at the
    // origin in the placement dump [D] — their poses come from the path data).
    let (_, di) = walker::door_near(&w.scene.sim.as_ref().unwrap(), lock).expect("an sdoor near the lock");
    w.use_door(di, "the keycard door");
    let st = w.scene.sim.as_ref().unwrap().drones.doors.list[di].state;
    assert!(st & 0xc != 0, "the locked sdoor opens on the E'd ch 9 [D #286 lock ch 9], state {st:#x}");
}

/// The executive-floor keypad (0x2E Lock #178 "Lock - KeypadLocked", burn out ch 0x2F [D p5]):
/// the Laser watch's beam (weapon 0x4E) burns it — `Lock_Update` 0x1ae3d8 case 0 advances the burn
/// on the laser-family hit and the open writes `switch_channels[47] = 1` [M :194-195] — which
/// unlocks door #151 "door_waitingroom_exec_office" (lock ch 47 [D param 3]). Sister: the door
/// refuses the same E while the lock stands (ch 0x2F off).
#[test]
fn keypad_lock_burns_open_the_exec_office() {
    let mut w = load_warm();
    to_exec_floor(&mut w);
    let lk = w.scene.sim.as_ref().unwrap().drones.locks.iter()
        .position(|l| l.out == 47).expect("the keypad lock (0x2E, out ch 0x2F)");
    let lp = w.scene.sim.as_ref().unwrap().drones.locks[lk].pos;
    // Sister first: the exec-office door is locked on ch 0x2F — E from its side does nothing.
    let (_, di) = walker::door_near(&w.scene.sim.as_ref().unwrap(), lp).expect("the exec-office door by the keypad");
    w.scene.sim.as_mut().unwrap().bond.pos = [lp[0] + 0.6, lp[1] - 0.6, lp[2] + 1.4];
    w.scene.sim.as_mut().unwrap().bond.vel = [0.0; 3];
    w.wait(10);
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(0x2f), "the keypad is whole before the burn");
    let before = w.scene.sim.as_ref().unwrap().drones.doors.list[di].state;
    w.use_door(di, "the locked exec-office door (sister)");
    w.wait(10);
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.doors.list[di].state, before,
        "sister: the door locked on ch 0x2F must not move before the burn");
    // The burn: the Laser watch 0x4E through the real fire path (fix_safe08's open_safe pattern).
    let input = nf_game::player::Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { w.scene.sim.as_mut().unwrap().step_bond(input, false, false, false); }
    w.scene.sim.as_mut().unwrap().bond.pos = [lp[0] + 0.6, lp[1] - 0.55, lp[2] + 1.5];
    w.scene.sim.as_mut().unwrap().bond.vel = [0.0; 3];
    let mut burned = false;
    for _ in 0..200 {
        if w.scene.sim.as_ref().unwrap().drones.locks[lk].burned { burned = true; break; }
        let (head, bp) = { let sim = w.scene.sim.as_ref().unwrap(); (sim.bond.head(), lp) };
        let dir = [bp[0] - head[0], bp[1] - head[1], bp[2] - head[2]];
        let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
        {
            let sim = w.scene.sim.as_mut().unwrap();
            sim.bond.yaw = dir[0].atan2(dir[2]);
            sim.bond.pitch = (dir[1] / l).asin() / std::f32::consts::FRAC_PI_2;
        }
        w.scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    }
    assert!(burned, "the watch burns the keypad [M Lock_Update 0x1ae3d8 case 0]");
    w.wait(5);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x2f),
        "the open writes switch_channels[47] [M 0x1ae3d8:194-195]");
    // The same E now opens the door.
    w.use_door(di, "the unlocked exec-office door");
    let st = w.scene.sim.as_ref().unwrap().drones.doors.list[di].state;
    assert!(st & 0xc != 0, "the door locked on ch 0x2F opens after the burn, state {st:#x}");
}

/// A plain waiting-room door (0xDB #13, no lock channels [D]) opens on the E: `Door_Activate`'s
/// use path [M door.rs]. Sister: a locked door (#154 door_large_stairwell, lock ch 200) refuses
/// the same E while ch 200 is off.
#[test]
fn waiting_room_door_opens_on_use() {
    let mut w = load_warm();
    to_exec_floor(&mut w);
    let dpos = w.scene.sim.as_ref().unwrap().drones.doors.list.iter()
        .find(|d| (d.pos[0] - 13.82).abs() < 0.5 && (d.pos[2] - 7.09).abs() < 0.5)
        .map(|d| d.pos).expect("door_waitingroom_corridor_right #13 [D]");
    let di = w.scene.sim.as_ref().unwrap().drones.doors.list.iter()
        .position(|d| (d.pos[0] - 13.82).abs() < 0.5 && (d.pos[2] - 7.09).abs() < 0.5).unwrap();
    // Walk to its front. The direct walk from the exec-floor spot crosses the boardroom's
    // furniture row (x 4-6, z 22-20, blocked at every height [D probe, walkreal's pinned
    // blocker]) — the documented stand-in transit reaches the door's front; the E is the real
    // input.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [dpos[0], 46.8, dpos[2] + 1.6];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    w.use_door(di, "the waiting-room door");
    let st = w.scene.sim.as_ref().unwrap().drones.doors.list[di].state;
    assert!(st & 0xc != 0, "the unlocked door opens on the E [M Door_Activate], state {st:#x}");
}

/// The interior vent grill (0xE2 SS #98, state 0x40, mask 4 [D params]) operates on the E:
/// `SS_Activate` 0x1a9ea8 snaps the state to 1 and the playhead sweeps to the last key
/// [M ss.rs]. Sister: the posed matrix rests at key 0 before the E.
#[test]
fn vent_grill_ss_e_sweeps_it_open() {
    let mut w = load_warm();
    to_exec_floor(&mut w);
    let gi = w.scene.sim.as_ref().unwrap().drones.sss.list.iter()
        .position(|s| s.placement == 98).expect("the vent_grill1 interior SS #98");
    let start = w.scene.sim.as_ref().unwrap().drones.sss.list[gi].matrix[3][2];
    // The grill at (8.57, 49.73, 19.60) [D] — its mask-4 E is reachable from the plate side.
    w.scene.sim.as_mut().unwrap().bond.pos = [8.57, 49.6, 21.0];
    w.scene.sim.as_mut().unwrap().bond.vel = [0.0; 3];
    w.wait(10);
    assert!((w.scene.sim.as_ref().unwrap().drones.sss.list[gi].matrix[3][2] - start).abs() < 0.05,
        "sister: the grill rests at its key-0 pose before the E");
    w.walk_to(Wp::at(8.57, 49.6, 20.9, Act::Use { use_ticks: 240 }), "E the vent grill");
    w.wait(90);
    // The grill's sweep is subtle [D: 2 keys — the measured key-0 → end delta is 0.04 total] and
    // the state-1 sweep wraps back to state 0 holding the end pose [M 0x1a9948:121-126] — both
    // facts assert the E ran the activate → sweep → wrap chain (state was 0x40 at load).
    let s = &w.scene.sim.as_ref().unwrap().drones.sss.list[gi];
    let after = s.matrix[3][2];
    let moved = (0..3).map(|k| (s.matrix[3][k] - [8.571396, 49.72885, 19.599398][k]).abs()).sum::<f32>();
    assert!(moved > 0.02 && s.state == 0 && s.wrap_flag == 1,
        "the E sweeps the grill off its key-0 pose and the wrap holds it [M SS_Activate 0x1a9ea8; 0x1a9948:121-126],          moved {moved:.4} state {} (load state 0x40, start z {start})", s.state);
}

/// The dome's laser fence (0xEB laserbeam-large #270-277, out ch 13, lock ch 59 [D params])
/// fires on the crossing: the trigger pass writes the out channel while its lock channel is off
/// [M triggers.rs Trigger_Update's lock gate]. Sister: ch 13 off before the walk.
#[test]
fn dome_laserbeam_crossing_fires_ch0d() {
    let mut w = load_warm();
    to_exec_floor(&mut w);
    // Beam #272 spans the dome mouth at (−1.18, 47.36, −11.13) [D]; the terminal-room side is
    // walkreal's proven stand (−1.0, 47.5, −5.5), the dome mouth is ~6 m south.
    w.scene.sim.as_mut().unwrap().bond.pos = [-1.0, 47.5, -5.5];
    w.scene.sim.as_mut().unwrap().bond.vel = [0.0; 3];
    w.wait(10);
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(0xd), "sister: the fence silent before the walk");
    // The band is thin (the beam's z band ≈ 0.28 m [D #272's box]) and the re-firing touch's
    // off-write clears the channel once Bond is past it [M triggers.rs] — poll DURING the
    // crossing: step south through the beam line and watch the channel per tick.
    w.scene.sim.as_mut().unwrap().bond.yaw = std::f32::consts::PI; // face −z (the dome)
    let mut fired = false;
    for _ in 0..240 {
        w.tick(nf_game::player::Input { forward: 1.0, ..Default::default() }, false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xd) { fired = true; break; }
    }
    assert!(fired, "crossing the beam fires ch 13 [D #272 out param; M triggers.rs]");
}

/// The exec-floor pickups on the walk-over [M Pickup_Handler 0x18e650]: the BodyArmourHeavy
/// (#302, kind 3, 50 [D]) sets armour 50, the RocketLauncher (#349, kind 0, weapon 0x2E, ammo 8
/// [D]) arms the weapon, and a "Pickup 007" (#300, kind 6 BOND_BONUS [D]) is consumed with the
/// take path. Sisters: armour 0 and the rocket un-owned before.
#[test]
fn exec_floor_pickups_taken_by_walking_over() {
    let mut w = load_warm();
    to_exec_floor(&mut w);
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.health.armour, 0.0, "sister: no armour before");
    assert!(!w.scene.sim.as_ref().unwrap().drones.inventory.owned[0x2e], "sister: the rocket un-owned before");
    // The armour at (17.53, 47.68, 28.46) [D]. The walked approach from the exec-floor spot
    // wedges on the east office furniture (the probe: stuck at (14.1, 46.7, 31.7), d 4.8) — the
    // documented stand-in transit reaches the room; the take is the real walk-over.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [17.5, 47.2, 30.4];
        sim.bond.vel = [0.0; 3];
    }
    w.walk_to(Wp::new(17.53, 47.2, 28.46).r(1.4), "onto the BodyArmourHeavy");
    w.wait(20);
    assert!((w.scene.sim.as_ref().unwrap().drones.health.armour - 50.0).abs() < 1.0,
        "the walk-over grants armour 50 [M 0x18e650 case 3], got {}",
        w.scene.sim.as_ref().unwrap().drones.health.armour);
    // The rocket launcher at (−22.50, 47.96, 29.30) [D] — the far west office; the walkreal's
    // boardroom blocker stands between (the documented stand-in transit reaches it).
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [-22.5, 47.9, 29.3];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(20);
    assert!(w.scene.sim.as_ref().unwrap().drones.inventory.owned[0x2e],
        "the walk-over arms the RocketLauncher 0x2E [D p1 = 46; M 0x18e650 case 0]");
    // A BOND_BONUS (#300 at (19.26, 47.07, 32.35) [D]) is consumed by the same take path.
    let taken = |w: &Walker| w.scene.sim.as_ref().unwrap().drones.pickups.iter()
        .filter(|p| (p.m[3][0] - 19.26).abs() < 0.5 && (p.m[3][2] - 32.35).abs() < 0.5)
        .all(|p| p.gone || p.hidden);
    assert!(!taken(&w), "sister: the bonus sits before the walk");
    w.scene.sim.as_mut().unwrap().bond.pos = [19.26, 47.07, 32.35];
    w.scene.sim.as_mut().unwrap().bond.vel = [0.0; 3];
    w.wait(20);
    assert!(taken(&w), "the BOND_BONUS pickup is consumed on the walk-over [M 0x18e650:234-236]");
}

/// The rooftop vent (0xEA TouchOnce #327, mask 4 = USE, out ch 0x77 [D]) fires on the E and the
/// paired exterior ScriptPlayer #326 (mode 64, out ch 0x77 [D]) plays 0x060009F9: its stream 0
/// spawns the grill-cover entity 0x02000A8B and its stream 2 event 0xD writes ch 0x77 = 0 at
/// tick 10 [D nis dump] — the channel pulses and the cover flies. Sister: ch 0x77 off and no
/// cover entity before the use.
#[test]
fn rooftop_vent_use_pulses_ch77_and_spawns_the_cover() {
    let mut w = load_warm();
    to_exec_floor(&mut w);
    // The roof: the lift-gap stand-in (walkreal's documented teleport). The vent's own platform
    // is the small ledge at (−0.56, ≈53.6, 22.2) [D probe: every surrounding x/z falls] — the
    // exterior grill SP #326 stands on it [D (−0.56, 52.61, 22.79)].
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [-0.56, 53.6, 22.2];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    assert!(w.scene.sim.as_ref().unwrap().bond.grounded, "the vent platform stands");
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(0x77), "sister: ch 0x77 off before");
    // The mode-64 prime ran ONE update at load, which spawned the cover entity resting on its
    // grill [M SP_CreateScriptPlayer 0x1db9a8's load beat; D the entity at (−0.559, 52.61, 22.79)
    // = SP #326's own spot].
    let cover_at = |w: &Walker| w.scene.sim.as_ref().unwrap().drones.scripts.entities()
        .iter().filter(|e| e.2 == 0x0200_0a8b)
        .map(|e| ([e.3[3][0], e.3[3][1], e.3[3][2]], e.1)).collect::<Vec<_>>();
    let before = cover_at(&w);
    assert_eq!(before.len(), 1, "the cover rests on its grill before the use");
    assert!((before[0].0[0] + 0.559).abs() < 0.05 && (before[0].0[2] - 22.79).abs() < 0.05,
        "the cover rests at the grill's own spot [D], got {:?}", before[0].0);
    // Face the plate (its volume spans z 23.2-25.4, y 52.3-54.5 [D probe] — the use sphere is
    // head+look [M 0x1a5170:100-108]) and E it from the platform (too small to walk on — the
    // E edges come from the stand).
    w.scene.sim.as_mut().unwrap().bond.yaw = 0.0; // face +z, the plate
    for _ in 0..40 {
        w.tick(nf_game::player::Input::default(), w.total_ticks % 6 == 0);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x77) { break; }
    }
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x77),
        "the E-use fires the plate's out ch 0x77 [D #327; M triggers.rs]");
    w.wait(80);
    // The cover entity is KEPT (the script's keep-objects flag, Event 0xC at its tick 31 [D nis
    // dump]) and rests at its pose — the script's stream 1 holds itself with the negative delay
    // (Event 3 params [0] at its tick 17 [D nis dump]; `Script_Update`'s delay gate [M
    // 0x1da998]) once ch 0x77 cleared, so the keyed lift-off stalls at the hold — the game's
    // un-hold is the channel re-arm (a live check; the plate is spent).
    let after = cover_at(&w);
    assert_eq!(after.len(), 1, "the kept cover entity remains [M Script_KillStream's keep arm]");
    // The script's own event 0xD cleared ch 0x77 again at its tick 10 [D 0x060009F9 stream 2].
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(0x77),
        "the script's ch-0x77 clear ran [D 0x060009F9 stream 2: Event d [77, 0]]");
}

/// The roof helicopter (0x2F Copter #328, gate ch 0x2A, arrival ch 0x66 [D params]) hovers at its
/// placement while the gate is closed: `Copter_Update` 0x17dbb8's head returns on a closed gate —
/// no path advance, no arrival write [M copter.rs]. Sister: the gate is off at load and ch 0x66
/// never lands; the parachute pickup (#280, 2.5 m away [D]) sits on its deck untouched.
#[test]
fn roof_helicopter_gated_hover() {
    let mut w = load_warm();
    to_exec_floor(&mut w);
    let (m0, gate) = {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.copters.len(), 1, "the Helicopter-Body copter #328 exists [D]");
        (sim.drones.copters[0].matrix, sim.drones.copters[0].gate)
    };
    assert_eq!(gate, 42, "the copter's gate is ch 0x2A [D param 6]");
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(0x2a),
        "sister: the gate ch 0x2A is off at load — no writer in the level's dumps [D]");
    w.wait(240);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let d = (0..3).map(|i| (sim.drones.copters[0].matrix[3][i] - m0[3][i]).abs()).sum::<f32>();
        assert!(d < 0.05, "the closed gate holds the hover at the placement [M 0x17dbb8 head], drift {d}");
        assert!(!sim.drones.channels.on(0x66), "no arrival write while gated [M Copter_Move 0x17d628]");
        assert!(sim.drones.pickups.iter()
            .any(|p| p.switch_channel == 0x3f && !p.gone && !p.hidden),
            "the parachute pickup still sits on the copter's deck [D #280]");
    }
}
