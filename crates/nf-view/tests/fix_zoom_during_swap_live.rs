//! fix-zoom-during-swap: the scope button (the right mouse) held ACROSS a weapon swap must zoom
//! the new weapon once its draw finishes, and the release must zoom back out cleanly.
//!
//! The user (playtest 1015-0046, `--give 0x02,0x1e,0x1c`): "If I'm holding aim while swap, like,
//! say I'm on the shotgun and I swap to the sniper rifle, I swap off the shotgun and before the
//! sniper rifle comes up and I'm pressing right click, it doesn't actually zoom in once the
//! sniper rifle's finished. And when I let go of right click, it like does like a half zoom, but
//! never actually zooms in."
//!
//! The game: during the holster/draw the scoped machine-state switch force-clears the aim bit
//! [M Player_Weapon 0x1a24c8:80-96 — the +0x68 & 0x40 switch clears in states 1..4, 6..8, 0xa,
//! 0xd..0x10], so when the draw finishes (machine idle) with the button STILL held, the input arm
//! re-sets the bit and THAT rise is the raise's edge: view state 0xF queues the raise +0xB8 at
//! 1.25 the very tick the draw ends [M 0x1a24c8:113-152]. The raise's finish sets the bit [M
//! Player_SetWeaponAnimObj case 0xf 0x1a2e38:554-561] and the bit-1 mirror flip re-targets the
//! zoom (+0x8D4 = the per-mode entry clamped by +0x88 = 10.0 on 0x1e) [M 0x1a24c8:156-186];
//! `Player_Zoom` rides the target at 1.025×/tick in, 1.15×/tick out [M 0x1a1a70]. The release
//! drops the bit, returns the zoom target to 1.0 [M 0x1a24c8:158-166] and queues the lower from
//! the idle machine [M 0x1a24c8:120-143].
//!
//! The port's pre-fix queue arm fired on the RAW button's edge (`prev_aiming`), which a button
//! held from BEFORE the swap never produces again — the raise never queued, the bit never set,
//! the zoom never moved; on release the lower still queued and played the raise's raised pose
//! from idle (the user's "half zoom").
//!
//! Records [L ee_start_05.bin @ weapon_data 0x2bf150, stride 0x10C]: the shotgun 0x1c is NOT
//! scoped (+0x68 = 0x0080_8100) and has no raise script; the Tactical Sniper 0x1e IS scoped
//! (+0x68 = 0x0040_8050), raises with 0x0600_07b9 (24 frames), has no +0xBC (the lower plays the
//! raise REVERSED), max zoom (+0x88) 10.0.
//!
//! Every step drives the REAL input path (`nf_view::load_weapons` + the panel give +
//! `Sim::step_bond`); the held right button is `Input { scope: true, zoom: true }` — exactly what
//! lib.rs feeds for a held right button (lib.rs sets `zoom` and `scope` from it).

use nf_game::player::Input;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[0x1c, 0x1e]).expect("load 07000005");
    for _ in 0..7200 {
        if scene.sim.as_ref().map_or(false, |s| !s.disabled) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert!(scene.sim.as_ref().map_or(false, |s| !s.disabled), "07000005 never enabled (intro stuck)");
    scene
}

fn idle(s: &Scene) -> bool {
    let x = s.sim.as_ref().unwrap();
    x.gun.switch_state == 0 && x.gun.reload_state == 0 && !x.gun.fire_state
}

