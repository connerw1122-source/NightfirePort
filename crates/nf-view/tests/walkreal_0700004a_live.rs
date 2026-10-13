//! walkreal-c: 0700004a (the Tower-2 elevator) walked with the player's own movement — no
//! `bond.pos` writes after the spawn. The route (docs/levels/0700004a.md; the walkreal_probe_c
//! dump + the spawn-support probe): the arrival (4.26, −7.71, −4.76) is ON the lift platform
//! (the top lobby's static floor is only the strip z 0..4; the shaft opens south — the probe's
//! t0-300 tick log shows Bond grounded on the platform from t30, descending with the script's
//! key line). The walk is therefore: board (the spawn), RIDE SP #42 "SCRIPTPLAYER_Lift" (mode 1
//! [D], 0x06000571: the platform descends 150 units t90→t5040 [D keys]) with no input, the floor
//! plates firing on the way (the first plate #277's ch 0x4d reveals the objective), and at the
//! bottom WALK north off the platform into the lobby. The script's end completes "Gain access to
//! lobby" (ch 0x4a — the documented nis.rs stand-in, docs/levels/0700004a.md §6).
//!
//! The shaft doors' auto-open on the lift's approach is the KNOWN GAP (§6): fix_finale_live had
//! to re-seat Bond every 30 ticks because the closed leaves scraped him off mid-ride. THIS test
//! rides with no re-seating — a scrape shows as the ride leaving the key line (the fail-first
//! for the fix). Opt-in via NF_ISO (the slim ISO is enough).

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

// GAP (fix-final-blockers, ROOT CAUSE FOUND — the door-solid hypothesis is WRONG): the ride
// freezes exactly when the −52 doors [110]/[111] start their ch-0x40 open, but the leaves never
// touch the rider (probe walkreal_probe_fb5 at t1615: bond capsule push (0,0,0) walkable false;
// the d50/d51 leaves span z −0.98..−0.82 while Bond stands at z −3.78 — a 2.8 gap). The real
// mechanism: at t1611 the lift script's landing beat sends PlayerDisable — the riding inherit
// (scene.rs step_bond, gated on !self.disabled [M 0x1949a8:24]) stops applying the platform's
// delta, Bond freezes at −54.13 and the platform descends away (the feet-ray gap grows 0.03/tick,
// grounded stays true via the floor ray's cc+0.1 margin). With the harness's skip_cutscenes the
// skip completes ~t1633 and the Script_Free re-enable places Bond at the script's matrix —
// −107.66, 53 units below the platform (this test's scrape). Without the skip Bond floats
// disabled while the platform departs (the probe). NEXT SESSION'S FIX: the game's landing beat —
// either the port's script must not disable the rider here (check the 0x06000571 event stream
// against the decompile: which command maps to PlayerDisable), or the re-enable's `at` matrix
// must be the platform's CURRENT pose (the game's rider stays ON the platform through the
// opening leaves — the live check in docs/reports/walkreal-c.md #2 still applies: gdbwatch
// BLData+0x94a through the −52 landing).
#[test]
#[ignore = "the walk's reached blocker — see docs/reports/walkreal-c.md; un-ignore when the blocker lands"]
fn ride_the_lift_from_the_spawn_and_walk_into_the_lobby() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "0700004a.bin", false).unwrap());
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(p[0] > 2.0 && p[0] < 6.0 && p[1] > -9.0 && p[1] < -6.0 && p[2] > -6.0 && p[2] < -3.0,
            "the arrival spawns at the shaft's top, on the platform (#1 [D]), got {p:?}");
    }
    // Board: settle onto the platform (the probe: grounded by t30). The reveal plate #277 is a
    // floor plate the stand-in toucher misses on a pure vertical ride (walkreal-a's documented
    // stand-in misses), so Bond walks the platform's north edge and back within the script's
    // t90 pre-descent window — the real player's first steps.
    let mut fired_plate = false;
    for k in 0..84 {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        let target = if k < 42 { [3.2, 0.0, -2.2] } else { [3.2, 0.0, -3.8] };
        let desired = (target[0] - p[0]).atan2(target[2] - p[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        w.tick(nf_game::player::Input { turn: err.clamp(-0.08, 0.08), forward: 1.0, ..Default::default() }, false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x4d) { fired_plate = true; }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.bond.grounded, "Bond settles back onto the lift platform after the first steps");
        assert!(fired_plate && sim.drones.channels.on(0x4d), "the shaft's first plate #277 fires ch 0x4d under the first steps [D]");
        assert_eq!(sim.drones.mission.states[17][0], 3, "ch 0x4d reveals 'Gain access to lobby' [L record 4a]");
    }
    let y_board = w.scene.sim.as_ref().unwrap().bond.pos[1];

    // The ride: no input while the script carries the platform down. The key line: y(t) =
    // y_board − max(0, t−90)·(150/4950) [D keys]; a rider off the line by > 5 (free fall diverges
    // in ~30 ticks) is the shaft-door scrape. The plates fire in descent order [D triggers].
    let t0 = w.total_ticks;
    let mut fired: Vec<u32> = Vec::new();
    let mut scrape: Option<(u32, f32, f32)> = None;
    for k in 0..5400u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        for ch in [0x3du32, 0x17, 0x18, 0x3f, 0x19, 0x45, 0x1a, 0x3e, 0x1b, 0x4f, 0x84] {
            if sim.drones.channels.on(ch) && !fired.contains(&ch) { fired.push(ch); }
        }
        let t = k + 60; // the board ticks count too
        let want = (y_board - ((t as f32) - 90.0).max(0.0) * (150.0 / 4950.0)).max(y_board - 150.0);
        let y = sim.bond.pos[1];
        if scrape.is_none() && t > 150 && (y - want).abs() > 5.0 {
            scrape = Some((t, y, want));
            eprintln!("[walk] the ride left the key line at t{t}: y {y:.2} want {want:.2} grounded={} substate={}",
                sim.bond.grounded, sim.bond.substate);
            break;
        }
        if sim.drones.channels.on(0x4a) { break; }
    }
    let ride_ticks = w.total_ticks - t0;
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(fired.contains(&0x17) || fired.contains(&0x3d),
            "the descent plates fire during the ride ([D] floors), got {fired:?}");
        assert!(scrape.is_none(),
            "the ride holds the script's key line (no shaft-door scrape) — left it at t{}, y {:.2} want {:.2}, fired {fired:?}",
            scrape.map(|s| s.0).unwrap_or(0), scrape.map(|s| s.1).unwrap_or(0.0), scrape.map(|s| s.2).unwrap_or(0.0));
        assert!(sim.drones.channels.on(0x4a),
            "the lift's lobby arrival completes the objective [L record 4a; nis.rs stand-in] — rode {ride_ticks} ticks, y {:.2} (boarded {y_board}), fired {fired:?}",
            sim.bond.pos[1]);
        assert_eq!(sim.drones.mission.states[17][0], 4, "the monitor completes 'Gain access to lobby'");
        assert!(sim.bond.pos[1] < y_board - 140.0, "the ride covered the shaft (y {:.2} from {y_board})", sim.bond.pos[1]);
    }

    // The walk out: north off the platform into the bottom lobby (the lobby strip at the shaft's
    // south edge; the lobby doors at z 0 are the mission's "access").
    w.walk_to(walker::Wp::new(3.0, y_board - 149.5, -2.5).r(1.0), "off the platform");
    w.walk_to(walker::Wp::new(3.0, y_board - 149.5, 2.0).r(1.2), "into the lobby");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.mission.states[17][0] != 0, "the objective holds through the walk-out");
    }
}
