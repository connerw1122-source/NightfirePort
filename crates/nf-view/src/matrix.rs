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
//!
//! weapon-matrix2 (task 103b) additions: the give now runs the REAL give path — the level is
//! pre-merged for the whole catalog through `nf_view::load_weapons` (the `--give`/lazy-give asset
//! merge 100a built), and the inventory is reset per cell from the load's own state, so a cell's
//! give is what a player's give is (the old run's `.` wall — the shared ammo bank filling across
//! 99 consecutive gives — was the harness, not the game: `Player_EquipWeapon` 0x1a8a58's owned
//! branch refuses at cap, and no play session gives 99 weapons in a row). Each cell also checks
//! the behaviours the fire-mode/gadget fixes added: the record's fire-mode chain (twin byte +0x05
//! / mode count +0x2e, `Player_Weapon` 0x1a24c8:204-260 — the twin switch and the mode label
//! `HUD_UpdateAmmoPane` 0x19a7f0 reads), the guided-missile camera (weapon +0x6C & 4 — the
//! AT-420 Sentinel 'Guided' 0x2C [L ee_start_05.bin]; `missile_cam` on the Sim), and the grenade
//! blast timing (the impact records 0x2A/0x1B blast on first contact, the +0x70 & 0x200 records
//! tick their 210-tick fuse [M Bullet_DoTrails 0x12d890:262-281] — the MGL twins 0x2a impact vs
//! 0x2b delay are checked against that).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use nf_formats::iso9660::IsoImage;

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
    /// The record's twin byte (+0x05, a RELATIVE id: the switch target is weapon + twin) and mode
    /// count (+0x2e) [M Player_Weapon 0x1a24c8; nf_game::gun::fire_mode_switch].
    pub twin: u8,
    pub mode_count: u8,
    /// The record has a fire-mode chain (twin != 0 or mode_count > 1) and the mode press was run.
    pub mode_tried: bool,
    /// A mode press landed: the twin got equipped (`gun.weapon` moved) or the mode byte cycled.
    pub mode_ok: bool,
    /// The fire-mode label ids (`fire_mode_label` = record +0x30 + mode·4; `HUD_UpdateAmmoPane`
    /// 0x19a7f0) — before the press for the record, after it for the equipped record.
    pub mode_label_base: u32,
    pub mode_label_after: u32,
    /// The guided-missile camera rode a live missile this cell (`Sim::missile_cam` is Some).
    pub guided: bool,
    /// Ticks from the first grenade spawn to the first explosion (0 = no explosion seen). The
    /// impact records blast on contact (a few ticks of flight), the fused ones at 210/300/420.
    pub expl_gap: u32,
    /// The stunner's no-target arm ran (`Sim::stunner_target == Some(false)` [M
    /// Player_WeaponInitBullet 0x1a4bf0:169-200]): the aim ray met no live guard within
    /// weapon_data+0x1c, so the game itself spawns no shot and draws no taser beam — the fire
    /// path is working; only the muzzle light and the sound show.
    pub no_target: bool,
    /// Why a red cell is red (the missing asset, from the level archive's own hash set).
    pub reason: Option<String>,
    /// The cell panicked or hard-failed (a panic is a bug per the task).
    pub error: Option<String>,
}

impl Cell {
    /// Anything observable from the trigger during the fire phase.
    /// Anything observable from the trigger during the fire phase (pub for the matrix test).
    pub fn fire_events(&self) -> u32 { self.shots + self.bullets + self.impacts + self.explosions + self.grenades + self.beam as u32 }

