//! elevator-buttons-09: the 07000009 elevator button through the REAL path — `nf_view::load` +
//! `Sim::step_bond` walking from the level start, opening the stairwell doors with E, pressing E
//! on the elevator panel and riding the exit into 0700000a.
//!
//! The mechanism (docs/systems/elevator-button.md): `Player_Activate` 0x1a5170:121-122 reaches
//! `SP_Activate` only on a hit object with `obj+0xE4 != 0 && class == 0` — a script ENTITY
//! (`Script_EntityStart` 0x1d8d78:108-110). SP #70 "SCRIPTPLAYER_Elevators" (mode 64, mask 4,
//! out ch 57 [D params]) preloads script 0x0600059B whose stream 0 creates entity 0x020005EA at
//! t=0 [D nis dump] — the panel entity exists from level load, its art baked absolute at the
//! gfx bounds (−18.38,−368.53,24.83)..(−17.78,−367.92,25.20) [D placements dump]. E at the panel
//! → latch → `switch_channels[57] = 1` [M 0x1dbda8:107-123,221-229] → the AND mux #274 completes
//! objective 2 (ch 0x46) [D] and the LIFTDOORS SS (channel 57 [D params (3,57)]) play their open
//! spline; Bond walks in, TOUCH_ONCE #67 → ch 0x32 → LOAD_LEVEL #63 → 0700000a [D].

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The panel's world centre from the placement gfx bounds [D].
const PANEL: [f32; 3] = [-18.079, -368.227, 25.015];
/// The lobby lift mouth (outside the LIFTDOORS plane z ≈ 13.2 [D SS placements]).
const LIFT_MOUTH: [f32; 3] = [-3.9, -368.5, 14.9];
/// Inside the shaft, in the TOUCH_ONCE #67 box (z 10.33..12.49 [D]).
const IN_CAB: [f32; 3] = [-3.9, -368.6, 11.5];

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, "07000009.bin", false).expect("load 07000009")
}

/// The elevator SP (out ch 57 = 0x39, mask 4 [D placement #70 params (5,4),(6,57)]).
fn elevator_sp(sim: &nf_view::scene::Sim) -> &nf_game::nis::Player {
    sim.drones.scripts.players.iter()
        .find(|p| p.out == 57 && p.mask & 4 != 0)
        .expect("SP #70 SCRIPTPLAYER_Elevators exists with mask 4 out 57 [D]")
}

/// The two LIFTDOORS SS (channel 57 [D placements #2/#3 param 3]).
fn lift_doors(sim: &nf_view::scene::Sim) -> Vec<&nf_game::ss::Ss> {
    sim.drones.sss.list.iter().filter(|s| s.channel == 57).collect()
}

/// Data probe (ISO-gated): the models table resolves the panel art — #70's own model and the
/// script entities' gfx 0x020005EA/0x020005EB — as small world-baked spheres at PANEL. Sister:
/// an unused hash resolves to nothing.
#[test]
fn panel_spheres_resolve_at_the_lobby_panel() {
    let iso = iso();
    let scene = load(&iso);
    let sim = scene.sim.as_ref().unwrap();
    let sp = elevator_sp(sim);
    assert_eq!(sp.scripts[0], 0x0600_059B, "SP #70's script A is the activation FX [D param 0]");
    let models = &sim.models;
    for (what, hash) in [("#70's own model", sp.model), ("entity A gfx", 0x0200_05EAu32), ("entity B gfx", 0x0200_05EBu32)] {
        let m = models.get(&hash)
            .unwrap_or_else(|| panic!("{what} ({hash:#x}) must resolve in the level's model table"));
        eprintln!("[09] {what} {hash:#x}: centre {:?} r {:.3}", m.centre, m.radius);
        assert!((m.centre[0] - PANEL[0]).abs() < 0.6 && (m.centre[1] - PANEL[1]).abs() < 0.6
            && (m.centre[2] - PANEL[2]).abs() < 0.6,
            "{what} must be baked at the lobby panel {PANEL:?}, got {:?}", m.centre);
        assert!(m.radius < 1.5, "{what} is a small panel, not a room (r {:.3})", m.radius);
    }
    // Sister: a hash the level does not carry resolves to nothing.
    assert!(models.get(&0x0200_9999u32).is_none(), "an unused gfx hash must not resolve");
}

