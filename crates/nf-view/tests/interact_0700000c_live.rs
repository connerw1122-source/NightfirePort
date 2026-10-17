//! The 0700000c interactables: one live test per interactive KIND (task level-0c-interactables),
//! each through the real input path (Bond walks/aims/presses E/shoots/grapples/rides), each with
//! a sister that fails without the operation. Every game claim cites its evidence: [M] the
//! decompile function and address, [D] the disc parse, [L] a live PCSX2 value. The census:
//! docs/levels/0700000c.md §7. Kinds already pinned green on this level elsewhere: the SS sweep +
//! the Lock burns + the AND mux (`fix_level_0c_live`), the Touch plate → objective 1 + the
//! CamSubject photograph + the LoadLevel gate (`walk_0700000c_live`), the yard-ladder climb
//! (`walkreal_0700000c_live` leg 5), the guards/spawners (guards-0c/-b).

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

/// Fire at an arbitrary world point through the real gun path (aims with turn/look, taps only
/// when the view is on the point, reloads when the clip empties).
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

/// Load, wait for the intro to release the controls, settle.
fn ready(iso: &str) -> Walker {
    let mut w = Walker::new(nf_view::load(iso, "0700000c.bin", false).expect("load 0700000c"));
    w.wait_enabled(3000, "the intro");
    w.wait(30);
    w
}

/// Teleport Bond to a spot and let him land (the documented reach stand-in: the tests that need
/// a far spot teleport to it, then operate the thing through the real input only).
fn drop_at(w: &mut Walker, p: [f32; 3]) -> [f32; 3] {
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = p;
        sim.bond.vel = [0.0; 3];
    }
    for _ in 0..200 {
        if w.scene.sim.as_ref().unwrap().bond.grounded { break; }
        w.tick(nf_game::player::Input::default(), false);
    }
    w.scene.sim.as_ref().unwrap().bond.pos
}

#[test]
fn the_static_world_kinds_all_register() {
    let iso = iso();
    let w = ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    // Every count is the disc's [D placements-all dump over 0700000c.bin; the kind→creator
    // switch is parsemap_create_dynamic_objects M 0x1d04f0].
    assert_eq!(sim.drones.doors.list.len(), 2, "doors: door2turbine #281 + lockeddoor #555 [D]");
    // The main map's ladders: #164 (9.67, 5.53, −187.34) + #495 (−6.38, 0.0, −177.55) [D]. The
    // sim's ladder vec also carries the intro-map's ladder (ladder::all scans every map [M
    // ladder::all]), so filter to the main map's two by position.
    let main_ladders = sim.ladders.iter()
        .filter(|l| dxz_pub(l.centre, [9.67, 5.53, -187.34]) < 2.0 || dxz_pub(l.centre, [-6.38, 0.0, -177.55]) < 2.0)
        .count();
    assert_eq!(main_ladders, 2, "the main map's ladders [D placements-all: kind 22 ×2 on map 45]");
    assert_eq!(sim.wires.len(), 4, "wires: three hand-over-hand + the Zipline [D]");
    assert_eq!(sim.drones.grapple_points.len(), 5, "grapple points: graple_pulley ×5 [D]");
    assert_eq!(sim.drones.locks.len(), 5, "locks: hingelock1/3 ×4 + VentLatch #182 [D]");
    assert_eq!(sim.drones.sss.list.len(), 8, "SSs: the two vent meshes + blockers + 3 winches + Door_secret1 [D]");
    assert_eq!(sim.drones.pickups.len(), 15, "pickups [D]");
    assert_eq!(sim.drones.props.destroy.len(), 12, "destroyables: the oil barrels [D]");
    // The twelve kind-0x31s are param-less [D], so FuseBox::create (which needs the script
    // hash in param 0 [M 0x1b0448]) creates none: the kind's 0c instances are inert.
    assert_eq!(sim.drones.fuseboxes.len(), 0, "param-less 0x31s create no fusebox [D; M 0x1b0448]");
    assert_eq!(sim.drones.props.subjects.len(), 2, "cam subjects: #414 + #454 [D]");
    assert_eq!(sim.icons.len(), 15, "third icons [D]");
    assert_eq!(sim.drones.sound_triggers.list.len(), 5, "sound triggers [D]");
    assert_eq!(sim.drones.worldobjs.music_triggers.len(), 2, "music triggers [D]");
    assert_eq!(sim.drones.worldobjs.dynobjs.len(), 5, "dynamic objects [D]");
    assert_eq!(sim.drones.worldobjs.thirdcams.len(), 4, "third cams [D]");
    // The param-less fuseboxes are INERT by disc data: no script hash [D placements], so
    // nothing registers into either wire group [M 0x1b0970 files only script-handed entities]
    // and their channel words are all zero [M 0x1b0448 stores params 1–4; there are none].
    // The port realises that as an empty vec (see the fusebox test).
}

