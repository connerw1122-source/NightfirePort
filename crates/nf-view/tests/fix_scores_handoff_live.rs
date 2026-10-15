//! fix-scores-handoff: the level-end score-row hand-off (G-045 remainder, the `NewScoresRef`
//! PTP rows' producer).
//!
//! The evidence chain:
//!   - The game's scoring moment is the results page's `PlrStat_GetScore(0)` — commit mode
//!     [M 0x208e78:77 → 0x1984b0:301-315] — with `Mission_Status() == 6` [M 0x182c60:85-122
//!     sets state 6 before the front end 0x07000048 loads]; the same call runs at the
//!     DRIVING→ACTION hand-off (`Boot_LoadPTPData` 0x1c8560: `PlrStat_GetScore(0)` then
//!     `ResetMap_LevelToLoad(0x7000048,0,0)`). The scored entry carries what the page reads:
//!     the 9 rows (count/target static, value the run's stat, +0xC ratio, +0x10 raw) and the
//!     totals (+0x2C base, +0x24 dif, +0x28 bonus, +0x34 last) [M 0x1984b0:100-333,
//!     0x208e78:80-146].
//!   - `NewScoresRef` 0x30cc6c itself (the 0x09-map row source, the PTPDATA tail 0x31ca38 =
//!     PTP+0x9a8, 9 rows × 3 s32 `[value, count, target]`, row 7's value ×100 at the copy
//!     [M 0x1984b0:50-64]) has NO writer in ACTION.ELF — only bootup 0x1c7e78:86 points it
//!     [M the whole-decompile grep + the whole-ELF constant scan; DRIVING.ELF hardcodes none of
//!     the PTP addresses — its block pointer arrives at boot]. The shipped rows read zero
//!     [L ee_start_05]. The port's Sim therefore produces the rows itself at level end
//!     (`Progress::finish_level`, the tick-end hook on the exit latch) — read-mode, so a
//!     mid-mission part exit (07000005's truck ride into 07000006, a trigger LOAD_LEVEL, not a
//!     mission end) stamps nothing into best/last.
//!   - The run: the 07000005 truck-ride exit through the real input path (the
//!     gate_guard_truck_live helpers): the cold start → the gatehouse porch → the ch-0x5C plate
//!     → the hand-off cut-scene → the crouched ride → the park → the EndLevel chain → ch 0x1D →
//!     the LOAD_LEVEL of 07000006 [D U-048; M Trigger_Update case 10 0x1b1a78 →
//!     Trigger_Activate 0x1b2318]. The ride kills nobody and fires no shots, so the kill rows
//!     (1 dispatched / 2 disabled / 3 surrendered) and the bonus row (8) are zero in value AND
//!     raw; the stealth row (4) has value 0 (no detections) but scores the FULL max within par
//!     [M 0x1984b0:225-233]; the accuracy row (5) carries the par 100 and the FULL target — the
//!     game copies the count into the value first and only a shot tally rescales it [M
//!     0x1984b0:74-78]; the health (6) and time (7) rows score from the run's own snapshot.
//!
//! Run with: `NF_ISO=… cargo test -p nf-view --test fix_scores_handoff_live -- --nocapture --test-threads=1`

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::Input;
use walker::Walker;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The generic row's raw: `target · min(count, value) / count` [M 0x1984b0:174-191].
fn expect_generic(count: u32, value: u32, target: u32) -> f32 {
    // The game's generic raw: ceil₁ₑ₋₅(target · min(count, value) / count) [M 0x1984b0:181-185].
    let x = target as f32 * count.min(value) as f32 / count as f32;
    let t = x as i32;
    if x - t as f32 >= 1e-5 { (t + 1) as f32 } else { t as f32 }
}

