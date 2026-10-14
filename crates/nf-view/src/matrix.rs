//! The weapon matrix (task 100d): every on-foot level × every weapon the dev tools can hand Bond.
//!
//! Per cell: load the level (the normal `nf_view::load` path — no dev-view-model merge), give the
//! weapon (the `--give` path — `panel::apply`'s "weapon" command), draw it through the real input
//! path (`Sim::step_bond` with `switch_to`), fire it 3 times (hold-to-use gadgets hold the trigger),
//! reload, switch away to the P2K (0x02) and back, and record what the sim did: the view model's
//! presence and muzzle-flash attachment, the view-anim scripts played (`Sim::view_script`), the
//! muzzle-flash ticks (`Sim::shot_tick`), the sound ids requested (`Sound_Play3D`'s list,
//! `Drones::sounds`) and started (the driver's `Cmd::Start` batch), the effects spawned (player
//! bullets, impacts, explosions, grenades, the laser beam) and any panic. A panic is a red cell, not
//! a crashed run: the cell's sequence runs under `catch_unwind` and the level is reloaded after one.
//!
//! The watch gadgets (Q-Claw grapple, Q-Laser, stunner) are always carried on every single-player
//! map (the user's correction, 2026-10-13), so their `fire` results are informational (`q`/`s`
//! marks), not failures: the Q-Claw's quietness here only says the test spot has no grapple point.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;

use crate::panel;
use crate::scene::{Scene, Sim};

/// The view-model build's sleeve hashes (scene.rs "The first-person gun models" — the same set,
/// quoted for the red-cell reasons; the build needs one of them in the level archive).
const SLEEVES: [u32; 8] = [0x0200_09d0, 0x0200_0900, 0x0200_01f7, 0x0200_0929, 0x0200_09e6, 0x0200_09ff, 0x0200_0a14, 0x0200_0a13];

/// The weapon the sequence switches away to (the P2K — Bond's own start weapon on 07000005).
const AWAY: u8 = 0x02;

/// One level × weapon cell's measurements.
#[derive(Default, Clone)]
pub struct Cell {
    pub weapon: u8,
    pub name: String,
    /// The `--give` path equipped it and returned the switch request.
    pub give_ok: bool,
    /// The record has no first-person model (`weapon_data+0xDC == 0`) — the game's own refusal
    /// (`Player_EquipWeapon` 0x1a8a58:20); giving is then impossible by design.
    pub no_fp_record: bool,
    /// The record's first-person skin hash (`weapon_data+0xDC`), when non-zero — the red-cell
    /// reasons check the level archive for it.
    pub fp_hash: Option<u32>,
    /// The draw finished (`gun.weapon == id`) — directly, or through the mode button.
    pub draw_ok: bool,
    /// The draw needed the mode button (`Player_Weapon` 0x1a24c8's twin chain, record `w[5]`):
    /// variant records (the P2K's silenced/dual modes etc.) refuse a direct `switch_to`.
    pub via_mode: bool,
    /// The give's own status text (the panel's refusal reason for a `.`/`n` cell).
    pub give_msg: Option<String>,
    /// A first-person view model was built for it in the level (`Scene::view_models`).
    pub view_model: bool,
    /// The view model's muzzle-flash attachment exists (record `weapon+0x48` mesh on skin datum 0).
    pub flash_attach: bool,
    /// The view-anim scripts played over the whole sequence (distinct `Sim::view_script` values).
    pub sequences: BTreeSet<u32>,
    /// Muzzle-flash ticks observed while firing (`Sim::shot_tick`).
    pub shots: u32,
    /// Sound ids requested this sequence (`Sound_Play3D`'s per-tick list, `Drones::sounds`).
    pub sounds: BTreeSet<u32>,
    /// Sound ids the audio driver was asked to start (`Cmd::Start`, `Drones::audio`).
    pub started: BTreeSet<u32>,
    /// Player bullets spawned (`Drones::player_bullets` growth while firing).
    pub bullets: u32,
    /// Bullet impact events (`Drones::impacts`, cleared per tick; accumulated).
    pub impacts: u32,
    /// Explosion events (`Drones::explosions`, cleared per tick; accumulated).
    pub explosions: u32,
    /// Grenades/throwables spawned (`Drones::grenades` growth while firing).
    pub grenades: u32,
    /// The laser-family beam showed this tick (`Sim::laser_beam`).
    pub beam: bool,
    /// The reload ran (state 5/6/7 seen, returned to 0) or the clip was already full.
    pub reload_ok: bool,
    /// The reload was not applicable (a gadget, or a clip ≤ 1 weapon).
    pub reload_na: bool,
    /// The switch away to the P2K finished.
    pub away_ok: bool,
    /// The switch back to the weapon finished.
    pub back_ok: bool,
    /// Why a red cell is red (the missing asset, from the level archive's own hash set).
    pub reason: Option<String>,
    /// The cell panicked or hard-failed (a panic is a bug per the task).
    pub error: Option<String>,
}

