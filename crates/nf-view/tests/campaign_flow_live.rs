//! campaign-flow: play every single-player mission's levels in order, headless, from the front
//! end's mission start (a COLD load — mission select → level [M docs/systems/mission.md §3]) and
//! through the transitions the way the game does them:
//! - between levels: the objectives fold → ch 100 → MissionState 6 → `LevelToEndTo` — 0x07000048
//!   (the cut-scene wrapper level the game loads between every pair of levels; default of
//!   Mission_Update case 6 [M 0x182c60:85-135]) or a DRIVING.ELF code for the mission-final levels
//!   (0x0730004/8/b/d/f/13/17/1b — out of scope, logged and skipped by the port) — then
//!   `ResetMap_LevelToLoad` [M 0x182c60 case 4]. The port: `Event::LevelEnd` → `drones.level_exit`
//!   (scene.rs hud_tick) or the logged skip.
//! - the level's own exit: the LOAD_LEVEL trigger (Trigger_Update case 10 0x1b1a78 →
//!   Trigger_Activate 0x1b2318:111), key-gated by param 1.
//! - on an exit the port captures the RamSave + mission states and the next archive loads warm
//!   (`load_with_carry`; the game's `ResetMap_LevelToLoad(next, 1, 0)` + `Player_RamSave`
//!   [M 0x1b2318:111-116]).
//! Bond is driven through the game's own objective layer: each objective's condition/enable
//! channels are the same switch channels the level's triggers and scripts set (`Mission_Update`
//! 0x182c60 reads them). Where an objective needs real gameplay, the runner forces the verdict
//! (ch 100) halfway through the budget and records the gap. Opt-in NF_ISO.

use nf_game::mission::{self, Event};
use nf_game::player::{Input, RamSave};
use nf_view::scene::Scene;

#[path = "common/walker_b2.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The four gameplay-driven legs (campaign-flow blocker 6): the runner drives each objective's
/// live writer through the real input path instead of forcing the channels. The stand-ins left
/// at the documented blockers are listed in the note the row carries.
fn driver_for(level: u32) -> Option<fn(&mut Walker) -> String> {
    match level {
        0x07000009 => Some(drive_09),
        0x0700000a => Some(drive_0a),
        0x07000008 => Some(drive_08),
        0x0700004a => Some(drive_4a),
        _ => None,
    }
}

/// Fire the held gadget at a static point through the real trigger path (the walkreal-0a aim
/// loop, compact): turn+look the view onto the point, pulse the trigger while within `range`.
/// The tap tick carries no turn/look (the game's fire path gates on the settled aim) and aims
/// held (the gadget fires from AIM mode; aiming collapses the hip spread [M Bullet_init]).
fn shoot_gadget_at(w: &mut Walker, target: [f32; 3], range: f32, max_ticks: u32, done: &dyn Fn(&nf_view::scene::Sim) -> bool) -> bool {
    let start = w.total_ticks;
    let mut last_tap = 0u32;
    loop {
        if done(&w.scene.sim.as_ref().unwrap()) { return true; }
        if w.total_ticks - start >= max_ticks { return false; }
        {
            let sim = w.scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let eye = bp[1] + sim.bond.eye - sim.bond.crouch_drop;
            let (dx, dy, dz) = (target[0] - bp[0], target[1] - eye, target[2] - bp[2]);
            if (dx * dx + dy * dy + dz * dz).sqrt() > range { return false; }
            let err = walker::wrap_pi(dx.atan2(dz) - sim.bond.yaw);
            let want_pitch = (dy / walker::dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            let perr = want_pitch - sim.bond.pitch;
            let on_target = err.abs() < 0.008 && perr.abs() < 0.012 && w.total_ticks - last_tap > 15;
            let trigger = on_target;
            if trigger { last_tap = w.total_ticks; }
            let input = if trigger {
                Input { aiming: true, ..Default::default() }
            } else {
                Input { turn: err.clamp(-0.08, 0.08), look: perr.clamp(-0.08, 0.08), aiming: true, ..Default::default() }
            };
            w.scene.sim.as_mut().unwrap().step_bond(input, trigger, false, false);
        }
        w.total_ticks += 1;
        if w.top_up_health { w.scene.sim.as_mut().unwrap().drones.health.health = 100.0; }
    }
}

/// 09 (walkreal-07000009's route): car park → stairwell → lobby → the lift mouth → ch 0x39 at
/// the documented SP-activation blocker → the AND mux completes obj 2 → the lift doors' SSs
/// open → the cab → the real TOUCH_ONCE #67 → ch 0x32 → the real LOAD_LEVEL exit → 0700000a.
/// (The runner's old self-loop "exit" — blocker 3 — is never touched: the walked cab exit wins.)
fn drive_09(w: &mut Walker) -> String {
    let mut note = String::from("real drive 09: ");
    w.wait(60);
    assert!(w.scene.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p| p.out == 0x39),
        "SP #70 SCRIPTPLAYER_Elevators (out ch 0x39) [D placement]");
    w.walk_nav_to([19.79, -376.68, 30.37], "the stair base", 1.4);
    w.walk_nav_to([23.61, -369.03, 28.74], "up the stairs", 1.4);
    w.walk_nav_to([-3.5, -369.0, 14.0], "the lift mouth", 1.4);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x46), "sister: the lobby AND mux #274 has not fired before the elevator ch [D]");
        assert!(!sim.drones.channels.on(0x61) && !sim.drones.channels.on(0x7c) && !sim.drones.channels.on(0x7d),
            "the walk must not raise the alarm (the switches are never touched)");
    }
    // SP #70's activation writer is the level's documented blocker (the E path has no reach for
    // it — walkreal_07000009 pins every dead end; the main agent's gdbwatch on ch 57 is the
    // live check). Drive ch 0x39 exactly as docs/levels/07000009.md's walkthrough does.
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x39);
    w.wait(6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x39) && sim.drones.channels.on(0x46),
            "AND mux #274: ch 0x39 → objective 2's ch 0x46 [D]");
    }
    note.push_str("ch 0x39 at the documented SP-activation blocker; ");
    w.wait(120); // the LIFTDOORS_*_ENDLEVELDOORS SSs play their open animation [M SS_Update 0x1a9948]
    w.walk_to(Wp::new(-4.0, -369.0, 11.4), "into the lift cab");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x32), "TOUCH_ONCE #67 inside the cab → ch 0x32 [D]");
        assert_eq!(sim.drones.level_exit, Some(0x0700_000a), "LOAD_LEVEL #63 fires to 0700000a [D]");
    }
    note.push_str("the cab touch → the real exit to 0a");
    note
}

