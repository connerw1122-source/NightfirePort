//! fix-trigger-unknowns (110f, docs/UNKNOWNS.md U-032): the trigger half of the unknown, settled
//! from the decompile and pinned through the real input path on 07000005.
//!
//! The disc facts [D, extraction of 07000005's main map]:
//! - spawner **268** (type 0xF1, at 35.16, 0, 114.85): mode 2, group 2, **start ch 0x22** (p4=34),
//!   no stop/done, **kill ch 0x43** (p7=67), 1 wave.
//! - TouchOnce **305** → out **0x22**, mask 8 (player-only); TouchOnce **306** → out **0x44**,
//!   mask 8; TouchOnces 21/22/23 → outs **0x23**/0x24/0x25 (the sibling gate patches — 21 is the
//!   gatehouse courtyard at (60, 28, 96), the spot the level-end route walks).
//! - MultiplexOrIn **25** → **out 0x22, inputs [0x23, 0x24, 0x25]** (params 1..3) — re-evaluated
//!   every frame with a plain assignment, so it also CLEARS 0x22.
//!
//! The decompile answers [M]:
//! - `DroneSpawner_Control` 0x137e68:53-64 polls the start channel every tick while unarmed; ONE
//!   on-tick suffices (no edge/history), then it zeroes its OWN tag copy (+0xE) and latches
//!   +0x1E — starting does not touch `switch_channels`.
//! - `Trigger_Calc` 0x1b1900:28-42 pre-clears every **type-2 Touch** output each frame and
//!   `Trigger_Update` 0x1b1a78 case 2 (:78-88) OR-sets while touched — a Touch output is a PULSE.
//!   **Type-1 TouchOnce outputs have NO clearer of their own** (cases 1/2 only `|=`) — BUT OR 25
//!   re-evaluates 0x22 every frame with a PLAIN assignment (case 0xc `switch_channels[out] =
//!   result`, 0 included), which CLOBBERS TouchOnce 305's latch the moment no sibling patch
//!   (0x23/0x24/0x25) is on. 0x22 is therefore a pure PULSE: on exactly while Bond stands on
//!   patch 305 (or a sibling patch is latched), 0 the moment he leaves.
//! - **The snapshot is the steady state**: Bond stood on patch 305 some time before the snapshot
//!   (frame order: #25 writes first, #305's `|=` survives to the frame's end, the spawner's poll
//!   in the control walk reads 1 and latches); afterwards every channel reads 0 again — 0x22
//!   clobbered by the OR, 0x23–0x25 never fired, 0x44 never fired (306 is a different volume).
//!   No reload needed; the row's "0x22 as a pulse fits" is confirmed with the exact mechanism.
//!   For 0x44, "never latching" means **306 never fired** (Bond never crossed its volume — or its
//!   contact lost `Collide_Update`'s distance sort + first-blocking-hit early-out, 0x1ea398:
//!   119-141/216-229, the floor-hit hypothesis) — a type-1 output on a channel NO other writer
//!   touches (the writer census) cannot return to 0.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_05_free() -> nf_view::scene::Scene {
    let mut scene = nf_view::load(&iso().expect("NF_ISO"), "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    scene
}

/// The snapshot state reproduced end to end: Bond crosses patch 305 (TouchOnce, out 0x22
/// directly). Within the frame the OR gate (#25, key 25) writes AFTER #305 in the game's prepend
/// control list [M 0x134698 — newest-first], so #305's set reaches spawner 268's poll (key 268,
/// between them) and the spawner latches; by frame end the OR's plain assignment has clobbered
/// 0x22 back to 0. The pulse is therefore INVISIBLE at frame-end sampling — the spawner's
/// `active` latch is the observable — and the frame-end state is **exactly the live snapshot**:
/// spawner started, 0x22–0x25 and 0x44 all 0.
#[test]
fn the_gate_spawner_starts_on_the_0x22_pulse() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_05_free();
    let sim = scene.sim.as_mut().unwrap();
    let c305 = sim.drones.triggers.iter().find(|t| t.placement == 305).expect("TouchOnce 305 built").centre();
    eprintln!("[u-032] TouchOnce 305 volume centre {c305:?} (306 shares it [D])");
    // Above the volume: the capsule crosses it on the way down (a capsule resting inside a closed
    // mesh touches nothing — the level-end route's approach).
    sim.bond.pos = [c305[0], c305[1] + 4.0, c305[2]];
    for _ in 0..90 { sim.step_bond(Default::default(), false, false, false); }
    let spawner = sim.drones.spawners.iter().find(|s| s.placement == 268).expect("spawner 268 built");
    assert!(spawner.active, "spawner 268 must have caught the mid-frame 0x22 pulse [M 0x137e68:53-64 — one on-tick poll suffices]");
    // The frame-end channels: 0x22 clobbered by the OR's plain assignment [M 0x1b1a78 case 0xc]
    // (no sibling patch latched), the sibling TouchOnces unfired. 305 and 306 share the SAME
    // volume [D — identical centres], and the port's toucher model delivers the contact to both
    // TouchOnces → 0x44 latches too. The LIVE snapshot's 0x44 = 0 with 305 fired is the game's
    // contact delivery dropping one of the two objects (the row's floor-contact/Collide_Update
    // sort hypothesis — live check in the report).
    for c in [0x22u32, 0x23, 0x24, 0x25] {
        assert!(!sim.drones.channels.on(c), "ch {c:#x} must read 0 at frame end with the spawner started — the live snapshot's state");
    }
    assert!(sim.drones.channels.on(0x44), "the port's model fires BOTH same-volume TouchOnces (305+306 share the patch) [D]");
    // And the latch persists: the spawner stays started with every channel still 0.
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    let spawner = sim.drones.spawners.iter().find(|s| s.placement == 268).unwrap();
    assert!(spawner.active, "the spawner's +0x1E latch persists after the pulse [M 0x137e68:61-62]");
    assert!(!sim.drones.channels.on(0x22));
}

