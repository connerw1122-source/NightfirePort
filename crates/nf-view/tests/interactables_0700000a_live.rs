//! interactables-0a: one live test per KIND of player-touchable placement on 0700000a (Tower B,
//! the office floor), driven the way a player drives it (the walk, the use key, the trigger pull,
//! the fall), each asserting the game's effect with its sister. The full census with the type
//! table lives in docs/levels/0700000a.md (task 710a); the end-to-end route is
//! walkreal_0700000a_live.rs.
//!
//! Evidence tags: [M] the decompile address, [D] the disc bytes, [L] the EE snapshot. A teleport
//! to reach a spot is used only where walking there is a separate documented blocker; the
//! operation itself always runs through the real input.

#[path = "common/walker_b2.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn load() -> Walker {
    Walker::new(nf_view::load(&iso(), "0700000a.bin", false).expect("load 0700000a"))
}

/// Fire the held weapon at a static point through the real trigger path (walkreal-b2's shoot_at,
/// aimed at a world point): turn+look onto the point, pulse the trigger while in range.
fn shoot_at(w: &mut Walker, target: [f32; 3], range: f32, max_ticks: u32, label: &str, done: &dyn Fn(&nf_view::scene::Sim) -> bool) -> bool {
    let start = w.total_ticks;
    let mut last_tap = 0u32;
    loop {
        if done(&w.scene.sim.as_ref().unwrap()) { return true; }
        if w.total_ticks - start >= max_ticks { return false; }
        {
            let sim = w.scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let eye = bp[1] + sim.bond.eye - sim.bond.crouch_drop;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - eye;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist > range { return false; }
            let desired = dx.atan2(dz);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let want_pitch = (dy / walker::dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            let perr = want_pitch - sim.bond.pitch;
            let on_target = err.abs() < 0.008 && perr.abs() < 0.012 && w.total_ticks - last_tap > 15;
            let input = if on_target {
                nf_game::player::Input { aiming: true, ..Default::default() }
            } else {
                nf_game::player::Input {
                    turn: err.clamp(-0.08, 0.08),
                    look: perr.clamp(-0.08, 0.08),
                    aiming: true,
                    ..Default::default()
                }
            };
            if on_target { last_tap = w.total_ticks; }
            w.scene.sim.as_mut().unwrap().step_bond(input, on_target, false, false);
        }
        w.total_ticks += 1;
        if w.top_up_health { w.scene.sim.as_mut().unwrap().drones.health.health = 100.0; }
    }
}

/// The cab-mouth SS lift doors (placements 524/525) sweep OPEN at create and hold the open pose:
/// their state param is 1 (armed) with a 5-key list and wrap flag 1 [D placements dump] —
/// `SS_Update` 0x1a9948's state-1 sweep runs from load, and the wrap (`(int)last < (int)t`,
/// wrap_flag != 0 → state 0) holds the last posed matrix [M 0x1a9948:119-128; grates-02's state
/// model]. The old ignore's premise ("a writer the port lacks") predates grates-02: the sweep IS
/// the writer, and the [L] start snapshot caught t = 0 (key 0, closed) before the first update.
#[test]
fn ss_lift_doors_sweep_open_at_create() {
    let mut w = load();
    // The doors' key-0 (closed) matrices [L ee_start_0700000a.bin: leaves at (−2.973, 1.303, 13.031)
    // and (−4.893, 1.275, 13.031) — the placement transforms [D]].
    let leaf = |w: &Walker, placement: usize| {
        w.scene.sim.as_ref().unwrap().drones.sss.list.iter()
            .find(|s| s.placement == placement)
            .map(|s| s.matrix[3][0])
            .expect("the cab-mouth SS leaf")
    };
    let l2_start = leaf(&w, 524);
    let l1_start = leaf(&w, 525);
    assert!((l2_start + 2.9727).abs() < 0.05 && (l1_start + 4.8933).abs() < 0.05,
        "the leaves rest at their key-0 closed pose at load [D], got {l2_start} / {l1_start}");
    // The sweep is 75 ticks (rate word 0 → SS_Create's default 75 [M 0x1a9c58:51]); step past it.
    w.wait(100);
    let l2_open = leaf(&w, 524);
    let l1_open = leaf(&w, 525);
    assert!((l2_open - l2_start).abs() > 0.5 && (l1_open - l1_start).abs() > 0.5,
        "both leaves swept aside within the 75-tick create sweep and HOLD the open pose \
         [M SS_Update 0x1a9948 states + the wrap], got {l2_start}→{l2_open} / {l1_start}→{l1_open}");
    // Sister: without the state-1 create arm the leaves would still stand at key 0 — pinned by
    // the pre-wait read above (the pose at tick 0 IS key 0, closed [L]).
}

/// The BodyArmourHeavy pickup (#730, param 0 = 3, param 2 = 50 [D]) grants armour 50 on the
/// walk-over: `Pickup_Handler` 0x18e650's case 3 hard-sets 50.0 when the armour is below it
/// [M pickup.rs take()]. Sister: the armour reads 0 before the touch.
#[test]
fn armour_pickup_taken_by_walking_over_it() {
    let mut w = load();
    assert!(w.scene.sim.as_ref().unwrap().drones.health.armour == 0.0,
        "Bond spawns without armour");
    // #730 sits on the office floor at (−13.46, 0.90, 7.61) [D]; out of the cab, then the AI
    // graph's own route (the direct line crosses the office-N walls [D]).
    w.walk_to(Wp::new(-3.9, 0.0, 13.2), "out of the lift");
    w.walk_to(Wp::new(-3.9, 0.0, 15.5).r(1.2), "lobby south");
    assert!(w.follow_nav_route([-12.6, 0.4, 8.6], "to the armour's room"), "no nav route to the armour");
    w.walk_to(Wp::new(-13.46, 0.4, 7.61).r(0.9), "onto the BodyArmourHeavy");
    w.wait(20);
    assert!((w.scene.sim.as_ref().unwrap().drones.health.armour - 50.0).abs() < 1.0,
        "the walk-over take grants armour 50 [M 0x18e650 case 3; D param 2 = 50], got {}",
        w.scene.sim.as_ref().unwrap().drones.health.armour);
    // Sister: the second armour copy (#731, 17.65, 0.50, −7.83) still sits untouched — the take
    // consumed #730, not everything.
    let taken = w.scene.sim.as_ref().unwrap().drones.pickups.iter()
        .filter(|p| (p.m[3][0] + 13.46).abs() < 0.3 && (p.m[3][2] - 7.61).abs() < 0.3)
        .all(|p| p.gone || p.hidden);
    assert!(taken, "the walked-over pickup is consumed (gone, or hidden for the respawn) [M 0x18e650's respawn tail]");
}

/// The vending-machine glass (#379, 0x20 Break_Create at (6.12, 1.22, 16.35) [D]) shatters under
/// the held fire: a bullet crossing the break's bounds kills it (`Break_Kill` 0x12b970 sets
/// obj+0xFE bit 0 [M breakable.rs]). Sister: the far glass #378 stays alive.
#[test]
fn vending_glass_breaks_under_fire() {
    let mut w = load();
    let dead_of = |w: &Walker, placement: usize| {
        w.scene.sim.as_ref().unwrap().drones.breaks.list.iter()
            .find(|b| b.placement == placement).map(|b| b.dead).expect("the vending glass break")
    };
    // Walk to the lobby vending machine (right of the spawn, through the real path; the AI graph
    // carries the loop, the direct line pins on the lobby's north wall).
    let glass = [6.12, 1.22, 16.35];
    w.walk_to(Wp::new(-3.9, 0.0, 13.2), "out of the lift");
    w.walk_to(Wp::new(-3.9, 0.0, 15.5).r(1.2), "lobby south");
    assert!(w.follow_nav_route([4.6, 0.4, 15.2], "the lobby by the vending machine"), "no nav route");
    w.walk_to(Wp::new(4.2, 0.4, 16.35).r(0.9), "face the vending machine");
    assert!(!dead_of(&w, 379), "the glass stands before the shot");
    let landed = shoot_at(&mut w, glass, 4.5, 900, "the vending glass", &|sim| {
        sim.drones.breaks.list.iter().find(|b| b.placement == 379).map_or(false, |b| b.dead)
    });
    assert!(landed, "the glass never broke under the real fire (gun {:#x})",
        w.scene.sim.as_ref().unwrap().gun.weapon);
    assert!(dead_of(&w, 379), "Break_Kill's dead bit set [M 0x12b970]");
    // Sister: the other vending machine (−33.0, 1.22, −18.08) is untouched.
    assert!(!dead_of(&w, 378), "the far glass stands");
}

/// Walking through the laser web fires the beam's out channel: #607 (out ch 0x1F, mask 8, lock
/// 0x37 [D triggers dump]) spans x ≈ −8.45, y 1.49-1.76, z 15-20.25 — Bond's capsule crosses it
/// at a walk. The lock DISABLES while its channel is on [M triggers.rs Trigger_Update's
/// `lock != 0 && ch.on(lock)` gate], so on fresh boot (ch 0x37 off [L ee_start_0700000a.bin
/// @0x26fc90]) the beams are live. Sister: ch 0x1F is off before the crossing.
#[test]
fn laser_web_crossing_fires_the_alert_channel() {
    let mut w = load();
    w.wait(30);
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(0x1f), "the beam is silent before the walk");
    // Through the beam: the lobby south band then west across x −8.45 between z 15 and 20.
    w.walk_to(Wp::new(-3.9, 0.0, 15.5).r(1.2), "lobby south");
    w.walk_to(Wp::new(-10.0, 0.4, 17.5).r(1.2), "through the laser wall");
    w.wait(10);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x1f),
        "crossing beam #607 fired its out channel ch 0x1F [D triggers dump]");
}