/// 0a: the arrival plate #58 completes 09's rec 3 (ch 0x47 — the writer lives HERE [D]) on
/// spawn; the nav loop to the Q-Worm PC; the gadget shot (or the documented cubicle-collision
/// stand-in); the keyboard ch 0x3A at the documented placement-transform blocker; the AND muxes
/// real; the hatch E through the real key gate → 0700000b.
fn drive_0a(w: &mut Walker) -> String {
    let mut note = String::from("real drive 0a: ");
    w.wait(30);
    let i09 = mission::LEVELS.iter().position(|l| l.level == 0x0700_0009).unwrap();
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x47), "the arrival plate #58 → ch 0x47 on spawn [D]");
    }
    w.wait(10);
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.mission.states[i09][2], 4,
        "09's rec 3 (ch 0x47, 'Take main elevator to office floor') folds HERE through the real plate [L ee_start_0700000a]");
    note.push_str("arrival plate #58 real (09's ch71 folds); ");
    let pc = w.scene.sim.as_ref().unwrap().drones.useobjs.pcs.iter().find(|p| p.ch == 0x48).expect("Q-WORM_PC_1 [D]").pos;
    // THE 0a WALK BLOCKERS (documented, live checks filed): the cab-mouth SS doors stand closed
    // (channel param 0, mask 0 — the opener is the live-checked arrival writer) and the lobby's
    // A* has no reachable start node, so the walked ring wedges inside the lobby. Place Bond at
    // each writer and drive the writers themselves (the plate, the gadget, the E holds) really.
    let pcp = [pc[0], 0.4, pc[2] + 1.6];
    w.scene.sim.as_mut().unwrap().bond = nf_game::player::Bond::new(pcp, 0.0);
    note.push_str("placed at the PC (the lobby walk wedges — cab SS doors + nav start); ");
    {
        // The gadget rides the mission inventory in play; the runner's synthetic carry needs the
        // hand-out (walk_0700000a's approach).
        let sim = w.scene.sim.as_mut().unwrap();
        sim.drones.inventory.owned[0x58] = true;
        sim.gun.weapon = 0x58;
        sim.gun.switch_request = 0x58;
    }
    let landed = shoot_gadget_at(w, [pc[0], pc[1] + 0.55, pc[2]], 2.2, 600, &|sim| sim.drones.channels.on(0x48));
    if !landed {
        // The cubicle-collision blocker: every eye-ray from every walked ring position is
        // blocked (walkreal_0700000a's 112-shot probe + this session's segment survey — a
        // full-height wall with normal (0.30, 0, 0.95) across the cubicle opening). Live check
        // for the main agent: walk to Q-WORM_PC_1 in PCSX2 — the wall is the port's bug.
        w.scene.sim.as_mut().unwrap().drones.set_channel(0x48);
        w.wait(6);
        note.push_str("ch 0x48 at the documented cubicle-collision blocker; ");
    } else {
        note.push_str("the Q-Worm install landed through the real shot; ");
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x48), "ch 0x48 (objective 4) [M PCQWorm_Update 0x1aee20]");
    }
    w.scene.sim.as_mut().unwrap().bond = nf_game::player::Bond::new([-6.0, 1.3, 14.5], 0.0);
    note.push_str("placed at the security centre; ");
    assert!(!w.scene.sim.as_ref().unwrap().drones.channels.on(0x49),
        "sister: AND mux #741 has not fired before the keyboard [D]");
    // SP #60's activation writer is the documented placement-transform blocker (its placement
    // matrix is identity [D probe]; the game's E path reaches SP_Activate only through
    // obj+0xE4-bound class-0 entities [M 0x1a5170:121-122]) — drive ch 0x3A as the doc records.
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x3a);
    w.wait(6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x49), "AND mux #741: ch 0x3A → ch 0x49 (objective 5) [D]");
        assert!(sim.drones.channels.on(0x4a), "AND mux #742: 0x48+0x49 → the exit's key ch 0x4A [D]");
    }
    note.push_str("ch 0x3A at the documented keyboard blocker; the muxes real; ");
    // The office door (−10.6, −19.3) is LOCKED on ch 0x30 with no writer in the dumps [D;
    // walkreal_0700000a's second blocker, live check filed] — drive the lock, place at the
    // door and hold the real E (the use path runs through step_bond's use arm).
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x30);
    w.wait(2);
    w.scene.sim.as_mut().unwrap().bond = nf_game::player::Bond::new([-10.6, 0.4, -19.3], 0.0);
    for _ in 0..400 { w.scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, true); }
    w.total_ticks += 400;
    w.scene.sim.as_mut().unwrap().bond = nf_game::player::Bond::new([-9.7, 0.4, -19.3], 0.0);
    w.scene.sim.as_mut().unwrap().bond = nf_game::player::Bond::new([21.72, 1.12, 19.4], 0.0);
    for _ in 0..600 { w.scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, true); }
    w.total_ticks += 600;
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_000b), "the hatch loads 0700000b [D]");
    note.push_str("the hatch E → the real exit to 0b");
    note
}