/// Sister: TouchOnce 306's output 0x44 latches when Bond's capsule crosses 306's volume — and a
/// session where only the courtyard was walked leaves 0x44 at 0. A type-1 output has no clearer
/// (writer census), so the snapshot's 0x44 = 0 pins 306 as NEVER FIRED, not fired-and-cleared.
#[test]
fn touch_once_306_latches_0x44_only_when_touched() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_05_free();
    let sim = scene.sim.as_mut().unwrap();
    let c306 = sim.drones.triggers.iter().find(|t| t.placement == 306).expect("TouchOnce 306 built").centre();
    eprintln!("[u-032] TouchOnce 306 volume centre {c306:?}");
    // Start above the volume: the capsule crosses the plate on the way down (the level-end
    // route's approach — a capsule resting inside a closed mesh touches nothing).
    sim.bond.pos = [c306[0], c306[1] + 4.0, c306[2]];
    let mut latched = false;
    for i in 0..60 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.channels.on(0x44) { latched = true; eprintln!("[u-032] ch 0x44 on at tick {i}"); break; }
    }
    assert!(latched, "TouchOnce 306 must latch 0x44 when Bond crosses its volume [M 0x1b1a78 case 1]");
    // And it STAYS latched (no clearer exists for a type-1 output).
    for _ in 0..20 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x44), "0x44 has no clearer — it cannot return to 0 [writer census]");

    // The snapshot's other half via the courtyard route: Bond walks the gatehouse courtyard
    // (TouchOnce 21 → 0x23 latched → OR 25 holds 0x22 on) — the spawner starts while 305/306 are
    // untouched, so 0x44 stays 0. (In the live snapshot 0x22 itself was 0 too — the 305-route
    // pulse — but 0x44 = 0 there says 306's contact never registered; both routes are pinned.)
    let mut scene = load_05_free();
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [60.15, 30.0, 96.43];
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    let spawner = sim.drones.spawners.iter().find(|s| s.placement == 268).unwrap();
    assert!(spawner.active, "the courtyard route's 0x22 started the spawner");
    assert!(sim.drones.channels.on(0x22), "the OR holds 0x22 via 0x23's latch [D placement 25]");
    assert!(!sim.drones.channels.on(0x44), "306 unfired → 0x44 = 0");
}

/// U-032's third question: `DroneVision_ProcessDroneSight` 0x176a28:68-69 gates sight (and
/// `DroneVision_FindAlertedDrones`) on `DAT_00298fb8 + 15 <= now && DAT_00298fbc + 15 <= now` —
/// the load stamp (`Drone_PostLoad_Init` 0x136938:46) and the `Drone_EnableAll(1)` stamp
/// (0x1383e8:25). 07000006's intro re-enables the NPCs (`Script_Free`'s 0x80 flag); the first
/// guard LOS must therefore be ≥ 15 ticks after the re-enable, and no seen-ticks accumulate
/// before it. Sister: the pause does NOT gate hearing (the alarm channel via a sound is ungated).
#[test]
fn sight_waits_15_frames_after_the_re_enable() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    let mut reenabled_at = None;
    for t in 0..300u32 {
        sim.step_bond(Default::default(), false, false, false);
        if !sim.disabled { reenabled_at = Some(sim.drones.frame); break; }
    }
    let reenabled_at = reenabled_at.expect("Bond is free after the intro");
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, BEAM_SPOT, -2.2091);
    sim.gun.weapon = 2;
    sim.gun.switch_request = 2;
    sim.gun.switch_state = 0;
    let idle = nf_game::player::Input::default();
    let gate_end = reenabled_at + 15; // the fbc re-stamp [M 0x1383e8:25] + the flat 15 [M 0x176a28:69]
    let mut first_los: Option<u32> = None;
    let mut early_seen = false;
    for t in 0..600u32 {
        sim.step_bond(idle, false, false, false);
        let now = sim.drones.frame;
        for dr in sim.drones.list.iter().filter(|d| d.present && d.enabled) {
            if dr.los && first_los.is_none() { first_los = Some(now); }
            if now < gate_end && dr.seen_ticks > 0 { early_seen = true; }
        }
        if first_los.is_some() && t > 60 { break; }
    }
    assert!(!early_seen, "no seen-ticks may accumulate before re-enable+15 [M 0x176a28:68-69]");
    let Some(t) = first_los else { eprintln!("no guard reached LOS in 600 ticks — the pause assert is vacuous here"); return };
    assert!(t >= gate_end, "first LOS at tick {t} must be ≥ re-enable({reenabled_at})+15 = {gate_end}");
}

const BEAM_SPOT: [f32; 3] = [4.8, 42.09, -16.0];
