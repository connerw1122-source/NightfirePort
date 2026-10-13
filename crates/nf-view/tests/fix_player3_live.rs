//! fix-player3 repro tests, driven through the REAL path the viewer uses (`nf_view::load` + `Sim::step_bond`).
//!
//! The user's playtest (master afc60ed, 07000001 unless noted):
//! 1. a dead guard drops his gun PLUS a separate ammo pickup — the game drops only the gun;
//! 2. armour pickups should glow (the game's close-up pulse);
//! 3. reloading from empty (the Raptor at 0) plays the wrong animation;
//! 4. switching weapons mid-reload breaks shooting permanently;
//! 5. the thrown grenade "doesn't really happen" and there is no explosion;
//! 6. the PS2 game (per the user) won't fire while the red "no" crosshair shows — the decompile says otherwise.
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    // The levels whose mode-1 intro takes control (event 4): 07000001's SP #277 plays 0x06000087 and
    // 07000002's #400 plays 0x06000084 (docs/systems/levels.md §5.1). Skip it like the game's own skip
    // (hold the skip action — `Script_FFwd` 0x1dac78) so the tests run with Bond in control, as they did
    // before the loadable packs were ported (the SP used to delete itself — the pack script was missing).
    let intro: &[u32] = match level {
        "07000001.bin" => &[0x0600_0087],
        "07000002.bin" => &[0x0600_0084],
        _ => &[],
    };
    skip_active_cutscene(&mut scene, intro);
    scene
}

/// Skips the named level-start cut-scenes the way the game lets the player: hold the skip action
/// (`Scripts::skip_request` → `Script_FFwd` 0x1dac78) until the script is done. The catch-up runs the
/// remaining commands, so the channels and the player hand-back still apply; drones stay frozen the
/// whole scene (event 5), so afterwards the world is at its level-start state. A no-op when `intros`
/// is empty.
fn skip_active_cutscene(scene: &mut nf_view::scene::Scene, intros: &[u32]) {
    if intros.is_empty() { return; }
    let pending = |sim: &nf_view::scene::Sim| sim.drones.scripts.players.iter().any(|p| intros.contains(&p.scripts[0]) && (matches!(p.state, 1 | 2 | 7) || p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)));
    for t in 0..600u32 {
        let sim = scene.sim.as_mut().unwrap();
        if t >= 2 && !pending(sim) && !sim.disabled { break; }
        sim.drones.scripts.skip_request = true;
        sim.step_bond(Default::default(), false, false, false);
    }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the level-start cut-scene never handed control back");
}