#[test]
fn the_use_key_opens_door2turbine_and_the_sister_keeps_it_shut() {
    let iso = iso();
    // Door #281 "door2turbine" (39.99, 12.57, −210.45), no lock, sounds 1/2 [D placements;
    // docs/fields/door.md]. E toggles it [M Door_Activate via Player_Activate 0x1a5170 case 0x1C].
    let mut w = ready(&iso);
    let di = {
        let sim = w.scene.sim.as_ref().unwrap();
        sim.drones.doors.list.iter().position(|d| dxz_pub(d.pos, [39.99, 12.57, -210.45]) < 2.0).expect("door2turbine in the doors list")
    };
    drop_at(&mut w, [39.99, 13.7, -208.6]);
    let before = w.scene.sim.as_ref().unwrap().drones.doors.list[di].state;
    assert_eq!(before & 0xc, 0, "door2turbine starts closed [D; the start snapshot]");
    w.use_door(di, "door2turbine");
    for _ in 0..90 { w.tick(nf_game::player::Input::default(), false); }
    assert_eq!(
        w.scene.sim.as_ref().unwrap().drones.doors.list[di].state & 0xc,
        nf_game::door::OPEN & 0xc,
        "the E press opens the door [M 0x1a5170 case 0x1C → Door_Activate]"
    );
    // Sister: the same drop, no E — the door never moves.
    let mut w2 = ready(&iso);
    let di2 = {
        let sim = w2.scene.sim.as_ref().unwrap();
        sim.drones.doors.list.iter().position(|d| dxz_pub(d.pos, [39.99, 12.57, -210.45]) < 2.0).unwrap()
    };
    drop_at(&mut w2, [39.99, 13.7, -208.6]);
    for _ in 0..90 { w2.tick(nf_game::player::Input::default(), false); }
    assert_eq!(w2.scene.sim.as_ref().unwrap().drones.doors.list[di2].state & 0xc, 0,
        "no E press → the door stays shut (the sister)");
}

#[test]
fn walking_the_mission_trigger_fires_channel_43_once() {
    let iso = iso();
    // TouchOnce #551 "Mission Trigger_Trigger Once" box [7.78, 15.32, −150.94]..[10.11, 16.26,
    // −148.98] at the spawn walkway, out ch 0x43 [D triggers dump]; its reader is SoundTrigger
    // #550 (id 1516, on ch 0x43) [D]. TouchOnce = Trigger_Update's once latch [M 0x1b1a78].
    let mut w = ready(&iso);
    // Bond spawns just under the plate's slab [D #1 y 15.07 vs the #551 lo 15.32]; the walk
    // south crosses the plate's plane (the real walk-over) and fires it once.
    w.walk_nav_to([8.94, 15.0, -146.8], "the walk across the mission trigger plate", 0.9);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x43),
        "the walk across TouchOnce #551 fires ch 0x43 [D triggers dump; M Trigger_Update 0x1b1a78 case 1]");
    // Sister: the same load carried to the yard (the reach stand-in), where the plate's plane
    // is never crossed — the channel stays off without the walk-over.
    let mut w2 = Walker::new(nf_view::load(&iso, "0700000c.bin", false).expect("reload"));
    {
        let sim = w2.scene.sim.as_mut().unwrap();
        sim.bond.pos = [56.5, 2.8, -176.5];
        sim.bond.vel = [0.0; 3];
    }
    for _ in 0..60 { w2.tick(nf_game::player::Input::default(), false); }
    assert!(!w2.scene.sim.as_ref().unwrap().drones.channels.on(0x43),
        "no walk onto the plate → ch 0x43 stays off (the sister)");
}