/// The shared route (the nav graph's own car-park → lobby chain, run as real waypoints): the
/// car park → stairwell door #107 E → the two-flight switchback (x≈20-24, z 21-31 [D nav nodes])
/// → the lobby door #97 E → the lobby zigzag past the desk → by the armour.
fn walk_to_the_lobby(w: &mut Walker) {
    w.walk_to(Wp::new(9.6, -376.7, 40.6), "car park east");
    w.walk_to(Wp { at: [21.2, -377.0, 41.3], act: Act::Use { use_ticks: 120 }, arrive: 0.8 }, "stairwell door #107 E");
    w.walk_to(Wp::new(22.8, -377.0, 41.4), "through #107");
    w.walk_to(Wp::new(23.8, -377.0, 38.0), "stairwell room");
    w.walk_to(Wp::new(23.8, -377.0, 34.1), "stairwell south");
    w.walk_to(Wp::new(22.4, -377.0, 32.1), "to the ramp foot");
    w.walk_to(Wp::new(19.8, -377.0, 30.4), "ramp foot");
    w.walk_to(Wp::new(19.8, -374.4, 26.3), "flight 1");
    w.walk_to(Wp::new(19.8, -372.7, 23.2), "landing 1");
    w.walk_to(Wp::new(20.0, -372.7, 21.5), "landing east");
    w.walk_to(Wp::new(21.7, -372.7, 21.0), "landing east 2");
    w.walk_to(Wp::new(23.4, -372.7, 21.3), "landing south");
    w.walk_to(Wp::new(23.8, -372.7, 22.5), "ramp 2 foot");
    w.walk_to(Wp::new(23.7, -370.4, 25.5), "flight 2");
    w.walk_to(Wp::new(23.6, -369.0, 28.7), "lobby floor");
    w.walk_to(Wp::new(23.7, -369.0, 36.2), "stairwell lobby side");
    w.walk_to(Wp::new(23.8, -369.0, 38.2), "toward #97");
    w.walk_to(Wp::new(21.9, -369.0, 39.5), "before #97");
    w.walk_to(Wp::new(19.3, -369.0, 39.6), "at #97");
    w.walk_to(Wp { at: [18.5, -368.6, 39.0], act: Act::Use { use_ticks: 120 }, arrive: 0.8 }, "lobby door #97 E");
    w.walk_to(Wp::new(17.0, -369.0, 39.9), "through #97");
    w.walk_to(Wp::new(13.6, -369.0, 40.7), "lobby entry");
    w.walk_to(Wp::new(10.9, -369.0, 34.6), "lobby south");
    w.walk_to(Wp::new(5.3, -369.0, 31.7), "lobby mid");
    w.walk_to(Wp::new(-0.7, -369.0, 33.9), "lobby desk");
    w.walk_to(Wp::new(-7.6, -369.0, 31.7), "lobby west");
    w.walk_to(Wp::new(-11.3, -369.0, 34.2), "west of the desk");
    w.walk_to(Wp::new(-16.6, -369.0, 34.0), "toward the armour corner");
    w.walk_to(Wp::new(-18.4, -368.8, 27.5), "by the panel");
}