/// 08 (the gondola): the intro plate #237 → ch 0x49 (07's rec 2 folds through the carried
/// records); the cable-car floor Touch #236 → ch 4 with the AND-mux sister; the briefcase ch
/// 0x4A at the documented laser-watch blocker; mux #235 → SP #295's real carriage ride →
/// ch 83 → the pack cinematic → ch 5 → the copter → ch 0x4B → all six fold → ch 100.
fn drive_08(w: &mut Walker) -> String {
    let mut note = String::from("real drive 08: ");
    w.wait(30);
    // The carried latches — their writers live on 05/06/07 [D docs/levels/*.md]; in play they
    // ride the RamSave, and the runner's earlier legs completed through their own monitors.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
    }
    let plate = w.scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::TOUCH_ONCE && t.out == 0x49).map(|t| t.centre()).expect("intro plate #237 [D]");
    w.walk_to(Wp::new(plate[0], plate[1], plate[2]).r(1.2), "onto the intro plate");
    for _ in 0..6000 { if !w.scene.sim.as_ref().unwrap().disabled { break; } w.wait(1); } // the station intro SP #294 (3880 ticks)
    w.wait(60);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x49),
        "the intro plate #237 → ch 0x49 (07's rec 2 completes here) [D]");
    note.push_str("intro plate real; ");
    let floor = w.scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::TOUCH && t.out == 4).map(|t| t.centre()).expect("cable-car floor Touch #236 [D]");
    let _walked = w.try_walk_to(Wp::new(floor[0], floor[1] + 0.2, floor[2]).r(1.5), "onto the cable-car floor");
    if !w.scene.sim.as_ref().unwrap().drones.channels.on(4) {
        // The walkway descent has no walked route yet (walk_07000008 teleports; no walkreal
        // covers 08) — place Bond on the floor plate for the touch and note it.
        let f = [floor[0], floor[1] + 0.2, floor[2]];
        w.scene.sim.as_mut().unwrap().bond = nf_game::player::Bond::new(f, 0.0);
        note.push_str("placed on the floor plate (no walked route down the walkway yet); ");
    }
    w.wait(10);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(4), "the cable-car floor Touch #236 → ch 4 [D]");
        assert!(!sim.drones.channels.on(1),
            "sister: AND mux #235 needs BOTH legs — with ch 4 alone ch 1 must stay off [D]");
    }
    // Now the boarding half: ch 0x4A's writer is the safe-room briefcase [M Pickup_Handler
    // 0x18e650] — behind the safe bolts, whose burner (the laser watch) is not ported (lock.rs's
    // burn hook has no real stimulus here). The documented blocker; drive ch 0x4A as walk_08 does.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.drones.channels.set(0x4a, 1, sim.drones.frame);
    }
    w.wait(6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(1), "AND mux #235: 0x4A AND ch 4 → ch 1 starts the ride SP [D]");
    }
    note.push_str("floor touch real; mux ch 1; ");
    // The ride: SP #295 (mask 1 on ch 1) plays 0x0600004C from the LEVEL BIN — the carriage
    // stream carries the cab (and Bond on its floor) down to (112.0, 18.0, −179.4), fires
    // ch 83 at t1410; ch 83 starts the pack cinematic SP #296 → ch 5 → the copter writes 0x4B.
    let mut rode = false;
    let mut ch83_seen = false;
    for t in 0..3200 {
        // Raw steps (the probe's shape): the walker's float-recovery hop must not disturb the cab.
        w.scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        w.total_ticks += 1;
        // ch 83 starts SP #296 (the pack cinematic 0x06000078, pack 07900008.bin) whose stream 10
        // writes ch 5. WARM-LOAD PORT GAP (this session): the pack loads per the load print
        // ("07900008.bin: 1 scripts, for 06000078") yet the SP's lazy load hits the missing-script
        // arm (state 5) — the cold probe_08_gondola_chain runs the cinematic and fires ch 5 at
        // t1753. Drive ch 5 at that blocker; the copter chain below stays real.
        if !ch83_seen && w.scene.sim.as_ref().unwrap().drones.channels.on(83) {
            ch83_seen = true;
            let frame = w.scene.sim.as_ref().unwrap().drones.frame;
            w.scene.sim.as_mut().unwrap().drones.channels.set(5, 1, frame);
            note.push_str("ch 5 at the warm-load pack-cinematic gap (cold runs it, probe); ");
        }
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x4b) { rode = true; break; }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(rode && sim.drones.channels.on(0x4b),
            "the gondola ride chain completes: carriage → ch 83 → the cinematic → ch 5 → the copter → ch 0x4B [D 07000008.md §5.3]");
        assert!(sim.bond.pos[2] < floor[2] - 40.0,
            "Bond rode the carriage down the mountain (z {:.1} from {:.1})", sim.bond.pos[2], floor[2]);
    }
    note.push_str("the real carriage ride → ch 0x4B");
    note
}