impl Cell {
    /// Anything observable from the trigger during the fire phase.
    fn fire_events(&self) -> u32 { self.shots + self.bullets + self.impacts + self.explosions + self.grenades + self.beam as u32 }

    /// The cell's mark: `-` clean; a red letter for a real failure; a dim letter for the by-design
    ////informational cases (the watch gadgets' quiet fire, a record without a first-person model).
    pub fn mark(&self) -> char {
        if self.error.is_some() { '!' }
        else if !self.give_ok && !self.no_fp_record { '.' }
        else if !self.give_ok && self.no_fp_record { 'n' }
        else if !self.view_model { 'I' }
        else if !self.draw_ok { 'D' }
        else if self.fire_events() == 0 && !self.quiet_fire() { 'F' }
        else if !self.reload_ok && !self.reload_na && !self.quiet_reload() { 'R' }
        else if !self.away_ok { 'W' }
        else if !self.back_ok { 'B' }
        else if self.sounds.is_empty() && self.started.is_empty() && !self.quiet_fire() { 'S' }
        else { '-' }
    }

    /// Hold-to-use gadgets (ids ≥ 0x48 — the gadget cycle starts at 0x4A, 0x48/0x49 are padding):
    /// their fire/silence is informational, not a failure (the Q-Claw has nothing to latch here,
    /// the laser/stunner's fire model is the fix-weapon-effects task's business).
    fn quiet_fire(&self) -> bool { self.weapon >= 0x48 }

    /// Gadgets and clip ≤ 1 weapons have no reload to run.
    fn quiet_reload(&self) -> bool { self.weapon >= 0x48 }
}

/// One level's row: the level archive and a cell per weapon (in catalog order).
#[derive(Default, Clone)]
pub struct LevelRow {
    pub level: String,
    pub cells: Vec<Cell>,
}

/// Run the whole matrix: every on-foot level in FILES.BIN (the sweep's set — 0700xxxx.bin minus
/// the driving missions 0x23-0x29, the shared packs and the front-end's own code-0x00 archives).
pub fn run(iso: &str, levels: Option<&[String]>, log: &mut dyn FnMut(&str)) -> anyhow::Result<Vec<LevelRow>> {
    let img = IsoImage::open(iso)?;
    let names = match levels {
        Some(l) => l.to_vec(),
        None => on_foot_levels(&img),
    };
    let mut rows = Vec::new();
    for name in &names {
        log(&format!("[matrix] level {name}"));
        match run_level(iso, name, log) {
            Ok(r) => rows.push(r),
            Err(e) => {
                log(&format!("[matrix] level {name} FAILED to load: {e}"));
                rows.push(LevelRow { level: name.clone(), cells: Vec::new() });
            }
        }
    }
    Ok(rows)
}

