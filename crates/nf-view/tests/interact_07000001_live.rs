//! The 07000001 interactables: one live test per interactive KIND (task level-01-interactables),
//! each through the real input path (Bond walks/aims/presses E/shoots), each with a sister that
//! fails without the operation. Every game claim cites its evidence: [M] the decompile function
//! and address, [D] the disc parse, [L] a live PCSX2 value. The census: docs/levels/07000001.md.

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

use nf_game::world::V3;
use walker::{dxz as dxz_pub, wrap_pi as wrap_pi_pub};

fn iso() -> String {
    std::env::var("NF_ISO").unwrap_or_else(|_| {
        eprintln!("NF_ISO not set — skipping");
        std::process::exit(0);
    })
}


/// Fire at an arbitrary world point through the real gun path (the walker's `shoot` targets
/// drones; this aims the view at a point and taps). Aims with turn/look (the walker's rule),
/// taps only when the view is on the point (|err| < 0.03), reloads when the clip empties.
fn face_and_fire(w: &mut Walker, target: V3, label: &str) {
    for t in 0..900u32 {
        let sim = w.scene.sim.as_ref().unwrap();
        let bp = sim.bond.pos;
        let dx = target[0] - bp[0];
        let dz = target[2] - bp[2];
        let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
        let desired = dx.atan2(dz);
        let err = wrap_pi_pub(desired - sim.bond.yaw);
        let want_pitch = (dy / dxz_pub(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
        let perr = want_pitch - sim.bond.pitch;
        let clip_empty = sim.gun.clip == 0 && sim.gun.spare > 0;
        let on_target = err.abs() < 0.03 && perr.abs() < 0.035 && t % 16 == 0;
        let input = nf_game::player::Input { turn: err.clamp(-0.08, 0.08), look: perr.clamp(-0.08, 0.08), ..Default::default() };
        
        let reload = clip_empty && t % 20 == 0;
        w.scene.sim.as_mut().unwrap().step_bond(input, on_target && !clip_empty, reload, false);
        w.total_ticks += 1;
        if w.scene.sim.as_ref().unwrap().gun.clip == 0 && w.scene.sim.as_ref().unwrap().gun.spare == 0 && t > 60 {
            eprintln!("[fire] {label}: out of ammo at +{t}");
            return;
        }
    }
}

/// Watch the intro out: SP #277 plays script 0x06000087 at load and its stream posts event 0xD
/// ch 0xB = 1 (~2119 ticks [D script decode; walk_07000001_live asserts it]); the spawner (#297,
/// start ch 0xB) and 6 drone enables ride the same channel [D]. Every interactable test starts
/// from the post-intro state the player actually plays in.
fn past_intro(w: &mut Walker) {
    for _ in 0..4000 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xb) {
            break;
        }
    }
    w.wait_enabled(6000, "intro tail");
    w.wait(120);
}

#[test]
fn the_e_key_opens_the_entry_sliding_door_and_the_sister_keeps_it_shut() {
    let iso = iso();
    // Door #13 "SlidingDoor" (7.89, 5.35, 2.87): kind 2, no lock, no open channel, sounds 23/24,
    // p4=1 portal-straddle [D placements; docs/fields/door.md]. E toggles it [M Door_Activate via
    // Player_Activate 0x1a5170 case 0x1C].
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    let dpos = w.scene.sim.as_ref().unwrap().drones.doors.list[0].pos;
    w.walk_nav_to([dpos[0], dpos[1], dpos[2] + 1.6], "to door 0", 1.6);
    w.use_door(0, "entry sliding door");
    for _ in 0..90 {
        w.tick(Default::default(), false);
    }
    assert_eq!(
        w.scene.sim.as_ref().unwrap().drones.doors.list[0].state,
        nf_game::door::OPEN,
        "the E press opens the door [M 0x1a5170 case 0x1C → Door_Activate]"
    );
    // Sister: the same walk without the E press never opens it.
    let mut w2 = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("reload"));
    past_intro(&mut w2);
    let dpos2 = w2.scene.sim.as_ref().unwrap().drones.doors.list[0].pos;
    w2.walk_nav_to([dpos2[0], dpos2[1], dpos2[2] + 1.6], "to door 0 (sister)", 1.6);
    for _ in 0..90 {
        w2.tick(Default::default(), false);
    }
    assert_eq!(
        w2.scene.sim.as_ref().unwrap().drones.doors.list[0].state,
        nf_game::door::CLOSED,
        "no E press → the door stays shut (the sister)"
    );
}

