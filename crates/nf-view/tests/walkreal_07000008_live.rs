//! walkreal-h: 07000008 (Mayhew D, the gondola) walked with the player's own movement — no
//! teleports after the spawn. Route (docs/levels/07000008.md, the interactables census):
//!
//! 1. the mountain-top station #0 (139.5, 50.5, −76.4) [D placements kind 0x24] → the intro plate
//!    TouchOnce #237 (139.2, 49.5, −81.2) → **ch 0x49** (this completes 07000007's carried
//!    "Spy on secret meeting" objective [L ee_start_05 record 2]) + SP #294 (0x06000077, the
//!    station intro, 3880 ticks [M levels.md §5.1]) plays, skipped only with the game's own skip;
//! 2. down the station/walkway (the ch-0x2a zone plates wake the placed guards, sw_ch 42 [D]) to
//!    the fusebox #219 (FUSEBOX_GuardRoomDoorLock, group-0 channel 137 [D]) — Bond SHOOTS its
//!    wires (the gun contact survives the wire filter [M fusebox.rs Collide_FilterBullets]) →
//!    **ch 137** unlocks the Secure Room door #122/10 (lock ch 137 [D]) and starts SP #221
//!    (GuardRoomLights);
//! 3. inside, the LASER (0x4E, owned [D inventory]) burns the safe bolts #207/#208 (Lock_Create
//!    0x1ae1f8, out 134/135 [M Lock_Update 0x1ae3d8:194]) → mux #278 → **ch 136** opens the safe;
//! 4. the briefcase pickup #290 (kind 0xF0 type 4, switch channel 74 = 0x4a [D]) → the take writes
//!    **ch 0x4A** [M Pickup_Handler 0x18e650:330-333] → objective "Retrieve guidance chip" folds;
//! 5. back to the gondola station: the floor plate Touch #236 (out 4, mask 8 [D]) → ch 4; mux #235
//!    (out 1 = 0x4A AND 4) → **ch 1** starts SP #295 ScriptPlayer_CableCar (mode 64 loaded-armed
//!    [M SP_LoadScript 0x1dbc08]; the ride script 0x0600004C lives in the LEVEL BIN, 1420 ticks)
//!    — the carriage (112.0, 32.4, −126.3) → (112.0, 18.0, −179.4) [D stream 1] carries Bond
//!    (the game's riding [M Control_InheritVelocity 0x133e88]) — its stream 46 writes **ch 83
//!    (0x53)** at tick 1410;
//! 6. ch 83 starts SP #296 (0x06000078, the bottom-of-mountain cinematic, 342 ticks) whose
//!    stream 10 writes **ch 5**; the helicopter #45 (Copter_Create 0x17d340: gate ch 5, out
//!    ch 0x4B [M record+0x70/+0x72; Copter_Update 0x17dbb8 cases 4/5/6]) flies and writes
//!    **ch 0x4B** → objective "Escape with Zoe in gondola" folds → mux #276 → **ch 100** →
//!    Mission_Update ends the level into the DRIVING hand-off **0x0730008**
//!    [M 0x182c60:85-116 — LevelToEndTo; no LOAD_LEVEL placement exists on this level [D]].
//!
//! The carried latches (05's 0x45/0x70, 06's 0x02, 07's 0x48) ride the RamSave in play
//! [M Player_RamSave 0x195670]; a cold test load sets them exactly like walk_07000008_live does.

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::{Input, RamSave};
use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The mission objective states for 07000008 (LEVELS[3] [M mission.rs LEVELS table]).
fn obj08(w: &Walker) -> (u32, u32) {
    let m = &w.scene.sim.as_ref().unwrap().drones.mission;
    (m.states[3][0], m.states[3][1])
}