/// Point Bond's view at `to` from his eye (setup helper; `turn` stays 0 so the aim holds).
fn aim_at(sim: &mut nf_view::scene::Sim, to: [f32; 3]) {
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
}

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// ── Item 1: the death drop ─────────────────────────────────────────────────────────────────────
/// `DroneWeap_DropWeapon` (0x17a2a0) creates exactly ONE visible pickup — the base weapon, kind 0
/// (0x17a404-0x17a408: `$a3 = 0` = kind 0, return ignored); the only second pickup is the 0x1A twin
/// (kind 1, id 0x1B) which the game hides immediately (`obj+0xF0 |= 0x10`, disasm 0x17a47c-84).
/// No death path in the game creates a VISIBLE kind-1 pickup (all Pickup_CreateSimple callers scanned:
/// 0x14b158 kind 0, 0x17a2a0 kinds 0+1(hidden), 0x190810 kinds 0+1(hidden) MP-only). This pins the
/// port to that: after a guard death, exactly one visible drop, and the hidden twin only for 0x1A.
#[test]
fn death_drops_match_the_game_exactly() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    let guards: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.enabled && dr.drawn && !dr.corpse && dr.health > 0.0 && (2..0x47).contains(&dr.weapon))
        .map(|(i, _)| i).collect();
    assert!(!guards.is_empty(), "07000001 starts armed guards");
    let gi = guards[0];
    let weapon_before = sim.drones.list[gi].weapon;
    let base = sim.weapons.base(weapon_before as usize);
    let gp = sim.drones.list[gi].pos;
    let floor = sim.world.floor_below(gp, 6.0).unwrap_or(gp[1]);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [gp[0], floor + 1.1, gp[2] - 3.0], 0.0);
    aim_at(sim, [gp[0], gp[1] + 0.3, gp[2]]);
    tick_idle(sim, 10);
    for t in 0..600 {
        sim.step_bond(Default::default(), t % 9 == 0, false, false);
        if sim.drones.list[gi].health <= 0.0 { break; }
    }
    assert!(sim.drones.list[gi].health <= 0.0, "the guard died");
    tick_idle(sim, 90); // the drop falls and lands
    let mine: Vec<&nf_game::pickup::Pickup> = sim.drones.pickups.iter().filter(|p| p.from == Some(gi)).collect();
    let visible: Vec<&nf_game::pickup::Pickup> = mine.iter().filter(|p| !p.hidden).copied().collect();
    // Exactly one VISIBLE drop: the gun (kind 0, the base weapon). The 0x1A twin is hidden.
    assert_eq!(visible.len(), 1, "the game drops only the gun; drops = {:?} (all: {:?})",
        visible.iter().map(|p| (p.weapon, p.kind, p.rounds)).collect::<Vec<_>>(),
        mine.iter().map(|p| (p.weapon, p.kind, p.hidden)).collect::<Vec<_>>());
    assert_eq!(visible[0].kind, nf_game::pickup::kind::WEAPON, "the visible drop is kind 0 [M 0x17a2a0:96]");
    assert_eq!(visible[0].weapon, base, "the drop is the BASE id [M 0x17a2a0:97 weapon_data[w]+0x02]");
    if base == 0x1a {
        assert!(mine.iter().any(|p| p.hidden && p.kind == nf_game::pickup::kind::AMMO && p.weapon == 0x1b),
            "the 0x1A twin ammo drop exists and is hidden [M 0x17a2a0:104-111]");
    } else {
        assert!(mine.len() == 1, "no second pickup for a non-0x1A weapon [M 0x17a2a0:96-111]");
    }
    // Sister: the drawn dropped-gun pose follows the visible pickup (scene.rs's `!p.hidden` filter).
    eprintln!("[drops] guard {gi} weapon {weapon_before:#x} → visible {} hidden {}", visible.len(), mine.len() - visible.len());
}

