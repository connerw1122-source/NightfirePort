//! laser-scripts: the level laser-grid beams run through the switches' embedded ScriptPlayers.
//!
//! The user (2026-10-16): "lasers arent accurate to the port look closely at their scripts". The
//! beams are NOT static placements: every laser-grid level drives its beams from a switch
//! (placement type 0x29) whose **param 1 is a NIS script** — `Switch_Create` 0x1adcc0:44-45 ends
//! with `SP_Create(pos, rot, param 1, 0, 8, 0, obj, 0)`, an embedded ScriptPlayer playing that
//! script **looping** (mode 8: `SP_LoadScript` 0x1dbc08:46-49 `Script_Play(loop=1)`, state 6). The
//! script's entity streams create/swap the laser art objects (`Script_CreateEntity` 0x1d8968:92-101
//! re-points a same-class stream object's gfx to the new hash; op 9 deletes it) and its event kind 3
//! with p0 = 0 parks `SCRIPTINFO+0x9c = -1` (delay = p0 − 1 [M Script_EventHandler 0x1d94c0]) —
//! the show freezes where the event fires and only `SP_UnPause` (the switch's channel-change arm,
//! `Switch_Update` 0x1adea8:19-24) resumes it.
//!
//! The disc's laser shows:
//! - 07000009 switch #71 "SCRIPTPLAYER_Lasers" (ch 0x37, param 1 = **0x06000667**, length 985,
//!   loop 0): entity stream creates SCRIPT_Lasers_ON 0x020005ed at t0, SCRIPT_Lasers_AMBER
//!   0x02000a29 at t31, SCRIPT_Lasers_OFF 0x020005ec at t496, AMBER again at t527, EntityEnd at
//!   t985; the event-3 hold fires at t2 [D `nf-extract nis … 06000667`].
//! - 0700000a switch #643 "SCRIPTPLAYER_KeyboardLasers" (ch 0x3B, param 1 = **0x06000664**, same
//!   shape): SCRIPT_KeyboardLasers_ON 0x020005e0 at t0, AMBER 0x02000a2d at t31, OFF 0x020005df at
//!   t496, AMBER at t527, EntityEnd at t985; the hold fires at t2 [D `nf-extract nis … 06000664`].
//!
//! The art glists are modelled in WORLD SPACE (the placements of all three 09 art objects sit at
//! the origin with identity rotation [D map 0100009f placements idx 68/69/143, flags 0x2031, no
//! params], like the level-1 rooms), so the beam picture is: the ON art alive and static, swapped
//! through AMBER/OFF only while an unpause plays a full cycle.
//!
//! The port skipped param 1 entirely (useobj's SwitchObj never read it), so neither show ever ran
//! and the beams did not exist. Fail-first on master: both tests below found no laser player and no
//! laser entity before the fix.
//!
//! Run with: `NF_ISO=… cargo test -p nf-view --test laser_scripts_live -- --nocapture --test-threads=1`

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The lobby laser show on 07000009: the switch's embedded looping player mounts, the ON art is
/// alive from the load beat, and the event-3 hold keeps AMBER/OFF from ever appearing while held.
#[test]
fn the_09_lobby_laser_show_runs_through_the_switch_player_and_holds_on_the_on_art() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000009.bin", false).expect("load 07000009");
    let sim = scene.sim.as_mut().unwrap();
    // The switch-embedded player exists with the disc's script [M Switch_Create 0x1adcc0:44-45].
    assert!(sim.drones.scripts.players.iter().find(|p| p.scripts[0] == 0x0600_0667)
        .is_some_and(|p| p.modes[0] == 8),
        "no mode-8 embedded ScriptPlayer for 0x06000667 — the laser show never mounts [M SP_Create 0x1dc7e0, Switch_Create 0x1adcc0:45]");
    // Walk the real path (no input): the load beat primes the player and its first commands run —
    // the ON art is created at t0 [D script 0x06000667 stream 0 Wait(0)].
    let mut input = Input::default();
    for _t in 1..=200u32 { sim.step_bond(input, false, false, false); input = Input::default(); }
    let ents = sim.drones.scripts.entities();
    let laser: Vec<u32> = ents.iter().filter(|(sh, ..)| *sh == 0x0600_0667).map(|&(_, _, hash, _, _)| hash).collect();
    assert!(!laser.is_empty(), "no laser entity alive after 200 ticks");
    assert!(laser.contains(&0x0200_05ed),
        "SCRIPT_Lasers_ON (0x020005ed) alive [D 0x06000667 stream 0 Wait(0)]");
    // The hold: the script's event kind 3 p0 = 0 fires at t2 → delay −1 → Script_Update's run
    // branch is gated on delay == 0 [M Script_Update 0x1da998:17-18], so the t31 AMBER creation
    // never runs. Sister that can fail: a broken hold (or an early unpause) puts the AMBER art up.
    assert!(sim.drones.scripts.players.iter().find(|p| p.scripts[0] == 0x0600_0667)
        .and_then(|p| p.info[p.cur].as_ref()).is_some_and(|s| s.delay < 0),
        "the show is held (SCRIPTINFO+0x9c = −1) [D 0x06000667 stream 1 Event kind 3 p0 = 0]");
    for _t in 201..=800u32 { sim.step_bond(input, false, false, false); input = Input::default(); }
    let ents = sim.drones.scripts.entities();
    let laser: Vec<u32> = ents.iter().filter(|(sh, ..)| *sh == 0x0600_0667).map(|&(_, _, hash, _, _)| hash).collect();
    assert_eq!(laser, vec![0x0200_05ed],
        "while held, exactly the ON art is alive — no AMBER 0x02000a29 / OFF 0x020005ec [D hold at t2]");
    // The switch link: `Switch_Create` stores the SP handle at ext+0x0C [M 0x1adcc0:46] — the
    // switch's channel-change arm unpauses it (`Switch_Update` 0x1adea8:19-24).
    let sw = sim.drones.useobjs.switches.iter().find(|s| s.ch == 0x37).expect("the 0x37 laser switch");
    assert!(sw.sp.is_some(), "the laser switch is linked to its embedded player");
}

/// Sister: the 0700000a keyboard-laser show — same mechanics, the level's own script and art
/// (switch #643, 0x06000664, SCRIPT_KeyboardLasers_ON 0x020005e0).
#[test]
fn the_0a_keyboard_laser_show_runs_through_the_switch_player() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000a.bin", false).expect("load 0700000a");
    let sim = scene.sim.as_mut().unwrap();
    let mut input = Input::default();
    for _t in 1..=200u32 { sim.step_bond(input, false, false, false); input = Input::default(); }
    assert!(sim.drones.scripts.players.iter().find(|p| p.scripts[0] == 0x0600_0664).is_some(),
        "no embedded ScriptPlayer for 0x06000664 — the keyboard lasers never mount");
    let ents = sim.drones.scripts.entities();
    let laser: Vec<u32> = ents.iter().filter(|(sh, ..)| *sh == 0x0600_0664).map(|&(_, _, hash, _, _)| hash).collect();
    assert!(!laser.is_empty(), "no keyboard-laser entity alive after 200 ticks");
    assert!(laser.contains(&0x0200_05e0),
        "SCRIPT_KeyboardLasers_ON (0x020005e0) alive [D 0x06000664 stream 0 Wait(0)]");
    assert!(sim.drones.scripts.players.iter().find(|p| p.scripts[0] == 0x0600_0664)
        .and_then(|p| p.info[p.cur].as_ref()).is_some_and(|s| s.delay < 0),
        "the keyboard show is held at t2 like the lobby show [D 0x06000664 stream 1]");
}
