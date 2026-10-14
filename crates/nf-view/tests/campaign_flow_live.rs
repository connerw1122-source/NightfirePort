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
    assert!(w.follow_nav_route([pc[0] + 2.0, 0.4, pc[2] + 3.0], "to the Q-Worm PC"), "no nav route to the PC");
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
    assert!(w.follow_nav_route([-6.0, 1.3, 14.0], "to the security centre"), "no nav route to the keyboard");
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
    assert!(w.follow_nav_route([-15.8, 0.4, -16.4], "to the south-west office"), "no nav route");
    // The office door (−10.6, −19.3) is LOCKED on ch 0x30 with no writer in the dumps [D;
    // walkreal_0700000a's second blocker, live check filed] — drive the lock as the doc does.
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x30);
    w.wait(2);
    w.walk_to(Wp::at(-10.6, 0.4, -19.3, Act::Use { use_ticks: 400 }), "E the locked office door");
    w.walk_to(Wp::new(-9.7, 0.4, -19.3).r(1.0), "through the doorway");
    assert!(w.follow_nav_route([21.72, 1.12, 19.4], "to the hatch"), "no nav route to the hatch");
    w.walk_to(Wp::at(21.72, 1.12, 19.79, Act::Use { use_ticks: 600 }), "E the hatch");
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
    for _ in 0..3200 {
        // Raw steps (the probe's shape): the walker's float-recovery hop must not disturb the cab.
        w.scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        w.total_ticks += 1;
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
                if o.flags & 1 != 0 { continue; } // channel ON = FAIL — leave it off
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
            let c = [(lo[0] + hi[0]) * 0.5, lo[1] + 0.1, (lo[2] + hi[2]) * 0.5];
            sim.bond = nf_game::player::Bond::new(c, 0.0);
            for _ in 0..240 {
                sim.step_bond(Default::default(), false, false, true); // the use key held
                if sim.drones.level_exit.is_some() { break; }
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
    for o in mission::LEVELS[i5].objectives { if o.flags & 1 == 0 { if o.enable != 0 { sim.drones.set_channel(o.enable); } if o.channel != 0 { sim.drones.set_channel(o.channel); } } }
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
    for o in mission::LEVELS[i6].objectives { if o.flags & 1 == 0 { if o.enable != 0 { sim2.drones.set_channel(o.enable); } if o.channel != 0 { sim2.drones.set_channel(o.channel); } } }
    for _ in 0..30 { sim2.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim2.drones.mission.states[i5][0], 4, "the past part's record survives the warm load");
    assert_eq!(sim2.drones.mission.states[i6][0], 4, "06's own objective folds again on the re-armed layer");
}
