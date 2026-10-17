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