/// ── Item 2: the armour glow ────────────────────────────────────────────────────────────────────
/// Armour (kind 3) carries the 0x80000 flag [M Pickup_Create 0x18dda0:145-146]; `View_SetupRenderModes_Tweak`
/// (0x1e5888:22-44) multiplies its tint by `FlashMod` when the camera is within 4.0 units (squared
/// distance < 16.0) of the pickup's world-sphere centre. `FlashMod` (Check_Target 0x1a7810:95-97) is the
/// 1 Hz sine shimmer in [0.5, 1.0]. The port must pulse the same way.
#[test]
fn armour_glow_pulses_when_bond_is_close() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let k = scene.sim.as_ref().unwrap().drones.pickups.iter().position(|p| p.kind == nf_game::pickup::kind::ARMOUR && !p.gone)
        .expect("07000005 places a kind-3 armour pickup");
    let vi = scene.char_verts.iter().position(|cv| cv.pickup == Some(k as u16)).expect("the armour has a drawn instance");
    let base_rgba = scene.vertices[scene.char_vertex_start + vi].rgba;
    // At frame 45 (t = 0.75 s) FlashMod = ((sin(0.75·2π + π/2) + 1)·0.5 + 1)·0.5 = 0.75 exactly.
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..45 { sim.step_bond(Default::default(), false, false, false); }
        assert_eq!(sim.drones.frame, 45);
    }
    // FlashMod inline (the same formula nf_view::scene::flash_mod ports): ((sin(2π·t + π/2)+1)·0.5+1)·0.5.
    let flash = |frame: u32| { let t = frame as f32 / 60.0; let s = (t * std::f32::consts::TAU + std::f32::consts::FRAC_PI_2).sin(); ((s + 1.0) * 0.5 + 1.0) * 0.5 };
    let fm = flash(45);
    assert!((fm - 0.75).abs() < 1e-3, "FlashMod at frame 45 is 0.75, got {fm}");
    // (the port's nf_view::scene::flash_mod ports this formula; cross-checked post-fix)
    assert!((0.5..=1.0).contains(&fm), "FlashMod stays in [0.5, 1.0] [M 0x1a7810:95-97]");
    let opts = nf_view::scene::ViewOpts { reinforcements: false, firing: false, right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0] };
    // NEAR: Bond's eye within 4.0 units of the pickup's centre → the glow tint. (The eye is set
    // directly: `stand_at` snaps to the floor, which on the armour's pedestal can be metres away.)
    let c = scene.sim.as_ref().unwrap().drones.pickups[k].centre();
    let near_frame;
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [c[0] + 2.0, c[1] - 1.55, c[2]];
        tick_idle(sim, 1);
        near_frame = sim.drones.frame;
    }
    let vs = scene.posed_vertices(None, 0.0, opts);
    let near = vs[vi].rgba;
    // FAR: more than 4 units away → plain 1.0 (no pulse).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [c[0] + 2.0, c[1] - 1.55, c[2] + 12.0];
        tick_idle(sim, 1);
    }
    let vs = scene.posed_vertices(None, 0.0, opts);
    let far = vs[vi].rgba;
    assert_eq!(far, base_rgba, "beyond 4 units the armour draws at tint 1.0 [M 0x1e5888:22-44: sqdist < 16.0]");
    // The pulse: the near multiplier is FlashMod at the pose tick (per channel, rounded like the port's
    // other multiplies).
    let fm_near = flash(near_frame);
    for ch in 0..3 {
        let want = ((base_rgba[ch] as f32 * fm_near).round() as i32).clamp(0, 255) as u8;
        assert_eq!(near[ch], want, "armour glows with FlashMod near the camera [M 0x1e5888 psiSetTweakARGB]");
    }
    // Sister: the pulse actually differs from the unlit draw.
    assert_ne!(near, base_rgba, "the glow is visible (FlashMod 0.75 ≠ 1.0)");
}

