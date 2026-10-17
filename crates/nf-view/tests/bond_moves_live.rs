//! bonus-007: the 007 bonus ("Bond moves") award, end to end on 07000005 through the real path.
//!
//! The award chain [M ACTION.ELF]: `Trigger_Calc` 0x1b1900:12-20 polls switch channels 0x65..=0x6E
//! and calls `PlrStat_DoneBondMoment(0, ch)` 0x199448 per risen channel — behind the mission gate
//! and the `BondMoments[id−0x65] == 0` new-moment gate it sets the flag, bumps PlrMissionStats+0x20
//! (the Mission Stats' Bond Moves row) and writes `glb_blokes[0]+0x948 = 200` — the HUD's 007-icon
//! flash timer, counted down by FRAME_RATE_MUL (2 at 60 Hz) per `HUD_UpdateHealthPane` 0x19b9c8:232-260
//! pane update (the icon animates only ≥ 101). The chime is level data: a SoundTrigger placement
//! (type 0xF9, `SoundTrigger_Create` 0x1b2628 / `SoundTrigger_Update` 0x1b2790) whose ON channel is
//! the bond channel plays the level's sting — on 07000005 placement 82 (gate 0x65) plays id 331
//! (0x14B `SFX_BOND_MOMENT_BM_CAS_BEAM` [D placement + the ELF name→id table]) and placement 81
//! (gate 0x69) plays 332 (0x14C `..._CAS_TRUCK`).
//!
//! The ride itself is the real path `gate_guard_truck_live.rs` drives: the cold start → the gatehouse
//! porch plate (ch 0x5C) → the hand-off cut-scene → the crouched truck ride → the park plate 0x70 →
//! mux 0x316 = AND(0x70, 0x71, 0x72) → ch 0x65 = bond move 101 [D placements 316/311-314/318;
//! tests/traces/05-truck.trace]. Bond move 105 (0x69, mux 0x80 = AND(127,128,129,133), the four
//! courtyard plates) does NOT fire on this ride — the sister.
//!
//! Run with: `NF_ISO=… cargo test -p nf-view --test bond_moves_live -- --nocapture --test-threads=1`

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::Input;
use walker::Walker;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The award tick: the count 0→1, the flag, the flash 200 and the chime, at the ch-0x65 tick.
#[test]
fn truck_ride_awards_bond_move_101_with_flash_and_chime() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"));
    w.skip_cutscenes = false;
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 104.95).abs() < 2.0 && (p[2] - 108.83).abs() < 2.0,
            "the cold start is placement #26 (104.95, 25.12, 108.83) [D], got {p:?}");
        assert_eq!(sim.drones.progress.stats().bond_moments, 0, "no award at the start");
        assert_eq!(sim.drones.health.bond_moment_flash, 0, "no flash at the start");
    }
    w.wait(30);
    w.walk_nav_to([102.0, 25.0, 110.5], "courtyard by the gatehouse porch", 1.6);
    assert!(w.climb_stairs([99.5, 26.0, 107.0], "onto the gatehouse porch", 1.0, 600),
        "Bond must climb the gatehouse porch step");
    w.walk_to(walker::Wp::new(99.0, 26.0, 106.6).r(0.7), "onto the ch-0x5C plate");
    let mut saw_5c = 0u32;
    for _ in 0..2400u32 {
        w.tick(Input::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x5c) { saw_5c = w.total_ticks; break; }
    }
    assert!(saw_5c > 0, "the porch plate must fire ch 0x5C (the hand-off)");
    eprintln!("[bonus] ch 0x5C at tick {saw_5c}");

    // The ride: per tick, watch the award (count 0→1), the flash and the chime.
    let mut award_tick: Option<u32> = None;
    let mut flash_at_award: Option<u16> = None;
    let mut flash_next: Option<u16> = None;
    let mut t65: Option<u32> = None;
    let mut t69: Option<u32> = None;
    let mut chime_tick: Option<u32> = None;
    let mut input = Input::default();
    let mut prev_dis = true;
    let mut crouch_armed = false;
    for _ in 0..6000u32 {
        let sim = w.scene.sim.as_ref().unwrap();
        let dis = sim.disabled;
        let px = sim.bond.pos[0];
        if !dis && prev_dis && px < 95.0 { crouch_armed = true; }
        prev_dis = dis;
        input.crouch = if crouch_armed && !w.scene.sim.as_ref().unwrap().bond.crouched { crouch_armed = false; true } else { false };
        w.tick(input, false);
        let sim = w.scene.sim.as_ref().unwrap();
        if t65.is_none() && sim.drones.channels.on(0x65) { t65 = Some(w.total_ticks); }
        if t69.is_none() && sim.drones.channels.on(0x69) { t69 = Some(w.total_ticks); }
        if award_tick.is_none() && sim.drones.progress.stats().bond_moments >= 1 {
            award_tick = Some(w.total_ticks);
            flash_at_award = Some(sim.drones.health.bond_moment_flash);
        } else if award_tick.is_some() && flash_next.is_none() {
            flash_next = Some(sim.drones.health.bond_moment_flash);
        }
        if chime_tick.is_none() && sim.drones.sound_triggers.playing(331) { chime_tick = Some(w.total_ticks); }
        if sim.drones.level_exit.is_some() { break; }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    let award_tick = award_tick.expect("the truck ride must award bond move 101 (mux #316 AND(0x70,0x71,0x72) → 0x65)");
    eprintln!("[bonus] award at tick {award_tick} flash {:?} ch 0x65 {t65:?} chime 331 {chime_tick:?} exit {:?}", flash_at_award, sim.drones.level_exit);

    // THE COUNT: exactly one bond move, moment 101 done, in the progress layer.
    assert_eq!(sim.drones.progress.stats().bond_moments, 1, "the ride awards exactly one bond move");
    assert_eq!(sim.drones.progress.bond_moments[0], 1, "BondMoments[0x65−0x65] = 1 [M 0x199448:23]");
    for (k, f) in sim.drones.progress.bond_moments.iter().enumerate().skip(1) {
        assert_eq!(*f, 0, "bond move {} must not be awarded by this ride", 0x65 + k);
    }
    // The award rides with ch 0x65 (the mux completes the same tick as the park plate).
    let t65 = t65.expect("ch 0x65 must rise (the AND completes)");
    assert!((award_tick as i32 - t65 as i32).abs() <= 1, "the award tick {award_tick} rides with ch 0x65 {t65}");

    // THE HUD FLAG: 200 at the award tick, then the pane's countdown −FRAME_RATE_MUL = 2/tick
    // [M 0x199448:25; 0x19b9c8:258-260 — the (short)(timer − FRAME_RATE_MUL) float step].
    assert_eq!(flash_at_award, Some(200), "the award writes BL+0x948 = 200");
    assert_eq!(flash_next, Some(198), "the next pane update subtracts 2");
    let decay = sim.drones.health.bond_moment_flash;
    assert_eq!(decay % 2, 0, "the countdown steps by FRAME_RATE_MUL = 2");
    assert!(decay < 198, "the flash keeps decaying after the award tick (now {decay})");
    // THE CHIME: the level's SoundTrigger (placement 82, gate 0x65 → id 331) played.
    let chime_tick = chime_tick.expect("the gate-0x65 SoundTrigger must play id 331 on the award");
    assert!(chime_tick >= award_tick.saturating_sub(1) && chime_tick <= award_tick + 5,
        "the chime rides the award (chime {chime_tick} vs award {award_tick})");

    // The sister inside the run: bond move 105's channel (0x69) never rose on this ride.
    assert!(t69.is_none(), "the courtyard-plates mux (0x69) must not fire on the truck ride");
    assert_eq!(sim.drones.progress.bond_moments[4], 0, "moment 105 stays unawarded");
    assert!(!sim.drones.sound_triggers.playing(332), "the 0x69 chime (332) never plays");
}

/// Sister: a non-bond action awards nothing — the level idles at the cold start (no award, no flag,
/// no flash, no chime), and the bond channels never rise by themselves.
#[test]
fn idling_at_the_start_awards_nothing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"));
    w.skip_cutscenes = false;
    w.wait(30);
    for _ in 0..600u32 {
        w.tick(Input::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.progress.stats().bond_moments, 0, "idling must not award");
        assert_eq!(sim.drones.health.bond_moment_flash, 0, "no flash without an award");
        for id in 0x65u32..=0x6E {
            assert!(!sim.drones.channels.on(id), "bond channel {id:#x} must stay off while idling");
        }
        if sim.drones.level_exit.is_some() { break; }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.progress.stats().bond_moments, 0);
    assert!(!sim.drones.sound_triggers.playing(331) && !sim.drones.sound_triggers.playing(332),
        "neither bond chime plays without the action");
}