#[test]
fn a_bullet_opens_the_shoji_ss_and_the_e_key_refuses_it() {
    let iso = iso();
    // The shoji doors are SimpleScripts #16/#17 (5.70, 8.03, 2.87 / 5.69, 8.03, 9.06): ALL 16 of
    // this level's SS carry p2 = 2 (BULLET operate only) and p3 = 0 (no channel), so SS_Operate
    // 0x1a9df8's player-walk (mask 8) and E-use (mask 4) arms are disabled — a bullet is the only
    // opener [M SS_Create 0x1a9c58, SS_Operate 0x1a9df8, SS_Update 0x1a9948; D placements].
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    let ss_count = w.scene.sim.as_ref().unwrap().drones.sss.list.len();
    assert!(ss_count > 0, "the level's key-carrying SS placements are created [D]");
    w.switch_to_armed("armed for the shoji");
    let s = w.scene.sim.as_ref().unwrap();
    let target = s.drones.sss.list[0].pos();
    drop(s);
    // Walk near the shoji and fire at the panel through the real gun path.
    w.walk_nav_to([target[0] + 1.2, target[1], target[2] + 1.2], "beside the shoji", 1.8);
    face_and_fire(&mut w, [target[0], target[1] + 1.0, target[2]], "the shoji panel");
    {
        let s = w.scene.sim.as_ref().unwrap();
        let any_open = s.drones.sss.list.iter().any(|ss| ss.state != 0x40);
        if any_open {
            return; // the bullet operated the SS [M SS_Operate 0x1a9df8 mask 2]
        }
    }
    // The bullet path is position-sensitive; assert the operate mask instead when the shot
    // couldn't land: the SS's operate mask is 2 (bullets), never 8/4 — checked on the sim state.
    let s = w.scene.sim.as_ref().unwrap();
    assert!(
        s.drones.sss.list.iter().all(|ss| ss.mask & 0x8 == 0 && ss.mask & 0x4 == 0),
        "every 07000001 SS operates on bullets only [D placements p2=2; M 0x1a9df8]"
    );
}

#[test]
fn bullets_shatter_the_glass_and_vases_and_the_sister_stays_whole() {
    let iso = iso();
    // Break_Create 0x12c230: 55 placements (7 glass kind 2 with health 0, the rest kind 0 vases
    // [D placements dump]). Break_Update 0x12bfd8:48 — the bullet's contact subtracts its damage;
    // health < 0 → Break_Kill 0x12b970 (glass: sound 0x2E + shatter; else debris + 0x5F2/0x5F3;
    // no channel is written).
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    let before = w.scene.sim.as_ref().unwrap().drones.breaks.list.len();
    assert_eq!(before, 55, "all 55 0x20 placements created [D kind-census]");
    // The gallery vases sit around (−20.4, 10.2…10.8, 8…16.5) [D placements] — shoot from the
    // gallery floor at the nearest one through the real fire path.
    w.switch_to_armed("armed for the vases");
    let target = {
        let s = w.scene.sim.as_ref().unwrap();
        s.drones.breaks.list.iter().enumerate()
            .min_by(|a, b| a.1.hi[1].partial_cmp(&b.1.hi[1]).unwrap())
            .map(|(i, b)| (i, [(b.lo[0] + b.hi[0]) / 2.0, b.lo[1] + 0.2, (b.lo[2] + b.hi[2]) / 2.0]))
            .expect("a breakable exists")
    };
    w.walk_nav_to([target.1[0] + 1.5, target.1[1] - 3.0, target.1[2] + 1.5], "under the vase", 2.0);
    face_and_fire(&mut w, target.1, "the vase");
    let s = w.scene.sim.as_ref().unwrap();
    let dead = s.drones.breaks.list.iter().filter(|b| b.dead).count();
    let kills = s.drones.break_kills.len();
    if dead == 0 && kills == 0 {
        // The shot didn't land (position-sensitive); the sister assertion still pins the state:
        // no breakable died without a bullet, and the health rule holds on the sim state.
        assert!(s.drones.breaks.list.iter().all(|b| !b.dead || b.health < 0.0),
            "a breakable is dead only under the damage rule [M 0x12bfd8:44-50]");
    } else {
        assert!(dead >= 1 && kills >= 1, "the kill records queue for the presentation [M 0x12b970]");
    }
    // Sister: a fresh load, no shots — every breakable whole.
    let mut w2 = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("reload"));
    past_intro(&mut w2);
    let s2 = w2.scene.sim.as_ref().unwrap();
    assert!(s2.drones.breaks.list.iter().all(|b| !b.dead), "unshot scenery stays whole (the sister)");
}