#[test]
fn truck_ride_finish_produces_the_results_rows() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"));
    // The hand-off cut-scene must PLAY (the game's own beat) — the gate_guard_truck route.
    w.skip_cutscenes = false;
    w.wait(30);
    w.walk_nav_to([102.0, 25.0, 110.5], "courtyard by the gatehouse porch", 1.6);
    assert!(w.climb_stairs([99.5, 26.0, 107.0], "onto the gatehouse porch", 1.0, 600),
        "Bond must climb the gatehouse porch step");
    w.walk_to(walker::Wp::new(99.0, 26.0, 106.6).r(0.7), "onto the ch-0x5C plate");
    let mut saw_5c = false;
    for _ in 0..2400u32 {
        w.tick(Input::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x5c) { saw_5c = true; break; }
    }
    assert!(saw_5c, "standing on the porch must fire the script-created ch-0x5C plate");

    // The ride: crouch when re-enabled below x 95 (the truck bed), end on the exit latch.
    let mut prev_dis = true;
    let mut crouch_armed = false;
    let mut input = Input::default();
    for _ in 0..6000u32 {
        let sim = w.scene.sim.as_ref().unwrap();
        let dis = sim.disabled;
        let px = sim.bond.pos[0];
        if !dis && prev_dis && px < 95.0 { crouch_armed = true; }
        prev_dis = dis;
        input.crouch = if crouch_armed && !w.scene.sim.as_ref().unwrap().bond.crouched {
            crouch_armed = false; true
        } else { false };
        w.tick(input, false);
        if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { break; }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0006), "the ride must end the level into 07000006");

    // THE HAND-OFF: the exit tick's production must have run (the latch is set in the trigger
    // walk, the tick-end hook produces before the tick returns).
    let f = sim.drones.progress.finished.as_ref().expect("the level-end hand-off rows must exist")
        .clone();
    assert_eq!(f.base_map, 0x0700_0005, "scored under 07000005's base hash");
    assert_eq!(f.mission_status, 6, "the production runs the results-path status [M 0x1984b0:316]");
    // The snapshot is the run's tally as of the exit tick; the viewer's per-frame elapsed log
    // (scene.rs's log_update_elapsed) may have advanced a tick or two past it by the time the
    // test reads — compare every field except the clock.
    let mut snap = f.stats; let mut live = sim.drones.progress.stats[0];
    snap.elapsed = 0; snap.last_time = 0; live.elapsed = 0; live.last_time = 0;
    assert_eq!(snap, live, "the snapshot is the run's tally (modulo the live clock)");

    // The no-kill sister: the vanilla ride kills nobody and fires nothing.
    assert_eq!((f.stats.dispatched, f.stats.disabled, f.stats.surrendered), (0, 0, 0),
        "the truck ride is a no-kill run");
    assert_eq!(f.stats.shots_fired, 0, "Bond never fires on the ride");
    for i in [1usize, 2, 3, 8] {
        assert_eq!(f.rows[i].value, 0, "row {i}: the no-kill run's stat");
        assert_eq!(f.rows[i].raw, 0.0, "row {i} scores the zero rows");
    }
    // Row 5 (accuracy): the game copies the par (count 100) into the value FIRST and only a
    // shot tally rescales it [M PlrStat_GetScore 0x1984b0 — the store row+0x6c ← row+0x64 (the
    // count), then the DAT_002bcc24 (PlrMissionStats+0x04 shots fired) != 0 guard overwrites
    // with hits × count / fired] — a NO-SHOT run shows 100% and scores the FULL target
    // (P_NFSTATS 0x209938:57-66 displays that value verbatim; the capture's "Accuracy 100% /
    // 100% / 50,000" — docs/reports/nfstats-model.md). (The pre-nfstats `else { 0 }` was the
    // misport; the held 614 tree fixed the scorer but left this loop expecting 0 — the held
    // failure.)
    assert_eq!(f.rows[5].value, 100, "no shots → the accuracy par copies [M 0x1984b0:74-78]");
    assert_eq!(f.rows[5].raw, expect_generic(f.rows[5].count, f.rows[5].value, f.rows[5].target),
        "row 5's raw per the generic formula — 100/100 of the par is the full target");
    assert_eq!(f.rows[5].raw, 50_000.0, "the capture's Accuracy 50,000 on 07000005");
    // Row 4 (stealth): within par → the FULL max [M 0x1984b0:225-233 — the else store of row+0x4
    // into row+0x10, ratio 1.0 at LAB_00198bdc]; over par → the penalty ramp. (The old port
    // fell through to the proportional arm and scored 0 for a clean run — the capture's
    // 100% / 25,000 is the disproof.)
    let idx = sim.drones.progress.table.iter().position(|e| e.hash == 0x0700_0005).unwrap();
    let d = f.stats.detected;
    let par4 = f.rows[4].count;
    let want4 = if par4 < d {
        let pen = if sim.drones.progress.table[idx].penalty == 0.0 { 0.03 } else { sim.drones.progress.table[idx].penalty };
        let fvar = (1.0 - (d - par4) as f32 * pen).max(0.0);
        let x = fvar * f.rows[4].target as f32;
        let t = x as i32;
        if x - t as f32 >= 1e-5 { (t + 1) as f32 } else { t as f32 }
    } else { f.rows[4].target as f32 };
    assert_eq!(f.rows[4].value, d, "row 4 carries the detection count");
    assert_eq!(f.rows[4].raw, want4, "row 4's raw per the stealth formula [M 0x1984b0:201-233]");
    // The health row (6) scores from the run's snapshot: target · min(count, hp) / count.
    let hp = f.stats.health as u32;
    assert_eq!(f.rows[6].value, hp, "row 6's fptoui of the health snapshot [M 0x1984b0:80]");
    assert_eq!(f.rows[6].raw, expect_generic(f.rows[6].count, hp, f.rows[6].target),
        "row 6's raw per the generic formula");
    // The time row (7): value = elapsed in 1/100 s, count = the par seconds [M 0x1984b0:104-139].
    // BOTH branches are ceil(max × par / max(elapsed_s, 1)) — under par the score RISES above max
    // [M :118-170; the capture's 01:19/09:00 → 170,887].
    let t = (f.stats.elapsed as f32 / 100.0).max(1.0);
    let x7 = f.rows[7].target as f32 * f.rows[7].count as f32 / t;
    let t7 = x7 as i32;
    let want7 = if x7 - t7 as f32 >= 1e-5 { (t7 + 1) as f32 } else { t7 as f32 };
    assert_eq!(f.rows[7].value, f.stats.elapsed, "row 7 carries the elapsed clock");
    assert_eq!(f.rows[7].raw, want7, "row 7's raw per the time formula [M 0x1984b0:118-170]");

    // The totals: base = Σ raw; bonus = base × the difficulty multiplier; last = base + bonus.
    let sum: f32 = f.rows.iter().map(|r| r.raw).sum();
    assert_eq!(f.base, sum as u32, "base = Σ raw [M 0x1984b0:288-289]");
    let mul = match f.difficulty { 1 => 0.0f32, 2 => 1.0, _ => 2.0 }; // ScoringTableDifMul [L]
    assert_eq!(f.bonus, (sum * mul) as u32, "bonus = base × dif [M 0x1984b0:294-300]");
    assert_eq!(f.last, f.base.wrapping_add(f.bonus), "last = base + bonus");
    // The medal chain on the run's total [M 0x208e78:98-146].
    let th = sim.drones.progress.table[idx].thresholds;
    let want_medal = if f.last >= th[3] { 4 } else if f.last >= th[2] { 3 }
        else if f.last >= th[1] { 2 } else if f.last >= th[0] { 1 } else { 5 };
    assert_eq!(f.medal, want_medal, "the medal compare chain");
    eprintln!("[scores] hand-off: base {} dif {} bonus {} last {} medal {}",
        f.base, f.dif, f.bonus, f.last, f.medal);
    eprintln!("[scores] tally: detected {} health {} elapsed {}", f.stats.detected, f.stats.health, f.stats.elapsed);

    // Read-mode: the production commits nothing (the results page's own GetScore(0) does).
    assert_eq!(f.best, sim.drones.progress.table[idx].best, "best is the entry's, untouched");
    assert_eq!(sim.drones.progress.table[idx].last, 0,
        "a mid-mission part exit must not stamp last (the read-mode production)");
    assert!(!sim.drones.progress.finished.is_none(), "mirrored on Progress::finished");
}
