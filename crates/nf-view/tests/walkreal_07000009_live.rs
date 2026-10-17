//! walkreal-b: 07000009 "Night of the Agent" (Tower A) walked with the player's own movement — no
//! teleports after the spawn. Route (docs/levels/07000009.md): the car-park start #68
//! (3.59, −377.08, 36.76) → the stair base → the AI graph's stairs up one floor to the lobby
//! (y ≈ −368.5) → the lift (shaft x −4.5..−2, z 10..15) → ch 0x39 → AND mux #274 → ch 0x46
//! (objective 2) → the LIFTDOORS_*_ENDLEVELDOORS SimpleScripts open on ch 0x39 → walk into the cab
//! → TOUCH_ONCE #67 → ch 0x32 → LOAD_LEVEL #63 → 0700000a. The alarm switches are never touched
//! (obj 1's inverted channel). The activation itself through the REAL E press lives in
//! elevator_buttons_09_live.rs (the panel is 15 units from the lift, by the armour); this test
//! exercises the downstream chain from the channel.

#[path = "common/walker_b2.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The elevator activation's reach, pinned: SP #70 (SCRIPTPLAYER_Elevators, mask 4, out 0x39) is
/// E-activated through its class-0 script ENTITY — `Player_Activate` 0x1a5170:121-122 dispatches
/// `SP_Activate` only on a hit object with `obj+0xE4 != 0 && class == 0`, and
/// `Script_EntityStart` 0x1d8d78:108-110 stores the owner SP there (elevator-buttons-09; the old
/// "no writer on the level / the E path cannot reach #70" reading was wrong — it fired on the
/// SP's entities, whose mode-64 preload creates entity 0x020005EA at t=0 [D script stream 0,
/// M 0x1dbc08:41-56]). The SP object itself never moves (its live matrix stays at the placement
/// origin), and the lift-door SSs carry mask 1 (no E arm [M SS_Activate 0x1a9ea8]).
#[test]
fn the_elevator_activation_writer_pinned() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000009.bin", false).expect("load 07000009"));
    w.wait(10);
    let sim = w.scene.sim.as_ref().unwrap();
    let sp = sim.drones.scripts.players.iter().find(|p| p.out == 0x39).expect("SP #70 (out ch 0x39)");
    assert_eq!(sp.mask & 4, 4, "the elevator SP is E-flagged [D placement #70 mask 4]");
    let t = sp.live_matrix[3];
    assert!(t[0].abs() < 0.5 && t[1].abs() < 0.5 && t[2].abs() < 0.5,
        "the SP object sits at its placement origin (no key stream moves it) [M 0600059b stream 0], got {t:?}");
    // The preload created the panel entity (the E target) [D script stream 0 t=0; M 0x1d8d78:108-110].
    assert!(sim.drones.scripts.objects.list.iter()
        .any(|o| matches!(o, Some(nf_game::nis::Obj::Entity { hash: 0x0200_05EA, .. }))),
        "entity 0x020005EA (the panel art, obj+0xE4 = SP#70) exists from the level load");
    // The two lift-door SSs wait on ch 0x39 at the lift mouth, mask 1 — no E, no toucher write [M].
    let doors: Vec<_> = sim.drones.sss.list.iter().filter(|s| s.channel == 0x39).collect();
    assert_eq!(doors.len(), 2, "the LIFTDOORS_*_ENDLEVELDOORS pair waits on ch 0x39 [D]");
    for d in &doors {
        assert!(d.matrix[3][2] > 12.0 && d.matrix[3][2] < 15.0 && d.matrix[3][0] > -5.0 && d.matrix[3][0] < -2.0,
            "the lift doors sit at the shaft mouth, got {:?}", d.matrix[3]);
    }
}

/// The full walk: car park → stairwell → lobby → the lift → E the cab → the doors open →
/// in the cab → the exit fires to 0700000a with objective 2 complete and no alarm.
#[test]
fn walk_car_park_to_the_main_lift() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000009.bin", false).expect("load 07000009"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 3.59).abs() < 2.0 && (p[1] + 377.0).abs() < 2.0, "the car-park start #68 [D], got {p:?}");
    }
    w.wait(60);

    // The stairs: the AI graph's own route (the nav nodes climb x≈20-24, z≈21-31 from the car-park
    // floor −376.68 to the lobby −369.03 [D nav dump]) — the door at (21.5, −377.08, 40.9) is a
    // different stairwell (the walk wedged inside it, see the report). The stair base anchor keeps
    // the nav leg honest, then the lobby.
    w.walk_nav_to([19.79, -376.68, 30.37], "the stair base", 1.4);
    w.walk_nav_to([23.61, -369.03, 28.74], "up the stairs", 1.4);
    w.walk_nav_to([-3.5, -369.0, 14.0], "the lift mouth", 1.4);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x61) && !sim.drones.channels.on(0x7c) && !sim.drones.channels.on(0x7d),
            "the walk must not raise the alarm (the switches are never touched)");
    }

    // The elevator activation: the downstream chain from ch 0x39 (the REAL E-press walk lives in
    // elevator_buttons_09_live.rs::walk_operate_the_button_and_ride_to_0700000a — the panel is 15
    // units west of the lift). Drive the channel here, then walk the rest with input only.
    let before = w.total_ticks;
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x39);
    w.wait(6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x39), "the stand-in fires the elevator ch 0x39 [D SP #70 out]");
        assert!(sim.drones.channels.on(0x46), "AND mux #274: ch 0x39 → objective 2's ch 0x46 [D]");
    }
    // The LIFTDOORS_*_ENDLEVELDOORS SimpleScripts play their open animation on ch 0x39
    // [M SS_Update 0x1a9948 state 0x40→1] — wait the swap out, then walk in.
    w.wait(120);
    w.walk_to(Wp { at: [-4.0, -369.0, 11.4], act: Act::None, arrive: 0.8 }, "into the lift cab");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x32), "TOUCH_ONCE #67 inside the cab → ch 0x32 [D]");
        assert_eq!(sim.drones.level_exit, Some(0x0700_000a), "LOAD_LEVEL #63 fires to 0700000a [D]");
    }
    eprintln!("[09] walked to the lift exit in {} ticks; stuck events: {}; falls: {}",
        w.total_ticks - before, w.report.stuck_events.len(), w.report.falls.len());
    for f in &w.report.falls { eprintln!("[09] fall: {f}"); }
}