#[test]
fn the_armour_pickup_grants_50_on_the_walk_over_and_the_sister_refuses() {
    let iso = iso();
    // Pickup #540 BodyArmourHeavy (9.18, 7.20, −115.66), param 0 = 3, param 2 = 50 [D]: case 3
    // hard-sets armour 50 when below [M Pickup_Handler 0x18e650 case 3].
    let mut w = ready(&iso);
    assert!(w.scene.sim.as_ref().unwrap().drones.health.armour == 0.0, "armour starts 0 [L ee_start]");
    // Pick the armour instance whose rest position has floor right under it (the six heavy
    // armours sit on different storeys [D]; this finds the reachable one).
    let mut spot: Option<([f32; 2], f32)> = None;
    {
        let sim = w.scene.sim.as_ref().unwrap();
        for p in &sim.drones.pickups {
            if p.kind != 3 { continue; }
            let px = p.m[3][0]; let pz = p.m[3][2]; let py = p.m[3][1];
            let floor = sim.world.segment_hit([px, py + 2.0, pz], [px, py - 3.0, pz]);
            eprintln!("[dbg] armour at ({px:.2},{py:.2},{pz:.2}) floor {:?}", floor.map(|(t, _)| py + 2.0 - 5.0 * t));
            if let Some((t, _)) = floor {
                let f = py + 2.0 - 5.0 * t;
                if (f - py).abs() < 0.7 { spot = Some(([px, pz], f)); break; }
            }
        }
    }
    let (p, f) = spot.expect("an armour pickup resting on its floor");
    drop_at(&mut w, [p[0], f + 1.4, p[1] + 1.6]);
    w.walk_nav_to([p[0], f, p[1]], "onto the armour", 0.5);
    for _ in 0..30 { w.tick(nf_game::player::Input::default(), false); }
    eprintln!("[dbg] armour now {}", w.scene.sim.as_ref().unwrap().drones.health.armour);
    assert!(w.scene.sim.as_ref().unwrap().drones.health.armour == 50.0,
        "the walk-over grants armour 50 [M 0x18e650 case 3; D p2=50]");
    // Sister: stop one step short — the pick sphere is never entered, armour stays 0.
    let mut w2 = ready(&iso);
    drop_at(&mut w2, [p[0], f + 1.4, p[1] + 2.6]);
    w2.walk_nav_to([p[0], f, p[1] + 1.8], "short of the armour (sister)", 0.9);
    for _ in 0..30 { w2.tick(nf_game::player::Input::default(), false); }
    assert!(w2.scene.sim.as_ref().unwrap().drones.health.armour == 0.0,
        "not walking into it grants nothing (the sister)");
}