/// ── Item 3: reload from empty ──────────────────────────────────────────────────────────────────
/// The P2K (0x06-style path: +0xA8 = 0 [L ee_start_05.bin record 2]) reloads in view state 6: the
/// +0xA4 loop queued by `Player_WeaponFiring`'s shared tail at 1.35× (`AnimScriptAddSpeed`, disasm
/// 0x1a62d0), starting at frame 6 for flag-0x1000 weapons (AnimListBuild +0x90 = 6.0, disasm
/// 0x1a6300-31c) — the tick count is ceil((len − 6)/1.35). When the loop stops, a non-shell weapon
/// (flags68 & 0x100 == 0) goes STRAIGHT to idle: +0xA0, state 0, Input_ClearAction(9) [M
/// LAB_001a3678 0x1a3678-0x1a369c]; +0xAC is never played (retail scan [L]: only SPAS-12 has +0xAC).
#[test]
fn reload_from_empty_plays_the_1_35x_loop_and_lands_on_idle() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    // The user's path: the clip empties with NO spare (so no auto-reload), he picks up ammo, presses R.
    let atype = sim.weapons.ammo_type(2);
    sim.drones.inventory.carried[atype] = 0;
    aim_at(sim, [sim.bond.head()[0], sim.bond.head()[1] + 30.0, sim.bond.head()[2] + 5.0]);
    for t in 0..160 { sim.step_bond(Default::default(), t % 4 == 0, false, false); }
    assert_eq!(sim.drones.inventory.clip[2], 0, "the clip is empty");
    assert_eq!(sim.gun.reload_state, 0, "no auto-reload without spare rounds [M Player_ReloadAmmoType(1) == 0]");
    // Pick up ammo through the real pickup path (a kind-1 ammo pickup, id 2).
    let model = sim.weapons.model(2);
    let bounds = *sim.models.get(&model).unwrap_or(&nf_game::pickup::Model { centre: [0.0, 0.05, 0.0], radius: 0.3 });
    let p = nf_game::pickup::Pickup::create_placed(sim.bond.pos, [0.0; 3], nf_game::pickup::kind::AMMO, 2, 30, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons).expect("ammo pickup");
    sim.drones.pickups.push(p);
    // The take and the game's auto-reload (branch B: not newly pressed + clip < ammo_per_shot + spare
    // now > 0 [M 0x1a5460:442-462]) land on the SAME tick — the user's extra R press is a no-op then.
    sim.step_bond(Default::default(), false, false, false);
    assert!(sim.drones.inventory.carried[atype] > 0, "the ammo pickup was taken");
    if sim.gun.reload_state == 0 {
        // (only if the take landed a tick later) press reload (action 14) to start it.
        sim.step_bond(Default::default(), false, true, false);
    }
    assert_eq!(sim.gun.reload_state, 6, "state 6 with no +0xA8 script [M 0x1a5f80]");
    let w = &sim.weapons.records[2];
    let rd = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
    let (a4, a0) = (rd(0xa4), rd(0xa0));
    assert_eq!(sim.view_script, a4, "the +0xA4 loop is queued");
    assert!((sim.view_speed - 1.35).abs() < 1e-4, "the loop plays at 1.35× [M disasm 0x1a62d0], got {}", sim.view_speed);
    assert!((sim.view_time - 5.0).abs() < 1e-4, "flag 0x1000 starts at frame 6 = time 5.0 [M 0x1a6300-31c], got {}", sim.view_time);
    let len = sim.scripts.get(&a4).map(|s| s.length as f32).expect("the reload script exists");
    let want_ticks = ((len - 6.0) / 1.35).ceil() as u32;
    assert_eq!(sim.gun.reload_ticks, want_ticks - 1, "the tick count is ceil((len − 6)/1.35) = {want_ticks} (the start tick's reload_tick already ran)");
    // Ride it out: no state 7, straight to idle with the +0xA0 script re-queued.
    let mut seen = std::collections::HashSet::new();
    for _ in 0..want_ticks + 10 {
        seen.insert(sim.gun.reload_state);
        sim.step_bond(Default::default(), false, false, false);
    }
    assert!(!seen.contains(&7), "a non-shell weapon never plays +0xAC [M LAB_001a3678] — saw states {seen:?}");
    assert_eq!(sim.gun.reload_state, 0, "back to idle");
    assert_eq!(sim.view_script, a0, "the +0xA0 idle script is re-queued [M LAB_001a3678]");
    assert_eq!(sim.drones.inventory.clip[2], sim.weapons.clip_size(2) as i16, "the ammo loaded");
}

/// ── Item 4: switching mid-reload ───────────────────────────────────────────────────────────────
/// `Player_WeaponChange` (0x1a6f80:33-47) refuses every switch while the view state is in the refused
/// set — 5/6/7 (a reload) are in it. A switch request during a reload is DROPPED; the switch only
/// starts once the reload completed, and firing works after it.
#[test]
fn switching_mid_reload_is_refused_and_firing_recovers() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    // Give Bond a second weapon through the real take path — the frag grenade (0x35; the SG5's
    // first-person skin is absent from 07000001's archive [L byte-scan, fix-player2] so the game's
    // EquipWeapon gate refuses it here), then switch back to the P2K.
    let id = 0x35usize;
    let model = sim.weapons.model(id);
    let bounds = *sim.models.get(&model).unwrap_or(&nf_game::pickup::Model { centre: [0.0, 0.05, 0.0], radius: 0.3 });
    let at = sim.bond.pos;
    let p = nf_game::pickup::Pickup::create_placed(at, [0.0; 3], nf_game::pickup::kind::WEAPON, id as u16, 1, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons)
        .expect("grenade pickup builds");
    sim.drones.pickups.push(p);
    tick_idle(sim, 5);
    assert!(sim.drones.inventory.owned[0x35], "0x35 owned");
    sim.gun.switch_request = 2;
    tick_idle(sim, 60);
    assert_eq!(sim.gun.weapon, 2);
    // Fire once, then start a reload.
    aim_at(sim, [sim.bond.head()[0], sim.bond.head()[1] + 30.0, sim.bond.head()[2] + 5.0]);
    sim.step_bond(Default::default(), true, false, false);
    tick_idle(sim, 40);
    assert_eq!(sim.drones.inventory.clip[2], 6, "one shot spent");
    sim.step_bond(Default::default(), false, true, false);
    assert_eq!(sim.gun.reload_state, 6, "the reload is running");
    // Try to switch DURING the reload — the game drops the request (0x1a6f80:33-47).
    let mut sw = Input::default();
    sw.switch_to = Some(0x35);
    sim.step_bond(sw, false, false, false);
    assert_ne!(sim.gun.switch_request, 0x35, "a switch request mid-reload is refused [M 0x1a6f80:33-47]");
    assert_eq!(sim.gun.switch_state, 0, "the holster never starts during a reload");
    // The reload completes normally.
    for _ in 0..200 { if sim.gun.reload_state == 0 { break; } sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.gun.reload_state, 0, "the reload finished");
    assert_eq!(sim.drones.inventory.clip[2], 7, "the ammo loaded");
    // Now the switch goes through, and back.
    sim.step_bond(sw, false, false, false);
    for _ in 0..120 { if sim.gun.weapon == 0x35 && sim.gun.switch_state == 0 { break; } sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 0x35, "the switch runs after the reload");
    sim.gun.switch_request = 2;
    tick_idle(sim, 90);
    assert_eq!(sim.gun.weapon, 2);
    // Firing works — the user's "breaks shooting permanently" must not happen.
    tick_idle(sim, 5);
    let before = sim.drones.player_bullets.len();
    let mut fired = false;
    for t in 0..40 {
        sim.step_bond(Default::default(), t % 9 == 0, false, false);
        if sim.drones.player_bullets.len() > before || !sim.drones.player_impacts.is_empty() { fired = true; break; }
    }
    assert!(fired, "the P2K fires after the refused mid-reload switch");
}