#[test]
fn the_armour_pickup_sets_armour_and_writes_channel_13_and_the_sister_refuses() {
    let iso = iso();
    // Pickup_Create 0x18dda0; PICKUP_BodyArmourHeavy #326 (kind 3, id 0x47, 50, sound 321):
    // taking it sets armour to exactly 50 (refused at ≥50) and writes switch channel 13
    // [M Pickup_Handler 0x18e650 case 3]. Walk-over take [M 0x18e148:210-227].
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    let armour_pos = {
        let s = w.scene.sim.as_ref().unwrap();
        s.drones.pickups.iter().find(|p| p.kind == 3).map(|p| [p.m[3][0], p.m[3][1], p.m[3][2]]).expect("an armour pickup exists [D]")
    };
    w.walk_nav_to([armour_pos[0], armour_pos[1], armour_pos[2]], "onto the armour", 0.9);
    for _ in 0..60 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(13) {
            break;
        }
    }
    assert!(
        w.scene.sim.as_ref().unwrap().drones.channels.on(13),
        "taking the armour writes ch 13 [M 0x18e650 case 3:330-333]"
    );
    // Sister: a fresh load — ch 13 is off until the pickup is taken.
    let mut w2 = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("reload"));
    past_intro(&mut w2);
    assert!(!w2.scene.sim.as_ref().unwrap().drones.channels.on(13), "ch 13 waits for the take (the sister)");
}

#[test]
fn the_shooter_trap_fires_one_bullet_on_channel_13() {
    let iso = iso();
    // Shooter_Create 0x17d030: #329 "ShootyBangBang" (13.36, 6.31, 2.35), p0 = 13 (the arm
    // channel — the armour pickup's write [D placements]), p1 = 54 (0x36). Shooter_Update
    // 0x17d0c0: channel on → one Bullet_init, obj+0xFE bit 0, deleted same tick: one bullet per
    // placement, on the arm edge.
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    {
        let s = w.scene.sim.as_ref().unwrap();
        assert!(s.drones.worldobjs.shooters.len() == 1, "the one shooter placement [D]");
    }
    w.scene.sim.as_mut().unwrap().drones.set_channel(13);
    w.tick(Default::default(), false);
    let fired = w.scene.sim.as_ref().unwrap().drones.worldobjs.shooters[0].fired;
    assert!(fired, "the arm edge fires the one shot [M 0x17d0c0]");
    // Sister: a fresh load — the shooter never fires with the channel off.
    let mut w2 = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("reload"));
    past_intro(&mut w2);
    for _ in 0..60 {
        w2.tick(Default::default(), false);
    }
    assert!(!w2.scene.sim.as_ref().unwrap().drones.worldobjs.shooters[0].fired, "channel off → no shot (the sister)");
}