#[test]
fn a_bullet_explodes_the_oil_barrel_and_the_far_barrel_stays_whole() {
    let iso = iso();
    // Destroy #431 "object_oilbarrel01" (13.12, 6.28, −110.50): bullets drain its damage pool
    // [M Destroy_CollisionHandler 0x12bd48:44-48] and the death removes it (smash type 4 → the
    // ×3 blast script 0x6000052 [M Destroy_Smash 0x12bc50]). Fire from the display ground.
    let mut w = ready(&iso);
    drop_at(&mut w, [5.4, 7.4, -113.0]);
    let target = [13.12, 6.9, -110.5];
    let idx = {
        let sim = w.scene.sim.as_ref().unwrap();
        sim.drones.props.destroy.iter().position(|d| dxz_pub([(d.lo[0] + d.hi[0]) / 2.0, (d.lo[1] + d.hi[1]) / 2.0, (d.lo[2] + d.hi[2]) / 2.0], target) < 1.5).expect("barrel #431 in the destroy list")
    };
    face_and_fire(&mut w, target.into(), "the oil barrel");
    for _ in 0..60 { w.tick(nf_game::player::Input::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.props.destroy[idx].dead, "the shot barrel dies [M 0x12bd48]");
    // Sister: the warehouse barrel #434 (3.94, 11.28, −132.07), five floors away from the
    // blast, is untouched.
    let far = sim.drones.props.destroy.iter().position(|d| dxz_pub([(d.lo[0] + d.hi[0]) / 2.0, (d.lo[1] + d.hi[1]) / 2.0, (d.lo[2] + d.hi[2]) / 2.0], [3.94, 11.28, -132.07]) < 1.5).expect("barrel #434");
    assert!(!sim.drones.props.destroy[far].dead, "the far barrel stands (the sister)");
}

#[test]
fn the_paramless_fuseboxes_take_no_use_and_the_door_takes_it() {
    let iso = iso();
    // The port's FuseBox::create takes only the 0x31 placements that carry a script hash in
    // param 0 [M 0x1b0448's SP_Create(param_3+0x2c, …); fusebox.rs header] — on 0c all twelve
    // are param-less [D placements-all], so the vec is empty: no wire groups, no channel writer
    // exists for the kind here. The E key near a barrel twin writes no channel either [M
    // Player_Activate 0x1a5170:95-160 has no class-0x3C arm]. The control: the SAME press on the
    // door opens it (the door test pins that arm).
    let mut w = ready(&iso);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.fuseboxes.is_empty(),
            "no fusebox creates without a script hash [D: all 12 param-less; M 0x1b0448]");
    }
    let fb = [13.12, 6.28, -110.5]; // the oil-barrel twin spot [D #431/#432]
    drop_at(&mut w, [fb[0], fb[1] + 1.2, fb[2] + 1.4]);
    let snap: Vec<u8> = {
        let sim = w.scene.sim.as_ref().unwrap();
        sim.drones.channels.v.clone()
    };
    for k in 0..30 {
        let press = k % 6 == 0;
        w.tick(nf_game::player::Input::default(), press);
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.channels.v, snap,
        "E at a param-less fusebox spot writes no channel [M 0x1a5170 has no 0x3C arm; D no params]");
}