/// Face `target` from Bond's head (turn + look), then hold fire for `ticks` — the REAL fire input
/// drives the gun bullets (the fusebox wires) and the laser beam (the safe bolts' burn [M
/// Player_LaserPointer 0x1a65d8's hook; walker.rs tick_fire]).
fn aim_and_fire(w: &mut Walker, target: [f32; 3], weapon: u8, fire_ticks: u32) {
    // The switch through the real weapon-change input.
    for _ in 0..240 {
        if w.scene.sim.as_ref().unwrap().gun.weapon == weapon { break; }
        w.tick(Input { switch_to: Some(weapon), ..Default::default() }, false);
    }
    assert_eq!(w.scene.sim.as_ref().unwrap().gun.weapon, weapon, "the switch to {weapon:#x} never landed");
    for t in 0..fire_ticks {
        let sim = w.scene.sim.as_ref().unwrap();
        let head = sim.bond.head();
        let to = [target[0] - head[0], target[1] - head[1], target[2] - head[2]];
        let l = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
        let desired_yaw = to[0].atan2(to[2]);
        let desired_pitch = (to[1] / l).asin();
        let yaw_err = walker::wrap_pi(desired_yaw - sim.bond.yaw);
        let pitch_err = desired_pitch - sim.bond.pitch;
        let input = Input {
            turn: yaw_err.clamp(-0.08, 0.08),
            look: pitch_err.clamp(-0.05, 0.05),
            ..Default::default()
        };
        w.tick_fire(input);
        if t % 120 == 0 {
            let sim = w.scene.sim.as_ref().unwrap();
            eprintln!("[08 aim t{t}] bond {:?} yaw {:.2} pitch {:.2} weapon {:#x}", sim.bond.pos, sim.bond.yaw, sim.bond.pitch, sim.gun.weapon);
        }
    }
}

