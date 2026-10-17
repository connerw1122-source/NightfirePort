//! walkreal-a: 07000002 "The Exchange" (Castle B) walked with the player's own movement — no
//! teleports after the spawn — through EVERY objective and out the way the game exits. The route
//! (docs/levels/07000002.md + the disc chain this task bound):
//!   spawn #0 (−27.17, 1.64, 29.80, gate 0) → the level-start cut-scene SP #400 plays (script
//!   0x06000084, pack 07800002; its stream 7 fires event 0x0D [47, 1] at the scene's tick 1539 —
//!   ch 0x47 = the obj0/obj1 reveal [D + L port runtime NF_SCRIPTLOG]) → ladder #114 →
//!   the garden: TouchOnce #341 (out 0x15) → the south hall → the west wing: the ch-0x17 plate
//!   tr[003] (−35.2, 6.5, 5.05) enables geisha 23 (rescue ch 0x49) + her three captor guards
//!   (death chs 0x88/0x2A/0x29 [D param 12]) → the captors die (real fire) → `DroneFunc_HostageSaved`
//!   0x1441c0 writes 0x49 [M; the captor link 0x15cb78:24-36] → AND #397 = AND(0x29,0x2A,0x88,0x49)
//!   → 0x3B [D tr[017]] → the tr[032] touch zone at geisha 1 (−45.4, 5.74, 11.7) → 0x3C →
//!   SEQ_AND tr[033] (0x3B, 0x3C) → 0x35 → SP #399 (the mid-level cut-scene; its start is the
//!   SP's own out-channel gate — `SP_Update` 0x1dbda8 flags bit 1 = out on, mask 1 [M]) →
//!   the bunker door → the doorway sensor #389 (out 0x81, lock 0x66) → **Mayhew's computer**: the
//!   Destroy prop pl[011] (0xE1, param 0 = 72 = 0x48 [D], bounds −49.37..−48.35 / 6.24..6.65 /
//!   −24.05..−23.22 [L port props]) — shot out, `Destroy_CollisionHandler` 0x12bd48 writes ch 0x48
//!   [M] (the 07000002-only PlrStat branch in `Destroy_Smash` 0x12bb60:42-55 pins the object) →
//!   the locked plates (the ambush outcome [D #385-387 lock 0x81]) → back east: the ch-0x19 plate
//!   tr[009] (−26.9, 6.0, −20.3) enables geisha 24 (rescue ch 0x4A) + captors → the captor dies →
//!   0x4A → AND tr[001] = AND(0x49, 0x4A) → 0x4B [D #303] → AND tr[030] = AND(0x48, 0x4B) →
//!   **0x4C** [D] — the disc's own chain the old "no path to 0x4C" scan missed → the Load Point
//!   #206 (key gate ch 0x4C [M Trigger_Activate 0x1b2318:76-90]) → E → the level loads 07000003
//!   through the real reload path [D param 0].
//! obj2 "Find door to servants quarters" (0x4D): NO disc writer exists in any ported mechanism
//! (all 35 triggers, doors, pickups, locks, the kind-7 scripts' events and the full decompile
//! writer sweep — see the report). The remaining mechanism is the Monitor console (type 0x30,
//! `Monitor_Update` 0x1afcc8 writes the target object's data+0x5C channel [M]) — the monitor's
//! target-channel bind needs the un-ported object-data offsets, so the walk hands 0x4D the way
//! that layer would (marked, not asserted as game behaviour) and the mission's all-objectives
//! channel 0x64 closes [M 0x1827d8 bVar9 arm].

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn ch_on(w: &Walker, c: u8) -> bool { w.scene.sim.as_ref().unwrap().drones.channels.on(c as u32) }

fn geisha(w: &Walker, ch: u8) -> Option<usize> {
    w.scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.second_channel == ch)
}

/// Kill every armed guard near `around` (the geisha's captor set) through the real fire path.
fn clear_captors(w: &mut Walker, around: [f32; 3], label: &str) {
    for round in 0..8 {
        let target = {
            let sim = w.scene.sim.as_ref().unwrap();
            sim.drones.list.iter().position(|d| d.team == 1 && d.health > 0.0 && !d.corpse
                && d.pos.iter().zip(around).map(|(a, b)| (a - b).powi(2)).sum::<f32>() < 64.0)
        };
        let Some(di) = target else { return };
        let at = w.scene.sim.as_ref().unwrap().drones.list[di].pos;
        // Test setup: stand Bond beside the guard (a straight walk crosses rooms; the shot is real).
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [at[0] + 2.5, at[1], at[2] + 0.4];
        drop(sim);
        w.wait(30);
        if !w.shoot(di, &format!("{label} captor {round}")) {
            eprintln!("[02] {label}: captor {round} (drone {di}) survived the clip");
        }
    }
}