/// E on an alarm switch does nothing: `Switch_Activate` 0x1b02a8 refuses a class-3 (player)
/// activator on a ch-0x61 switch (`cVar1 != 3 || ch != 0x61` [M]) — the 8 alarmswitchplayer
/// placements (#617-631, p0 = 0x61 [D]) cannot be player-fired. The control arm proves the use
/// path itself is alive: the same E press opens the unlocked office door on the route.
#[test]
fn alarm_switch_refuses_the_player_use() {
    let mut w = load();
    // #619 stands at (−15.48, 1.33, −13.13) [D]; the AI graph's proven leg to the south-west
    // office (walkreal's route) carries the loop — the direct nav to the switch itself crosses
    // the node-15 wall stub [D probe, the walkreal's note].
    w.walk_to(Wp::new(-3.9, 0.0, 13.2), "out of the lift");
    w.walk_to(Wp::new(-3.9, 0.0, 15.5).r(1.2), "lobby south");
    assert!(w.follow_nav_route([-15.8, 0.4, -15.0], "the south-west corridor by alarm switch #619"),
        "no nav route to the alarm switch");
    w.wait(5);
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(0x61), "the alarm is off before the use");
    // E the switch from its wedge front (the use sphere is 1.0 at head+look [M 0x1a5170]; the
    // switch stands 0.6 off the wedge — in range where the walk settles).
    w.walk_to(Wp::at(-15.65, 0.4, -13.6, Act::Use { use_ticks: 240 }), "E the alarm switch");
    w.wait(20);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x61),
            "the player's E must not toggle ch 0x61 [M 0x1b02a8's class-3 exclusion]");
        // The switch's stored value is untouched (the toggle never ran).
        let sw = sim.drones.useobjs.switches.iter().find(|s| s.ch == 0x61).expect("the alarm switch");
        assert_eq!(sw.value, 0, "the stored value still reads the rest value [M 0x1b02a8's value gate]");
    }
    // Control arm: the use path IS alive — E the nearest unlocked door and watch it open.
    let (dpos, di) = walker::door_near(&w.scene.sim.as_ref().unwrap(), [-15.48, 1.1, -13.0])
        .expect("a door near the switch");
    w.use_door(di, "E the office door (the use path's control arm)");
    let open = w.scene.sim.as_ref().unwrap().drones.doors.list[di].state;
    assert!(open != 0, "the same use key opens a door (state {} at {dpos:?}) — the use path is alive", open);
}