/// The sweep's on-foot level set (levels_sweep_live.rs's `on_foot_levels`, verbatim).
pub fn on_foot_levels(img: &IsoImage) -> Vec<String> {
    use nf_formats::disc;
    let (entries, _) = disc::files_bin_entries(img).unwrap();
    let mut names: Vec<String> = entries.iter()
        .map(|e| e.name.to_ascii_lowercase())
        .filter(|n| n.starts_with("0700") && n.ends_with(".bin"))
        .filter(|n| {
            let code = u32::from_str_radix(&n[..8], 16).unwrap_or(0) & 0xff;
            !(0x23..=0x29).contains(&code) && code != 0x00 && n != "07000500.bin" && n != "07000999.bin"
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Run one level's row: load once, then a cell per weapon the dev catalog lists. A panicking cell
/// is recorded (`Cell::error`) and the level is reloaded before the next cell (the sim's state
/// after a panic is not trusted).
pub fn run_level(iso: &str, level: &str, log: &mut dyn FnMut(&str)) -> anyhow::Result<LevelRow> {
    // The tests pass the bare code ("07000005"); the archive name is 07000005.bin.
    let level = if level.contains('.') { level.to_string() } else { format!("{level}.bin") };
    let mut row = LevelRow { level: level.clone(), cells: Vec::new() };
    // The level archive's own hash set — the red-cell reasons read it (the view-model build's four
    // conditions all resolve against `level.skins` / `level.by_hash`, scene.rs "The first-person
    // gun models").
    let img = IsoImage::open(iso)?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&level))
        .ok_or_else(|| anyhow::anyhow!("no {level} in FILES.BIN"))?;
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let lvl = Level::parse(&data)?;

    let mut scene = crate::load(iso, &level, false)?;
    anyhow::ensure!(scene.sim.is_some(), "level {level} loaded without a sim");
    // Wait out the level's intro cut-scene: `Player_Disable` by a script blocks every input until
    // `Player_Enable` (scene.rs's `Sim::disabled` — the F cells on 07000001/02/04 were the manor's
    // intro, not the weapon). The matrix gives no input while it holds.
    wait_enabled(&mut scene, 7200);
    // The away weapon: give the P2K up front so the switch-away step is always possible.
    give_via_panel(&mut scene, AWAY, None);
    let weapons: Vec<(u8, String)> = panel::catalog_entries(&scene).into_iter().map(|e| (e.id, e.name)).collect();
    for (id, name) in weapons {
        wait_enabled(&mut scene, 600);
        let mut cell = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_cell(&mut scene, id)))
            .unwrap_or_else(|p| Cell { weapon: id, error: Some(panic_text(p)), ..Default::default() });
        cell.name = name;
        if cell.error.is_some() {
            log(&format!("[matrix] {level} weapon {id:#04x} PANICKED — reloading the level"));
            scene = crate::load(iso, &level, false)?;
            give_via_panel(&mut scene, AWAY, None);
        }
        annotate(&lvl, &mut cell);
        log(&format!("[matrix] {level} weapon {:#04x} {}: {}", id, cell.name, cell_summary(&cell)));
        row.cells.push(cell);
    }
    Ok(row)
}