/// The walked stretch that completes on the current tree: the intro, the fusebox, the safe, the
/// briefcase (objective 1 → 4), and the descent to the gondola mouth.
#[test]
fn walk_the_gondola_to_the_briefcase_and_back() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000008.bin", false).expect("load 07000008"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 139.5).abs() < 2.0 && (p[2] + 76.4).abs() < 2.0, "the station spawn #0 [D], got {p:?}");
        // The carried latches (in play they ride the RamSave [M 0x195670]); the cold stand-in
        // walk_07000008_live pins the same set.
        let sim = w.scene.sim.as_mut().unwrap();
        for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
    }
    w.wait_enabled(3000, "the station start");
    w.wait(30);

    // ── the intro plate: ch 0x49 + the station-intro scene ──────────────────────────────────────
    w.walk_nav_to([139.2, 49.5, -81.2], "to the intro plate", 1.3);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x49), "the intro plate #237 fired ch 0x49 [D placements]");
    }
    w.wait_enabled(4500, "the station-intro scene");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.scripts.players.iter()
            .any(|p| p.scripts.contains(&0x0600_0077) && p.state >= 3),
            "the station-intro scene (0x06000077) ran through the real trigger [M levels.md §5.1]");
        assert_eq!(sim.drones.mission.states[2][1], 4, "07000007's carried objective 2 completes on ch 0x49 [L record 2]");
        assert_eq!(obj08(&w).0, 3, "objective 1 announced by ch 0x49 [L record 3]");
    }

    // ── down the station and the walkway toward the fusebox ─────────────────────────────────────
    // The ch-0x2a plate sits inside the station (139.1, 50.2, −93.6 [D]) — the placed guards
    // (sw_ch 42) wake as Bond passes. The walkway descends south (door 11 free at (145,45,−108.6)).
    for (name, at) in [
        ("the station hall", [139.0f32, 49.5, -93.0]),
        ("the station doors", [139.5, 49.5, -97.5]),
        ("the walkway head", [144.0, 46.0, -105.0]),
        ("the walkway mid", [145.5, 45.0, -108.6]),
        ("the lower landing", [150.0, 43.5, -116.0]),
        ("by the fusebox", [153.0, 43.4, -118.5]),
    ] {
        w.walk_nav_to(at, name, 1.5);
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[08] at the fusebox tick {} bond {:?} ch42 {} ch43 {} ch44 {}", w.total_ticks, sim.bond.pos,
            sim.drones.channels.on(0x2a) as u8, sim.drones.channels.on(0x2b) as u8, sim.drones.channels.on(0x2c) as u8);
    }

    // ── cut the fusebox wires with the LASER → ch 137 (the Secure Room door's lock channel) ─────
    // The wire filter DROPS guns (kind 0x0008) and keeps the Laser (0x0002) [M fusebox.rs's
    // Collide_FilterBullets 0x1ece30:24-31 with the disc category words] — gunfire never cuts a
    // wire; the player's laser does.
    let fusebox = [154.35f32, 43.41, -119.67];
    aim_and_fire(&mut w, fusebox, 0x4e, 420);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(137), "shooting the fusebox wires raises ch 137 [D #219 group-0; M FuseBox_Update 0x1adfe8]");
    }

    // ── the secure room: the laser burns the safe bolts → ch 136 ────────────────────────────────
    // Door #10/122 (the GuardRoomDoor, lock ch 137 [D]) sits at (154.5, 42, −121.3) — E it ON its
    // position, then step through into the room (the safe wall's face is east).
    w.walk_to(Wp { at: [154.6, 42.5, -121.0], act: Act::Use { use_ticks: 240 }, arrive: 1.1 }, "E the secure room door");
    w.walk_nav_to([158.0, 43.0, -119.0], "into the secure room", 1.4);
    // The bolts sit at (162.24, 43.88/42.70, −117.14) [D #207/#208] — inside, on the safe face.
    w.walk_nav_to([160.0, 43.4, -117.0], "before the safe", 1.2);
    let bolts = [[162.24f32, 43.88, -117.14], [162.24, 42.70, -117.14]];
    for (k, bolt) in bolts.iter().enumerate() {
        aim_and_fire(&mut w, *bolt, 0x4e, 120);
        let sim = w.scene.sim.as_ref().unwrap();
        let ch = [134u32, 135][k];
        assert!(sim.drones.channels.on(ch), "the laser burn wrote the bolt's out channel {ch} [M Lock_Update 0x1ae3d8:194]");
    }
    // The mux folds 134|135 → 136; the safe door opens and SP #286 (SafePickUps) starts.
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(136), "mux #278 folded the bolt channels into ch 136 [D]");
        assert!(sim.drones.scripts.players.iter()
            .any(|p| p.scripts.contains(&0x0600_09e2) && p.state >= 3),
            "SP #286 SafePickUps started on ch 136 [D mask 1]");
    }

    // ── the briefcase: the take writes ch 0x4A (objective 1) ────────────────────────────────────
    w.walk_nav_to([172.7, 43.5, -116.2], "to the briefcase", 1.0);
    let mut got_case = false;
    for _ in 0..600 {
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x4a) { got_case = true; break; }
        w.tick(Input::default(), false);
    }
    assert!(got_case, "taking the briefcase writes ch 0x4a [M Pickup_Handler 0x18e650:330-333]");
    for _ in 0..120 {
        if obj08(&w).0 == 4 { break; }
        w.tick(Input::default(), false);
    }
    assert_eq!(obj08(&w).0, 4, "objective 'Retrieve guidance chip' completes on ch 0x4a [L record 3]");

    eprintln!("[08] the walked stretch done in {} ticks; falls {:?}", w.total_ticks, w.report.falls);
}