#[test]
fn the_zipline_grabs_on_the_action_key_and_carries_bond() {
    let iso = iso();
    // Zipline #512 (46.79, 17.85, −195.26) p0=10 p1=2 [D]. The grab is the ACTION edge scanning
    // for the nearest class-0x2C object [M Player_HandleJump 0x1916b0:126-147]; the ride holds
    // the wire's switch channel p1 and moves Bond along the segment [M Player_Wire 0x1976bc/
    // 0x1978c4; the walker's mode-7 ride].
    let mut w = ready(&iso);
    let grab = {
        let sim = w.scene.sim.as_ref().unwrap();
        let zi = sim.wires.iter().position(|x| dxz_pub(x.centre, [46.79, 17.85, -195.26]) < 2.0).expect("the zipline in the wires list");
        assert_eq!(sim.wires[zi].wire_type, 2, "the zipline is wire_type 2 [D p1=2; M Player_CollWire 0x197130:99-106]");
        sim.wires[zi].grab_segment()
    };
    eprintln!("[dbg] zipline grab segment {:?} -> {:?}", grab.0, grab.1);
    // The real approach is the walkway at the grab end; find the floor under the grab point and
    // stand Bond on it (the reach stand-in), then the action-key grab [M 0x1916b0:126-147].
    let fy = {
        let sim = w.scene.sim.as_ref().unwrap();
        let f = sim.world.segment_hit([grab.0[0], grab.0[1] + 4.0, grab.0[2]], [grab.0[0], grab.0[1] - 30.0, grab.0[2]]);
        eprintln!("[dbg] floor under grab: {:?}", f.map(|(t, _)| grab.0[1] + 4.0 - 34.0 * t));
        f.map(|(t, _)| grab.0[1] + 4.0 - 34.0 * t).unwrap_or(grab.0[1] - 1.1)
    };
    let start = [grab.0[0], fy + 1.3, grab.0[2]];
    drop_at(&mut w, start);
    // The grab needs the wire icon's context 3 [M 0x1916b0:126-147 reads BLData+0x95F] — the
    // ThirdIcon #509 (46.83, 21.43, −205.49, p0=3 [D]) arms it in range; the roof spot arms it.
    for _ in 0..30 { w.tick(nf_game::player::Input::default(), false); }
    assert_eq!(w.scene.sim.as_ref().unwrap().bond.icon_ctx, 3,
        "the zipline icon arms the wire context [D #509 p0=3; M 0x1916b0:126-147]");
    // Pulse the action key until the grab (the jump-scan takes the nearest class-0x2C [M]),
    // then ride: strafe carries Bond along the segment [M Player_Zipline 0x19806c].
    let mut grabbed = false;
    for t in 0..600u32 {
        let riding = {
            let s = w.scene.sim.as_ref().unwrap().bond.substate;
            s == nf_game::player::SS_WIRE || s == nf_game::player::SS_ZIPLINE
        };
        if riding { grabbed = true; break; }
        w.tick(nf_game::player::Input { forward: 0.2, jump: t % 6 == 0, ..Default::default() }, false);
    }
    assert!(grabbed, "the action key grabs the zipline [M 0x1916b0:126-147]");
    for _ in 0..120 { w.tick(nf_game::player::Input { strafe: 1.0, ..Default::default() }, false); }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(
        sim.bond.substate == nf_game::player::SS_WIRE || sim.bond.substate == nf_game::player::SS_ZIPLINE,
        "Bond is riding after the action grab [M 0x1916b0:126-147]"
    );
    // The ride moved him along the segment toward its B end [M Player_Zipline 0x19806c].
    let along = (sim.bond.pos[0] - grab.0[0]) * (grab.1[0] - grab.0[0])
        + (sim.bond.pos[2] - grab.0[2]) * (grab.1[2] - grab.0[2]);
    assert!(along > 0.5, "the ride progressed along the zipline (got along {along:.2}, pos {:?})", sim.bond.pos);
    // Sister: the same drop, no action key — no grab, no channel.
    let mut w2 = ready(&iso);
    drop_at(&mut w2, start);
    for _ in 0..120 { w2.tick(nf_game::player::Input::default(), false); }
    let sim2 = w2.scene.sim.as_ref().unwrap();
    assert!(sim2.bond.substate != nf_game::player::SS_WIRE && sim2.bond.substate != nf_game::player::SS_ZIPLINE,
        "no action key → no grab (the sister)");
    assert_eq!(sim2.wire_channel, 0, "no ride → no wire channel (the sister)");
}

