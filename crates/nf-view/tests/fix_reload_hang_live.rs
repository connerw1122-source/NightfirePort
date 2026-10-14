//! fix-reload-hang: the reload state machine hangs (state 6, clip 0). The silenced twin 0x03's
//! auto-reload sticks (`rel=6` forever, `gun.clip` never refills although carried > 0) and every
//! later switch is refused while the view anim is not idle [M `Player_WeaponChange` 0x1a6f80:33-47;
//! docs/reports/weapon-matrix2.md "The remaining red cells" — the D/M/R mass]. The AT-420 Sentinel
//! 0x2D's reload (the 5/6/7 machine) never returns either, and a shotgun's per-shell loop must be
//! interruptible by fire.
//!
//! The game's exits [M]:
//! - `Player_SetWeaponAnimObj` case 6 (0x1a2e38:345-373): when the +0xA4 script STOPS,
//!   `Player_ReloadAmmoType(_, 0)` moves the rounds; records without flag 0x100 go straight to
//!   idle (LAB_001a3678: +0xA0 queued, state 0, `Input_ClearAction(9)`); flag-0x100 (per-shell)
//!   records probe with `Player_ReloadAmmoType(_, 1)` — shells left AND no fresh fire press
//!   replay +0xA4 (state 6 continues), else +0xAC queues and state 7 runs.
//! - `Player_ReloadAmmoType` 0x1a1988: the rounds moved per call are min(room in the clip slot,
//!   carried); twins (+0x05 set, same ammo type as base) read and write the BASE's clip slot;
//!   carried == 0 or a full clip still EXITS (probe 0 → +0xAC / idle) — no path stays in 6.
//! - `Player_WeaponFiring` 0x1a5460's shared tail (0x1a5fe4-0x1a631c): the reload START probes
//!   mode 1 first — with no ammo at all there is no reload (`Player_HandleHasNoAmmo` instead).
//!
//! Every test drives the real input path: `nf_view::load_weapons` (the give path's asset merge) +
//! `panel::apply`'s give + `Sim::step_bond` with real `Input` — no state pokes on the reload path.