    /// The cell's mark: `-` clean; a red letter for a real failure; a dim letter for the by-design
    ////informational cases (the watch gadgets' quiet fire, a record without a first-person model).
    pub fn mark(&self) -> char {
        // The gadget cells (0x4A..0x5E) run their own chain: the fire checks are REAL now (the
        // beam / the bolt / the thrown camera / the sound — the task's four), and a level that
        // does not carry the id is the dim `n` (the game's cycle cannot reach it either).
        let gadget = (0x4a..=0x5e).contains(&self.weapon);
        if self.error.is_some() { '!' }
        else if gadget {
            if !self.give_ok { 'n' }
            else if !self.view_model { 'I' }
            else if !self.draw_ok { 'D' }
            else if self.fire_events() == 0 && !self.no_target { 'F' }
            else if self.sounds.is_empty() && self.started.is_empty() { 'S' }
            else if !self.away_ok { 'W' }
            else if !self.back_ok { 'B' }
            else { '-' }
        }
        else if !self.give_ok && !self.no_fp_record { '.' }
        else if !self.give_ok && self.no_fp_record { 'n' }
        else if !self.view_model { 'I' }
        else if !self.draw_ok { 'D' }
        else if self.fire_events() == 0 && !self.quiet_fire() { 'F' }
        else if !self.reload_ok && !self.reload_na && !self.quiet_reload() { 'R' }
        else if !self.away_ok { 'W' }
        else if !self.back_ok { 'B' }
        else if self.mode_tried && !self.mode_ok { 'M' }
        else if self.weapon == 0x2c && !self.guided { 'G' }
        else if self.weapon == 0x2a && self.explosions > 0 && self.expl_gap > 90 { 'X' }
        else if self.weapon == 0x2b && self.explosions > 0 && self.expl_gap > 0 && self.expl_gap < 150 { 'X' }
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

    let mut scene = crate::load(iso, &level, false)?;
    anyhow::ensure!(scene.sim.is_some(), "level {level} loaded without a sim");
    let weapons: Vec<(u8, String)> = panel::catalog_entries(&scene).into_iter().map(|e| (e.id, e.name)).collect();
    // The REAL give path (100a): the level pre-merged for the whole catalog — the same
    // `load_weapons` merge a `--give`/dev-menu give runs — so a cell measures the weapon the way
    // the user's give draws it, not a stub with the fp skin missing.
    let ids: Vec<u8> = weapons.iter().map(|(id, _)| *id).collect();
    scene = crate::load_weapons(iso, &level, false, &ids)?;
    anyhow::ensure!(scene.sim.is_some(), "level {level} loaded without a sim (merged)");
    // Wait out the level's intro cut-scene: `Player_Disable` by a script blocks every input until
    // `Player_Enable` (scene.rs's `Sim::disabled` — the F cells on 07000001/02/04 were the manor's
    // intro, not the weapon). The matrix gives no input while it holds.
    wait_enabled(&mut scene, 7200);
    // The per-cell inventory reset: the load's OWN inventory state (the level loadout's). A cell's
    // give then lands on the same state a player's give lands on — the old run's `.` wall was the
    // shared ammo bank filling across 99 consecutive gives (`Player_EquipWeapon` 0x1a8a58's owned
    // branch refuses at cap; the game never gives 99 weapons in a row).
    let snap = snap_inv(&scene);
    for (id, name) in weapons {
        wait_enabled(&mut scene, 600);
        // The cell must run with Bond alive and in control: the prior cells' gunfire draws the
        // level's guards, and a dead Bond refuses every input (a dead tail measures nothing).
        // The harness repair is the cold reload (the same block the stuck-machine path uses): a
        // fresh load puts Bond back at the spawn alive with the load's own hp — the warm
        // auto-restart does NOT (its Player_RamLoad carry keeps the dead 0 in the port's mission
        // layer, measured here). The game's own testing session would restart the level too.
        if scene.sim.as_ref().map_or(false, |s| s.drones.health.health <= 0.0) {
            log(&format!("[matrix] {level} weapon {id:#04x}: Bond is dead — cold reload"));
            scene = crate::load_weapons(iso, &level, false, &ids)?;
            wait_enabled(&mut scene, 7200);
            restore_inv(&mut scene, &snap);
        }
        // Unstick the switch machine before the cell: a previous cell's hung reload (a stuck 5/6/7)
        // refuses every switch — `Player_WeaponChange` 0x1a6f80:33-47 refuses while the view anim is
        // not idle — and would poison the whole tail with D cells that say nothing about THIS
        // weapon. 300 idle ticks, then a cold reload of the level (the harness recovers; the hung
        // cell kept its own honest mark).
        {
            let mut stuck = { let sim = scene.sim.as_ref().unwrap();
                sim.gun.switch_state != 0 || sim.gun.reload_state != 0 || sim.gun.fire_state };
            for _ in 0..300 {
                if !stuck { break; }
                scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
                let sim = scene.sim.as_ref().unwrap();
                stuck = sim.gun.switch_state != 0 || sim.gun.reload_state != 0 || sim.gun.fire_state;
            }
            if stuck {
                log(&format!("[matrix] {level} weapon {id:#04x}: the switch machine never idled — cold reload"));
                scene = crate::load_weapons(iso, &level, false, &ids)?;
                restore_inv(&mut scene, &snap);
                if std::env::var("NF_MATRIX_DBG").is_ok() {
                    let sim = scene.sim.as_ref().unwrap();
                    eprintln!("[dbg cold {id:#04x}] held={:02x} req={:02x} sw={} rel={} fire={}", sim.gun.weapon, sim.gun.switch_request, sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8);
                }
            } else if std::env::var("NF_MATRIX_DBG").is_ok() {
                let sim = scene.sim.as_ref().unwrap();
                eprintln!("[dbg pre {id:#04x}] held={:02x} req={:02x} sw={} rel={} fire={}", sim.gun.weapon, sim.gun.switch_request, sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8);
            }
        }
        let mut cell = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_cell(&mut scene, id, &snap)))
            .unwrap_or_else(|p| Cell { weapon: id, error: Some(panic_text(p)), ..Default::default() });
        cell.name = name;
        if cell.error.is_some() {
            log(&format!("[matrix] {level} weapon {id:#04x} PANICKED — reloading the level"));
            scene = crate::load_weapons(iso, &level, false, &ids)?;
            restore_inv(&mut scene, &snap);
        }
        annotate(&scene, &mut cell);
        log(&format!("[matrix] {level} weapon {:#04x} {}: {}", id, cell.name, cell_summary(&cell)));
        row.cells.push(cell);
    }
    Ok(row)
}