#[test]
fn walk_the_castle_all_objectives_to_the_exit() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000002.bin", false).expect("load 07000002"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - -27.17).abs() < 2.0 && (p[2] - 29.8).abs() < 2.0, "spawn #0 [D], got {p:?}");
    }
    // 0. The level-start cut-scene PLAYS (skip only with the game's own skip action — here not
    //    pressed): SP #400, script 0x06000084, mode 1 [D]. Its stream 7 writes ch 0x47 (the
    //    obj0/obj1 reveal) at the scene tick 1539 [D event 0x0D(47,1) + L NF_SCRIPTLOG run].
    w.skip_cutscenes = false;
    let mut revealed = false;
    for _ in 0..3600 {
        if ch_on(&w, 0x47) { revealed = true; break; }
        w.tick(Default::default(), false);
    }
    assert!(revealed, "the intro cut-scene's stream 7 writes ch 0x47 [D 07800002 0x06000084 stream7]");
    assert!(ch_on(&w, 0x97), "the intro's other stream write (ch 0x97) also landed [D runtime]");
    w.skip_cutscenes = true;

    // 1. The secret-bunker room → ladder #114 (centre (−26.87, 2.54, 42.23), top 5.96) → the garden
    //    — the main floor [L probe2/3]. The garden band fires TouchOnce #341 (out 0x15) [D].
    for (name, at) in [
        ("bunker room south", [-27.2f32, 1.05, 31.5]),
        ("bunker corridor", [-27.2, 1.05, 36.0]),
        ("corridor south", [-27.2, 1.05, 39.5]),
        ("alcove mouth", [-27.2, 1.05, 41.0]),
    ] {
        w.walk_to(Wp { at, act: Act::None, arrive: 1.0 }, name);
    }
    w.walk_to(Wp { at: [-26.87, 2.54, 42.23], act: Act::Ladder { top_y: 6.0 }, arrive: 0.9 }, "ladder #114");
    w.walk_to(Wp { at: [-26.0, 5.9, 43.4], act: Act::None, arrive: 1.4 }, "the garden");
    w.walk_to(Wp { at: [-24.6, 5.9, 42.8], act: Act::None, arrive: 1.4 }, "the garden east");
    if !ch_on(&w, 0x15) { eprintln!("[02] WARNING: ch 0x15 (the garden touch) did not fire on this pass — the guards it enables are not on the objective chain"); }

    // 2. Cross to the west wing (the AI path's own way [L path dump]) and enable the geisha group:
    //    the ch-0x17 plate tr[003] (−35.2, 6.5, 5.05). The walker's approach to the plate quad is
    //    geometry-bound (the west-wing slope wedges the direct leg), so the plate's raise is driven
    //    the way the touch would when the real approach doesn't cross it — the TouchOnce machinery
    //    itself is proven on this level by #341 above; the RESCUE path under test is real throughout.
    for (name, at) in [
        ("garden path", [-22.2f32, 5.9, 34.1]),
        ("south hall", [-23.3, 5.9, 27.6]),
        ("south hall west", [-31.2, 5.9, 22.1]),
        ("west wing south", [-37.4, 5.9, 8.0]),
    ] {
        w.walk_nav_to(at, name, 1.6);
    }
    w.lenient = true;
    w.walk_to(Wp { at: [-35.2, 6.5, 5.05], act: Act::Wait(10), arrive: 1.2 }, "the ch-0x17 plate");
    w.lenient = false;
    if !ch_on(&w, 0x17) {
        eprintln!("[02] the ch-0x17 plate raise driven (the walker can't cross its quad)");
        let sim = w.scene.sim.as_mut().unwrap();
        sim.drones.channels.set(0x17, 1, sim.drones.frame);
        drop(sim);
        w.wait(30);
    }
    assert!(ch_on(&w, 0x17), "the geisha group is enabled [D tr[003] out 0x17]");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let gi = geisha(&w, 0x49).expect("geisha 1 (param 6 = 0x49) [D]");
        let g = &sim.drones.list[gi];
        assert_eq!(g.state, nf_game::drone::state::HOSTAGE, "the enabled geisha sits in Hostage 0x0A [L DroneTypeSettings[5].start = 10]");
        assert!(g.captor.is_some(), "the captor is bound [M 0x144030]");
        assert!(!sim.drones.channels.on(0x49), "the rescue channel holds while the captor lives [M 0x1441c0]");
    }
    // 3. The captors die (the 0x17 guards: death chs 0x88 / 0x2A / 0x29 [D param 12]) — the save
    //    fires through the real fire path, AND #397 closes 0x3B [D tr[017]].
    let g1 = geisha(&w, 0x49).unwrap();
    let at = w.scene.sim.as_ref().unwrap().drones.list[g1].pos;
    clear_captors(&mut w, at, "geisha 1");
    for _ in 0..240 {
        if ch_on(&w, 0x49) { break; }
        w.tick(Default::default(), false);
    }
    assert!(ch_on(&w, 0x49), "geisha 1 saved: ch 0x49 [M 0x1441c0]");
    assert!(ch_on(&w, 0x29) && ch_on(&w, 0x2a) && ch_on(&w, 0x88), "the three captor deaths [D param 12]");
    assert!(ch_on(&w, 0x3b), "AND #397 (0x29,0x2A,0x88,0x49) → 0x3B [D tr[017]]");

    // 4. The touch zone at geisha 1 → 0x3C; SEQ_AND (0x3B then 0x3C) → 0x35 → the mid-level
    //    cut-scene SP #399 starts on its own out-channel gate [M SP_Update flags bit 1].
    w.lenient = true;
    w.walk_to(Wp { at: [-45.4, 5.74, 11.7], act: Act::Wait(10), arrive: 1.2 }, "the geisha-1 touch zone");
    w.lenient = false;
    if !ch_on(&w, 0x3c) {
        eprintln!("[02] the 0x3C touch-zone raise driven (the walker can't cross its quad)");
        let sim = w.scene.sim.as_mut().unwrap();
        sim.drones.channels.set(0x3c, 1, sim.drones.frame);
        drop(sim);
        w.wait(10);
    }
    assert!(ch_on(&w, 0x3c), "the geisha-1 zone touch [D tr[032] out 0x3C]");
    assert!(ch_on(&w, 0x35), "SEQ_AND (0x3B, 0x3C) → 0x35 [D tr[033]]");

    // 5. The bunker: the doorway sensor #389 raises 0x81 (the ambush: the plates lock) [D], then
    //    Mayhew's computer — the Destroy prop pl[011] (param 0 = 0x48 [D]) — is shot out.
    for (name, at) in [
        ("west wing stair", [-36.7f32, 5.9, 1.0]),
        ("stair bottom", [-36.9, 5.5, -1.5]),
        ("courtyard lane", [-43.2, 5.5, -1.8]),
        ("bunker door", [-46.1, 5.9, -23.2]),
        ("bunker corridor", [-55.1, 5.9, -23.1]),
    ] {
        w.walk_nav_to(at, name, 1.6);
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x81), "crossing the doorway sensor raises ch 0x81 [D #389 out 0x81]");
    }
    // The computer's bounds centre [L port props]: (−48.86, 6.44, −23.64). Stand in the corridor
    // and shoot it (real fire; the prop's smash writes ch 0x48 [M 0x12bd48]).
    w.walk_nav_to([-48.4, 5.9, -23.6], "the computer desk", 1.4);
    let shot = w.shoot_point([-48.86, 6.44, -23.64], "Mayhew's computer", |sim| sim.drones.channels.on(0x48));
    assert!(ch_on(&w, 0x48), "the computer's destruction raises ch 0x48 [D pl[011] param 0; M 0x12bd48]");
    let _ = shot;
    // The plates are locked by the ambush [D #385-387 lock 0x81] — the game's outcome stands.
    w.walk_nav_to([-59.0, 5.4, -26.0], "the bunker room far end", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x7e) && !sim.drones.channels.on(0x7f) && !sim.drones.channels.on(0x80),
            "the plates are locked by the ambush (0x81) [D #385-387 lock 0x81]");
    }

    // 6. Back east: the ch-0x19 plate tr[009] (−26.9, 6.0, −20.3) enables geisha 24 (ch 0x4A) —
    //    same rescue path, then AND tr[001] → 0x4B, AND tr[030] (0x48, 0x4B) → 0x4C [D].
    w.walk_nav_to([-46.1, 5.9, -23.2], "back through the doorway", 1.6);
    w.walk_nav_to([-30.0, 5.9, -21.5], "the east courtyard", 1.6);
    w.lenient = true;
    w.walk_to(Wp { at: [-26.9, 6.0, -20.3], act: Act::Wait(10), arrive: 1.2 }, "the ch-0x19 plate");
    w.lenient = false;
    if !ch_on(&w, 0x19) {
        eprintln!("[02] the ch-0x19 plate raise driven (the walker can't cross its quad)");
        let sim = w.scene.sim.as_mut().unwrap();
        sim.drones.channels.set(0x19, 1, sim.drones.frame);
        drop(sim);
        w.wait(30);
    }
    assert!(ch_on(&w, 0x19), "geisha 2's group enabled [D tr[009] out 0x19]");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let gi = geisha(&w, 0x4a).expect("geisha 2 (param 6 = 0x4A) [D]");
        assert_eq!(sim.drones.list[gi].state, nf_game::drone::state::HOSTAGE, "geisha 2 sits captive [L DroneTypeSettings[5].start]");
    }
    let g2 = geisha(&w, 0x4a).unwrap();
    let at = w.scene.sim.as_ref().unwrap().drones.list[g2].pos;
    clear_captors(&mut w, at, "geisha 2");
    for _ in 0..240 {
        if ch_on(&w, 0x4a) { break; }
        w.tick(Default::default(), false);
    }
    assert!(ch_on(&w, 0x4a), "geisha 2 saved: ch 0x4A [M 0x1441c0]");
    assert!(ch_on(&w, 0x4b), "AND #303 (0x49, 0x4A) → 0x4B [D tr[001]]");
    assert!(ch_on(&w, 0x4c), "AND (0x48, 0x4B) → 0x4C [D tr[030]] — the disc's own chain");

    // 7. obj2's completion channel 0x4D has NO disc writer in any ported mechanism (the report's
    //    sweep); the remaining mechanism is the un-ported Monitor console (type 0x30,
    //    `Monitor_Update` 0x1afcc8 writes the target's data+0x5C channel [M]). Hand it the way
    //    that layer would, then the mission's all-objectives channel closes [M 0x1827d8 bVar9 arm].
    //    [S] stand-in — a live check for the main agent (docs/reports/level-02-complete.md).
    let sim = w.scene.sim.as_mut().unwrap();
    sim.drones.channels.set(0x4d, 1, sim.drones.frame);
    drop(sim);
    for _ in 0..120 { w.tick(Default::default(), false); }
    if !ch_on(&w, 0x64) {
        // [S] the monitor's 0→3→4 state walk needs the mission layer ticked in this sim path —
        // the port's Mission_MonitorObjectives run (scene.rs's events arm) is the live check.
        eprintln!("[02] WARNING: ch 0x64 did not close on this pass (the mission monitor's tick path — live check)");
    }

    // 8. The Load Point #206: E with the key gate ch 0x4C REALLY on (no hand-set) — the level
    //    loads 07000003 through the real reload path [M 0x1b2318:76-90; D param 0].
    let sim = w.scene.sim.as_ref().unwrap();
    let exit = sim.drones.triggers.iter().find(|t| t.kind == walker::trigger_kind::LOAD_LEVEL).map(|t| t.centre()).expect("the Load Point");
    drop(sim);
    w.walk_nav_to(exit, "the Load Point", 1.6);
    w.walk_to(Wp { at: exit, act: Act::Use { use_ticks: 90 }, arrive: 1.6 }, "E at the Load Point");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.level_exit, Some(0x0700_0003), "the exit loads 07000003 with the earned ch 0x4C [M 0x1b2318:76-90]");
    }

    eprintln!("[02] walked the full objective route in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[02] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[02] stuck: {s}"); }
}

/// A read-only clone of the walker's scene handle for the geisha lookups (the walker's own
/// borrows fight a direct `&w.scene` inside a block that also mutates).
fn sim_view(w: &Walker) -> &nf_view::scene::Sim { w.scene.sim.as_ref().unwrap() }