/// Gives `id` through the panel and rides the switch to the fully idle machine.
fn give_and_settle(scene: &mut Scene, id: u8, rounds: i32) {
    let (status, sw) = nf_view::panel::apply(scene, &[
        ("do".to_string(), "weapon".to_string()),
        ("id".to_string(), format!("0x{id:02x}")),
        ("rounds".to_string(), rounds.to_string()),
    ]);
    assert_eq!(sw, Some(id), "give refused: {status}");
    for _ in 0..60 {
        if scene.sim.as_ref().unwrap().gun.weapon == id { break; }
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the switch never landed");
    for _ in 0..7200 {
        let x = scene.sim.as_ref().unwrap();
        if idle(scene) && x.view_script == x.weapons.idle_script(id as usize) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    let x = scene.sim.as_ref().unwrap();
    assert!(idle(scene) && x.view_script == x.weapons.idle_script(id as usize),
        "never settled to idle (script {:#x} switch {} reload {} fire {})",
        x.view_script, x.gun.switch_state, x.gun.reload_state, x.gun.fire_state);
}

/// The raise tick count from the script's frame count: ceil((len−1)/1.25) at the game's 1.25×.
fn raise_ticks(scene: &Scene, w: u8) -> u32 {
    let s = scene.sim.as_ref().unwrap();
    let h = s.weapons.raise_script(w as usize);
    let len = s.scripts.get(&h).unwrap_or_else(|| panic!("raise script {h:#x} missing")).length as f32;
    ((len - 1.0) / 1.25).ceil() as u32
}

/// The user's setup: the shotgun out and idle, the right button held from BEFORE the swap, the
/// swap to the sniper 0x1e ridden out with the button NEVER released. Returns after the zoom is
/// up (the raise finished, the bit set, the zoom climbing), with the button still held.
fn swap_while_holding_scope(scene: &mut Scene) {
    // Both weapons owned through the panel gives (load_weapons' list merges the ASSETS only —
    // the inventory ownership comes from the equip, like the user's --give 0x02,0x1e,0x1c).
    give_and_settle(scene, 0x1e, 10);
    give_and_settle(scene, 0x1c, 16);
    // The record facts the scenario rests on [L weapon_data 0x2bf150 + 0x1c/0x1e]:
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.weapons.raise_script(0x1c), 0, "the shotgun carries no aim-raise script");
        assert_eq!(s.weapons.flags(0x1c) & 0x40, 0, "the shotgun is not scoped (+0x68 & 0x40)");
        assert_eq!(s.weapons.raise_script(0x1e), 0x0600_07b9, "the sniper's raise script (+0xB8)");
        assert_eq!(s.weapons.lower_script(0x1e), 0, "the sniper has no +0xBC — the lower plays the raise reversed");
        assert_eq!(s.weapons.flags(0x1e) & 0x40, 0x40, "the sniper is scoped (+0x68 & 0x40)");
    }
    // Hold the right button on the SHOTGUN: the mouse scope reaches the aim bit only on a scoped
    // weapon [M Player_Weapon 0x1a24c8:66-96 gates the raise/zoom arms on +0x68 & 0x40], so it
    // must not aim the shotgun and the zoom must stay down.
    let hold = Input { scope: true, zoom: true, ..Default::default() };
    for _ in 0..5 { scene.sim.as_mut().unwrap().step_bond(hold.clone(), false, false, false); }
    {
        let s = scene.sim.as_ref().unwrap();
        assert!(!s.aiming, "the right button must not aim the unscoped shotgun");
        assert_eq!(s.zoom, 1.0, "the shotgun must not zoom");
        assert_eq!(s.zoom_target, 1.0, "the shotgun's zoom target must stay 1.0");
    }
    // Swap to the sniper WHILE STILL HOLDING (the game refuses a swap only with the aim BIT set
    // [M Player_WeaponChange 0x1a6f80:62-64] — the shotgun's bit never sets, so the swap goes).
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(
            Input { scope: true, zoom: true, switch_to: Some(0x1e), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x1e { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x1e, "the swap never landed");
    // Ride the draw out with the button still held. During the draw the game has the bit cleared
    // (the scoped machine-state switch [M 0x1a24c8:80-96]) — no zoom. THE FIX: the tick the draw
    // finishes, the held button re-sets the bit from the idle machine and that rise queues the
    // raise (state 0xF) — the queue must not wait for a raw press edge.
    let idle_script = scene.sim.as_ref().unwrap().weapons.idle_script(0x1e);
    let mut raise_tick = 0;
    for t in 1..=400 {
        scene.sim.as_mut().unwrap().step_bond(hold.clone(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if s.view_script == 0x0600_07b9 && s.view_aim_raising { raise_tick = t; break; }
        assert_eq!(s.zoom, 1.0, "tick {t}: the zoom moved before the raise queued");
    }
    assert!(raise_tick > 0,
        "the raise never queued from the button held across the swap (view_script {:#x}, switch {}, aiming {})",
        scene.sim.as_ref().unwrap().view_script, scene.sim.as_ref().unwrap().gun.switch_state,
        scene.sim.as_ref().unwrap().aiming);
    // The raise advances 19 ticks (24 frames @1.25), then its finish sets the bit and the zoom
    // target leaves 1.0 [M 0x1a2e38:554-561 + 0x1a24c8:168-186].
    let n = raise_ticks(scene, 0x1e);
    assert_eq!(n, 19, "the sniper's raise: ceil((24-1)/1.25) = 19 ticks");
    let mut aimed_tick = 0;
    for t in 1..=n + 3 {
        scene.sim.as_mut().unwrap().step_bond(hold.clone(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if s.aiming { aimed_tick = t; break; }
        assert!(!s.aiming, "tick {t}: the aim bit set before the raise's last tick ({n})");
    }
    assert!(aimed_tick > 0, "the aim bit never set after the raise finished");
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.zoom_target, 10.0, "the zoom target must be the sniper's max zoom (+0x88) once the bit sets");
        assert!(s.zoom > 1.0, "the zoom must start the tick the raise finishes");
    }
    // Keep holding: the zoom climbs toward 10 and the raise must NOT re-queue (the finish tick's
    // bit-set is not an edge — the game reads its edge reference after AnimObjectUpdate [M
    // 0x1a24c8:44,113-152]).
    for _ in 0..80 { scene.sim.as_mut().unwrap().step_bond(hold.clone(), false, false, false); }
    let s = scene.sim.as_ref().unwrap();
    assert!(s.zoom > 4.0, "the zoom never climbed while the button stayed held (zoom {})", s.zoom);
    assert_eq!(s.view_script, idle_script, "the raise re-queued while the button stayed held");
}

/// The held-across-swap zoom: the sniper zooms once its draw finishes, without any new press.
#[test]
fn holding_scope_across_the_swap_zooms_the_sniper_once_its_draw_finishes() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    swap_while_holding_scope(&mut scene);
}

/// Sister: releasing the button after the swap zoom drops the bit and the zoom target at once,
/// plays the lower (the raise REVERSED — 0x1e has no +0xBC) and returns the zoom to 1.0.
#[test]
fn releasing_scope_after_the_swap_zooms_back_out_and_lowers() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    swap_while_holding_scope(&mut scene);
    let idle_script = scene.sim.as_ref().unwrap().weapons.idle_script(0x1e);
    // The release tick: the bit drops, the zoom target returns to 1.0 [M 0x1a24c8:158-166] and
    // the lower queues from the idle machine [M :120-143].
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    {
        let s = scene.sim.as_ref().unwrap();
        assert!(!s.aiming, "the aim bit must drop on the release tick");
        assert_eq!(s.zoom_target, 1.0, "the zoom target must return to 1.0 on the release tick");
        assert_eq!(s.view_script, 0x0600_07b9, "the lower must queue on the release tick");
        assert!(s.view_speed < 0.0, "the lower must play the raise REVERSED (no +0xBC on 0x1e)");
    }
    // The zoom returns to 1.0 (1.15×/tick out [M Player_Zoom 0x1a1a70]) and the idle returns.
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let s = scene.sim.as_ref().unwrap();
    assert_eq!(s.zoom, 1.0, "the zoom must return to 1.0 after the release");
    assert_eq!(s.view_script, idle_script, "the idle must return after the lower");
}