/// The inventory state a cell's give starts from (the load's own — see run_level).
#[derive(Clone)]
pub struct Snap { owned: Vec<bool>, carried: Vec<i16>, clip: Vec<i16>, fire_mode: Vec<u8> }

fn snap_inv(scene: &Scene) -> Snap {
    let inv = &scene.sim.as_ref().expect("snap: no sim").drones.inventory;
    Snap { owned: inv.owned.clone(), carried: inv.carried.clone(), clip: inv.clip.clone(), fire_mode: inv.fire_mode.clone() }
}

fn restore_inv(scene: &mut Scene, s: &Snap) {
    let inv = &mut scene.sim.as_mut().expect("restore: no sim").drones.inventory;
    inv.owned = s.owned.clone(); inv.carried = s.carried.clone(); inv.clip = s.clip.clone(); inv.fire_mode = s.fire_mode.clone();
}

/// One cell's sequence: reset → give → draw → fire ×3 → blast watch → reload → mode press →
/// switch away → switch back, all through the real input path (`Sim::step_bond`; the give is
/// `panel::apply`'s "weapon" — the `--give` command's own path).
fn run_cell(scene: &mut Scene, id: u8, snap: &Snap) -> Cell {
    let mut c = Cell { weapon: id, ..Default::default() };
    let (clip, fp) = {
        let Some(sim) = scene.sim.as_ref() else { c.error = Some("no sim".into()); return c };
        (sim.weapons.clip_size(id as usize), sim.weapons.first_person(id as usize))
    };
    c.no_fp_record = fp == 0;
    c.fp_hash = (fp != 0).then_some(fp);
    // The record's fire-mode chain and its default-mode label [M Player_Weapon 0x1a24c8;
    // HUD_UpdateAmmoPane 0x19a7f0 — the label id is record +0x30 + mode·4].
    if let Some(w) = scene.sim.as_ref().unwrap().weapons.records.get(id as usize) {
        c.twin = w[5];
        c.mode_count = w[0x2e];
    }
    c.mode_tried = c.twin != 0 || c.mode_count > 1;
    // The stale fire_busy script (see unstick_view_script): it blocks the switch/ cycle block for
    // the GUN cells too — the D-block from 0x2e on in the first sweep was this, not the weapons.
    unstick_view_script(scene);
    c.mode_label_base = {
        let sim = scene.sim.as_ref().unwrap();
        let m = sim.drones.inventory.fire_mode.get(id as usize).copied().unwrap_or(0);
        sim.weapons.fire_mode_label(id as usize, m as usize)
    };
    restore_inv(scene, snap);
    // The held weapon inherits the reset (its clip entry reverts to the load's own — often 0 for
    // the twin the previous cell's away switch landed on, twin_skip's first-offer step
    // [M Player_WeaponChange 0x1a6f80:179-198]). An empty held clip makes the next draw tick start
    // the auto-reload instead of the holster — top the HELD weapon's clip so the cell's draw starts
    // from the idle machine (harness state repair; the give below still measures the real give).
    {
        let sim = scene.sim.as_mut().unwrap();
        let held = sim.gun.weapon as usize;
        let cs = sim.weapons.clip_size(held).max(1);
        if let Some(c) = sim.drones.inventory.clip.get_mut(held) { if *c <= 0 { *c = cs; } }
        if sim.gun.clip == 0 { sim.gun.clip = cs as u32; }
    }

    // 1. Acquire. The watch gadgets (0x4A..0x5E) are CARRIED, not given: the give path's equip
    //    declines an already-owned record at the ammo cap (`Player_EquipWeapon` 0x1a8a58's owned
    //    branch — the gadgets' ammo types are 0 / own-growth), and the player reaches them with
    //    the GADGET CYCLE (`Player_WeaponChange` 0x1a6f80 mode=1, bounds 0x4A..0x5E — the port's
    //    `Input::gadget_cycle`; gadget_select_live proves the wheel end to end). A level that does
    //    not carry the id gets the dim `n`: the game's own cycle cannot reach it either (the cycle
    //    lands only on owned records with the record's +4 selectable flag [M 0x1a6f80's walk]).
    let gadget = (0x4a..=0x5e).contains(&id);
    if gadget {
        let owned = scene.sim.as_ref().unwrap().drones.inventory.owned.get(id as usize).copied().unwrap_or(false);
        if !owned {
            c.give_msg = Some("not carried on this level — the gadget cycle (0x1a6f80 mode=1) cannot reach it".into());
            return c;
        }
        // Carried from the level start: no give event — the cycle below IS the draw.
        c.give_ok = true;
    } else {
        // The `--give` path: panel::apply's "weapon" command — equip + 3 clips; a partial first
        // clip so the reload has work. The game refuses an already-owned weapon when the shared
        // ammo bank is at cap (`equip_weapon`'s owned branch — `Player_EquipWeapon` 0x1a8a58); the
        // cell records the refusal (the reason names it) and stops: proceeding without fresh ammo
        // turns the cell into an NPC-interference magnet (the guards reach the start spot mid-run
        // on some levels — see the report's caveats).
        let rounds = if clip >= 4 { clip.min(8) } else { clip.max(1) };
        let (ok, msg) = {
            let mut q = vec![("do".to_string(), "weapon".to_string()), ("id".to_string(), format!("0x{id:02x}")), ("rounds".to_string(), rounds.to_string())];
            let (m, sw) = panel::apply(scene, &q);
            (sw == Some(id), Some(m))
        };
        c.give_ok = ok;
        c.give_msg = msg.filter(|_| !ok);
        if !c.give_ok { return c; }
    }

    // 2. Draw: the real input path. Guns: a direct `switch_to` reaches every base record; the
    //    mode button (`Player_Weapon` 0x1a24c8 — the twin chain on record `w[5]`,
    //    `fire_mode_switch`) reaches the variant records (the probe: 0x03 refused `switch_to`
    //    while its base 0x02 was held; the player reaches it the same way). Gadgets: the gadget
    //    cycle (`Input::gadget_cycle` — 0x1a6f80 mode=1), one press per owned gadget in
    //    0x4A..0x5E, settling the ~50-tick draw between presses. 25 settle ticks sample the draw
    //    anim's script and sounds.
    let mut drew = false;
    let dbg = std::env::var("NF_MATRIX_DBG").is_ok();
    if gadget {
        drew = cycle_to(scene, id);
    } else {
        for tick in 0..60 {
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { switch_to: Some(id), ..Default::default() }, false, false, false);
            sample(&mut c, scene);
            if dbg && (tick == 0 || tick == 20 || tick == 40 || tick == 59) {
                let sim = scene.sim.as_ref().unwrap();
                let t = sim.weapons.ammo_type(id as usize);
                eprintln!("[dbg {id:#04x} t{tick}] held={:02x} req={:02x} sw={} rel={} fire={} clip={} carried{}={} owned_new={} saved={:02x} prev_w={:02x} prev_g={:02x} skip02={:?}",
                    sim.gun.weapon, sim.gun.switch_request, sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8,
                    sim.gun.clip, t, sim.drones.inventory.carried.get(t as usize).copied().unwrap_or(-1),
                    sim.drones.inventory.owned.get(id as usize).copied().unwrap_or(false),
                    sim.gun.saved_weapon, sim.gun.prev_weapon, sim.gun.prev_gadget,
                    sim.gun.twin_skip.get(2));
            }
            if scene.sim.as_ref().unwrap().gun.weapon == id { drew = true; break; }
        }
        if !drew {
            if mode_to(scene, id) { c.via_mode = true; drew = true; }
        }
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
    //    The loop also clocks the grenade blast timing (spawn tick → first explosion tick — the
    //    impact records blast on contact, the fused ones at 210/300/420 [M Bullet_DoTrails
    //    0x12d890:262-281]) and watches the guided-missile camera (via `sample`).
    let (mut grenades, mut bullets) = { let sim = scene.sim.as_ref().unwrap(); (sim.drones.grenades.len(), sim.drones.player_bullets.len()) };
    let mut impacts = 0u32;
    let mut explosions = 0u32;
    let mut tick: u32 = 0;
    let mut spawn_tick: Option<u32> = None;
    let mut expl_tick: Option<u32> = None;
    for _attempt in 0..3 {
        let hold = if id >= 0x48 { 45 } else { 60 };
        let (shots_before, gren_before) = (c.shots, c.grenades);
        for _ in 0..hold {
            let (imp0, exp0) = { let sim = scene.sim.as_ref().unwrap(); (sim.drones.impacts.len(), sim.drones.explosions.len()) };
            scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
            tick += 1;
            let (st, beam, imp, exp, g, b) = { let sim = scene.sim.as_ref().unwrap();
                (sim.shot_tick, sim.laser_beam.is_some(), sim.drones.impacts.len(), sim.drones.explosions.len(),
                 sim.drones.grenades.len(), sim.drones.player_bullets.len()) };
            if st { c.shots += 1; }
            c.beam |= beam;
            impacts += imp.saturating_sub(imp0) as u32;
            if exp > exp0 { explosions += (exp - exp0) as u32; if expl_tick.is_none() { expl_tick = Some(tick); } }
            if g > grenades { if spawn_tick.is_none() { spawn_tick = Some(tick); } c.grenades += (g - grenades) as u32; grenades = g; }
            if b > bullets { c.bullets += (b - bullets) as u32; bullets = b; }
            sample(&mut c, scene);
            if c.shots > shots_before || c.grenades > gren_before { break; } // this press fired — release
        }
        for _ in 0..6 {
            let (imp0, exp0) = { let sim = scene.sim.as_ref().unwrap(); (sim.drones.impacts.len(), sim.drones.explosions.len()) };
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            tick += 1;
            let (imp, exp, g, b) = { let sim = scene.sim.as_ref().unwrap();
                (sim.drones.impacts.len(), sim.drones.explosions.len(), sim.drones.grenades.len(), sim.drones.player_bullets.len()) };
            impacts += imp.saturating_sub(imp0) as u32;
            if exp > exp0 { explosions += (exp - exp0) as u32; if expl_tick.is_none() { expl_tick = Some(tick); } }
            if g > grenades { if spawn_tick.is_none() { spawn_tick = Some(tick); } c.grenades += (g - grenades) as u32; grenades = g; }
            if b > bullets { c.bullets += (b - bullets) as u32; bullets = b; }
            sample(&mut c, scene);
        }
    }
    c.impacts = impacts;
    c.explosions = explosions;
    // 3.5 The blast watch: a fused grenade (210-tick fuse [M Bullet_DoTrails 0x12d890]) outlives
    //     the fire presses — hold the sim (no input) until the explosion lands or 230 ticks pass,
     //    so the impact-vs-delay gap is measured for the delay twins too. Impact records already
    //     blasted inside the fire loop (their blast rides first contact) and skip this.
    if spawn_tick.is_some() && expl_tick.is_none() {
        for _ in 0..230 {
            let exp0 = scene.sim.as_ref().unwrap().drones.explosions.len();
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            tick += 1;
            let exp = scene.sim.as_ref().unwrap().drones.explosions.len();
            sample(&mut c, scene);
            if exp > exp0 { explosions += (exp - exp0) as u32; expl_tick = Some(tick); c.explosions = explosions; break; }
        }
    }
    c.explosions = explosions;
    c.expl_gap = match (spawn_tick, expl_tick) { (Some(s), Some(e)) => e - s, _ => 0 };

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

    // 4.5 The fire-mode press (the task's per-weapon check): with the machine idle, the mode
    //     button either switches to the twin (relative id weapon + w[5], needs ammo
    //     [M fire_mode_switch / 0x1a24c8:246-260]) or cycles the mode byte up to +0x2e. Landed =
    //     the equipped weapon moved OR the mode byte changed; the label after is the equipped
    //     record's own (+0x30 + mode·4).
    if c.mode_tried && id < 0x48 && c.draw_ok {
        let mut landed = false;
        for _ in 0..4 {
            for _ in 0..40 {
                let sim = scene.sim.as_ref().unwrap();
                if sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
                scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
                sample(&mut c, scene);
            }
            let before = { let sim = scene.sim.as_ref().unwrap();
                (sim.gun.weapon, sim.drones.inventory.fire_mode.get(id as usize).copied().unwrap_or(0)) };
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { mode_switch: true, ..Default::default() }, false, false, false);
            for _ in 0..40 {
                scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
                sample(&mut c, scene);
            }
            let after = { let sim = scene.sim.as_ref().unwrap();
                (sim.gun.weapon, sim.drones.inventory.fire_mode.get(id as usize).copied().unwrap_or(0)) };
            if after.0 != id || after.1 != before.1 {
                landed = true;
                c.mode_label_after = { let sim = scene.sim.as_ref().unwrap();
                    let w = after.0 as usize;
                    let m = sim.drones.inventory.fire_mode.get(w).copied().unwrap_or(0);
                    sim.weapons.fire_mode_label(w, m as usize) };
                break;
            }
        }
        c.mode_ok = landed;
    }

    // 5. Switch away to the P2K and back (direct `switch_to`; the mode-button fallback is only
    //    for the draw of a variant record — cycling away from 0x02 would move the weapon off the
    //    target, not onto it). The P2K is given fresh here: the per-cell reset may have left it
    //    unowned, and a give is the real path to own it.
    give_via_panel(scene, AWAY, None);
    c.away_ok = switch_to(scene, AWAY, false);
    for _ in 0..25 { if c.away_ok { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); sample(&mut c, scene); } }
    // Back to a gadget the way the player does it: the cycle again (the direct `switch_to` targets
    // the weapon keys; the gadget wheel is 0x1a6f80 mode=1).
    c.back_ok = if gadget { cycle_to(scene, id) } else { switch_to(scene, id, true) };
    for _ in 0..25 { if c.back_ok { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); sample(&mut c, scene); } }
    c
}