/// The full route: spawn → the lobby → E on the panel → ch 0x39/0x46, the FX (entity B), the
/// doors open → into the cab → 0700000a with objective 2 complete.
#[test]
fn walk_operate_the_button_and_ride_to_0700000a() {
    let iso = iso();
    let mut w = Walker::new(load(&iso));
    w.wait(30);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        // The doors start shut and #70 primed-but-idle (the prime runs on the first tick).
        let p = sim.bond.pos;
        assert!((p[0] - 3.59).abs() < 2.0 && (p[1] + 377.0).abs() < 2.0, "the car-park start [D], got {p:?}");
        let sp = elevator_sp(sim);
        assert_eq!(sp.state, 1, "mode 64 primes #70 on the first tick [M 0x1dbc08:41-56]");
        assert!(sim.drones.scripts.objects.list.iter().any(|o| matches!(o, Some(nf_game::nis::Obj::Entity { hash: 0x0200_05EA, .. }))),
            "the preload's t=0 Entity op created the panel entity [D script stream 0; M 0x1dbc08:41-56]");
        let doors = lift_doors(sim);
        assert_eq!(doors.len(), 2, "two LIFTDOORS SS [D]");
        assert!(doors.iter().all(|s| s.state == 0x40), "the doors rest in the sound-then-animate state [D param 0 = 64]");
        assert!(!sim.drones.channels.on(0x39) && !sim.drones.channels.on(0x46) && !sim.drones.channels.on(0x32));
    }

    walk_to_the_lobby(&mut w);

    // E on the panel: aim the view AT it and pulse E — the real player's camera. The panel is
    // knee-high (centre y −368.23, feet −368.36 [D gfx bounds]), the use pick is the 1.0-radius
    // sphere at head+view [M 0x1a5170], and positive pitch looks UP [M Bond::look's RotMatrixX]
    // — so the aim pitches down. The walker's horizontal-only use cannot reach it.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let mut fired = false;
        for k in 0..360 {
            let b = &mut sim.bond;
            let dy = PANEL[1] - (b.pos[1] + b.eye);
            let dx = PANEL[0] - b.pos[0];
            let dz = PANEL[2] - b.pos[2];
            let dxz = (dx * dx + dz * dz).sqrt();
            b.yaw = dx.atan2(dz);
            b.pitch = (dy / dxz).atan() / std::f32::consts::FRAC_PI_2;
            // Close the distance first (the pick reaches 1.0 + the panel's 0.43 sphere): walk in
            // to ~0.9, then hold and press.
            let forward = if dxz > 0.9 { 1.0 } else { 0.0 };
            sim.step_bond(nf_game::player::Input { forward, ..Default::default() }, false, false, dxz <= 1.2 && k % 6 == 0);
            if sim.drones.channels.on(0x39) { fired = true; break; }
        }
        assert!(fired, "the E pulses at the panel must activate SP #70 (bond at {:?}, panel {PANEL:?})",
            w.scene.sim.as_ref().unwrap().bond.pos);
    }

    w.wait(5);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x39), "the panel press writes ch 0x39 [M 0x1dbda8:221-229]");
        assert!(sim.drones.channels.on(0x46), "AND mux #274 completes objective 2 [D]");
        let sp = elevator_sp(sim);
        assert!(matches!(sp.state, 2 | 5), "the SP plays (state 2) or has finished-and-deleted (stay 1, state 5) [M 0x1dbda8], got {}", sp.state);
    }
    // The FX runs 25 ticks: entity B (0x020005EB) appears at t=16 [D script stream 0], sound
    // 0x4A1 plays [D stream 2].
    w.wait(40);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.scripts.objects.list.iter().any(|o| matches!(o, Some(nf_game::nis::Obj::Entity { hash: 0x0200_05EB, .. }))),
            "the activation FX created entity B [D script t=16]");
        let closed: Vec<[[f32; 4]; 4]> = lift_doors(sim).iter().map(|s| s.matrix).collect();
        drop(sim);
        let sim = w.scene.sim.as_mut().unwrap();
        for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
        let doors = lift_doors(sim);
        for (di, s) in doors.iter().enumerate() {
            assert_eq!(s.state, 0, "the one-shot door SS disarms at state 0 (param 1 = 1) [M 0x1a9948:121]");
            let moved: f32 = (0..3).map(|ax| (s.matrix[3][ax] - closed[di][3][ax]).abs()).sum();
            assert!(moved > 0.3, "door {di} must have posed open (moved {moved:.3})");
        }
    }

    // Into the lift: through the opened doors, the TOUCH_ONCE #67 box → ch 0x32 → LOAD_LEVEL #63.
    w.walk_to(Wp::new(LIFT_MOUTH[0], LIFT_MOUTH[1] + 0.5, LIFT_MOUTH[2]), "to the lift mouth");
    w.stop_on_exit = false;
    w.walk_to(Wp::new(IN_CAB[0], IN_CAB[1], IN_CAB[2]), "into the cab");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x32), "TOUCH_ONCE #67 fires in the cab [D]");
        assert_eq!(sim.drones.level_exit, Some(0x0700_000a), "LOAD_LEVEL #63 exits to 0700000a [D]");
        assert!(sim.drones.channels.on(0x46), "objective 2 still live-complete at the exit");
        assert!(!sim.drones.channels.on(0x61) && !sim.drones.channels.on(0x7c) && !sim.drones.channels.on(0x7d),
            "the walk must not raise the alarm (objective 1)");
    }
    eprintln!("[09] walked + rode in {} ticks; falls {:?}", w.total_ticks, w.report.falls);
}

/// Sister: the same walk but the panel is never operated — ch 0x39/0x46 stay off, the doors stay
/// shut (state 0x40, solid), Bond cannot reach the cab, and the level does not end.
#[test]
fn without_the_button_the_doors_stay_shut() {
    let iso = iso();
    let mut w = Walker::new(load(&iso));
    w.wait(30);
    walk_to_the_lobby(&mut w);
    // Past the panel without a press: the route's own arrival by the armour corner IS the
    // no-E pass (the panel sits on that leg's path).
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x39), "no E on the panel → ch 0x39 stays off");
        assert!(!sim.drones.channels.on(0x46) && !sim.drones.channels.on(0x32), "no objective, no exit channel");
        assert_eq!(elevator_sp(sim).state, 1, "#70 still primed-idle [M 0x1dbc08]");
        assert!(lift_doors(sim).iter().all(|s| s.state == 0x40), "the doors still rest shut [D param 0 = 64]");
        assert!(sim.drones.level_exit.is_none());
    }
    // The closed doors are solid: pushing at the shaft mouth must not reach the touch box
    // (z 10.33..12.49 [D]) nor fire it.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let yaw = (IN_CAB[2] - LIFT_MOUTH[2]).atan2(IN_CAB[0] - LIFT_MOUTH[0]);
        for _ in 0..300 {
            sim.bond.pos = [LIFT_MOUTH[0], LIFT_MOUTH[1], LIFT_MOUTH[2]];
            sim.bond.yaw = yaw;
            sim.step_bond(nf_game::player::Input { forward: 1.0, ..Default::default() }, false, false, false);
        }
        let z = sim.bond.pos[2];
        assert!(z > 12.9, "the closed LIFTDOORS (plane z ≈ 13.2, solid SS collision) must stop Bond, got z {z:.2}");
        assert!(!sim.drones.channels.on(0x32) && sim.drones.level_exit.is_none(),
            "outside the cab nothing fires");
        assert!(lift_doors(sim).iter().all(|s| s.state == 0x40), "the doors never left the resting state");
    }
}