#[test]
fn the_q_claw_grapple_latches_the_pulley_and_reels() {
    let iso = iso();
    // Grapple point #500 graple_pulley (−4.60, 13.16, −189.71) [D], class 0x51 [M 0x198298].
    // The Q-Claw (0x50) is in the 0c loadout [M 0x1a7ef8 case 0x700000c]; the fire latches the
    // aimed point and Bond reels [M Player_SetGrapplePoint 0x195da0; scene.rs's fire gate].
    let mut w = ready(&iso);
    // The player-real switch: the Input weapon select [M the switch request writes obj+0x63].
    for _ in 0..40 { w.tick(nf_game::player::Input { switch_to: Some(0x50), ..Default::default() }, false); }
    assert_eq!(w.scene.sim.as_ref().unwrap().gun.weapon, 0x50, "the Q-Claw is held [M 0x1a7ef8 loadout; D the 0c archive has the 0x50 view model]");
    let point = [-4.60, 13.16, -189.71];
    let mut latched = false;
    for t in 0..600u32 {
        let sim = w.scene.sim.as_ref().unwrap();
        let bp = sim.bond.pos;
        let desired = (point[0] - bp[0]).atan2(point[2] - bp[2]);
        let err = wrap_pi_pub(desired - sim.bond.yaw);
        let dy = point[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
        let want_pitch = (dy / dxz_pub(bp, point.into()).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
        let input = nf_game::player::Input { turn: err.clamp(-0.08, 0.08), look: (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08), ..Default::default() };
        let on = err.abs() < 0.02 && (want_pitch - sim.bond.pitch).abs() < 0.03;
        w.scene.sim.as_mut().unwrap().step_bond(input, on && t % 24 == 0, false, false);
        w.total_ticks += 1;
        if w.scene.sim.as_ref().unwrap().bond.grapple.is_some() { latched = true; break; }
    }
    assert!(latched, "the aimed fire latches the grapple point [M 0x195da0]");
    // The reel runs while held: phase 1 advances pull 0.05/tick with no fire test [M
    // 0x192860:79-121]; at the point the hang (phase 2) releases on the dropped fire [M
    // 0x192860:64-66], so the observation window is the first held ticks.
    for _ in 0..6 { w.tick(nf_game::player::Input::default(), true); }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.bond.grapple.is_some() && sim.bond.grapple.as_ref().unwrap().pull > 0.0,
        "the reel pulls [M 0x192860:79-104]");
    // Sister: the same aim, no fire → never latches.
    let mut w2 = ready(&iso);
    drop_at(&mut w2, [9.7, 12.9, -187.3]);
    for _ in 0..40 { w2.tick(nf_game::player::Input { switch_to: Some(0x50), ..Default::default() }, false); }
    for _ in 0..120 { w2.tick(nf_game::player::Input::default(), false); }
    assert!(w2.scene.sim.as_ref().unwrap().bond.grapple.is_none(), "no fire → no latch (the sister)");
}

#[test]
fn the_music_trigger_fires_its_event_while_bond_is_in_its_zone() {
    let iso = iso();
    // MusicTrigger #502 (3.0, 0.0, −238.38) p0=36 p2=13 [D]: while switch ch 0x24 is on it fires
    // Music_Event(13, 2) [M MusicTrigger_Update 0x1b31a0:19-27]. Ch 0x24 is written by the big
    // player-touch volumes #501/#503/#504 [D triggers dump]. The zone needs y 9.65..16.88 [D
    // #501 box], so the test probes the turbine-hall walkway floor, drops Bond into the zone,
    // and asserts the event board slot received the write.
    let mut w = ready(&iso);
    // The trigger's own game state is the re-arm countdown (ext+8: 0x1e armed, drained per tick
    // [M 0x1b31a0:15-27]); the event write it drives is the per-frame mailbox the music updater
    // consumes the same pass [M music.rs's updater clears the board each frame].
    let candidates: [[f32; 3]; 4] = [[36.0, 13.2, -215.0], [30.0, 13.2, -218.0], [36.0, 13.2, -211.5], [20.0, 13.2, -212.0]];
    let mut armed: Option<(usize, i16)> = None;
    for (ci, c) in candidates.iter().enumerate() {
        let p = drop_at(&mut w, *c);
        if p[1] < 10.5 { continue; } // fell out of the zone's y band — try the next hall spot
        for _ in 0..30 { w.tick(nf_game::player::Input::default(), false); }
        let sim = w.scene.sim.as_ref().unwrap();
        let t = sim.drones.worldobjs.music_triggers.iter().position(|t| t.event_id == 13).expect("music trigger #502 (event 13) [D p2=13]");
        let cd = sim.drones.worldobjs.music_triggers[t].countdown;
        eprintln!("[dbg] candidate {ci} landed {:?} countdown {cd}", p);
        if cd > 0 { armed = Some((t, cd)); break; }
    }
    let (ti, cd) = armed.expect("standing in the ch-0x24 zone re-arms trigger #502 [M 0x1b31a0:19-27]");
    assert!(cd > 0 && cd <= 30, "countdown armed [M 0x1b31a0:20]");
    // The event write the countdown drives [M 0x1b31a0:23 music.event(event_id, 2)].
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.worldobjs.music_triggers[ti].event_id, 13, "[D #502 p2=13]");
    // Sister: a fresh load that never enters the zone keeps the countdown drained.
    let mut w2 = ready(&iso);
    for _ in 0..90 { w2.tick(nf_game::player::Input::default(), false); }
    assert!(w2.scene.sim.as_ref().unwrap().drones.worldobjs.music_triggers.iter().find(|t| t.event_id == 13).map(|t| t.countdown).unwrap_or(0) == 0,
        "out of the zone → no re-arm (the sister)");
}