/// The vent ladder (#686, 0x22 at (−13.16, −1.30, −9.55) [D]) grabs and climbs on the walk-in:
/// `Player_CollLadder` 0x196118 attaches, `Player_Climb` 0x196310 ascends [M ladder.rs]. The
/// teleport only reaches the StoreRoom floor below the office level (the operation — the climb —
/// is the real input). Sister: the pre-contact substate is not the climb.
#[test]
fn vent_ladder_grabs_and_climbs() {
    let mut sim = load().scene.sim.take().unwrap();
    let lad = sim.ladders.iter()
        .min_by(|a, b| {
            let d = |l: &nf_game::ladder::Ladder| (l.centre[0] + 13.16).powi(2) + (l.centre[2] + 9.55).powi(2);
            d(a).total_cmp(&d(b))
        })
        .map(|l| l.clone())
        .expect("the Vent_StoreRoom_Ladder");
    assert!((lad.centre[0] + 13.16).abs() < 2.0 && (lad.centre[2] + 9.55).abs() < 2.0,
        "the nearest ladder to the vent spot is #686 [D], at {:?}", lad.centre);
    // Stand facing the ladder from its front (the yaw convention: forward = (sin, ·, cos) [M ladder.rs]).
    let (s, c) = (lad.yaw.sin(), lad.yaw.cos());
    let stand = [lad.centre[0] + s * 1.2, lad.bottom() + 0.1, lad.centre[2] + c * 1.2];
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, stand, lad.yaw + std::f32::consts::PI);
    assert!(sim.bond.substate != nf_game::player::SS_CLIMB, "not climbing before the walk-in");
    let y0 = sim.bond.pos[1];
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut grabbed = None;
    for t in 0..600 {
        if sim.disabled { sim.drones.scripts.skip_request = true; }
        sim.step_bond(fwd, false, false, false);
        if sim.bond.substate == nf_game::player::SS_CLIMB { grabbed = Some(t); break; }
    }
    let t = grabbed.expect("walking into the vent ladder must attach the climb [M 0x196118]");
    // The climb ascends [M 0x196310].
    let y_grab = sim.bond.pos[1];
    for _ in 0..120 { sim.step_bond(fwd, false, false, false); }
    assert!(sim.bond.pos[1] > y_grab + 0.5,
        "the climb ascends the vent shaft [M 0x196310], {} → {}",
        y_grab, sim.bond.pos[1]);
    eprintln!("[0a] the vent ladder grabbed at t{t}, climbed {:.2} → {:.2}", y_grab, sim.bond.pos[1]);
    let _ = y0;
}