/// One cell's sequence: give → draw → fire ×3 → reload → switch away → switch back, all through
/// the real input path (`Sim::step_bond`; the give is `panel::apply`'s "weapon" — the `--give`
/// command's own path).
fn run_cell(scene: &mut Scene, id: u8) -> Cell {
    let mut c = Cell { weapon: id, ..Default::default() };
    let (clip, fp) = {
        let Some(sim) = scene.sim.as_ref() else { c.error = Some("no sim".into()); return c };
        (sim.weapons.clip_size(id as usize), sim.weapons.first_person(id as usize))
    };
    c.no_fp_record = fp == 0;
    c.fp_hash = (fp != 0).then_some(fp);

    // 1. Give (the `--give` path: panel::apply's "weapon" command — equip + 3 clips; a partial
    //    first clip so the reload has work). The game refuses an already-owned weapon when the
    //    shared ammo bank is at cap (`equip_weapon`'s owned branch — `Player_EquipWeapon`
    //    0x1a8a58); the cell records the refusal (the reason names it) and stops: proceeding
    //    without fresh ammo turns the cell into an NPC-interference magnet (the guards reach the
    //    start spot mid-run on some levels — see the report's caveats).
    let rounds = if clip >= 4 { clip.min(8) } else { clip.max(1) };
    let (ok, msg) = {
        let mut q = vec![("do".to_string(), "weapon".to_string()), ("id".to_string(), format!("0x{id:02x}")), ("rounds".to_string(), rounds.to_string())];
        let (m, sw) = panel::apply(scene, &q);
        (sw == Some(id), Some(m))
    };
    c.give_ok = ok;
    c.give_msg = msg.filter(|_| !ok);
    if !c.give_ok { return c; }

    // 2. Draw: the real input path. A direct `switch_to` reaches every base record; the mode
    //    button (`Player_Weapon` 0x1a24c8 — the twin chain on record `w[5]`, `fire_mode_switch`)
    //    reaches the variant records (the probe: 0x03 refused `switch_to` while its base 0x02 was
    //    held; the player reaches it the same way). 25 settle ticks sample the draw anim's script
    //    and sounds.
    let mut drew = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        sample(&mut c, scene);
        if scene.sim.as_ref().unwrap().gun.weapon == id { drew = true; break; }
    }
    if !drew {
        if mode_to(scene, id) { c.via_mode = true; drew = true; }
    }
    c.draw_ok = drew;
    for _ in 0..25 { if drew { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); sample(&mut c, scene); } }
    c.view_model = scene.view_models.contains_key(&id);
    if let Some(&ci) = scene.view_models.get(&id) {
        c.flash_attach = scene.attachments.iter().any(|a| a.character == ci as u32 && a.flash.is_some());
    }
    if !c.draw_ok { return c; }

    // 3. Fire ×3. Gadgets (hold-to-use) hold the trigger a fixed 45 ticks per press; guns hold
    //    until one new shot tick (or a spawn) then release. Between presses a short release.
    let (mut grenades, mut bullets) = { let sim = scene.sim.as_ref().unwrap(); (sim.drones.grenades.len(), sim.drones.player_bullets.len()) };
    let mut impacts = 0u32;
    let mut explosions = 0u32;
    for _attempt in 0..3 {
        let hold = if id >= 0x48 { 45 } else { 60 };
        let (shots_before, gren_before) = (c.shots, c.grenades);
        for _ in 0..hold {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
            let (st, beam, imp, exp, g, b) = { let sim = scene.sim.as_ref().unwrap();
                (sim.shot_tick, sim.laser_beam.is_some(), sim.drones.impacts.len(), sim.drones.explosions.len(),
                 sim.drones.grenades.len(), sim.drones.player_bullets.len()) };
            if st { c.shots += 1; }
            c.beam |= beam;
            impacts += imp as u32;
            explosions += exp as u32;
            if g > grenades { c.grenades += (g - grenades) as u32; grenades = g; }
            if b > bullets { c.bullets += (b - bullets) as u32; bullets = b; }
            sample(&mut c, scene);
            if c.shots > shots_before || c.grenades > gren_before { break; } // this press fired — release
        }
        for _ in 0..6 {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            let (imp, exp, g, b) = { let sim = scene.sim.as_ref().unwrap();
                (sim.drones.impacts.len(), sim.drones.explosions.len(), sim.drones.grenades.len(), sim.drones.player_bullets.len()) };
            impacts += imp as u32;
            explosions += exp as u32;
            if g > grenades { c.grenades += (g - grenades) as u32; grenades = g; }
            if b > bullets { c.bullets += (b - bullets) as u32; bullets = b; }
            sample(&mut c, scene);
        }
    }
    c.impacts = impacts;
    c.explosions = explosions;

    // Settle: the fire script (view-anim state 9) refuses switches AND swallows the reload edge
    // while it plays (`Player_WeaponChange` 0x1a6f80:33-47 refuses every switch in state 9); the
    // press loop breaks on the first new shot, so the last script is still running here. The
    // player's own R press lands after the script ends — the matrix waits the same way.
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); sample(&mut c, scene); }

    // 4. Reload (gadgets have none): pulse the R edge, ride the 5/6/7 machine back to 0.
    if id < 0x48 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, true, false);
        let mut started = false;
        for _ in 0..240 {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            let st = scene.sim.as_ref().unwrap().gun.reload_state;
            if st != 0 { started = true; }
            sample(&mut c, scene);
            if started && st == 0 { c.reload_ok = true; break; }
        }
        // A full clip refuses the reload by design (nothing to refill): not a red cell.
        if !started { let sim = scene.sim.as_ref().unwrap(); c.reload_ok = sim.gun.clip as i16 >= clip; }
    } else {
        c.reload_na = true;
        c.reload_ok = true;
    }

    // 5. Switch away to the P2K and back (direct `switch_to`; the mode-button fallback is only
    //    for the draw of a variant record — cycling away from 0x02 would move the weapon off the
    //    target, not onto it).
    c.away_ok = switch_to(scene, AWAY, false);
    for _ in 0..25 { if c.away_ok { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); sample(&mut c, scene); } }
    c.back_ok = switch_to(scene, id, true);
    for _ in 0..25 { if c.back_ok { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); sample(&mut c, scene); } }
    c
}

/// The `--give` path: `panel::apply`'s "weapon" command (the dev menu's Give button and lib.rs's
/// `--give` flag both land here — `Player_EquipWeapon` 0x1a8a58 + three clips). `rounds` None = a
/// full clip. Returns whether the equip happened (the switch request came back).
fn give_via_panel(scene: &mut Scene, id: u8, rounds: Option<i16>) -> bool {
    let mut q = vec![("do".to_string(), "weapon".to_string()), ("id".to_string(), format!("0x{id:02x}"))];
    if let Some(r) = rounds { q.push(("rounds".to_string(), r.to_string())); }
    let (_, sw) = panel::apply(scene, &q);
    sw == Some(id)
}