use nf_game::player::Input;
use nf_view::panel;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Step until the level hands control back (`Sim::disabled` clears), or `cap`.
fn wait_enabled(scene: &mut Scene, cap: u32) {
    for _ in 0..cap {
        if scene.sim.as_ref().map_or(true, |s| !s.disabled) { return; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
}

/// The view machine fully idle: no switch, no reload, no fire script.
fn idle(scene: &Scene) -> bool {
    let s = scene.sim.as_ref().unwrap();
    s.gun.switch_state == 0 && s.gun.reload_state == 0 && !s.gun.fire_state
}

/// Step with no input until `idle` holds, or `cap` ticks pass.
fn wait_idle(scene: &mut Scene, cap: u32) -> bool {
    for _ in 0..cap {
        if idle(scene) { return true; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    idle(scene)
}

/// The give path (`panel::apply` "weapon" — `Player_EquipWeapon` 0x1a8a58 + spare rounds) and the
/// draw through the real input path, then settle. The give sets the switch request; the draw runs
/// the holster/draw machine.
fn give_and_draw(scene: &mut Scene, id: u8) {
    let q = vec![("do".to_string(), "weapon".to_string()), ("id".to_string(), format!("0x{id:02x}"))];
    let (msg, sw) = panel::apply(scene, &q);
    assert_eq!(sw, Some(id), "the give of {id:#04x} was refused: {msg}");
    let mut drew = false;
    for _ in 0..90 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id { drew = true; break; }
    }
    assert!(drew, "{id:#04x} never drew");
    assert!(wait_idle(scene, 200), "{id:#04x}'s draw never settled idle");
}

/// Empty the held weapon's clip with real trigger presses (1-tick press, release, wait the
/// refire out). Returns (presses, spare after the last shot).
fn empty_the_clip(scene: &mut Scene, max_presses: u32) -> (u32, u32) {
    let mut presses = 0;
    while presses < max_presses {
        let clip = scene.sim.as_ref().unwrap().gun.clip;
        if clip == 0 { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
        for _ in 0..12 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
        presses += 1;
    }
    let s = scene.sim.as_ref().unwrap();
    (presses, s.gun.spare)
}

/// The twin family shares ONE physical magazine [M Player_ReloadAmmoType 0x1a1988:18-22: a twin
/// record (+0x05 set) with its base's ammo type reloads `clip[base]`; pickup.rs `clip_slot` (the
/// give) equips into the same slot]. The port's runtime boundary wrote the twin's reload into the
/// twin's OWN slot, so hopping back to the base read the STALE base slot (still 0) — a second
/// auto-reload fired and the carried rounds were spent twice for one magazine.
#[test]
fn twin_hop_keeps_the_magazine_shared_with_the_base() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x02, 0x03]).expect("load 07000005");
    wait_enabled(&mut scene, 7200);

    give_and_draw(&mut scene, 0x02);
    let clip0 = scene.sim.as_ref().unwrap().gun.clip;
    assert_eq!(clip0, 7, "the give filled the P2K's clip [L 07000005 loadout]");

    // Hop to the silenced twin (the magazine follows — one physical gun).
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    assert!(wait_idle(&mut scene, 200), "the 0x03 draw never settled");
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x03, "the mode press landed the silenced twin");
    assert_eq!(scene.sim.as_ref().unwrap().gun.clip, clip0, "the magazine did not survive the twin hop");

    // Empty the TWIN's clip and let its auto-reload fill it back (spare spent exactly once).
    let (presses, spare0) = empty_the_clip(&mut scene, 40);
    let mut started = false;
    for _ in 0..300 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.reload_state != 0 { started = true; }
        if started && scene.sim.as_ref().unwrap().gun.reload_state == 0 { break; }
    }
    let s = scene.sim.as_ref().unwrap();
    eprintln!("[twin reload] held={:02x} clip={} spare={} rel={} (spare0={spare0}, presses={presses})", s.gun.weapon, s.gun.clip, s.gun.spare, s.gun.reload_state);
    assert!(started, "the twin's auto-reload never started after {presses} presses (spare0={spare0})");
    assert_eq!(s.gun.reload_state, 0, "the twin's reload never returned");
    assert_eq!(s.gun.clip, clip0, "the reloaded magazine is not visible on the twin");
    assert_eq!(s.gun.spare, spare0 - clip0, "the reload spent carried rounds more than once");

    // Hop back: the base reads the SAME reloaded magazine — no second reload, no second spend.
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    let mut second = false;
    for _ in 0..300 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.reload_state != 0 { second = true; }
        if second && scene.sim.as_ref().unwrap().gun.reload_state == 0 { break; }
    }
    let s = scene.sim.as_ref().unwrap();
    eprintln!("[hop back] held={:02x} clip={} spare={} rel={} second_reload={second}", s.gun.weapon, s.gun.clip, s.gun.spare, s.gun.reload_state);
    assert_eq!(s.gun.weapon, 0x02, "the mode press landed the base");
    assert_eq!(s.gun.clip, clip0, "the base read the STALE slot — the shared magazine was lost");
    assert!(!second, "a second auto-reload fired after hopping back (the carried rounds were spent twice)");
    assert_eq!(s.gun.spare, spare0 - clip0, "the carried pool was spent twice for one magazine");
}