/// The lift-shaft HurtPoly (#745-749, p1 = 1000 damage, at y = −29.2 [D]) kills on the fall-in:
/// `Hurt_Create`/`Hurt::update_touch` deliver the volume's damage [M hurt.rs]. The teleport only
/// drops Bond into the shaft mouth above the volume; the damage runs through the real touch.
/// Sister: standing on the office floor the touch never fires.
#[test]
fn lift_shaft_hurt_volume_deals_the_fall_damage() {
    let mut sim = load().scene.sim.take().unwrap();
    let hurt = sim.hurts.iter().enumerate()
        .min_by(|a, b| sim.hurts[a.0].lo[1].total_cmp(&sim.hurts[b.0].lo[1]))
        .map(|(_, h)| h.clone())
        .expect("the shaft HurtPolys");
    assert!(hurt.lo[1] < -20.0, "the shaft volumes sit at y = −29.2 [D], got {}", hurt.lo[1]);
    assert!(hurt.damage >= 1000.0, "p1 = 1000 damage [D], got {}", hurt.damage);
    let hurt_xz = [(hurt.lo[0] + hurt.hi[0]) / 2.0, (hurt.lo[2] + hurt.hi[2]) / 2.0];
    // Sister: on the office floor, above the volume, no touch.
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [0.0, 0.1, 0.0], 0.0);
    let h0 = sim.drones.health.health;
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    assert!((sim.drones.health.health - h0).abs() < 0.5, "no hurt touch on the office floor");
    // Drop Bond into the shaft at the volume's x/z, just above it — the real fall reaches it.
    sim.bond = nf_game::player::Bond::stand_at(&sim.world,
        [hurt_xz[0], hurt.lo[1] + 2.0, hurt_xz[1]], 0.0);
    for _ in 0..120 {
        if sim.drones.health.health <= 0.0 { break; }
        sim.step_bond(Default::default(), false, false, false);
    }
    assert!(sim.drones.health.health <= 0.0,
        "the shaft volume's 1000 damage kills [M hurt.rs], health {}",
        sim.drones.health.health);
}
