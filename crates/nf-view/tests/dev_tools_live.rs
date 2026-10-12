//! Headless NF_ISO tests for the fix-dev dev tools (cut-scene skip, infinite ammo, no reload, HUD clamp,
//! level select).
//!
//! Every F2 action needs a headless test (no panic + the effect), and each fix needs a test that would have
//! failed before it:
//!
//! - `dev_infinite_ammo_survives_sustained_fire` — the user's "infinite ammo isn't always reliable": the old
//!   refill only topped the gun's transient `gun.clip`/`gun.spare` copies, but `Sim::run` reads the INVENTORY
//!   (the game's own store, `BL+0x170`/`+0x1B8`) into those copies every tick, so the ammo kept emptying and
//!   the gun stopped firing while the HUD still showed a full clip. The test fires sustained shots through the
//!   real `step_bond` path with infinite ammo on and asserts the inventory still holds rounds.
//! - `dev_no_reload_keeps_clip_full` — the F2 "no reload" toggle: the current weapon's clip is topped to its
//!   `weapon_data` clip size each tick (0x92..0x94), so it never empties and the game never reloads.
//! - `dev_skip_ends_the_gatehouse_cutscene` — the skip ports the game's own one: `SP_Update` state 2 checks
//!   `Input_Action(-1, 0x19, 1)` and calls `Script_FFwd(cur, 1)` unless the script is a death NIS
//!   [M SP_Update 0x1dbda8:129-141, Script_FFwd 0x1dac78, Script_Update 0x1da998:19-110]. The test drives the
//!   exact field the viewer's key handler writes (`Scripts::skip_request`) and asserts the cut-scene ends with
//!   its end-of-script effects applied (the player handback). Sister phase: without skip the cut-scene is still
//!   running 150 ticks later.
//! - `hud_ammo_strip_clamped` — the user's stacked/garbled ammo icons: the strip's drawn height is clamped to
//!   the weapon's clip size, so a dev-refilled clip (or any clip above its size) can't overflow the pane.
//! - `dev_level_select_rows_and_load` — the F2 level-select rows exist for every level archive on the disc and
//!   the archives load headless through the same `nf_view::load` path the menu action feeds.
//!
//! Opt-in via `NF_ISO`.