/// THE HEADLINE REPRO (would fail on master): empty the silenced twin 0x03, let the auto-reload
/// start, and ride it — it must RETURN to idle with the clip refilled [M 0x1a2e38 case 6:
/// LAB_001a3678; 0x1a1988: the rounds move]. On master the machine sticks in rel=6 with clip 0
/// while carried > 0, and every later switch is refused [M 0x1a6f80:33-47].
#[test]
fn empty_silenced_twin_0x03_reloads_to_idle_with_the_clip_refilled() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x02, 0x03]).expect("load 07000005");
    wait_enabled(&mut scene, 7200);

    give_and_draw(&mut scene, 0x02);
    // The mode press lands the twin (record +0x05 ≠ 0, `fire_mode_switch` needs the twin to have
    // ammo — the give filled the shared carried pool) [M 0x1a24c8:246-260].
    let mut landed = false;
    for _ in 0..10 {
        assert!(wait_idle(&mut scene, 60), "the machine never idled for the mode press");
        scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x03 { landed = true; break; }
    }
    assert!(landed, "the mode press never landed the silenced twin 0x03");
    assert!(wait_idle(&mut scene, 200), "the 0x03 draw never settled idle");
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x03);

    let clip_size = scene.sim.as_ref().unwrap().weapons.clip_size(0x03) as u32;
    assert!(clip_size > 1, "clip_size {clip_size}: the empty-and-refill needs a real magazine");
    let (presses, spare0) = empty_the_clip(&mut scene, 40);
    let (clip0, spare0) = { let s = scene.sim.as_ref().unwrap(); (s.gun.clip, spare0) };
    assert_eq!(clip0, 0, "the clip never emptied after {presses} trigger presses");
    assert!(spare0 > 0, "the give left no spare rounds to reload with (spare={spare0})");

    // Release the trigger: the auto-reload starts (the game's branch B — the trigger not newly
    // pressed [M 0x1a5460:442-445]) and must RETURN with the clip refilled. The last shot's fire
    // script is still playing when the clip hits 0, so wait for the start first.
    let mut trace = String::new();
    let mut started = false;
    for t in 0..120u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.reload_state != 0 { started = true; break; }
        if t % 10 == 0 { trace += &format!("  pre-t{t}: rel={} clip={} spare={} fire={}\n",
            scene.sim.as_ref().unwrap().gun.reload_state, scene.sim.as_ref().unwrap().gun.clip,
            scene.sim.as_ref().unwrap().gun.spare, scene.sim.as_ref().unwrap().gun.fire_state as u8); }
    }
    assert!(started, "the auto-reload never started after the clip emptied\n{trace}");
    let (mut returned, mut refilled) = (false, false);
    for t in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if t % 10 == 0 { trace += &format!("  t{t}: rel={} clip={} spare={}\n", s.gun.reload_state, s.gun.clip, s.gun.spare); }
        if s.gun.reload_state == 0 {
            returned = true;
            refilled = s.gun.clip > 0;
            break;
        }
    }
    eprintln!("[0x03 reload] clip_size={clip_size} presses={presses} clip0={clip0} spare0={spare0}\n{trace}");
    assert!(returned, "the auto-reload never returned to idle (the rel=6 hang)\n{trace}");
    assert!(refilled, "the reload returned but left the clip empty\n{trace}");
    // Sister: after the machine is back, a switch lands (the game's refused set only holds while
    // the view anim is busy) [M 0x1a6f80:33-47].
    let mut back = false;
    for _ in 0..90 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x02), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x02 { back = true; break; }
    }
    assert!(back, "the switch back to 0x02 was refused after the reload\n{trace}");
}

/// During the reload a switch_to is refused (the refused set {5,6,7} [M 0x1a6f80:33-47]); once the
/// reload completes a fresh request lands. On master the reload never completes, so the switch
/// never lands at all.
#[test]
fn switch_pressed_mid_reload_lands_only_after_the_reload_settles() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x02, 0x16]).expect("load 07000005");
    wait_enabled(&mut scene, 7200);

    give_and_draw(&mut scene, 0x16); // own the SG5 so 0x02's cycle has somewhere to go
    give_and_draw(&mut scene, 0x02);

    // Spend one round, let the fire script finish, then pulse the reload edge [M action 14].
    scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    assert!(wait_idle(&mut scene, 200), "the fire script never finished");
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, true, false);
    let mut started = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.reload_state != 0 { started = true; break; }
    }
    assert!(started, "the R-edge reload never started");

    // Mid-reload switch presses are refused while the machine is busy.
    let mut landed_mid = false;
    for _ in 0..20 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x16), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x16 { landed_mid = true; break; }
    }
    assert!(!landed_mid, "the switch landed DURING the reload (the refused set {{5,6,7}} must hold)");

    // The reload must settle; then the same request lands.
    let mut trace = String::new();
    let mut settled = false;
    for t in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if t % 10 == 0 { trace += &format!("  t{t}: rel={} clip={}\n", s.gun.reload_state, s.gun.clip); }
        if s.gun.reload_state == 0 { settled = true; break; }
    }
    eprintln!("[mid-reload switch] {trace}");
    assert!(settled, "the reload never settled — the switch can never land\n{trace}");
    let mut landed = false;
    for _ in 0..90 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x16), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x16 { landed = true; break; }
    }
    assert!(landed, "the switch never landed after the reload settled\n{trace}");
}

