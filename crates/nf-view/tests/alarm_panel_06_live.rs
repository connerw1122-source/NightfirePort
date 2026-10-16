//! alarm-panel-06: the 07000006 alarm panel, end to end (docs/levels/07000006.md §5, the
//! fusebox cluster). The game's silencer is a three-object chain [M]:
//! - TouchOnce **#765** (0xEA, out ch 177, mask 8 = player contact, volume = main-map object 195's
//!   world-space tris (31.1, 42.2, −59.1)..(37.0, 46.8, −48.9)) — Bond walks into the wall panel
//!   by the fusebox and ch 177 rises;
//! - InverterTrigger **#749** (0x41, `InverterTrigger_Update` 0x1b30a8: in 177 → out 55 forced 0
//!   every frame) — the alarm channel the tracking searchlights hold (`Searchlight_Update`
//!   0x1cb900 ext+0x36) is cleared;
//! - the siren SoundTrigger **#702** (0xF9, global sound 329, ON ch 55, OFF ch 177, keep byte
//!   param 7 = 0 `SoundTrigger_Create` 0x1b2628 / `SoundTrigger_Update` 0x1b2790) — stops both on
//!   ch 177 and on ch 55 clearing.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::{Input, RamSave};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

/// The gate-searchlight heads that hold ch 55 while tracking (07000006 placements #2/#4/#6/#8,
/// p3 = 55 [D]); Bond stands under head #2's courtyard sweep until the spot passes within 2.0 and
/// the head locks on (`Searchlight_Update` 0x1cb900 state 0 → 1).
const SPOTS: [[f32; 3]; 3] = [
    [-1.0, 43.5, -24.0],
    [7.0, 43.5, -30.0],
    [-1.0, 43.5, -46.0],
];

#[test]
fn the_searchlight_alarm_silences_at_the_wall_panel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load 07000006");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        assert!(!sim.disabled, "Bond is free after the intro");
        assert!(!sim.drones.channels.on(55), "the alarm starts off");
        // The siren trigger #702 exists with the game's fields [D placement #702].
        let siren = sim.drones.sound_triggers.list.iter()
            .find(|t| t.on == 55 && t.id == 329).expect("the ch-55 siren trigger (#702) is created");
        assert_eq!((siren.off, siren.is2d, siren.keep_on), (177, 1, 0),
            "off ch 177, a global (2-D) sound, keep byte = param 7 = 0 [M 0x1b2628]");
        // The inverter #749 watches ch 177 and holds ch 55 low [D placement #749; M 0x1b30a8].
        let inv = sim.drones.worldobjs.inverters.iter()
            .find(|i| i.ch_in == 177).expect("the ch-177 inverter (#749) is created");
        assert_eq!(inv.ch_out, 55);
    }
    // 1. Raise the alarm the real way: Bond under a sweeping gate searchlight until it tracks.
    let mut raised = false;
    'spots: for spot in SPOTS {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = spot;
        for _ in 0..1500u32 {
            sim.step_bond(Input::default(), false, false, false);
            if sim.drones.channels.on(55) { raised = true; break 'spots; }
        }
    }
    let sim = scene.sim.as_mut().unwrap();
    assert!(raised, "a gate searchlight tracks Bond and raises ch 55");
    // The siren (#702) starts while ch 55 rides (the sim's driver drains the Start commands
    // itself, so assert the live voice pool).
    assert!(sim.drones.audio.live_ids().contains(&329), "the alarm siren (sound 329) is live while ch 55 is held");
    // 2. Bond leaves the courtyard for the panel room (the real flow): the head loses him and
    //    gives up after 900 ticks (`Searchlight_Update` 0x1cb900: dist > 2.0 for FRAME_RATE·15),
    //    but ch 55 STAYS LATCHED — the game never auto-clears a channel [M]. That latch is the
    //    symptom: the siren and the alarm music keep going with nothing tracking.
    let panel = sim.drones.triggers.iter().find(|t| t.out == 177 && t.kind == nf_game::triggers::kind::TOUCH_ONCE)
        .expect("the panel TouchOnce (out 177) exists").clone();
    let c = panel.centre();
    sim.bond.pos = [c[0], c[1] - 1.0, c[2]];
    let mut gave_up = false;
    for _ in 0..1200u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.searchlights.parts.iter().filter(|p| p.kind == 1).all(|p| p.state != 1) { gave_up = true; break; }
    }
    assert!(gave_up, "the searchlight gives up once Bond is away");
    assert!(sim.drones.channels.on(55), "ch 55 stays latched after the giveup — the game's symptom");
    assert!(sim.drones.audio.live_ids().contains(&329),
        "the siren still rides the latched channel");
    // 3. The panel: Bond into the wall panel's TouchOnce #765 (object 195's volume + the
    //    placement offset (−0.58, 0, −1.40) [D]): ch 177 fires and the inverter clears ch 55.
    let mut silenced = 0;
    for t in 1..=60u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(177) { silenced = t; break; }
    }
    assert!(silenced > 0, "the panel's ch 177 fires on contact");
    // The inverter clears the alarm channel the same/next tick [M 0x1b30a8 runs every frame].
    for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    assert!(!sim.drones.channels.on(55), "the inverter (#749) holds ch 55 low after the panel");
    // The siren stops: its ON channel cleared with keep byte 0 [M 0x1b2790:49-59] (and its OFF
    // channel 177 is up besides) — the voice leaves the pool at the next sound update.
    let mut siren_stopped = false;
    for _ in 0..10 {
        sim.step_bond(Input::default(), false, false, false);
        // The trigger released its voice (the OFF channel 177 is up besides; the pool node itself
        // drains at the driver's next Sound_UpdateSounds, which the silent headless sim defers).
        if !sim.drones.sound_triggers.playing(329) { siren_stopped = true; break; }
    }
    assert!(siren_stopped, "the siren trigger releases its voice once ch 55 clears (keep byte 0)");
    // The alarm-music poster stops: `DroneFunc_CheckAlarmRaised` 0x1453a8 posts Music_Event(2,1)
    // only while its ch-55 counter runs — the counter keeps counting but the channel is 0, so the
    // music event is no longer re-posted; assert the channel view the poster reads.
    assert!(!sim.drones.channels.on(55) && !sim.drones.channels.on(0x61),
        "no alarm channel rides after the panel");
}

#[test]
fn the_panel_with_no_alarm_changes_nothing_but_its_own_channel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load 07000006");
    let c = {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        let panel = sim.drones.triggers.iter().find(|t| t.out == 177 && t.kind == nf_game::triggers::kind::TOUCH_ONCE)
            .expect("the panel TouchOnce exists").clone();
        panel.centre()
    };
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [c[0], c[1] - 1.0, c[2]];
    // The game's TouchOnce fires its channel regardless of the alarm state [M Trigger_Activate
    // 0x1b2318 kind-1 arm: no alarm gate]; with ch 55 low there is nothing to silence.
    let mut fired = false;
    for _ in 0..60u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(177) { fired = true; break; }
    }
    for _ in 0..60u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(177) { fired = true; break; }
    }
    assert!(fired, "the panel fires ch 177 with no alarm");
    for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    assert!(!sim.drones.channels.on(55), "ch 55 stays low — no siren, no alarm to clear");
    assert!(!sim.drones.audio.live_ids().contains(&329), "the siren never started");
}
