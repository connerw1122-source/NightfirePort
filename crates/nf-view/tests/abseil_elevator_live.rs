//! drone-abseil-elevator live tests: the abseil family (0x8e–0x92) and the ElevatorJumper (0x8d)
//! through the real levels. Run with:
//! `NF_ISO=… cargo test -p nf-view --test abseil_elevator_live -- --test-threads=1`
//!
//! Disc reach [D]: the five 0x06000892 drones (nf-extract drones: 07000011 ×2 at (18.895, 46.516,
//! 1.434)/(19.745, 46.580, 0.363), 0700004a ×3 in the shaft at Y −7.5/−50/−95) — placement msg
//! 0x06000892 → mode (`Drone+0xC5`) 0x1c [M DefaultInit 0x14b300:615-627], the mode in {0x1b,0x1c}
//! arms the rope flag +0x23 and start 0x8e [M 0x14b300:757-766], and `DroneTypeSettings` rows 27/28
//! start at AbseilInit 0x8e [L rows = 8e00…]. ElevatorJumper 0x8d has NO decomp writer (every
//! SetState constant and +0x5a2 writer accounted; no DroneTypeSettings row starts there) — the test
//! drives it directly, the way a coder slot would.

use nf_game::drone::state;
use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The two 07000011 abseilers start in the abseil chain: mode 0x1c, the rope flag, anim set 0xf,
/// start script consumed, and the state machine walks Init → Slide → Hang (no opponent at the
/// start: the Slide's first UPDATE hangs) — without these states the drones would stand in Idle.
#[test]
fn the_07000011_abseilers_run_the_rope_chain() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000011.bin", false).expect("load 07000011");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0011);
    // The two 0x06000892 abseilers [D]; at difficulty 1 the game's own gate [M 0x136be0:49]
    // (param 2 = the minimum-difficulty tag) spawns only the first — the second (param 2 = 2) is
    // a difficulty-2+ spawn. The rope drone, by the mode the placement msg wired.
    let idx: Vec<usize> = sim.drones.list.iter().enumerate().filter(|(_, d)| d.mode == 0x1c).map(|(i, _)| i).collect();
    assert_eq!(idx.len(), 1, "one abseiler at difficulty 1, got {}", idx.len());
    let rope_top: Vec<[f32; 3]> = idx.iter().map(|&i| sim.drones.list[i].pos).collect();
    assert!((rope_top[0][1] - 46.5).abs() < 0.2, "the placement height [D] {rope_top:?}");
    // Tick 1: the drone waits for its switch channel 13 (param 3 [D] — the game's WAIT_SWITCH gate;
    // the trigger that fires it lives in the level's triggers), the rope fields already wired.
    sim.step_bond(Input::default(), false, false, false);
    for &i in &idx {
        let d = &sim.drones.list[i];
        assert_eq!(d.state, state::WAIT_SWITCH, "the abseiler waits for its trigger channel [D]");
        assert_eq!(d.switch_channel, 13);
        assert!(d.air, "+0x23 set [M 0x14b300:763]");
        assert_eq!(d.anim_set, 0xf, "the rope set 0xf [M 0x14b300:626]");
        assert_eq!(d.start_script, 0, "the 0x06000892 word is consumed, not a script");
        assert!(d.saved_anim_set != 0, "the skin's own set is saved at +0xdc");
        assert_eq!(d.descend_speed, 0.05, "DefaultInit 0x14b300:772-774");
        assert_eq!(d.rope[1], rope_top[idx.iter().position(|&x| x == i).unwrap()][1], "the rope point = the placement [M 0x169610:32-35]");
    }
    assert!(sim.drones.list.iter().filter(|d| d.enabled).all(|d| d.mode != 0x1b), "no 0x1b drone on disc [D]");
    // The trigger fires ch 13 → the WAIT_SWITCH update dispatches the START state: AbseilInit
    // 0x8e (DroneTypeSettings rows 27/28 [L]; the Global enter reads it through start_state).
    let frame = sim.drones.frame;
    sim.drones.channels.set(13, 1, frame);
    sim.step_bond(Input::default(), false, false, false);
    for &i in &idx {
        assert_eq!(sim.drones.list[i].state, state::ABSEIL_INIT, "the abseiler starts in AbseilInit 0x8e");
    }
    // The Init timer C (now + Rand_Rand(60)) fires into the Slide, whose first UPDATE hangs (no
    // opponent: `+0x170 == 0`). Both states must be reached inside 120 ticks.
    let mut slide_at = None;
    let mut hang_at = None;
    for t in 1..=120u32 {
        sim.step_bond(Input::default(), false, false, false);
        for &i in &idx {
            let d = &sim.drones.list[i];
            if d.state == state::ABSEIL_SLIDE && slide_at.is_none() { slide_at = Some(t); }
            if d.state == state::ABSEIL_HANG && hang_at.is_none() { hang_at = Some(t); }
        }
        if slide_at.is_some() && hang_at.is_some() { break; }
    }
    let (s, h) = (slide_at.expect("the Init timer C fires within 60 ticks"), hang_at.expect("the empty-rope Slide hangs at once"));
    assert!(s <= 61 && h == s + 1, "Init → Slide at tick {s} → Hang at {h} (the Slide's first UPDATE with no opponent)");
    // The hung drone holds the placement (the rope point never tracked an opponent; only the Init
    // window's 0.05-speed creep moved it) and keeps the rope flag + set 0xf.
    for (&i, top) in idx.iter().zip(&rope_top) {
        let d = &sim.drones.list[i];
        assert_eq!(d.state, state::ABSEIL_HANG);
        assert!(d.air);
        assert_eq!(d.anim_set, 0xf);
        assert!(d.pos[1] <= top[1] && top[1] - d.pos[1] < 0.5, "the creep is the Init window only, {:?}", d.pos);
    }
}