/// 4a: the lift ride from the spawn — the platform carries Bond down the shaft (no input), the
/// first plate reveals the objective, the landing NIS plays, and the lift script's end fires
/// ch 0x4a (the documented nis.rs stand-in — no writer exists in the level's data, docs/levels/
/// 0700004a.md §6; the game's own landing writer is the live-checked #289 NIS enable).
fn drive_4a(w: &mut Walker) -> String {
    let mut note = String::from("real drive 4a: ");
    // 12's forced-complete carry holds ch 99/100 — they never survive a real transition; bring
    // the channels to what a real level entry finds (the re-arm test's clears).
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.drones.channels.set(100, 0, sim.drones.frame);
        sim.drones.channels.set(99, 0, sim.drones.frame);
    }
    w.wait(30);
    let i4a = mission::LEVELS.iter().position(|l| l.level == 0x0700_004a).unwrap();
    let y0 = w.scene.sim.as_ref().unwrap().bond.pos[1];
    let mut revealed = false;
    let mut mid_ride_complete = false;
    for k in 0..5600 {
        // Raw steps (the probe's shape): the platform carries Bond; health topped like fix_finale's ride.
        let sim = w.scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        sim.drones.health.health = 100.0;
        w.total_ticks += 1;
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0x4d) { revealed = true; }
        if k == 1200 { mid_ride_complete = sim.drones.channels.on(0x4a); }
        if sim.drones.channels.on(0x4a) { break; }
    }
    w.wait(10); // the monitor folds the objective the tick after the writer
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(revealed && sim.drones.channels.on(0x4d),
            "the shaft's first plate #277 revealed 'Gain access to lobby' during the ride [D]");
        assert!(!mid_ride_complete, "sister: ch 0x4a is OFF mid-ride — only the script's end writes it");
        assert!(sim.drones.channels.on(0x4a),
            "the lift ride reaches the lobby (ch 0x4a at the script's end — the documented stand-in [S 0700004a.md §6])");
        assert!(sim.bond.pos[1] < y0 - 100.0, "the ride covered the shaft (y {:.2} from {y0:.2})", sim.bond.pos[1]);
        // The fold itself is the runner's row.complete (the mission layer's arm after the banner;
        // the warm-carry fold timing is the live-checked open item, see the report).
    }
    note.push_str("the ride real (the script-end writer is the documented stand-in)");
    note
}

/// The story order [M ResetMap_LevelCode2Img 0x1d5258:77-350 — the on-foot cases ascend in mission
/// order; 0x0700000e/0f/10 have no case and are not story levels].
const STORY: [u32; 20] = [
    0x07000001, 0x07000002, 0x07000003, 0x07000004, 0x07000005, 0x07000006, 0x07000007, 0x07000008,
    0x07000009, 0x0700000a, 0x0700000b, 0x0700000c, 0x0700000d, 0x07000011, 0x07000012, 0x07000013,
    0x07000014, 0x07000015, 0x07000016, 0x0700001b,
];

/// The missions in story order: (base-map hash, [(level, part)] in part order) from the MissionData
/// records' `mission` field ([L ee_start_05.bin @ 0x2a4350 — 24 records; the port's LEVELS table]).
fn missions() -> Vec<(u32, Vec<(u32, u32)>, bool)> {
    let mut out: Vec<(u32, Vec<(u32, u32)>)> = Vec::new();
    for l in mission::LEVELS {
        match out.iter_mut().find(|(m, _)| *m == l.mission) {
            Some((_, v)) => v.push((l.level, l.part)),
            None => out.push((l.mission, vec![(l.level, l.part)])),
        }
    }
    for (_, v) in out.iter_mut() { v.sort_by_key(|(_, p)| *p); }
    let story_rank = |levels: &[(u32, u32)]| STORY.iter().position(|s| levels.iter().any(|(l, _)| l == s)).unwrap_or(usize::MAX);
    out.sort_by_key(|(_, levels)| story_rank(levels));
    out.into_iter().map(|(m, levels)| {
        let story = levels.iter().any(|(l, _)| STORY.contains(l));
        (m, levels, story)
    }).collect()
}

#[derive(Clone)]
struct Row {
    level: u32,
    parsed: bool,
    spawned: bool,
    objectives: String,
    complete: bool,
    exit_next: Option<u32>,
    note: String,
}

impl Row {
    fn cells(&self) -> String {
        format!("| 0x{:08x} | {} | {} | {} | {} | {} | {} |",
            self.level, self.parsed, self.spawned, self.objectives, self.complete,
            self.exit_next.map(|n| format!("0x{n:08x}")).unwrap_or_else(|| "—".into()), self.note)
    }
    fn blank(level: u32) -> Row { Row { level, parsed: false, spawned: false, objectives: "—".into(), complete: false, exit_next: None, note: String::new() } }
}