/// The port's `fire_busy` approximation (scene.rs: `view_script == the held record's fire scripts
/// (+0xb0/+0xb4) && view_time < len − 1`) reads a STALE script after the previous cell's draw
/// settled: the last script stays in `view_script` with `view_time` frozen at 0 (an idle script of
/// length ≤ 1 clamps to 0), and the gate then blocks the whole cycle/switch block — the game's own
/// gate there is the view-anim STATE 9 [M `Player_WeaponChange` 0x1a6f80:33-47], which is CLOSED
/// (idle) in this state. The harness clears the stale field when the machine is idle and two idle
/// ticks agree the script is finished (the same hash, the same frozen 0): the same repair class as
/// the held-clip one. A genuinely playing script advances `view_time` and is left alone.
fn unstick_view_script(scene: &mut Scene) {
    let machine_idle = { let s = scene.sim.as_ref().unwrap();
        s.gun.switch_state == 0 && s.gun.reload_state == 0 && !s.gun.fire_state };
    if !machine_idle { return; }
    let (script, time) = { let s = scene.sim.as_ref().unwrap(); (s.view_script, s.view_time) };
    if script == 0 { return; }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    let (script2, time2) = { let s = scene.sim.as_ref().unwrap(); (s.view_script, s.view_time) };
    if script2 == script && time2 == time && time == 0.0 {
        scene.sim.as_mut().unwrap().view_script = 0;
    }
}