/// The hang ↔ slide hysteresis through the real handlers: an opponent appearing more than 3.0 below
/// a hung abseiler sends it back down the rope (the re-slide arm [M 0x169940:42-47]), the rope point
/// tracks the opponent's Y down [M 0x169730:52-54], and closing back within 3.0 hangs again — the
/// elevator-shaft follow. Then the floor probe lands it (the anim+0x60 & 8 poll [M 0x169730:32]).
#[test]
fn the_rope_follows_the_opponent_down_and_lands() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000011.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let idx: Vec<usize> = sim.drones.list.iter().enumerate().filter(|(_, d)| d.mode == 0x1c).map(|(i, _)| i).collect();
    assert_eq!(idx.len(), 1, "one abseiler at difficulty 1");
    // The trigger: ch 13 releases the abseiler from its WAIT_SWITCH gate [D param 3].
    sim.drones.channels.set(13, 1, sim.drones.frame);
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    let i = idx[0];
    assert_eq!(sim.drones.list[i].state, state::ABSEIL_HANG, "hung at the rope top (test 1)");
    // An opponent far below: the hang's re-slide arm fires on the next update.
    let d = &mut sim.drones.list[i];
    d.ever_seen = true; // `+0x170 != 0` stand-in
    d.player_pos = [d.pos[0], d.pos[1] - 20.0, d.pos[2]];
    let rope_y0 = d.rope[1];
    sim.step_bond(Input::default(), false, false, false);
    let d = &sim.drones.list[i];
    assert_eq!(d.state, state::ABSEIL_SLIDE, "the opponent 20 below resumes the slide [M 0x169940:42-47]");
    assert_eq!(d.rope[1], d.player_pos[1], "the rope point tracked the opponent's Y [M 0x169730:52-54]");
    assert_eq!(d.descend_speed, 6.5, "the slide's +0x50 [M 0x169730:22]");
    // Descending: within 40 ticks the drone must have dropped well past the rope top (the
    // acceleration reaches 6.5 u/s after a second).
    let y0 = d.pos[1];
    for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    let d = &sim.drones.list[i];
    assert!(d.pos[1] < y0 - 1.0, "the rope slide descends, {:?} → {:?}", y0, d.pos);
    assert!(d.rope[1] <= rope_y0 - 20.0, "the rope stays tracked to the opponent");
    // The opponent level with the drone: within 3.0 → hang, and the velocity decays ×0.98/tick.
    let d = &mut sim.drones.list[i];
    d.player_pos[1] = d.pos[1] - 1.0;
    sim.step_bond(Input::default(), false, false, false);
    let d = &sim.drones.list[i];
    assert_eq!(d.state, state::ABSEIL_HANG, "within 3.0 of the opponent → hang [M 0x169730:55-59]");
    let v0 = d.velocity[1];
    sim.step_bond(Input::default(), false, false, false);
    let d = &sim.drones.list[i];
    assert!((d.velocity[1] - v0 * 0.98).abs() < 1e-6, "the hang decays the velocity [M 0x169940:33]");
    // Landing: the floor probe (anim+0x60 & 8) restores the saved anim set, mode 0 (+0xc4), the land
    // anim 0x21 with the DrawWeapon follow-on [M 0x169730:32-38]. Emulate the drone reaching the
    // floor: teleport onto a walkable surface and tick.
    let d = &mut sim.drones.list[i];
    d.pos = [18.9, 41.6, 1.4]; // the office floor under the rope (the atrium plates are 42.01 [D])
    d.velocity = [0.0; 3];
    let mut landed = false;
    for _ in 0..40 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.list[i].mode == 0 { landed = true; break; }
    }
    let d = &sim.drones.list[i];
    assert!(landed, "the probe lands the rope drone (mode restored to +0xc4 = 0)");
    assert!(!d.air, "+0x23 cleared on landing [M 0x169730:34]");
    assert_eq!(d.anim_set, d.saved_anim_set, "the skin's set restored [M 0x169730:35]");
}