/// Step with no input until the script releases the player (`Sim::disabled` clears), or `cap`.
fn wait_enabled(scene: &mut Scene, cap: u32) -> bool {
    for _ in 0..cap {
        if scene.sim.as_ref().map_or(true, |s| !s.disabled) { return true; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    scene.sim.as_ref().map_or(false, |s| !s.disabled)
}

/// Switch away and back through the real input path (direct `switch_to`, then the mode button's
/// twin chain for the variant records); true when the weapon is held.
fn switch_to(scene: &mut Scene, id: u8, allow_mode: bool) -> bool {
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id { return true; }
    }
    allow_mode && mode_to(scene, id)
}

/// Reach a variant record with the mode button (`Player_Weapon` 0x1a24c8's twin chain, record
/// `w[5]` → `fire_mode_switch`). Each press needs the machine idle (`switch_state == 0`,
/// `reload_state == 0`, no fire script — the game refuses otherwise [M 0x1a24c8]), and a press
/// starts a holster+draw (~20 ticks), so the loop waits the machine out between presses.
fn mode_to(scene: &mut Scene, id: u8) -> bool {
    for _ in 0..12 {
        for _ in 0..40 {
            let sim = scene.sim.as_ref().unwrap();
            if sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        }
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { mode_switch: true, ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id { return true; }
    }
    false
}

/// Per-tick observation: sound requests, the driver's batch (built here the way the windowed
/// loop's `sound_tick` builds it — lib.rs 966 — so headless `started` records what would play),
/// and the view scripts.
fn sample(c: &mut Cell, scene: &mut Scene) {
    let Some(sim) = scene.sim.as_mut() else { return };
    for s in &sim.drones.sounds { if s.id != 0 { c.sounds.insert(s.id); } }
    let (pos, f) = (sim.bond.head(), sim.bond.look());
    let norm = { let l = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6); [f[2] / l, 0.0, -f[0] / l] };
    sim.drones.map_sounds.update(&mut sim.drones.audio, pos);
    sim.drones.audio.update(sim.sfx.as_ref(), (pos, norm, [0.0, 1.0, 0.0], f), &|_| false);
    for cmd in sim.drones.audio.take_commands() {
        if let nf_game::sound::Cmd::Start { id, .. } = cmd { if id != 0 { c.started.insert(id); } }
    }
    if sim.view_script != 0 { c.sequences.insert(sim.view_script); }
}

/// Fill the red-cell reason from the level archive's own hash set (the view-model build's four
/// conditions and the mesh fields the build reads).
fn annotate(lvl: &Level, c: &mut Cell) {
    if c.error.is_some() || c.view_model || !c.give_ok { return; }
    if let Some(fp) = c.fp_hash {
        if !lvl.skins.contains_key(&fp) {
            c.reason = Some(format!("fp skin {fp:08x} not in the level archive"));
            return;
        }
        if let Some(skin) = lvl.skins.get(&fp) {
            if !SLEEVES.iter().any(|h| lvl.by_hash.contains_key(h)) {
                c.reason = Some("no sleeve mesh in the level archive".into());
            } else {
                c.reason = Some(format!("skin skeleton {:08x} missing", skin.skeleton));
            }
        }
    } else {
        c.reason = Some("no first-person record (+0xDC == 0)".into());
    }
}

/// The compact mark table: levels as rows, weapon ids as columns (2-char hex per column).
pub fn mark_table(rows: &[LevelRow]) -> String {
    let mut ids: Vec<u8> = Vec::new();
    for r in rows { for c in &r.cells { if !ids.contains(&c.weapon) { ids.push(c.weapon); } } }
    ids.sort();
    let mut out = String::new();
    let _ = writeln!(out, "level       {}", ids.iter().map(|i| format!("{i:02x} ")).collect::<String>());
    let _ = writeln!(out, "marks       {}", ids.iter().map(|_| "---").collect::<String>());
    for r in rows {
        let _ = write!(out, "{:12}", r.level.trim_end_matches(".bin"));
        for id in &ids {
            let m = r.cells.iter().find(|c| c.weapon == *id).map_or(' ', |c| c.mark());
            let _ = write!(out, "{m}  ");
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "marks: - clean | ! panic/error | . give refused | n record has no fp model (by design) | I no view model | D draw failed | F no observable fire | R reload broken | W switch-away failed | B switch-back failed | S silent");
    let _ = writeln!(out, "(the watch gadgets 0x4a+ — Q-Claw, Q-Laser, stunner — are informational: q = no observable fire, s = silent, by the 2026-10-13 correction they are carried everywhere and their fire model is not a matrix failure)");
    out
}

/// The per-weapon aggregate across levels (the numbers behind the marks).
pub fn aggregate_table(rows: &[LevelRow]) -> String {
    let mut ids: Vec<u8> = Vec::new();
    for r in rows { for c in &r.cells { if !ids.contains(&c.weapon) { ids.push(c.weapon); } } }
    ids.sort();
    let mut out = String::new();
    let _ = writeln!(out, "weapon levels gave drew vm flash shots snd started fx reload away back err");
    for id in &ids {
        let cells: Vec<&Cell> = rows.iter().filter_map(|r| r.cells.iter().find(|c| c.weapon == *id)).collect();
        let n = cells.len();
        let gave = cells.iter().filter(|c| c.give_ok).count();
        let drew = cells.iter().filter(|c| c.draw_ok).count();
        let vm = cells.iter().filter(|c| c.view_model).count();
        let flash = cells.iter().filter(|c| c.flash_attach).count();
        let shots: u32 = cells.iter().map(|c| c.shots).sum();
        let snd: usize = cells.iter().map(|c| c.sounds.len()).sum();
        let started: usize = cells.iter().map(|c| c.started.len()).sum();
        let fx: u32 = cells.iter().map(|c| c.bullets + c.impacts + c.explosions + c.grenades).sum();
        let rel = cells.iter().filter(|c| c.reload_ok || c.reload_na).count();
        let away = cells.iter().filter(|c| c.away_ok).count();
        let back = cells.iter().filter(|c| c.back_ok).count();
        let err = cells.iter().filter(|c| c.error.is_some()).count();
        let name = cells.iter().find(|c| !c.name.is_empty()).map(|c| c.name.clone()).unwrap_or_default();
        let _ = writeln!(out, "{id:#04x} {name:<18.18} {n:>2} {gave:>4} {drew:>4} {vm:>2} {flash:>5} {shots:>5} {snd:>3} {started:>7} {fx:>2} {rel:>6} {away:>4} {back:>4} {err:>3}");
    }
    out
}

/// The red cells with their reasons (every red mark gets one at the end of the task).
pub fn red_cells(rows: &[LevelRow]) -> String {
    let mut out = String::new();
    for r in rows {
        for c in &r.cells {
            let m = c.mark();
            if m == '-' || m == 'n' || m == 'q' || m == 's' { continue; }
            let _ = write!(out, "{} {:#04x} {} [{m}]", r.level.trim_end_matches(".bin"), c.weapon, c.name);
            if let Some(e) = &c.error { let _ = write!(out, " PANIC: {e}"); }
            if let Some(rsn) = &c.reason { let _ = write!(out, " — {rsn}"); }
            if let Some(gm) = &c.give_msg { let _ = write!(out, " — give: {gm}"); }
            let _ = writeln!(out, " (vm{} mode{} shots {} bullets {} impacts {} expl {} gren {} beam {} sounds {:?} started {:?} seq {:?})",
                c.view_model as u8, c.via_mode as u8, c.shots, c.bullets, c.impacts, c.explosions, c.grenades, c.beam as u8,
                c.sounds.iter().take(6).collect::<Vec<_>>(), c.started.iter().take(6).collect::<Vec<_>>(),
                c.sequences.iter().take(4).collect::<Vec<_>>());
        }
    }
    out
}

fn cell_summary(c: &Cell) -> String {
    format!("{} vm{} draw{} shots{} snd{} start{} fx({}b/{}i/{}e/{}g){}",
        c.mark(), c.view_model as u8, c.draw_ok as u8, c.shots, c.sounds.len(), c.started.len(),
        c.bullets, c.impacts, c.explosions, c.grenades,
        c.error.as_deref().map(|e| format!(" PANIC {e}")).unwrap_or_default())
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() { s.to_string() }
    else if let Some(s) = p.downcast_ref::<String>() { s.clone() }
    else { "panic (no message)".into() }
}