/// Reach a gadget with the gadget cycle (`Player_WeaponChange` 0x1a6f80 mode=1; the port's
/// `Input::gadget_cycle`): press +1, settle the ~50-tick equip, until `id` is held. The cycle
/// walks the owned, selectable records in 0x4A..0x5E only — a gadget the level does not carry is
/// unreachable by the game's own rule (the runner's caller screens for that first).
fn cycle_to(scene: &mut Scene, id: u8) -> bool {
    let dbg = std::env::var("NF_MATRIX_DBG").is_ok();
    if scene.sim.as_ref().unwrap().gun.weapon == id { return true; }
    for press in 0..12 {
        {
            let sim = scene.sim.as_ref().unwrap();
            if dbg {
                let carried = sim.drones.inventory.carried.clone();
                let clip = sim.drones.inventory.clip.clone();
                let weapons = &sim.weapons;
                let has_ammo = |w: u8| nf_game::gun::Gun::has_ammo(w, &carried, &clip, |x| weapons.ammo_type(x as usize), |x| weapons.flags(x as usize));
                eprintln!("[dbg cyc {id:#04x} p{press}] held={:02x} sw={} rel={} fire={} owned{}={} sel={} ammo={} clip{}={} carried{}={} disabled={} hp={:.1} vs={:#x}/{:.1} sub={}",
                    sim.gun.weapon, sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8,
                    id, sim.drones.inventory.owned.get(id as usize).copied().unwrap_or(false),
                    weapons.selectable(id), has_ammo(id),
                    id, sim.drones.inventory.clip.get(id as usize).copied().unwrap_or(-1),
                    weapons.ammo_type(id as usize), sim.drones.inventory.carried.get(weapons.ammo_type(id as usize) as usize).copied().unwrap_or(-1),
                    sim.disabled, sim.drones.health.health, sim.view_script, sim.view_time, sim.bond.substate);
            }
        }
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for t in 0..50 {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            if scene.sim.as_ref().unwrap().gun.weapon == id {
                if dbg { eprintln!("[dbg cyc {id:#04x}] landed on press {press} tick {t}"); }
                return true;
            }
        }
    }
    false
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
    c.guided |= sim.missile_cam.is_some();
    let (pos, f) = (sim.bond.head(), sim.bond.look());
    let norm = { let l = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6); [f[2] / l, 0.0, -f[0] / l] };
    sim.drones.map_sounds.update(&mut sim.drones.audio, pos);
    sim.drones.audio.update(sim.sfx.as_ref(), (pos, norm, [0.0, 1.0, 0.0], f), &|_| false);
    for cmd in sim.drones.audio.take_commands() {
        if let nf_game::sound::Cmd::Start { id, .. } = cmd { if id != 0 { c.started.insert(id); } }
    }
    c.no_target |= sim.stunner_target == Some(false);
    if sim.view_script != 0 { c.sequences.insert(sim.view_script); }
}

