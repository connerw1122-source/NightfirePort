//! fix-truck: the 07000005 truck park against the PCSX2 reference trace (tests/traces/05-truck.trace).
//!
//! The real game parks Bond in the truck bed at obj+0x30 = (29.885, 37.887, 51.656) and the channel
//! chain fires in order: 0x72 (the gate guard reaches his post, trace rel −317) → 0x70+0x65 (the
//! truck's parking plate, rel −300) → 0x73/0x74 (rel −273/−272) → 0x1D (rel 0) → the LOAD_LEVEL
//! trigger loads 07000006 on the next frame.
//!
//! Root cause of the port's miss (this file's probe history): the game's capsule height is the
//! DYNAMIC `obj+0xCC = obj+0x3C` [M Player_Update 0x193600:449, Player_CollisionHandler 0x1949a8:182]
//! — the stance's root height above the feet. The ride happens CROUCHED: the trace parks Bond at
//! pos−floor 0.607 (bed floor 37.280 [D block 0x2E of 0x020003be, measured through the port's own
//! shape parse]) over the standing offset 1.086 [L trace level start: 24.963 over 23.877]. The port
//! had a FIXED standing capsule (ROOT_HEIGHT 1.0347) so it parked at 38.376 — 0.489 too high — and
//! the ch 0x70 touch (a flat quad at y = 37.7418 [D script stream 6, key 8], whose touch capsule runs
//! from pos+0.15 up, radius 0.55) missed the plate by 0.23. The fix ports the stance-driven cc
//! (standing 1.0347183 [L], crouched 0.55 [L trace-derived ≈ the capsule radius, CROUCH_ROOT_HEIGHT]).
//!
//! Run with: `NF_ISO=… cargo test -p nf-view --test fix_truck_live -- --nocapture --test-threads=1`

use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The real game's parked Bond (player obj+0x30, trace frame 37173, the last 07000005 frame).
const GAME_PARK: [f32; 3] = [29.885, 37.887, 51.656];
/// The plate (ch 0x70's TouchOnce quad) the parked Bond must touch [D script stream 6 key 8].
const PLATE_Y: f32 = 37.7418;

/// The ride with the viewer's `--bond 100,26,108` setup path, crouched like the trace's play.
/// Asserts the parked position against the trace's numbers and the full channel order.
#[test]
fn truck_ride_parks_bond_at_the_game_height_and_ends_the_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    // The viewer's `--bond` setup (nf-view lib.rs headless path): a fresh Bond exactly there.
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    let mut prev_dis = sim.disabled;
    let mut first_on: [(u8, u32); 6] = [(0x22, 0), (0x72, 0), (0x70, 0), (0x65, 0), (0x73, 0), (0x1d, 0)];
    let mut crouch_armed = false;
    let mut park_tick = 0u32;
    let mut input = Input::default();
    for t in 1..=7000u32 {
        // Crouch as the ride begins (the hand-off re-enables Bond on the truck bed): the trace's
        // ride is crouched (pos−floor 0.607 at the park vs the standing 1.086), and a standing
        // capsule misses the plate by 0.23 (see the sister test below).
        if !sim.disabled && prev_dis && sim.bond.pos[0] < 95.0 { crouch_armed = true; }
        prev_dis = sim.disabled;
        if crouch_armed && !sim.bond.crouched { input.crouch = true; crouch_armed = false; } else { input.crouch = false; }
        sim.step_bond(input, false, false, false);
        for (ch, at) in first_on.iter_mut() {
            if *at == 0 && sim.drones.channels.on(*ch as u32) { *at = t; }
        }
        if sim.bond.pos[1] > 37.0 && sim.bond.pos[2] < 53.0 && sim.bond.grounded && park_tick == 0 && t > 4000 {
            park_tick = t;
        }
        if sim.drones.level_exit.is_some() { eprintln!("[t{t}] level_exit {:?}", sim.drones.level_exit); break; }
    }
    let bond = sim.bond.pos;
    eprintln!("[park] bond ({bond:?})  game {GAME_PARK:?}  crouched {}", sim.bond.crouched);
    eprintln!("[order] first-on ticks: {:?}", first_on);
    // The parked height against the trace (the task's bar: y within 0.05). x/z ride on the truck's
    // own park path — the game and port park within a bed width of each other.
    assert!((bond[1] - GAME_PARK[1]).abs() <= 0.05, "parked Bond y {:.3} must match the trace's 37.887 ±0.05 (crouched cc)", bond[1]);
    assert!((bond[0] - GAME_PARK[0]).abs() < 0.5 && (bond[2] - GAME_PARK[2]).abs() < 0.5, "parked Bond xz {:?} vs the game {:?}", &bond[0..1], &GAME_PARK[0..1]);
    // The plate touch geometry: the crouched capsule (a = pos+0.15, r 0.55) must reach the plate.
    assert!(bond[1] + 0.15 - PLATE_Y <= 0.55, "the touch capsule must reach the plate at y 37.7418");
    // The chain order: 0x72 (the guard at his post) → 0x70+0x65 (the plate) → 0x73/0x74 → 0x1D → exit.
    let (_, t22) = first_on[0];
    let (_, t72) = first_on[1];
    let (_, t70) = first_on[2];
    let (_, t65) = first_on[3];
    let (_, t73) = first_on[4];
    let (_, t1d) = first_on[5];
    assert!(t22 > 0 && t72 > t22, "ch 0x72 (the gate guard, trace rel −317) must fire after the gatehouse fight (0x22)");
    assert!(t70 > t72, "the plate (0x70, trace rel −300) fires after the guard is at his post (0x72, rel −317)");
    assert!((t65 as i32 - t70 as i32).abs() <= 2, "0x65 fires with 0x70 (the trace: the same frame, rel −300)");
    assert!(t73 > t70, "0x73 (trace rel −273) fires after the plate");
    assert!(t1d > t73, "0x1D (trace rel 0) fires after 0x73/0x74");
    assert_eq!(sim.drones.level_exit, Some(0x0700_0006), "the ride must end the level into 07000006");
}

/// Sister: the STANDING ride does not touch the plate — the standing capsule's touch axis starts at
/// pos+0.15 (38.53 at the standing park height), 0.78 above the plate at 37.7418, and the 0.55-radius
/// capsule reaches only to 37.98 — a 0.23 miss [M Toucher::player, Player_Collision 0x194df0's 0.55].
/// Proves the plate assertion can fail and pins the mechanism: it is the crouched stance that reaches
/// the plate, as the trace's play did.
#[test]
fn standing_ride_never_touches_the_parking_plate() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    for t in 1..=7000u32 {
        sim.step_bond(Input::default(), false, false, false);
        // The park plateau: the truck has parked and Bond stands still on it.
        if t > 5400 && sim.bond.pos[1] > 38.0 && sim.bond.pos[2] < 53.0 && t > 6400 {
            eprintln!("[standing] t{t} bond {:?}", sim.bond.pos);
            break;
        }
    }
    let bond = sim.bond.pos;
    eprintln!("[standing] parked bond ({bond:?}) 0x70:{}", sim.drones.channels.on(0x70) as i32);
    assert!(!sim.drones.channels.on(0x70), "the standing ride must not fire the parking plate (the capsule misses it by ~0.23)");
    // The miss: (pos + 0.15) − plate − 0.55 > 0 — the touch capsule's reach stops short of the plate.
    assert!(bond[1] + 0.15 - PLATE_Y - 0.55 > 0.0, "the standing touch capsule cannot reach the plate (miss {:.3})",
        bond[1] + 0.15 - PLATE_Y - 0.55);
}