/// Play one level headless. `save` = None for the mission start (cold), Some for a warm
/// continuation. The level ends when `drones.level_exit` lands — the view's chain from the mission
/// layer's LevelEnd or the exit trigger's touch. Returns the row and the carry for the next leg.
fn play_level(iso_path: &str, level: u32, budget: u32, save: Option<&RamSave>) -> (Row, Option<RamSave>) {
    let mut row = Row::blank(level);
    let name = nf_view::level_archive_name(level);
    let loaded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| nf_view::load_with_carry(iso_path, &name, false, save)));
    let mut sc: Scene = match loaded {
        Ok(Ok(s)) => { row.parsed = true; s }
        Ok(Err(e)) => { row.note = format!("load error: {e}"); return (row, None); }
        Err(_) => { row.note = "CRASH in load".into(); return (row, None); }
    };
    let Some(sim) = sc.sim.as_mut() else { row.note = "no sim".into(); return (row, None); };
    let (start, yaw) = sc.start.unwrap_or(([0.0, 2.0, 0.0], 0.0));
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, start, yaw);
    row.spawned = true;
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    if sim.drones.mission.state == 0 { row.note.push_str("mission not inited; "); }
    drop(sim);

    // The gameplay-driven legs (blocker 6): the driver walks Bond through the level's own
    // writers (walk there, use the gadget, ride). The channel fallback below stays for the
    // levels without a driver.
    let mut driver_note: Option<String> = None;
    let mut wk = Walker::new(sc);
    if let Some(drive) = driver_for(level) {
        driver_note = Some(drive(&mut wk));
    }
    let mut sc = wk.scene;
    let Some(sim) = sc.sim.as_mut() else { row.note = "no sim".into(); return (row, None); };

    // Fire the objectives' own channels (the objective-trigger fallback; the game's triggers and
    // scripts set exactly these channels — `channel` is record+0xa, `enable` record+0xe [M 0x1827d8]).
    let idx = mission::LEVELS.iter().position(|l| l.level == level);
    let def = idx.map(|i| (i, &mission::LEVELS[i]));
    let mut total = 0usize;
    if let Some((i, d)) = def {
        total = d.objectives.len();
        if driver_note.is_none() {
            let mut unfired: Vec<u16> = Vec::new();
            for o in d.objectives {
                // fix-level-chain: the FAIL flag is record+0x10 bit 1 (0x02) [M 0x1827d8:32-38] — the old
                // `& 1` never skipped, so on 07000009 the runner raised the alarm itself and the
                // fail-restart (level_exit = the level) masked the real 0xE8 exit.
                if o.flags & 2 != 0 { continue; } // channel ON = FAIL — leave it off
                if o.enable != 0 { sim.drones.set_channel(o.enable); }
                if o.channel != 0 { sim.drones.set_channel(o.channel); }
                for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
            }
            for (j, s) in sim.drones.mission.states[i].iter().enumerate() {
                if *s < 2 { unfired.push(d.objectives[j].channel.into()); }
            }
            row.objectives = format!("{}/{}", total - unfired.len(), total);
            let no_exit_trigger = !sim.drones.triggers.iter().any(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.next_level != 0);
            if !unfired.is_empty() {
                row.note.push_str(&format!("unfired objective channels {unfired:?} (forced complete below); "));
            }
            // The verdict the game's own monitor would reach once real gameplay fires them: ch 100
            // is the AND of every part's condition [M 0x1827d8:22-38, 119-123]. A mid-mission level
            // with NO LOAD_LEVEL trigger (07000006 — its 0xE9-class placements are Drone_AIPoint
            // records [M parsemap_create_dynamic_objects 0x1d04f0 case 0xe9], so no exit exists in
            // the data) can only end through the mission-complete arm, whose ch 100 the level's own
            // scripts can also write — force it for the flow leg and record the gap either way.
            if !unfired.is_empty() || no_exit_trigger {
                sim.drones.set_channel(100);
            }
        } else {
            // The driven leg: count the objectives the driver's real writers actually folded.
            let unfired: Vec<u16> = d.objectives.iter().enumerate()
                .filter(|(j, _)| sim.drones.mission.states[i][*j] < 2)
                .map(|(_, o)| u16::from(o.channel))
                .collect();
            row.objectives = format!("{}/{}", total - unfired.len(), total);
            row.note.push_str(driver_note.as_deref().unwrap_or_default());
            row.note.push_str("; ");
        }
    } else {
        row.objectives = "no table".into();
    }

    // Tick the budget. The end arrives through the view's chain (hud_tick drains the mission
    // events into `drones.level_exit`); the driving interlude codes are logged and skipped there.
    for t in 0..budget {
        sim.step_bond(Default::default(), false, false, false);
        for ev in std::mem::take(&mut sim.drones.mission_events) {
            if let Event::LevelEnd { next } = ev { row.note.push_str(&format!("LevelEnd 0x{next:08x}; ")); }
        }
        if sim.drones.level_exit.is_some() { break; }
        if t == budget - 1 && sim.drones.mission.state == 6 && sim.drones.mission.to_end == 0x0700_0048 {
            // The banner is long over (case 7's 240 ticks) — the complete arm should have landed
            // already; give the pause→ResetMap handoff a final explicit window.
            for _ in 0..240 {
                sim.step_bond(Default::default(), false, false, false);
                if sim.drones.level_exit.is_some() { break; }
            }
        }
    }
    row.complete = sim.drones.mission.state == 6;

    // The level's own exit trigger, if the mission layer did not already end the level. The game
    // ends a level two ways: the auto-fire (Trigger_Update case 10 0x1b1a78:162-166 — needs a
    // NON-ZERO gate channel index that is ON) or Bond's E-use at the trigger (Player_Activate
    // 0x1a5170 → Trigger_Activate 0x1b2318 — the cancel channels 0x60-0x63 and the key gate aside,
    // BOTH gate arms load: success write when the gate is on, fallback write when off). The
    // fallback drives the E-path: stand Bond in the trigger's bbox and hold use.
    if sim.drones.level_exit.is_none() {
        let exit = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.next_level != 0)
            .map(|t| (t.lo, t.hi, t.gate, t.key_gate, t.next_level));
        if let Some((lo, hi, gate, key_gate, next)) = exit {
            row.note.push_str(&format!("exit trigger→0x{next:08x} gate ch{gate} key{key_gate}; E at the exit; "));
            if key_gate != 0 { sim.drones.set_channel(key_gate); } // the key pickup's channel [M 0x1b2318:76-90]
            // fix-level-chain: a trigger with no collision mesh has lo = inf / hi = -inf; the old
            // centre formula produced NaN and Bond never stood anywhere. Aim at the finite box only.
            if lo.iter().any(|v| !v.is_finite()) {
                row.note.push_str("no exit mesh (unbounded bbox) — E-aim skipped; ");
                let carry = sim.drones.level_exit.is_some().then(|| RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels).with_mission(&sim.drones.mission));
                row.exit_next = sim.drones.level_exit.take();
                if row.exit_next.is_none() && row.complete { row.note.push_str(&format!("mission end → LevelToEndTo 0x{:08x}", sim.drones.mission.to_end)); }
                return (row, carry);
            }
            let c = [(lo[0] + hi[0]) * 0.5, lo[1] + 0.1, (lo[2] + hi[2]) * 0.5];
            // fix-level-chain: the 0xE8's mesh is the door slab it sits on — the collision push-out
            // lands Bond beside it, so the use sphere reaches it only from the right FACING
            // [M 0x1a5170:100-137 head + dir sphere]. Sweep the four facings like a player would.
            'facing: for k in 0..4u32 {
                sim.bond = nf_game::player::Bond::new(c, k as f32 * std::f32::consts::FRAC_PI_2);
                for _ in 0..60 {
                    // fix-level-chain: hold the key channel across the sweep — on 0700000a the "key"
                    // (ch 0x4a) is an AND mux's output the mux recomputes every tick, so a one-shot
                    // write is reclaimed before the E-path reads it.
                    if key_gate != 0 { sim.drones.set_channel(key_gate); }
                    sim.step_bond(Default::default(), false, false, true); // the use key held
                    if sim.drones.level_exit.is_some() { break 'facing; }
                }
            }
        } else if !row.complete {
            row.note.push_str("no LOAD_LEVEL trigger and no mission complete; ");
        }
    }
    row.exit_next = sim.drones.level_exit.take();
    if row.exit_next.is_none() && row.complete {
        // The mission-final driving arms (0x0730004/8/b/d/f/13/17/1b) are logged and skipped by
        // the view (DRIVING.ELF out of scope); the default arm 0x07000048 is the FRONT-END level
        // (menu.rs FRONTEND_LEVEL — the game returns there when the flow ends without a driving
        // interlude). Either way the mission's chain ends here and the next mission starts cold.
        row.note.push_str(&format!("mission end → LevelToEndTo 0x{:08x} ({})",
            sim.drones.mission.to_end,
            if sim.drones.mission.to_end & 0xffff_0000 == 0x0730_0000 { "driving, skipped" } else { "the front end" }));
    }
    if row.exit_next.is_none() && !row.complete { row.note.push_str("STUCK: no level change"); }
    // The carry the game would hand the next level: RamSave + the mission states [M 0x1b2318:111-116].
    let carry = row.exit_next.map(|_| {
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
            .with_mission(&sim.drones.mission)
    });
    (row, carry)
}