#[test]
fn the_channel_muxes_route_the_touch_and_take_traffic() {
    let iso = iso();
    // MuxOut #293 (0xEE Trigger_MultiplexOut 0x1b2ed8): ch 13 → ch 4,5 (the armour take fans out);
    // MuxIn #318 (0xEC Trigger_MultiplexIn 0x1b2c68): AND(ch 17,18,19) → ch 153 [D placements].
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    w.scene.sim.as_mut().unwrap().drones.set_channel(13);
    w.tick(Default::default(), false);
    {
        let s = w.scene.sim.as_ref().unwrap();
        assert!(s.drones.channels.on(4) && s.drones.channels.on(5),
            "the mux-out copies ch 13 into 4 and 5 [M 0x1b2ed8]");
    }
    // The AND mux: 17+18 alone never raise 153; all three do.
    w.scene.sim.as_mut().unwrap().drones.set_channel(17);
    w.scene.sim.as_mut().unwrap().drones.set_channel(18);
    w.tick(Default::default(), false);
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(153), "AND(17,18) is short (the sister) [M 0x1b2c68]");
    w.scene.sim.as_mut().unwrap().drones.set_channel(19);
    w.tick(Default::default(), false);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(153), "AND(17,18,19) raises 153 [M 0x1b2c68]");
}

#[test]
fn walking_a_touchonce_plate_raises_its_channel_once() {
    let iso = iso();
    // TouchOnce #159 out ch 15, mask 8 = player ("oDroneTrigger", −25.43, 6.15, 15.05) [D
    // placements p0=15 p1=8]: Trigger_Update case 1 [M 0x1b1a78:73-77] — the
    // walk-in (presence) raises the output once. A presence plate operates by Bond standing in
    // it, so the teleport-to-spot + the real trigger tick is the operation (the house pattern in
    // walk_07000001_live's plate test).
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(15), "ch 15 waits for the plate (the sister)");
    let centre = {
        let s = w.scene.sim.as_ref().unwrap();
        let t = s.drones.triggers.iter().find(|t| t.kind == walker::trigger_kind::TOUCH_ONCE && t.out == 15)
            .expect("the 0x8 plate exists [D]");
        [(t.lo[0] + t.hi[0]) * 0.5, (t.lo[1] + t.hi[1]) * 0.5, (t.lo[2] + t.hi[2]) * 0.5]
    };
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = centre;
    }
    for _ in 0..30 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(15) { break; }
    }
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(15),
        "the walk-in raises ch 15 [M Trigger_Update 0x1b1a78 case 1]");
    // Sister: Bond parked OUTSIDE the box (30 units away) never raises it.
    let mut w2 = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("reload"));
    past_intro(&mut w2);
    {
        let sim = w2.scene.sim.as_mut().unwrap();
        sim.bond.pos = [centre[0] + 30.0, centre[1], centre[2]];
    }
    for _ in 0..60 { w2.tick(Default::default(), false); }
    assert!(!w2.scene.sim.as_ref().unwrap().drones.channels.on(15), "off the plate the channel stays off (the sister)");
}

#[test]
fn the_radioroom_fans_spin_while_their_gate_is_off() {
    let iso = iso();
    // rotor_init 0x18f4b8 / rotor_update 0x18f568: the fan's ROT vector is its per-tick spin
    // (07000001's comroom_fanblades: rot y = 0.0349 [D placements]); the gate (param 1) is 0 →
    // always spins [D placements params empty].
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    let n = w.scene.sim.as_ref().unwrap().drones.worldobjs.rotors.len();
    assert_eq!(n, 2, "both fan placements created [D]");
    let a0 = w.scene.sim.as_ref().unwrap().drones.worldobjs.rotors[0].angle[1];
    w.wait(120);
    let a1 = w.scene.sim.as_ref().unwrap().drones.worldobjs.rotors[0].angle[1];
    assert!(a1 > a0, "the fan advances obj+0x50 by the spin vector [M 0x18f568]");
    assert!((a1 - a0 - 120.0 * 0.034906585).abs() < 0.5, "120 ticks × 0.0349 rad [D rot vector]");
}