/// Fill the red-cell reason from the MERGED scene's own tables (the view-model build reads the
/// same merged skins — `Scene::build`'s first-person section). After 100a a missing skin means the
/// disc-wide index could not supply it either — a real asset gap, not a per-level archive gap.
fn annotate(scene: &Scene, c: &mut Cell) {
    if c.error.is_some() || c.view_model || !c.give_ok { return; }
    if let Some(fp) = c.fp_hash {
        let sim = scene.sim.as_ref().unwrap();
        if !sim.skins.contains_key(&fp) {
            c.reason = Some(format!("fp skin {fp:08x} not resolvable from the disc (index miss)"));
            return;
        }
        c.reason = Some("fp skin resolves but no view model (sleeve/skeleton gap in the merged tables)".into());
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
    let _ = writeln!(out, "marks: - clean | ! panic/error | . give refused | n record has no fp model (by design) | I no view model | D draw failed | F no observable fire | R reload broken | W switch-away failed | B switch-back failed | M mode press failed | G guided camera missing (0x2c) | X blast timing wrong (MGL 0x2a impact / 0x2b delay) | S silent");
    let _ = writeln!(out, "(weapon-matrix3: the gadget cells 0x4A..0x5E are REAL cells now — the carried gadgets equip through the gadget cycle (Player_WeaponChange 0x1a6f80 mode=1) and their fire checks (beam / bolt / thrown camera / sound) gate the marks; a gadget a level does not carry prints n — the game's own cycle cannot reach it either [M 0x1a6f80 walks owned records only])");
    out
}

/// The per-weapon aggregate across levels (the numbers behind the marks).
pub fn aggregate_table(rows: &[LevelRow]) -> String {
    let mut ids: Vec<u8> = Vec::new();
    for r in rows { for c in &r.cells { if !ids.contains(&c.weapon) { ids.push(c.weapon); } } }
    ids.sort();
    let mut out = String::new();
    let _ = writeln!(out, "weapon levels gave drew vm flash shots snd started fx reload away back mode gcam gap err");
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
        let mtry = cells.iter().filter(|c| c.mode_tried && c.draw_ok && c.error.is_none()).count();
        let mok = cells.iter().filter(|c| c.mode_ok).count();
        let gcam = cells.iter().filter(|c| c.guided).count();
        let gaps: Vec<u32> = cells.iter().filter(|c| c.expl_gap > 0).map(|c| c.expl_gap).collect();
        let (gmin, gmax) = (gaps.iter().min().copied().unwrap_or(0), gaps.iter().max().copied().unwrap_or(0));
        let _ = writeln!(out, "{id:#04x} {name:<18.18} {n:>2} {gave:>4} {drew:>4} {vm:>2} {flash:>5} {shots:>5} {snd:>3} {started:>7} {fx:>2} {rel:>6} {away:>4} {back:>4} {mok:>2}/{mtry:<2} {gcam:>2} {gmin:>3}/{gmax:<3} {err:>3}");
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
            let _ = writeln!(out, " (vm{} mode{} twin{} mc{} mok{} lbl{:08x}->{:08x} gcam{} gap{} shots {} bullets {} impacts {} expl {} gren {} beam {} sounds {:?} started {:?} seq {:?})",
                c.view_model as u8, c.via_mode as u8, c.twin, c.mode_count, c.mode_ok as u8, c.mode_label_base, c.mode_label_after,
                c.guided as u8, c.expl_gap, c.shots, c.bullets, c.impacts, c.explosions, c.grenades, c.beam as u8,
                c.sounds.iter().take(6).collect::<Vec<_>>(), c.started.iter().take(6).collect::<Vec<_>>(),
                c.sequences.iter().take(4).collect::<Vec<_>>());
        }
    }
    out
}

fn cell_summary(c: &Cell) -> String {
    format!("{} vm{} draw{} shots{} snd{} start{} fx({}b/{}i/{}e/{}g) nt{}{}",
        c.mark(), c.view_model as u8, c.draw_ok as u8, c.shots, c.sounds.len(), c.started.len(),
        c.bullets, c.impacts, c.explosions, c.grenades, c.no_target as u8,
        c.error.as_deref().map(|e| format!(" PANIC {e}")).unwrap_or_default())
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() { s.to_string() }
    else if let Some(s) = p.downcast_ref::<String>() { s.clone() }
    else { "panic (no message)".into() }
}