/// The AT-420 Sentinel 0x2D: fire, reload, idle — the 5/6/7 machine must return (the matrix's
/// "0x2d's own reload hang, same family"). 0x2d is the GUIDED twin of base 0x2c [L probe dump:
/// base=0x2c, +0x05 = 0xff], so the give owns the base and the mode press (record +0x05 chain,
/// `fire_mode_switch`) reaches the twin — the same way the player and the matrix reach it.
#[test]
fn sentinel_0x2d_fire_then_reload_returns_to_idle() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x2c, 0x2d]).expect("load 07000005");
    wait_enabled(&mut scene, 7200);

    give_and_draw(&mut scene, 0x2c);
    let mut landed = false;
    for _ in 0..10 {
        assert!(wait_idle(&mut scene, 60), "the machine never idled for the mode press");
        scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x2d { landed = true; break; }
    }
    assert!(landed, "the mode press never landed the Sentinel twin 0x2d");
    assert!(wait_idle(&mut scene, 200), "the 0x2d draw never settled idle");
    let clip_size = scene.sim.as_ref().unwrap().weapons.clip_size(0x2d) as u32;
    // One real shot, then the fire script must finish before the reload edge can land.
    let mut fired = false;
    for _ in 0..60 {
        let before = scene.sim.as_ref().unwrap().gun.clip;
        scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
        let after = scene.sim.as_ref().unwrap().gun.clip;
        if after < before { fired = true; break; }
    }
    assert!(wait_idle(&mut scene, 200), "the fire script never finished");
    let (clip1, spare1) = { let s = scene.sim.as_ref().unwrap(); (s.gun.clip, s.gun.spare) };
    eprintln!("[0x2d] clip_size={clip_size} fired={fired} clip1={clip1} spare1={spare1}");
    assert!(fired, "the Sentinel never fired");
    if clip1 as u32 >= clip_size || spare1 == 0 {
        eprintln!("[0x2d] nothing to reload (clip full or no spare) — the assertion below is vacuous");
    }
    // Pulse the reload edge and ride the machine back to 0 with the clip refilled.
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, true, false);
    let mut trace = String::new();
    let (mut started, mut settled) = (false, false);
    for t in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if t % 10 == 0 { trace += &format!("  t{t}: rel={} clip={} spare={}\n", s.gun.reload_state, s.gun.clip, s.gun.spare); }
        if s.gun.reload_state != 0 { started = true; }
        if started && s.gun.reload_state == 0 { settled = true; break; }
    }
    eprintln!("[0x2d reload] {trace}");
    assert!(started, "the Sentinel's reload never started\n{trace}");
    assert!(settled, "the Sentinel's reload never returned to idle (the 5/6/7 hang)\n{trace}");
    assert!(scene.sim.as_ref().unwrap().gun.clip > clip1, "the clip never refilled\n{trace}");
}

/// The SPAS-12's per-shell loop (flag 0x100): each +0xA4 pass moves ONE shell, a fire press
/// interrupts the loop to state 7 (+0xAC), and the machine returns to idle [M 0x1a2e38 case 6:
/// `Input_Action(9,1)` → +0xAC, state 7].
#[test]
fn shotgun_per_shell_loop_moves_one_per_pass_and_fire_interrupts_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x1c]).expect("load 07000005");
    wait_enabled(&mut scene, 7200);

    give_and_draw(&mut scene, 0x1c);
    let clip_size = scene.sim.as_ref().unwrap().weapons.clip_size(0x1c) as u32;
    // Spend 4 rounds so the per-shell loop has several shells to load (after ONE the clip is full
    // and the game's own probe ends the reload: Player_ReloadAmmoType(_,1) returns 0 at room 0
    // [M 0x1a1988:27-41] — state 7 with no fire press needed).
    for _ in 0..4 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
        assert!(wait_idle(&mut scene, 200), "the fire script never finished between shots");
    }
    let clip_at_start = scene.sim.as_ref().unwrap().gun.clip;
    assert!(clip_at_start as u32 + 2 <= clip_size, "need ≥2 shells of room, clip={clip_at_start}/{clip_size}");
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, true, false);
    let mut started = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.reload_state != 0 { started = true; break; }
    }
    assert!(started, "the shotgun's reload never started");
    // Ride ONE pass: the clip grows by 1 while the state stays in the 5/6/7 family.
    let clip_at_start = scene.sim.as_ref().unwrap().gun.clip;
    let mut moved = false;
    for t in 0..240u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if s.gun.clip > clip_at_start { moved = true; }
        if moved && s.gun.clip > clip_at_start + 1 { break; } // a second shell — enough
        if s.gun.reload_state == 0 { break; }                  // finished (clip had room for 1)
        if t == 239 { break; }
    }
    assert!(moved, "the per-shell loop never moved a shell");
    // Fire interrupts: a held trigger during state 6 must end the loop (state 7) and return.
    let mut interrupted = false;
    let mut settled = false;
    for _ in 0..240 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
        let s = scene.sim.as_ref().unwrap();
        if s.gun.reload_state == 7 { interrupted = true; }
        if s.gun.reload_state == 0 { settled = true; break; }
    }
    eprintln!("[0x1c interrupt] interrupted={interrupted} settled={settled}");
    assert!(settled, "the shotgun never returned to idle after the fire interrupt");
    assert!(interrupted, "the fire press never pushed the shell loop to state 7 (+0xAC) [M 0x1a2e38 case 6]");
}
