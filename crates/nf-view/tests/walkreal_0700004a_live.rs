//! walkreal-c: 0700004a (the Tower-2 elevator) walked with the player's own movement — no
//! `bond.pos` writes after the spawn. The route (docs/levels/0700004a.md; the walkreal_probe_c
//! dump): the arrival (4.26, −7.71, −4.76, the shaft's top lobby) walks onto the lift platform
//! (3.0, −7.0, −3.4 — SP #42 "SCRIPTPLAYER_Lift" mode 1 [D], the script starts at t0, the descent
//! t90→t5040 [D 0x06000571 keys]) and RIDES it down the shaft: the floor plates fire their
//! channels as the platform passes (the first plate #277's ch 0x4d reveals the objective; the
//! descent plates 0x3d/0x17/0x18/... [D triggers]), the ch-0x40 script event at t1610 unlocks the
//! −52 floor doors [D #110/111], and the script's end completes "Gain access to lobby" (ch 0x4a —
//! the documented nis.rs stand-in, docs/levels/0700004a.md §6).
//!
//! The shaft doors' auto-open on the lift's approach is the KNOWN GAP (§6): fix_finale_live had to
//! re-seat Bond every 30 ticks because the closed leaves scraped him off around the −52 floor.
//! THIS test rides with no re-seating — if Bond is scraped, the walk fails with the scraper's
//! dump (the fail-first for the fix). Opt-in via NF_ISO (the slim ISO is enough).

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn walk_lobby_to_the_lift_and_ride_to_the_lobby_arrival() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "0700004a.bin", false).unwrap());
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(p[0] > 2.0 && p[0] < 6.0 && p[1] > -9.0 && p[1] < -6.0 && p[2] > -6.0 && p[2] < -3.0,
            "the arrival spawns at the shaft's top lobby (#1 [D]), got {p:?}");
    }
    // No intro on a bare load (the probe dump: disabled=false at t0) — the lift script starts at
    // t0 [D mode 1], so Bond must board at once: two steps west onto the platform.
    w.wait_enabled(600, "the elevator arrival");
    w.walk_to(walker::Wp::new(3.0, -7.0, -3.4).r(0.6), "board the platform");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x4d), "the shaft's first plate #277 fires ch 0x4d under the boarding [D]");
        assert_eq!(sim.drones.mission.states[17][0], 3, "ch 0x4d reveals 'Gain access to lobby' [L record 4a]");
    }

    // The ride: Bond stands on the platform (no input) while the script carries him down. The
    // plates fire in descent order; the ride is 4950 ticks [D keys t90→t5040]. Watch for the
    // scrape: any fall > 4 units or a y that stops following the key line (−150/4950 per tick
    // after t90) is the shaft-door gap.
    let y0 = w.scene.sim.as_ref().unwrap().bond.pos[1];
    let mut fired: Vec<u32> = Vec::new();
    let mut scrape_tick: Option<u32> = None;
    for t in 0..5400u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        for ch in [0x3du32, 0x17, 0x18, 0x3f, 0x19, 0x45, 0x1a, 0x3e, 0x1b, 0x4f, 0x84] {
            if sim.drones.channels.on(ch) && !fired.contains(&ch) { fired.push(ch); }
        }
        // The key line: y(t) = y0 − max(0, t−90)·(150/4950), clamped at y0−150. A rider > 1.5 off
        // the line for 60+ ticks (and not grounded on the lobby floor at the end) = scraped.
        let want = (y0 - ((t as f32) - 90.0).max(0.0) * (150.0 / 4950.0)).max(y0 - 150.0);
        let y = sim.bond.pos[1];
        if scrape_tick.is_none() && t > 120 && (y - want).abs() > 1.5 && sim.drones.channels.on(0x4d) {
            scrape_tick = Some(t);
            eprintln!("[walk] the ride left the key line at t{t}: y {y:.2} want {want:.2} grounded={} substate={}",
                sim.bond.grounded, sim.bond.substate);
            break;
        }
        if sim.drones.channels.on(0x4a) { break; }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(fired.contains(&0x17) || fired.contains(&0x3d),
            "the descent plates fire during the ride ([D] floors), got {fired:?}");
        assert!(fired.len() >= 6, "most descent plates fired (the ride passed them), got {fired:?}");
        assert!(sim.drones.channels.on(0x4a),
            "the lift's lobby arrival completes the objective [L record 4a; nis.rs stand-in] — scrape at {scrape_tick:?}, y {:.2} (started {y0}), fired {fired:?}",
            sim.bond.pos[1]);
        assert_eq!(sim.drones.mission.states[17][0], 4, "the monitor completes 'Gain access to lobby'");
    }
}
