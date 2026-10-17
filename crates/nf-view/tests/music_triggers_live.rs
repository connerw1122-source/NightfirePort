//! music-triggers (task 750): the music trigger placements (0xfb → `MusicTrigger_Create`
//! 0x1b30e0 + `MusicTrigger_Update` 0x1b31a0) through the real path — the trigger exists after
//! the level load, raising switch channel p0 fires `Music_Event(p2, 2)` every tick
//! (`UpdateMusicalEvents` pins `bonus = 600` / `under_attack`), and clearing the channel lets the
//! 0x1e countdown expire so the event stops. Opt-in: `NF_ISO`.

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn settle(scene: &mut nf_view::scene::Scene, ticks: u32) {
    for _ in 0..3000 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); if !scene.sim.as_ref().unwrap().disabled { break; } }
    for _ in 0..ticks { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
}

/// 07000001's single music trigger [D placement bytes: params (13, 0, 2, 1), "Music Trigger"]:
/// channel 13 drives `Music_Event(2, 2)` — the alarm slot (`UpdateMusicalEvents`'s
/// `e[2] == 2 → bonus = 600, under_attack = true` [M 0x114e70 via music.rs's port, cited at
/// music.rs:158]).
#[test]
fn the_01_music_trigger_raises_and_expires_the_alarm_event() {
    let mut scene = nf_view::load(&iso(), "07000001.bin", false).expect("load 07000001");
    // The training level's own intro settles by ~600 ticks; two fixed phases give the decay
    // head-room before the baseline reads (the settle's `disabled` gate alone can exit early).
    for _ in 0..1300 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    // Created at load, through the real placement scan: exactly one, the disc's params
    // (p0 = 13, p2 = 2; the create gates pass [M 0x1b30e0:12-16]).
    assert_eq!(sim.drones.worldobjs.music_triggers.len(), 1, "the 0xfb placement created its trigger");
    let t = &sim.drones.worldobjs.music_triggers[0];
    assert_eq!((t.event_id, t.ch_on, t.ch_flag, t.flag_byte), (2, 13, 0, 1));
    assert_eq!(t.countdown, 0, "nothing armed the countdown during the settle");
    assert!(!sim.drones.music.alarm_state().0 && sim.drones.music.alarm_state().1 == 0, "baseline: no alarm event ran");
    drop(sim);
    // The training level's intro leaves `nis_playing` set (the e[0xb] latch) and the game's
    // per-frame alarm logic runs only while no NIS camera holds [M music.rs's `!nis_playing`
    // gate]. Clear it the game's way: the NIS end's `Music_Event(0xC, script)` [M music.rs's
    // event-12 note] — the same board write the script system makes when a camera quits.
    scene.sim.as_mut().unwrap().drones.music.event(0xc, 1);
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    // Raise channel 13 through the real channel writer: the countdown re-arms and the event fires
    // every tick; `UpdateMusicalEvents` sees e[2] == 2, sets bonus 600 and under_attack, then the
    // same frame's decay decrements bonus once (the game's set-then-decay order [M 0x114e70]).
    scene.sim.as_mut().unwrap().drones.set_channel(13);
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.music.alarm_state().0, "the trigger's Music_Event(2, 2) reached the music system");
        assert_eq!(sim.drones.music.alarm_state().1, 599, "the event re-pins bonus each tick (600 set, then the frame's decay)");
        assert!(sim.drones.worldobjs.music_triggers[0].countdown > 0);
        assert!(sim.drones.worldobjs.music_triggers[0].playing);
    }
    // Leave: the channel clears, the 0x1e countdown drains [M 0x1b31a0:20-23] and the event stops —
    // the hysteresis pin lifts and bonus decays below 600 (it would stay pinned at 600 while the
    // event kept firing).
    let frame = scene.sim.as_ref().unwrap().drones.frame;
    scene.sim.as_mut().unwrap().drones.channels.set(13, 0, frame);
    for _ in 0..45 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.worldobjs.music_triggers[0].countdown, 0, "the countdown expired at the game's tick");
    assert!(sim.drones.music.alarm_state().1 < 600, "the event stopped: bonus is decaying instead of re-pinning");
}