use nf_view::dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A loaded scene (Bond at the level start) — the same path the viewer uses.
fn loaded() -> Option<nf_view::scene::Scene> {
    let iso = iso()?;
    Some(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"))
}

/// One tick of the viewer's loop: `step_bond`, then the dev after-tick (god mode / infinite ammo), in that
/// order — lib.rs:801-844.
fn viewer_tick(scene: &mut nf_view::scene::Scene, dev: &mut dev::Dev, trigger: bool) {
    if let Some(sim) = scene.sim.as_mut() { sim.step_bond(Default::default(), trigger, false, false); }
    dev.after_tick(scene);
}

/// The weapon's clip size (`weapon_data+0x92..0x94`) for Bond's current gun.
fn clip_size_of(scene: &nf_view::scene::Scene) -> f32 {
    let sim = scene.sim.as_ref().unwrap();
    let w = sim.gun.weapon as usize;
    u16::from_le_bytes(sim.weapons.records[w][0x92..0x94].try_into().unwrap()) as f32
}

/// Sustained fire with infinite ammo must never run out — the user's "isn't always reliable".
///
/// Before the fix the refill only touched `gun.clip`/`gun.spare` (transient copies `Sim::run` overwrites from
/// the inventory each tick), so the inventory emptied, the gun stopped firing, and the HUD still showed a full
/// clip. This test fails on the old code after the (clip + spare) rounds are spent.
#[test]
fn dev_infinite_ammo_survives_sustained_fire() {
    let Some(mut scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut dev = dev::Dev { infinite_ammo: true, ..Default::default() };
    // Settle the draw script, then drain everything: the current weapon's clip AND its carried spare.
    viewer_tick(&mut scene, &mut dev, false);
    {
        let sim = scene.sim.as_mut().unwrap();
        let w = sim.gun.weapon as usize;
        let t = sim.weapons.ammo_type(w);
        assert!(w < sim.drones.inventory.clip.len(), "weapon {w} has no clip slot");
        sim.drones.inventory.clip[w] = 2;
        sim.drones.inventory.carried[t] = 2;
        sim.gun.clip = 2;
        sim.gun.spare = 2;
        eprintln!("[ammo] weapon {w} ammo type {t}: drained to 2 + 2 (clip size {})", clip_size_of(&scene));
    }
    // 240 ticks, trigger every other tick (semi-auto pulls): at least (2 + 2) shots must be possible; with
    // infinite ammo every one of them fires and the store never empties.
    for i in 0..240 {
        viewer_tick(&mut scene, &mut dev, i % 2 == 0);
    }
    let sim = scene.sim.as_ref().unwrap();
    let w = sim.gun.weapon as usize;
    let t = sim.weapons.ammo_type(w);
    let clip = sim.drones.inventory.clip[w];
    let carried = sim.drones.inventory.carried[t];
    eprintln!("[ammo] after 240 ticks (120 pulls): inventory clip {clip}, carried {carried}, gun {} / {}", sim.gun.clip, sim.gun.spare);
    assert!(clip > 0, "infinite ammo ran out: the current weapon's clip slot is empty after 120 pulls (got {clip})");
    assert!(carried > 0, "infinite ammo ran out: the weapon's carried spare is empty (got {carried})");
    assert!(sim.gun.clip > 0, "the gun's clip is empty with infinite ammo on");
    // The refill must cover the whole store, not just the current weapon: every OWNED clip slot stays
    // topped (a weapon switch mid-fight must not land on an empty gun — the switch cycle only offers
    // owned weapons). fix-pickups-trace: the refill is the recorder's `--ammo` cheat (tools/trace/
    // trace_record.py): owned clips and all spares topped back to the most seen this run — unowned
    // slots stay 0, as the trace's first Raptor take requires (clip 0→9, no spare leak).
    let empty_clips: Vec<usize> = sim.drones.inventory.clip.iter().enumerate()
        .filter(|(i, &c)| c <= 0 && sim.weapons.clip_size(*i) > 0 && sim.drones.inventory.owned.get(*i).copied().unwrap_or(false))
        .map(|(i, _)| i).collect();
    assert!(empty_clips.is_empty(), "infinite ammo left owned clip slots empty: {empty_clips:?} (a switch to them would find no ammo)");
}

/// Sister assertion: with infinite ammo OFF the same fire drains the store (the test can fail).
#[test]
fn sister_infinite_ammo_off_still_runs_out() {
    let Some(mut scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut dev = dev::Dev::default(); // infinite_ammo: false
    viewer_tick(&mut scene, &mut dev, false);
    let t;
    {
        let sim = scene.sim.as_mut().unwrap();
        let w = sim.gun.weapon as usize;
        t = sim.weapons.ammo_type(w);
        sim.drones.inventory.clip[w] = 2;
        sim.drones.inventory.carried[t] = 2;
        sim.gun.clip = 2;
        sim.gun.spare = 2;
    }
    for i in 0..240 { viewer_tick(&mut scene, &mut dev, i % 2 == 0); }
    let sim = scene.sim.as_ref().unwrap();
    let w = sim.gun.weapon as usize;
    let clip = sim.drones.inventory.clip[w];
    eprintln!("[ammo] sister (OFF): clip {clip}, carried {} after 240 ticks", sim.drones.inventory.carried[t]);
    assert_eq!(clip, 0, "sister: without infinite ammo the clip must run dry (got {clip}) — the main test must be able to fail");
}

/// The F2 "no reload" toggle: the current weapon's clip is topped to its clip size each tick, so it never
/// empties and the game never reloads.
#[test]
fn dev_no_reload_keeps_clip_full() {
    let Some(mut scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut dev = dev::Dev { no_reload: true, ..Default::default() };
    viewer_tick(&mut scene, &mut dev, false);
    let cs = clip_size_of(&scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        let w = sim.gun.weapon as usize;
        sim.drones.inventory.clip[w] = 1;
        sim.gun.clip = 1;
    }
    dev.after_tick(&mut scene);
    let sim = scene.sim.as_ref().unwrap();
    let w = sim.gun.weapon as usize;
    let clip = sim.drones.inventory.clip[w];
    assert_eq!(clip as f32, cs, "no reload did not top the clip to its size (clip {clip}, size {cs})");
    assert_eq!(sim.gun.clip as f32, cs, "no reload did not top the gun's copy");
    // Sister: toggle off and the clip stays where it is (the tool is off — no game change).
    dev.no_reload = false;
    {
        let sim = scene.sim.as_mut().unwrap();
        let w = sim.gun.weapon as usize;
        sim.drones.inventory.clip[w] = 3;
        sim.gun.clip = 3;
    }
    dev.after_tick(&mut scene);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.inventory.clip[w], 3, "no reload is OFF: the clip must be left alone");
}

/// The cut-scene skip, on the level's own cut-scene script players: 0x60008C8 (out ch 92) and 0x6000994
/// (out ch 67) are camera cut-scenes waiting on their activation channels — the game's own chains set those
/// channels (here the way the game would, as the K key does), and Bond is disabled by their event 4 while
/// they play.
///
/// Phase A (sister): 150 idle ticks and Bond is still held. Phase B: mashing any key (the field the viewer's
/// key handler writes) arms the fade (`Script_FFwd`, ≤ 45 ticks) and then `Script_Update`'s catch-up loop
/// runs the remaining frames in one tick — both scripts reach their done bit with the end effects applied
/// (control returns: the handback event 6 → `Player_Enable`).
///
/// Note: the ambient truck script (0x6000A22) plays from level start with NO camera (`nis.camera` false) —
/// the game's own skip requires the camera flag (`Script_FFwd` 0x1dac78:19-21), so it is correctly NOT
/// skippable. The death NIS (0x6000952, `Script_IsDeathNIS`) is excluded in the unit test below.
#[test]
fn dev_skip_ends_the_gatehouse_cutscene() {
    let Some(mut scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut cutscene = |scene: &nf_view::scene::Scene| -> Option<(u32, f32, u8, bool)> {
        scene.sim.as_ref().unwrap().drones.scripts.players.iter()
            .find(|p| matches!(p.scripts[0], 0x0600_08c8 | 0x0600_0994) && p.state == 2)
            .and_then(|p| p.info[p.cur].as_ref().map(|s| (p.scripts[0], s.clock, s.play, s.fade_flags & 0x10 != 0)))
            .filter(|(_, _, play, _)| play & 1 != 0)
    };
    // Bond walks (the real per-tick path) while the game's activation channels for the two cut-scene SPs go
    // on (set the way the game's trigger chains set them — the same thing K does for every channel).
    let mut started = false;
    for t in 0..300u32 {
        {
            let sim = scene.sim.as_mut().unwrap();
            if t == 30 { sim.drones.set_channel(92); sim.drones.set_channel(67); }
            sim.bond.yaw = std::f32::consts::FRAC_PI_2;
            sim.step_bond(nf_game::player::Input { forward: 1.0, ..Default::default() }, false, false, false);
        }
        if let Some((h, c, p, cam)) = cutscene(&scene) {
            if cam && c > 2.0 { eprintln!("[skip] cut-scene {h:08x} playing (clock {c}, play {p:#x}, camera on, Bond disabled={})", scene.sim.as_ref().unwrap().disabled); started = true; break; }
        }
    }
    assert!(started, "no cut-scene script player (0x60008C8 / 0x6000994) started with its camera within 300 ticks");
    assert!(scene.sim.as_ref().unwrap().disabled, "the cut-scene did not take control (event 4 never fired)");
    // Phase A (sister): 150 ticks WITHOUT skip — Bond still held.
    for _ in 0..150 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (_, clock_a, play_a, _) = cutscene(&scene).expect("the cut-scene player vanished");
    assert_eq!(play_a & 2, 0, "sister: the cut-scene finished within 150 idle ticks — the skip phase proves nothing");
    eprintln!("[skip] sister: after 150 idle ticks still playing (clock {clock_a}, play {play_a:#x})");
    // Phase B: mash any key every tick. The viewer's key handler (lib.rs) sets exactly this field.
    let mut ended_at: Option<u32> = None;
    for i in 0..150 {
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.drones.scripts.skip_request = true;
            sim.step_bond(Default::default(), false, false, false);
        }
        if scene.sim.as_ref().unwrap().drones.scripts.players.iter()
            .filter(|p| matches!(p.scripts[0], 0x0600_08c8 | 0x0600_0994) && p.state == 2)
            .all(|p| p.info[p.cur].as_ref().is_none_or(|s| s.play & 2 != 0)) {
            ended_at = Some(i);
            break;
        }
    }
    assert!(ended_at.is_some(), "the cut-scene did not end within 150 ticks of mashing skip (fade ≤ 45 ticks + one fast-forward tick)");
    eprintln!("[skip] the cut-scene finished {} ticks after the mash started", ended_at.unwrap());
    // The end effects: control returned (the handback event 6 → `Player_Enable`; `Sim::disabled` clears).
    let mut control = false;
    for _ in 0..120 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        if !sim.disabled { control = true; break; }
    }
    assert!(control, "control did not return after the skipped cut-scene (the end-of-script handback never ran)");
}

/// The HUD ammo strip clamps what it draws to the weapon's clip size — the user's stacked/garbled icons when
/// a dev tool pushes the clip past its size.
#[test]
fn hud_ammo_strip_clamped() {
    let Some(mut scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let cs = clip_size_of(&scene);
    // The helper (what the HUD draws with): an overflowing clip draws at the clip size, anything below as-is.
    assert_eq!(nf_view::hud::strip_clip(500.0, cs), cs, "an overflowing clip must draw at the clip size");
    assert_eq!(nf_view::hud::strip_clip(cs, cs), cs, "a full clip draws at the clip size");
    assert_eq!(nf_view::hud::strip_clip(cs * 0.5, cs), cs * 0.5, "a low clip draws as-is");
    assert_eq!(nf_view::hud::strip_clip(0.0, cs), 0.0, "an empty clip draws nothing");
    // The real overlay: with the clip pushed past its size, no bullets-strip quad is taller than clip_size.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.clip = 500;
    }
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let sim = scene.sim.as_ref().unwrap();
    let w = sim.gun.weapon as usize;
    let t = sim.weapons.ammo_type(w);
    let hpr = scene.hud.as_ref().expect("hud tables").bullets[t][3];
    let bullets_tex = scene.hud_textures.iter().position(|(h, _)| *h == 0x0300_0128).expect("the bullets strip texture is loaded") + 1;
    let max_h = f.runs.iter().filter(|r| r.0 == bullets_tex).flat_map(|&(_, _, s, c)| {
        f.v[s as usize..(s + c) as usize].chunks(6).map(|q| {
            q.iter().map(|v| v.p[1]).fold(f32::MIN, f32::max) - q.iter().map(|v| v.p[1]).fold(f32::MAX, f32::min)
        })
    }).fold(0.0f32, f32::max);
    let limit = cs * hpr / 224.0 * 1.02 + 1e-4;
    eprintln!("[hud] clip 500 vs size {cs}: tallest bullets-strip quad {max_h:.4} NDC, limit {limit:.4}");
    assert!(max_h <= limit, "the ammo strip drew {:.4} NDC tall (> {:.4}): the infinite-ammo overflow the user screenshotted", max_h, limit);
}

/// The F2 level-select rows exist for every level archive on the disc, the menu action carries the right id,
/// and the archives load headless through the same path the action feeds (`pending_level` → `load_with_carry`).
#[test]
fn dev_level_select_rows_and_load() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    assert!(!dev::LEVELS.is_empty(), "the level-select list is empty");
    let items = dev::menu_items(&dev::Dev::default());
    for &id in dev::LEVELS {
        assert!(items.iter().any(|(_, _, a)| matches!(a, dev::MenuAction::Level(x) if *x == id)),
            "no level-select row for {id:08x}");
    }
    // The rows are navigable: activating a level row yields its MenuAction.
    let idx = items.iter().position(|(_, _, a)| matches!(a, dev::MenuAction::Level(x) if *x == dev::LEVELS[0])).unwrap();
    let mut d = dev::Dev::default();
    d.menu_sel = idx;
    assert!(matches!(dev::menu_key(&mut d, dev::MenuKey::Activate), Some(dev::MenuAction::Level(_))));
    // The first three archives load headless (the rest exist per the manifest; 34 loads would be slow).
    for &id in &dev::LEVELS[..3.min(dev::LEVELS.len())] {
        let name = nf_view::level_archive_name(id);
        let scene = nf_view::load(&iso, &name, false);
        assert!(scene.is_ok(), "level-select archive {name} does not load");
        eprintln!("[levels] {name} loads ({} cels)", scene.unwrap().characters.len());
    }
}