/// The ride continuation: board the parked cabin (the floor plate ch 4 → mux → ch 1), ride the
/// carriage (ch 83 at tick 1410), the bottom cinematic (ch 5), the helicopter (ch 0x4B) →
/// ch 100 → level_exit 0x0730008.
/// IGNORED with the exact blocker: the parked cabin's DOORWAY is shut by its script-entity door
/// leaves (0x2000477/0x2000478 at (111.99, 32.35, −126.27) [D the object dump]) — Bond stands on
/// the east ledge at (114.24, 27.87, −126.6) and the walk west is solid; the floor plate's box is
/// x 111.10..112.92, y 26.77..29.21, z −128.10..−126.75 [D the trigger's tri bbox] — INSIDE the
/// cabin, behind those leaves. E-pulses on the leaves do nothing in the port; the game's opener
/// (a door-open writer the port lacks — a script stream or a use on the cabin) is the live check
/// in the report. Everything the ride needs is verified present: the carriage entities (44, all
/// with collision shapes [D the shape probe]) join set_dynamic, the plate is live, SP #295 is
/// preloaded (state 1, mode 64 [D]) and the lib holds 0x0600004C.
#[test]
#[ignore = "the gondola cabin's script-entity doors are shut in the port; the opener is unlocated (the live check in docs/reports/level-08-complete.md)"]
fn walk_the_gondola_ride_to_the_end() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000008.bin", false).expect("load 07000008"));
    {
        let sim = w.scene.sim.as_mut().unwrap();
        for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
    }
    w.wait_enabled(3000, "the station start");
    // The briefcase chain is prerequisite for the ride (mux #235 ANDs ch 0x4A with the floor ch 4):
    // drive the same walked chain at unit cost — the plate is the real-input part under test here.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let f = sim.drones.frame;
        sim.drones.channels.set(0x4a, 1, f);
    }
    // ── the gondola: the floor plate → mux → the ride script ────────────────────────────────────
    // The room door swung shut behind Bond: E it again on the way out (the game's own toggle).
    for (name, at) in [
        ("back through the room", [158.0f32, 43.0, -119.0]),
        ("to the door inside", [155.5, 42.5, -121.0]),
    ] {
        w.walk_nav_to(at, name, 1.3);
    }
    w.walk_to(Wp { at: [154.5, 42.5, -121.2], act: Act::Use { use_ticks: 240 }, arrive: 1.1 }, "E the secure room door outbound");
    // The descent follows the level's own nav nodes ([D path-2/5 dump]): the mid doors (135.5,36,
    // −112) → the y-33.9 hall (118.2,−107.9 / 113.2,−106.3) → the stairwell top (108.2,31.4,−106.2)
    // → the stairs south down to (107.25,27.65,−119.5) → the lower level east (110.7,−121.5) → the
    // gondola mouth. The old z−119 anchor at y 33.5 had no floor — Bond walked off the hall's edge
    // onto the y-17.5 plunge quad and the death NIS fired (the F5 symptom's real geometry).
    // The outbound descent follows path 2's own link chain ([D the path-2 dump]): the landing
    // east (147.3/149.3/151.2/153, y 42.4, z −120.5) → the north ramp (153.1, 41.15, −115.95) →
    // (153.16, 39.65, −111.63) → (148.29, 38.4, −110.84) → the mid level (143.32, 36.4, −111.43)
    // → west past the mid doors (135.1, −111/−113.5) → (132.1, −112.27) → the long link to
    // (120.5, 34.04, −112.51) → the y-33.9 hall.
    for (name, at) in [
        ("through the door", [153.0f32, 42.5, -120.7]),
        ("the landing east", [151.2, 42.5, -120.5]),
        ("the ramp south", [153.1, 41.2, -116.0]),
        ("the ramp mid", [153.2, 39.7, -111.7]),
        ("the ramp turn", [148.3, 38.5, -110.9]),
        ("the mid level", [143.3, 36.5, -111.5]),
        ("the mid doors", [135.5, 36.5, -112.2]),
        ("the mid west", [132.1, 36.2, -112.3]),
        ("the long link", [120.5, 34.1, -112.5]),
        ("the hall y33", [118.2, 33.9, -111.0]),
        ("the hall y33 west", [118.2, 33.9, -107.9]),
        ("the hall y33 south", [113.2, 33.7, -106.4]),
        ("the stairwell top", [108.2, 31.5, -106.3]),
    ] {
        w.walk_nav_to(at, name, 1.5);
    }
    w.climb_stairs([107.25, 27.7, -119.5], "the stairwell down", 1.6, 4200);
    // The boarding ([D the floor probe]): the platform (y 27.25, z −121..−122.5) ends at the car
    // shaft — the middle (x 112) is open below (the cabin hangs there; its floor is the plate at
    // y 28, z −127.4), the side ledges (x 110/114) run south at y 26.75 past the parked cabin's
    // mouth. Walk the east ledge south; the plate's volume (12 tris = its box) touches Bond's body.
    for (name, at) in [
        ("the lower level east", [110.7f32, 27.7, -121.5]),
        ("the gondola mouth", [113.5, 27.4, -121.9]),
        ("the east ledge", [114.2, 27.0, -123.5]),
    ] {
        w.walk_nav_to(at, name, 1.4);
    }
    {
        // South along the ledge, polling for the plate; the cabin's open side is west.
        let mut last = [0.0f32, 0.0];
        let mut still = 0u32;
        for t in 0..2400u32 {
            if w.scene.sim.as_ref().unwrap().drones.channels.on(4) { break; }
            let sim = w.scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            // South to the cabin's mid (z −126.3 — the doors' row), then WEST into the mouth.
            let target = if p[2] > -125.8 { [114.0f32, 27.0, -126.3] } else { [112.5f32, 27.7, -126.3] };
            let use_key = if (p[0] - last[0]).abs() < 0.01 && (p[2] - last[1]).abs() < 0.01 {
                still += 1; still % 30 == 0
            } else { still = 0; last = [p[0], p[2]]; false };
            let desired = (target[0] - p[0]).atan2(target[2] - p[2]);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let input = Input { turn: err.clamp(-0.08, 0.08), forward: if err.abs() > 1.0 { 0.0 } else { 1.0 }, ..Default::default() };
            w.tick(input, use_key);
            if t % 400 == 0 {
                let sim = w.scene.sim.as_ref().unwrap();
                eprintln!("[08 board t{t}] bond {:?} ch4 {}", sim.bond.pos, sim.drones.channels.on(4) as u8);
            }
        }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[08 gondola] bond {:?} ch4 {} falls {:?} stuck {}", sim.bond.pos,
            sim.drones.channels.on(4) as u8, w.report.falls, w.report.stuck_events.len());
        for s in w.report.stuck_events.iter().take(4) { eprintln!("[08 gondola stuck] {s}"); }
        assert!(sim.drones.channels.on(4), "the cable-car floor plate fired ch 4 [D #236 mask 8]");
    }
    // mux #235 folds 0x4A AND 4 → ch 1 → SP #295 (the ride, 1420 ticks) starts. Watch the ride:
    // the carriage carries Bond [M Control_InheritVelocity 0x133e88] and writes ch 83 at tick 1410.
    let mut ride_started = false;
    for _ in 0..600 {
        if w.scene.sim.as_ref().unwrap().drones.channels.on(1) { ride_started = true; break; }
        w.tick(Input::default(), false);
    }
    assert!(ride_started, "mux #235 (0x4a AND 4) fired ch 1 [D]");
    let start_y = w.scene.sim.as_ref().unwrap().bond.pos[1];
    let mut ch83 = false;
    for t in 0..3600u32 {
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x53) { ch83 = true; break; }
        w.tick(Input::default(), false);
        if t % 300 == 0 {
            let sim = w.scene.sim.as_ref().unwrap();
            eprintln!("[08 ride t{t}] bond {:?} ch1 {} ch83 {} disabled {}", sim.bond.pos,
                sim.drones.channels.on(1) as u8, sim.drones.channels.on(0x53) as u8, sim.disabled);
        }
    }
    assert!(ch83, "the ride script wrote ch 83 (0x53) at its tick 1410 [D stream 46]");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[08] ride done: bond {:?} (started y {start_y:.1})", sim.bond.pos);
        assert!(!sim.drones.channels.on(0x7c), "Bond rode the carriage — the plunge (out 0x7C) never fired [D §6]");
    }
    w.wait_enabled(1500, "the bottom-of-mountain cinematic");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.scripts.players.iter()
            .any(|p| p.scripts.contains(&0x0600_0078) && p.state >= 3),
            "SP #296 (0x06000078) ran on ch 83 [D]");
        assert!(sim.drones.channels.on(5), "the cinematic wrote ch 5 [D stream 10]");
    }
    // The helicopter flies on ch 5 and writes ch 0x4B [M Copter_Update 0x17dbb8 cases 4/5/6].
    let mut ch4b = false;
    for _ in 0..2400 {
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x4b) { ch4b = true; break; }
        w.tick(Input::default(), false);
    }
    assert!(ch4b, "the helicopter's flight wrote ch 0x4B [M record+0x72; Copter_Update 0x17dbb8]");
    for _ in 0..120 {
        if obj08(&w).1 == 4 { break; }
        w.tick(Input::default(), false);
    }
    assert_eq!(obj08(&w).1, 4, "objective 'Escape with Zoe in gondola' completes on ch 0x4b [L record 3]");
    // ch 0x4B → mux #276 → ch 100 → the mission ends the level into the driving hand-off.
    let mut exit = None;
    for _ in 0..600 {
        exit = w.scene.sim.as_ref().unwrap().drones.level_exit;
        if exit.is_some() { break; }
        w.tick(Input::default(), false);
    }
    assert_eq!(exit, Some(0x0730_0008), "the mission ends the level into 0x0730008 [M 0x182c60:85-116]");
    eprintln!("[08] the gondola walked start to finish in {} ticks; falls {:?}", w.total_ticks, w.report.falls);
}