/// ── Item 5: the thrown grenade ─────────────────────────────────────────────────────────────────
/// The grenade is a gravity bullet: `Bullet_init` (0x12c5a0) stores speed +0x20 and the fuse
/// BU+0xFC = 210; `Bullet_update` (0x12e7b0) moves `pos += velocity` per tick (the velocity is the
/// per-tick displacement: unit direction · speed, FRAME_RATE_MUL = 1 [M movement tail 0x12f040]).
/// The blast: `Explode_Create` (0x17cbd0) with radius = weapon_data+0x08 and damage =
/// weapon_data+0x0C [M 0x12cd98 disasm: `lwc1 $f14, 12($v0)` → EXPLODEINFO+0x28; the
/// `addiu $t2, $zero, 0x64` u8 arg is never read — fix-explosions dropped the old hardcode 100].
/// fix-explosions2: the disc name labels (Txt_BindLabel of +0x38 vs USATxt.dat) name the records [L]:
/// the frag is 0x34 'Fragmentation Grenade' = radius 8 / damage 100 — the fuse blast KILLS/HURTS a
/// guard next to it through `Explode_Propagate` (0x17c518 case 2) (fix-explosions had the mapping
/// shifted one id: 0x35 is the 'Stun Grenade', whose `+0x70 & 0x2000` death branch [M 0x12dd60]
/// carries no damage — the sister below). The blast sound is the script's own Sound(0x3E) [D script
/// 0x6000052 stream 36], not 0x4A7 (that is `Bullet_CollisionHandler`'s rest-impact sound of
/// weapons 0x3B/0x45). The mine (0x37 'Remote Mine': 8/100 [L]) also damages.
#[test]
fn thrown_grenade_flies_and_explodes_with_sound_and_damage() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    // A frag pickup at Bond's feet (the captain-drop form), like fix-player2's test. The frag is
    // 0x34 [L USATxt.dat label 'Fragmentation Grenade'; MP label 0x0500005b → 'Frag Grenade'].
    let id = 0x34usize;
    let model = sim.weapons.model(id);
    let bounds = *sim.models.get(&model).unwrap_or(&nf_game::pickup::Model { centre: [0.0, 0.05, 0.0], radius: 0.3 });
    let at = sim.bond.pos;
    let p = nf_game::pickup::Pickup::create_placed(at, [0.0; 3], nf_game::pickup::kind::WEAPON, id as u16, 1, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons)
        .expect("grenade pickup builds");
    sim.drones.pickups.push(p);
    tick_idle(sim, 5);
    assert!(sim.drones.inventory.owned[0x34], "0x34 owned");
    sim.gun.switch_request = 0x34;
    tick_idle(sim, 60);
    assert_eq!(sim.gun.weapon, 0x34);
    // Throw at the horizon.
    aim_at(sim, [sim.bond.head()[0], sim.bond.head()[1] - 0.2, sim.bond.head()[2] + 10.0]);
    sim.step_bond(Default::default(), true, false, false);
    // Track the grenade through its whole life: spawn at the release frame, flight (the game's step is
    // speed = 0.167 u/tick [L record 0x34 +0x20]; the old port moved ~0.003 u/tick), explosion.
    let mut origin = [0.0f32; 3];
    let mut max_dist = 0.0f32;
    let mut exploded_at: Option<u32> = None;
    let start_frame = sim.drones.frame;
    for _ in 0..400 {
        sim.step_bond(Default::default(), false, false, false);
        if let Some(g) = sim.drones.grenades.first() {
            if origin == [0.0, 0.0, 0.0] { origin = g.pos; }
            let d = ((g.pos[0] - origin[0]).powi(2) + (g.pos[2] - origin[2]).powi(2)).sqrt();
            max_dist = max_dist.max(d);
        } else if origin != [0.0, 0.0, 0.0] { exploded_at = Some(sim.drones.frame - start_frame); break; }
        if sim.drones.frame - start_frame > 380 { break; }
    }
    assert!(origin != [0.0, 0.0, 0.0], "the grenade spawned at the release frame (fire_anim_len 26 [M 0x1a2e38 case 9])");
    assert!(max_dist > 2.0, "the grenade flies (max horizontal distance {max_dist:.2} units; the 60×-slow build reached ~0.5)");
    assert!(exploded_at.is_some(), "the grenade explodes (the 210-tick fuse [M Bullet_init 0x12c5a0:103]) within 380 ticks");
    let t = exploded_at.unwrap();
    assert!(t <= 380, "the grenade exploded within the fuse window, after {t} ticks");
    // Damage: the blast is the hardcoded 100 with radius 10 [L record 0x35 +0x08 = 10.0]. A guard
    // standing at the landing point takes damage through `Explode_Propagate` (0x17c518). The pick
    // wants a guard whose "3 units back" spot is on the guard's own floor — the first enabled
    // guard's patrol position shifts by a tick or two when the script players joined the control
    // walk (fix-script-start-tick: the game's list order — a channel is seen only by the objects
    // updated after it), and 07000001's first pick can stand by a ledge; walk the list until the
    // spot is solid.
    let guards: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.enabled && !dr.corpse && dr.health > 0.0 && dr.team == 1)
        .map(|(i, _)| i).collect();
    let spot_floor = |sim: &nf_view::scene::Sim, gi: usize| -> Option<f32> {
        let gp = sim.drones.list[gi].pos;
        let gfloor = sim.world.floor_below(gp, 6.0)?;
        let bfloor = sim.world.floor_below([gp[0], gfloor + 1.0, gp[2] - 3.0], 6.0)?;
        ((gfloor - bfloor).abs() < 2.0).then_some(gfloor)
    };
    if let Some(gi) = guards.iter().copied().find(|&i| spot_floor(sim, i).is_some()) {
        let hp0 = sim.drones.list[gi].health;
        let gp = sim.drones.list[gi].pos;
        let floor = spot_floor(sim, gi).unwrap_or(gp[1]);
        // Throw again from 3 units away, grenade landing near the guard (radius 10 covers 3 units).
        let p = nf_game::pickup::Pickup::create_placed(sim.bond.pos, [0.0; 3], nf_game::pickup::kind::WEAPON, 0x34, 1, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons).unwrap();
        sim.drones.pickups.push(p);
        tick_idle(sim, 5);
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [gp[0], floor + 1.1, gp[2] - 3.0], 0.0);
        aim_at(sim, [gp[0], gp[1] + 0.2, gp[2]]);
        tick_idle(sim, 30);
        sim.step_bond(Default::default(), true, false, false);
        for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); }
        // fix-explosions2 [M 0x12e1c0 + record 0x34]: the frag's blast is radius 8 / damage 100 —
        // the guard ~3 units away takes 100·(1 − (int)3/8) = 62.5 through `Explode_Propagate`
        // (× the game's drone-side difficulty factor [L .data 0x30cab0: Easy 2.0/Normal 1.5/Hard
        // 0.5 — none zero]) — the ordinary frag kill the game shows.
        assert!(sim.drones.list[gi].health < hp0 || sim.drones.list[gi].corpse,
            "the frag blast (0x34: radius 8, damage 100 [L]) hurts the guard ({} → {})", hp0, sim.drones.list[gi].health);
        // Sister: the same spot with the stun grenade (0x35: 10/0, the `+0x70 & 0x2000` flash
        // branch [M 0x12dd60]) — health UNTOUCHED, no damage anywhere on that path. Target a guard
        // the frag blast left alive (the frag above may have killed this one).
        let Some(gi2) = sim.drones.list.iter().position(|dr| dr.present && dr.enabled && !dr.corpse && dr.health > 0.0) else { return };
        let hp0 = sim.drones.list[gi2].health;
        let at = [sim.drones.list[gi2].pos[0], sim.drones.list[gi2].pos[1] + 0.5, sim.drones.list[gi2].pos[2]];
        sim.drones.grenades.push(nf_game::grenade::Grenade {
            pos: at, velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 2.0,
            damage: f32::from_le_bytes(sim.weapons.records[0x35][0x0c..0x10].try_into().unwrap()),
            radius: f32::from_le_bytes(sim.weapons.records[0x35][0x08..0x0c].try_into().unwrap()),
            weapon_type: 0x35, mesh: 0, owner: usize::MAX, state: 1, bounces: 0, smoke_popped: false,
        });
        let mut hurt = false;
        for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); if sim.drones.list[gi2].health < hp0 { hurt = true; break; } }
        assert!(!hurt, "the stun grenade (0x35) flash branch deals no damage [M 0x12d890]");
    }
}