/// Play one mission's chain: its levels in part order (warm-carrying between them), every
/// 0x07000048 wrapper leg played, the driving interludes recorded as skipped.
fn play_mission(iso_path: &str, m: u32, levels: &[(u32, u32)]) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut save: Option<RamSave> = None;
    for &(level, _part) in levels {
        let (row, carry) = play_level(iso_path, level, 600, save.as_ref());
        eprintln!("m0x{m:08x} 0x{level:08x}: {}", row.cells());
        rows.push(row.clone());
        save = carry;
        // 0x07000048 landings are the front end (the mission-complete default arm) — the chain
        // ends there; the next mission starts cold from the front end's mission select. No hop.
        let _ = row.exit_next;
    }
    rows
}

fn print_table(m: u32, rows: &[Row]) {
    eprintln!("\n=== mission 0x{m:08x} ===");
    eprintln!("| level | parsed | spawned | objectives | complete | exit→ | note |");
    eprintln!("|---|---|---|---|---|---|---|");
    for r in rows { eprintln!("{}", r.cells()); }
}

#[test]
fn campaign_flow_story_missions_1_to_3() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (m, levels, story) in missions().into_iter().filter(|(m, _, _)| [0x07000001, 0x07000005, 0x07000009].contains(m)) {
        assert!(story, "mission 0x{m:08x} should be a story mission");
        let rows = play_mission(&iso, m, &levels);
        print_table(m, &rows);
    }
}

#[test]
fn campaign_flow_story_missions_4_to_7() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (m, levels, story) in missions().into_iter().filter(|(m, _, _)| [0x0700000c, 0x07000011, 0x07000014, 0x0700001b].contains(m)) {
        assert!(story, "mission 0x{m:08x} should be a story mission");
        let rows = play_mission(&iso, m, &levels);
        print_table(m, &rows);
    }
}

/// The non-story MissionData records (0x0700000e/0x07000010): not in the LevelCode2Img switch —
/// the runner records whether their archives even parse, then stops.
#[test]
fn campaign_flow_nonstory_records_parse() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (m, levels, _) in missions().into_iter().filter(|(_, _, story)| !*story) {
        eprintln!("mission 0x{m:08x}: non-story (no LevelCode2Img case) [M 0x1d5258]");
        for (level, _) in levels {
            let (row, _) = play_level(&iso, level, 60, None);
            eprintln!("  0x{level:08x}: {}", row.cells());
        }
    }
}

/// The mission layer re-arms on a warm transition [M Mission_Init 0x182270:20-28 re-arms
/// InternalState/MissionState 1; LS_LoadMission 0x1cd880 is the save-file path, not a level load —
/// `ResetMap_GameInit` 0x1d5080:53 runs Mission_Init and nothing restores the layer's globals].
/// Pre-fix the carried snapshot's internal 5 (the post-banner state case 4 leaves [M 0x182c60:83])
/// froze the next level's monitor (case 5 is a bare break): no objective ever folded again.
#[test]
fn the_mission_layer_re_arms_on_a_warm_transition() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // 05: fire its objective, run the complete arm to internal 5 (the post-banner state), capture.
    let mut sc: Scene = match nf_view::load_with_carry(&iso, "07000005.bin", false, None) { Ok(s) => s, Err(e) => { panic!("05: {e}") } };
    let sim = sc.sim.as_mut().unwrap();
    let (start, yaw) = sc.start.unwrap_or(([0.0, 2.0, 0.0], 0.0));
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, start, yaw);
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    let i5 = mission::LEVELS.iter().position(|l| l.level == 0x07000005).unwrap();
    for o in mission::LEVELS[i5].objectives { if o.flags & 2 == 0 { if o.enable != 0 { sim.drones.set_channel(o.enable); } if o.channel != 0 { sim.drones.set_channel(o.channel); } } } // FAIL bit is 0x02 [M 0x1827d8]
    for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    sim.drones.set_channel(100); // the mission-complete verdict [M 0x1827d8:119-123]
    for _ in 0..500 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.drones.mission.internal, 5, "05 should sit in the post-banner arm (case 4) after the complete");
    assert_eq!(sim.drones.mission.states[i5][0], 4, "05's objective should have folded to done");
    let save = RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
        .with_mission(&sim.drones.mission);
    // 06 warm with that carry: the layer must re-arm and 06's own objective must fold again.
    let mut sc2: Scene = match nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&save)) { Ok(s) => s, Err(e) => { panic!("06: {e}") } };
    let sim2 = sc2.sim.as_mut().unwrap();
    let (s2, y2) = sc2.start.unwrap_or(([0.0, 2.0, 0.0], 0.0));
    sim2.bond = nf_game::player::Bond::stand_at(&sim2.world, s2, y2);
    // ch 99/100 never survive a real transition (the complete arms load DRIVING.ELF or the front
    // end, never the next story level warm) — the runner's forced complete left them set; bring
    // the channels to what a real level entry finds.
    sim2.drones.channels.set(100, 0, 0);
    sim2.drones.channels.set(99, 0, 0);
    for _ in 0..30 { sim2.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim2.drones.mission.internal, 1, "the warm load re-arms the layer (Mission_Init 0x182270)");
    let i6 = mission::LEVELS.iter().position(|l| l.level == 0x07000006).unwrap();
    for o in mission::LEVELS[i6].objectives { if o.flags & 2 == 0 { if o.enable != 0 { sim2.drones.set_channel(o.enable); } if o.channel != 0 { sim2.drones.set_channel(o.channel); } } } // FAIL bit is 0x02 [M 0x1827d8]
    for _ in 0..30 { sim2.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim2.drones.mission.states[i5][0], 4, "the past part's record survives the warm load");
    assert_eq!(sim2.drones.mission.states[i6][0], 4, "06's own objective folds again on the re-armed layer");
}

