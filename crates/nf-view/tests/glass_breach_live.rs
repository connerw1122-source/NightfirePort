//! glass-breach-guards (750p): the Mayhew-start glass breach — the guards charge through the
//! entry-hall windows.
//!
//! The user, 2026-10-16 evening: "beginning of mayhew level guards are supposed to break through
//! the glass". The level is 07000001 (Paris Prelude, Castle A — the Mayhew estate's grounds and
//! entry hall, docs/levels/07000001.md). The game's mechanism [M]:
//!
//! - **`Break_Update` 0x12bfd8:23-34** — the class-2 (drone/player) smash arm, gated on
//!   `GameState._12_4_ == 0x7000001` and the pane's kind == 2: every drone whose collision
//!   registered against the pane this tick drains `health -= SqDist3D(hitter+0x30, hitter+0x40)
//!   × 20.0` (the tick's own movement, the movement pass copying +0x30 → +0x40 after the move
//!   [M 0x1336f8:212]); at health < 0 → `Break_Kill` (the pane leaves the world, the glass sound
//!   0x2e, no RNG draw for kind 2 [M 0x12b970:60-76]). The Mayhew kind-2 panes all carry health
//!   0.0 [D placements], so ANY movement through the pane plane shatters it.
//! - The intro cut-scene (SP #277, script 0x06000087) posts **ch 0xB = 1** at ~2119 [D script
//!   decode; the port's walk_07000001_live pins the timing], which wakes the entry-hall guards
//!   (7 drones waiting on ch 0xB: placements 158/161/168/169/261/298/299 [D params — param 3
//!   = 11]); their AI charges Bond in the courtyard and runs straight into the entry-hall glass
//!   wall — kind-2 panes **130/136/155** (bounds: 130 lo [−7.94, 5.40, −1.01] hi [−2.07, 7.85,
//!   −1.01]; 136 [−1.91, 5.34, −1.167]..[−0.09, 7.95, −1.167]; 155 the transom above 136
//!   [D nf_view bounds]) — and shatters them on contact.
//!
//! The real path: `nf_view::load` + `Sim::step_bond` only — the intro plays itself (no input,
//! the cut-scene), then Bond stands while the woken guards do their own AI. No teleports; Bond
//! holds the spawn point the whole run, like the player watching the breach.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The kind-2 entry-hall wall panes by placement id [D the level's placements + the bounds dump].
const ENTRY_PANES: [usize; 3] = [130, 136, 155];

#[test]
fn the_woken_guards_smash_through_the_entry_hall_glass() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.level, 0x0700_0001);
        // [D placements] the entry-hall panes exist, are kind 2, health 0, and start intact.
        for pl in ENTRY_PANES {
            let b = sim.drones.breakables.list.iter().find(|b| b.placement == pl)
                .unwrap_or_else(|| panic!("breakable placement {pl} exists [D]"));
            assert_eq!(b.kind, 2, "pane {pl} is the glass kind [D]");
            assert_eq!(b.health, 0.0, "pane {pl} carries health 0 — any brush kills [D param 1]");
            assert!(!b.broken, "pane {pl} starts intact [D]");
        }
        assert!(!sim.drones.channels.on(0xb), "ch 0xB is off at spawn [L ee_start_07000001.bin 0x26fc90]");
    }
    // The intro cut-scene plays itself; its stream 8 posts ch 0xB [D script decode]. Bond holds
    // the spawn (the player watches the cut-scene — no input at all).
    let mut enabled_at = None;
    for t in 0..3000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.channels.on(0xb) { enabled_at = Some(t); break; }
    }
    enabled_at.expect("the intro script 0x06000087 writes ch 0xB [D stream-8 event 0xD]");
    // The breach squad: the 0xF1 spawner #297 (mode 2, group 1, start ch 11 [D placement params,
    // the port's Spawner parse]) holds 5 group-1 guards as templates (present=false) and spawns
    // them when ch 11 fires; the contact (ch 11, group 0) walks his own route.
    let waiters: Vec<usize> = {
        let sim = scene.sim.as_ref().unwrap();
        (0..sim.drones.list.len()).filter(|&i| sim.drones.list[i].group == 1).collect()
    };
    assert_eq!(waiters.len(), 5, "the breach squad is the 5 group-1 templates [D spawner #297 group 1]");

    // The breach: the woken guards charge. Watch the entry-hall panes and the guards' side of
    // the glass wall (the pane plane sits at z ≈ −1.0..−1.17; the courtyard side is z BELOW it).
    let mut broken_at: Option<(u32, usize)> = None;
    let mut crossed: Option<(u32, usize)> = None;
    let mut seen = Vec::new();
    for t in 0..3600u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        for pl in ENTRY_PANES {
            if broken_at.is_none() {
                if let Some(b) = sim.drones.breakables.list.iter().find(|b| b.placement == pl) {
                    if b.broken { broken_at = Some((t, pl)); }
                }
            }
        }
        for &i in &waiters {
            let dr = &sim.drones.list[i];
            // Through the wall: the guard's position crossed the pane plane to the courtyard side.
            if dr.pos[2] < -1.25 && crossed.is_none() { crossed = Some((t, i)); }
        }
        if t % 300 == 0 {
            let broken: Vec<usize> = sim.drones.breakables.list.iter().filter(|b| b.broken).map(|b| b.placement).collect();
            let mut line = format!("t{t}: broken {broken:?}");
            for &i in &waiters { let dr = &sim.drones.list[i]; line += &format!(" | g{i} z {:.2} st {:#x}", dr.pos[2], dr.state); }
            seen.push(line);
        }
        if broken_at.is_some() && crossed.is_some() { break; }
    }
    for l in &seen { eprintln!("[breach] {l}"); }
    let (_, pl) = broken_at.unwrap_or_else(|| panic!(
        "the woken guards shatter an entry-hall pane (130/136/155) on contact [M Break_Update 0x12bfd8:23-34 — the class-2 arm, level 0x7000001, kind 2, health −= SqDist(pos, prev)·20 → Break_Kill] — trace:\n{}",
        seen.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")));
    eprintln!("[breach] pane {pl} broke");
    let (t, gi) = crossed.unwrap_or_else(|| panic!(
        "a woken guard crosses the broken pane into the courtyard (z < −1.25) — the game's breach scene"));
    eprintln!("[breach] guard {gi} crossed at t{t}");
    // The kill leaves the pane out of the world: no further hits register [M 0x12b970:60] — and
    // the sister: a NON-entry pane (the east-wing radioroom glass 121) stays intact unless a
    // guard paths there (pin the observed state, not an absolute).
    let sim = scene.sim.as_ref().unwrap();
    let east = sim.drones.breakables.list.iter().find(|b| b.placement == 121).expect("pane 121 exists [D]");
    eprintln!("[breach] east pane 121 broken = {} (informational)", east.broken);
}