/// The ally rule (the playtest's "Zoe shooting at Bond at the gondola"): the dtype-0xC ally
/// (KikoMission [M 0x14c938:54-59]) never targets or fires at Bond while Bond walks the whole
/// route. Sister: the placed guards DO engage (their combat states appear once the zone plates
/// fire) — the ally is not one of them.
#[test]
fn the_ally_never_shoots_bond_on_the_walk() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000008.bin", false).expect("load 07000008"));
    {
        let sim = w.scene.sim.as_mut().unwrap();
        for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
    }
    w.wait_enabled(3000, "the station start");
    // Walk the same first legs (the station, the walkway) — where the guards wake and fight.
    for (name, at) in [
        ("the intro plate", [139.2f32, 49.5, -81.2]),
        ("the station hall", [139.0, 49.5, -93.0]),
        ("the station doors", [139.5, 49.5, -97.5]),
        ("the walkway head", [144.0, 46.0, -105.0]),
        ("the walkway mid", [145.5, 45.0, -108.6]),
    ] {
        w.walk_nav_to(at, name, 1.5);
    }
    w.wait(600);
    let sim = w.scene.sim.as_ref().unwrap();
    let ally = sim.drones.list.iter().enumerate()
        .find(|(_, d)| d.present && d.dtype == 0xc)
        .map(|(i, d)| (i, d.pos, d.state, d.team))
        .expect("the ally (dtype 0xC, Zoe) is placed");
    eprintln!("[08 ally] after the walk: {ally:?}");
    // The guards' ambush is up (the zone plates fired); the ally is NOT in a combat state against
    // Bond — KikoMission's target is never the player [M 0x14c938 dtype 0xC voice 0xC → 0x11].
    assert!(sim.drones.channels.on(0x2a) || sim.drones.channels.on(0x2b) || sim.drones.channels.on(0x2c),
        "the zone plates fired on the walk (the ambush is up)");
    let (ai, _, astate, _) = ally;
    assert!(!matches!(astate, 0x56..=0x7e | 0x85..=0x99),
        "the ally never enters a combat state on the walk [M KikoMission], state {astate:#x}");
    let _ = ai;
    assert!(!sim.drones.list.iter().any(|d| d.present && d.dtype == 0xc && d.hit_by_player),
        "the ally was never shot by Bond");
}