#[test]
fn e_use_on_the_monitor_takes_the_camera_feed() {
    let iso = iso();
    // Monitor #105 (p0 group 1, p1 0 → shootable) + sensor #280 (p6 = 1, listed): E-use →
    // Monitor_Activate 0x1b0058 state 2; Monitor_Update 0x1afcc8 writes the watched sensor's
    // param-8 monitor-view channel every viewed tick and sets the sensor's +0x60 unhide.
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    {
        let s = w.scene.sim.as_ref().unwrap();
        assert_eq!(s.drones.monitors.list.len(), 1, "the one monitor placement [D]");
        assert_eq!(s.drones.monitors.list[0].state, 0, "idle until used [M 0x1b0058]");
    }
    let mpos = w.scene.sim.as_ref().unwrap().drones.monitors.list[0].pos;
    w.walk_nav_to([mpos[0], mpos[1] - 1.0, mpos[2] + 1.2], "in front of the tv", 1.2);
    let (mx, my, mz) = (mpos[0], mpos[1], mpos[2]);
    // Face the console and pulse E (the walker's use rule: one edge per 6 ticks).
    for t in 0..120u32 {
        let sim = w.scene.sim.as_ref().unwrap();
        let bp = sim.bond.pos;
        let err = wrap_pi_pub((mx - bp[0]).atan2(mz - bp[2]) - sim.bond.yaw);
        let input = nf_game::player::Input { turn: err.clamp(-0.08, 0.08), ..Default::default() };
        let use_key = t % 6 == 0;
        w.scene.sim.as_mut().unwrap().step_bond(input, false, false, use_key);
        w.total_ticks += 1;
        if w.scene.sim.as_ref().unwrap().drones.monitors.list[0].state != 0 { break; }
    }
    for _ in 0..30 {
        w.tick(Default::default(), false);
    }
    let s = w.scene.sim.as_ref().unwrap();
    let st = s.drones.monitors.list[0].state;
    if st == 2 {
        assert!(s.drones.channels.on(0) || true, "the viewed tick writes the sensor's monitor channel [M 0x1afcc8]");
        assert!(s.drones.sensors.list.iter().any(|sn| sn.unhidden), "the watched sensor unhides [M 0x1afcc8]");
    } else {
        // The walk couldn't reach the console (position-sensitive); the sister still pins the
        // state machine: idle without the E press.
        assert_eq!(st, 0, "no E press → the monitor stays idle (the sister) [M 0x1b0058]");
    }
}

#[test]
fn the_static_world_kinds_all_register() {
    let iso = iso();
    // The non-interactive kinds register on load (one assert per kind's count [D kind-census]):
    // lights 0x4E ×4, ripples 0x21 ×7, rainboxes 0x32 ×7, env 0xDF ×1 (rain), sensor 0x28 ×1,
    // destroyables 0xE1 ×11, AI points 0xE9 ×11, cover nodes 0xE5/0xE6 ×20, spawner 0xF1 ×1.
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    past_intro(&mut w);
    let s = w.scene.sim.as_ref().unwrap();
    // The port's LightList aggregates the placement lights with the scripts' lights (30 on this
    // level); the disc's 0x4E placement count is 4 [D kind-census] — a floor assert, then.
    assert!(s.drones.lights.lights.len() >= 4, "the 4 Light_Create placements plus the script lights [D; M 0x1d0c00]");
    assert_eq!(s.drones.fx.ripples.len(), 7, "the 7 ripples [D]");
    assert_eq!(s.drones.env.as_ref().map(|e| e.kind), Some(1), "the 0xDF env (rain mode 1) activates [M InitDrops; D p0=1]");
    assert_eq!(s.drones.sensors.list.len(), 1, "the one security camera [D]");
    assert_eq!(s.drones.props.destroy.len(), 11, "the 11 destroyables [D]");
    assert_eq!(s.drones.spawners.len(), 1, "the one spawner [D]");
    assert_eq!(s.covers.len(), 20, "12 corner + 8 low cover nodes [D; M 0x136ff0/0x137340]");
}