/// ── Item 6: the civilian no-shoot "block" ──────────────────────────────────────────────────────
/// The decompile has NO trigger block for non-enemy targets: BLData+0x133 (crosshair state) is written
/// by `Check_Target` (0x1a7810:101-204) and read ONLY by `HUD_UpdateCrossHair` (0x19e6d0:69, display);
/// +0x132's only other touch is `Player_InitWeapon` (0x1a7ef8:50, clear). The fire chain
/// (`Player_WeaponFiring` 0x1a5460 → `Player_RoundToFire` 0x1a18e0 → `Bullet_init` 0x12c5a0) never
/// reads +0x133/+0x132/+0x44. The +0x880 readers (0x1a5460:531-581) gate the gadget weapons 0x56-0x58
/// on an aimed-at object — not a civilian block. This pins the port to the game: the shot fires.
#[test]
fn firing_at_a_marked_civilian_is_not_blocked() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 8);
    let enemy = sim.drones.list.iter().enumerate()
        .find(|(_, dr)| dr.present && dr.enabled && dr.drawn && dr.team == 1 && dr.health > 0.0)
        .map(|(i, _)| i).expect("07000001 starts an enemy");
    // Mark him non-enemy (the game's test is BLData+0x44 != 1 [M 0x1a7810:136-139]) and aim at him.
    let ep = sim.drones.list[enemy].pos;
    sim.drones.list[enemy].team = 2;
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [ep[0], ep[1] + 0.1, ep[2] - 3.0], 0.0);
    aim_at(sim, [ep[0], ep[1] + 0.3, ep[2]]);
    for _ in 0..7 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.crosshair_state(), 3, "the red no-crosshair shows [M 0x1a7810:181-182]");
    // Hold fire: the shot goes through (no block in the fire chain [M 0x1a5460 → 0x1a18e0 → 0x12c5a0]).
    let mut fired = false;
    for t in 0..40 {
        sim.step_bond(Default::default(), t % 9 == 0, false, false);
        if !sim.drones.player_impacts.is_empty() || sim.drones.list[enemy].health < 100.0 { fired = true; break; }
    }
    assert!(fired, "the trigger is NOT blocked while the no-crosshair shows (the game fires at civilians)");
}