// ── front-end-chain: the front-end leg (campaign-flow blocker 7) ──────────────────────────────
// After each mission's final level completes the chain drives the front end the way the game
// does: Mission_Update case 4 (mission_ended) → the outro movie wrap (the loader's phase 0/2) →
// the results page 0x40000036 on the map [M 0x1d6360:160-163] → P_NFRESULTS case 0x4c (the
// fields + Menu_SetLevelBonus 0x1ffa10) → Confirm → the mission list 0x4000001c [M 0x208e78:43]
// → the next mission's row confirmed [M C_SBNFMAP 0x20b8d8:18-41]. From 01's end to 05's start.

use nf_view::menu::{self, Action, CAMPAIGN, FrontEnd, Outcome, Page, Stats};

/// The game's strings [M Txt_LoadLanguage 0x1caad0 loads USATxt.dat out of FILES.BIN].
fn chain_iso_lang(iso_path: &str) -> Option<nf_formats::hud::Lang> {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat"))?;
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::hud::Lang::parse(&bytes).ok()
}

/// Completing 01-04 unlocks 05: the results fields are the game's own numbers, the unlock rides
/// `nightfire_status` into the codename profile's MSSN, and the mission page offers 05's start.
#[test]
fn front_end_chain_01_to_05() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = chain_iso_lang(&iso);
    // The live progress layer + the front end, both fresh [D sp_level boot: rows 0,1 unlocked].
    let mut progress = nf_game::progress::Progress::default();
    progress.set_nightfire_status(0x3);
    let mut fe = FrontEnd::new();
    // A codename card: the unlock rides the profile's MSSN [M LS_MakeMission 0x1cd720:49-50].
    let dir = std::env::temp_dir().join(format!("nf-front-end-chain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    fe.set_profile_dir(dir.clone());
    fe.create_codename("CHAIN01", &[]).expect("create the codename");

    // 01-04 in story order (the campaign-flow table's missions 1-4). 01 starts OUTSIDE the front
    // end — the chain begins at 01's END (its row sits behind the driving missions in the wheel).
    const CHAIN: [u32; 4] = [0x0700_0001, 0x0700_0005, 0x0700_0009, 0x0700_000c];
    for (mi, &m) in CHAIN.iter().enumerate() {
        if mi > 0 {
            // The mission list offers the next mission: its row unlocked, confirmed, and the
            // game's mission start riding the request [M C_SBNFMAP 0x20b8d8:18-41].
            let rowi = CAMPAIGN.iter().position(|c| c.3 == m).unwrap();
            assert!(fe.unlocked_row(rowi), "the mission page lists 0x{m:08x} (unlocked by the previous award)");
            fe.cursor = rowi;
            assert!(fe.action(Action::Confirm), "the unlocked row confirms");
            assert_eq!(fe.requested_level.take(), Some(m), "the mission start rides the request [M 0x20b8d8:21-24]");
        }
        // Play the mission's parts (warm carries between them), as the runner always has.
        let (_, levels, _) = missions().into_iter().find(|(mm, _, _)| *mm == m).unwrap();
        let mut save: Option<RamSave> = None;
        let mut last_level = 0u32;
        let mut complete = false;
        for &(level, _) in &levels {
            let (row, carry) = play_level(&iso, level, 600, save.as_ref());
            eprintln!("m0x{m:08x} 0x{level:08x}: {}", row.cells());
            assert!(row.parsed, "0x{level:08x} parses");
            assert!(row.spawned, "Bond spawns in 0x{level:08x}");
            save = carry;
            last_level = level;
            complete = row.complete;
        }
        // 0700000b's leg races the forced complete against its own self-loop exit trigger
        // (blocker 4 [M the trigger's next_level is the level's own hash]) — either end drives
        // the same front-end leg; the mission layer's forced verdict stands in for the AND.
        assert!(complete || last_level == 0x0700_000b,
            "mission 0x{m:08x} ends headless (complete={complete})");

        // ── the front-end leg ──
        // The flow rides the runner state while the level played (the game: state 2 under
        // Mission_Update); mission_ended's ResetMap_LevelToLoad needs that state [M 0x1d3800].
        fe.flow.stack = vec![menu::ST_RUNNER];
        fe.flow.current_level = last_level;
        let rowi = CAMPAIGN.iter().position(|c| c.3 == m).unwrap();
        fe.mission = rowi;
        fe.mission_ended(Outcome::Complete, Stats::default());
        // Case 6 routes the mission-final level to its outro code [M 0x182c60:89-116].
        assert_eq!(fe.requested_level.take(), Some(menu::outro_for(last_level)),
            "the outro code rode the load request");
        // The wrap: the fade holds, the loader's phase 0 plays the movie, phase 2 resolves to the
        // front-end map with the results page [M 0x1d6360:160-163 + 0x1d3560]. The resolve is
        // consumed per leg (the host's own pattern — `requested_level.take()`).
        let mut resolved = None;
        for _ in 0..64 {
            fe.tick();
            if let Some(r) = fe.flow.phase2_resolved.take() { resolved = Some(r); break; }
        }
        assert_eq!(resolved, Some(menu::FRONTEND_LEVEL), "the outro routes to the front end");
        assert_eq!(fe.flow.page, menu::PAGE_RESULTS, "phase 2 sets +0x28 = 0x40000036 [M 0x1d6360]");
        fe.tick();
        assert_eq!(fe.flow.current_level, menu::FRONTEND_LEVEL, "the front-end map loads [M 0x1d6360 phase 3]");
        // P_NFRESULTS case 0x4c against the live layer: the fields + the award.
        let difficulty = progress.difficulty;
        assert!(fe.results_enter(&mut progress, m, 6, difficulty, lang.as_ref()),
            "the results fields are built [M 0x208e78:77]");
        let rf = fe.results.as_ref().expect("the fields");
        let idx = nf_game::progress::SCORING_TABLE.iter().position(|e| e.hash == m).unwrap();
        let th = nf_game::progress::SCORING_TABLE[idx].thresholds;
        assert_eq!(rf.score, progress.table[idx].last, "the score line is the entry's last [M :96]");
        // The medal/target pairing [M :98-142]: the target is the threshold ABOVE the score.
        let (want_medal, want_target, want_label) = if rf.score >= th[3] { (4u32, th[3], 0x2ccu32) }
            else if rf.score >= th[2] { (3, th[3], 0x2cc) }
            else if rf.score >= th[1] { (2, th[2], 0x2cd) }
            else if rf.score >= th[0] { (1, th[1], 0x2ce) }
            else { (5, th[0], 0x2cf) };
        assert_eq!(rf.medal, want_medal, "the medal counter matches the compare");
        assert_eq!(rf.target, want_target, "the target is the next threshold");
        assert_eq!(rf.target_label, want_label, "the target label pairs the medal");
        assert_eq!(rf.emblem, match want_medal {
            1 => 0x0300_013d, 2 => 0x0300_0140, 3 => 0x0300_013e, 4 => 0x0300_013f, _ => 0x0300_019a,
        }, "the emblem texture pairs the medal [M :109-139]");
        // The award [M 0x1ffa10:62-66]: the row complete + the next row unlocked, in the packed
        // nightfire_status the save carries [M Menu_GetNightfireStatus 0x202360].
        assert_eq!(progress.nightfire_status() & (1 << rowi), 1 << rowi, "the completed row stamps");
        assert!(fe.unlocked_row((rowi + 1).min(11)), "the next campaign row unlocks");
        // Confirm opens the save prompt (the commit improved + CHAIN01 is a saved codename [M
        // P_NFRESULTS 0x208e78:24-32]); Yes → the card flow's overwrite question (the file
        // exists [M MemCard_SAVE_DATA 0x10fa08 sub-7]) → Yes → "Saving data…" runs and the
        // completion routes to the mission list [M the terminal branch 0x217bd8:62-71 +
        // 0x208e78:203-211]. It opens on the highest unlocked row [M P_NFMAP 0x20b608 case
        // 0x4c scans sp_level+0x10 from row 11 down].
        assert!(fe.action(Action::Confirm));
        assert!(fe.results_box, "the save prompt stands [M 0x1000206]");
        assert!(fe.action(Action::Confirm), "Yes requests the save");
        assert!(fe.card_answer(menu::BTN_YES), "the overwrite question's Yes [M ASK_OVERWRITE]");
        fe.tick();
        assert_eq!(fe.card_message(), None, "the flow closed");
        assert_eq!(fe.page, menu::Page::MissionSelect);
        assert_eq!(fe.flow.page, menu::PAGE_MISSION_SELECT);
        let want_cursor = (0..CAMPAIGN.len()).rev().find(|&i| fe.unlocked_row(i)).unwrap();
        assert_eq!(fe.cursor, want_cursor, "the list opens on the highest unlocked row");
    }

    // 05 (07000011) is unlocked and listed after 01-04.
    let row5 = CAMPAIGN.iter().position(|c| c.3 == 0x0700_0011).unwrap();
    assert!(fe.unlocked_row(row5), "completing 01-04 unlocks 05 in the front end");
    assert_eq!(CAMPAIGN[row5].1, 0x0300_0009, "05's row label id [L sp_level]");
    // The profile: 05's bit rode the MSSN nightfire_status [M LS_MakeMission 0x1cd720:49-50].
    let profile = nf_game::profile::Profile::load_file(&nf_game::profile::Profile::path_in(&dir, "CHAIN01"))
        .expect("the card's file loads");
    let status = profile.progress.nightfire_status();
    assert_eq!(status & (1 << row5), 1 << row5, "05's bit is in the profile's MSSN");
    // The exact set the chain writes: boot rows 0-1, 02's award (row 1 + unlock 2), 01's (4+5),
    // 03's (5+6), 04's (6+7) — row 3 (09000006, a driving mission the chain never plays) stays clear.
    assert_eq!(status, 0b1111_0111, "01-04's awards + the boot rows rode the saves");
    // The mission page offers 05: confirming its row starts the mission.
    fe.cursor = row5;
    assert!(fe.action(Action::Confirm), "05's unlocked row confirms [M C_SBNFMAP 0x20b8d8:18-41]");
    assert_eq!(fe.requested_level, Some(0x0700_0011), "05's start is requested");
    let _ = std::fs::remove_dir_all(&dir);
}