/// ElevatorJumper 0x8d (0x169488) — no decomp writer reaches the state, so this drives it directly:
/// ENTER finds and opens the elevator door (the class-0x1c object within 2.0 [M 0x14a940/0x14aa38]),
/// the door opening starts the jump anim 0x1e and arms timer C at +30 [M 0x169488:23-34], and the
/// timer's msg 0xc lands in Attack 0x56 [M 0x169488:40].
#[test]
fn the_elevator_jumper_opens_the_door_and_attacks() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000014.bin", false).expect("load 07000014 (the EvilBase: Rook's level, an elevator door present)");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0014);
    assert!(!sim.drones.doors.list.is_empty(), "07000014 has doors");
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    // Take a live team-1 drone, park it 1.0 from a door, and force the state.
    let door_pos = sim.drones.doors.list[0].pos;
    let i = sim.drones.list.iter().position(|d| d.enabled && d.present && d.team == 1 && !d.corpse).expect("a live guard");
    let d = &mut sim.drones.list[i];
    d.pos = [door_pos[0] + 1.0, door_pos[1], door_pos[2]];
    d.next_state = Some(state::ELEVATOR_JUMPER);
    let mut latch_tick = None;
    for t in 1..=90u32 {
        sim.step_bond(Input::default(), false, false, false);
        let d = &sim.drones.list[i];
        if d.state != state::ELEVATOR_JUMPER { assert_eq!(d.state, state::ELEVATOR_JUMPER, "stays until the timer"); }
        if d.elevator_latch { latch_tick = Some(t); break; }
    }
    let d = &sim.drones.list[i];
    assert_eq!(d.state, state::ELEVATOR_JUMPER);
    assert!(d.hit_door.is_some(), "FindElevatorDoor bound the door within 2.0 [M 0x14a940]");
    let latch = latch_tick.expect("the door opens (the activation runs; the swing takes ticks) → the jump arm");
    assert!(latch > 1, "the jump waits for the door, not the same tick");
    let t0 = d.timer_c;
    assert!(t0.is_some(), "timer C armed at +30 [M 0x169488:31]");
    // The door actually opened (the activation ran through Door_Activate).
    let di = d.hit_door.unwrap();
    assert!(sim.drones.doors.list[di].state & (nf_game::door::OPEN | nf_game::door::OPENING) != 0,
        "OpenDoor activated it [M 0x14aa38], state {:#x}", sim.drones.doors.list[di].state);
    // 30 ticks later the timer's msg 0xc lands in Attack.
    let deadline = t0.unwrap();
    for _ in 0..40 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.list[i].state == state::ATTACK { break; }
    }
    let d = &sim.drones.list[i];
    assert_eq!(d.state, state::ATTACK, "the timer C msg 0xc → Attack 0x56 [M 0x169488:40]");
    assert!(sim.drones.list[i].attack_frame != 0 || deadline > 0, "reached via the timer");
}
