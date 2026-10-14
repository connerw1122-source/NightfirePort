//! In-game test tooling, gated behind `--dev` (WORKLOG 0081). Tools only call into existing systems or set state the
//! way the game itself would — never change game logic or the order of RNG calls. Logic stays at a fixed 60 Hz; the
//! time scale changes the number of ticks run per render frame, not the tick rate.
//!
//! - **F1**: the AI overlay — each guard within 30 m shows its state name, alertness, health and active timers; a line
//!   to its target or cover point; live grenades' arcs and explosion radii.
//! - **F2**: the dev menu — the WHOLE external panel (panel.rs) moved into the window (fix-ingame-dev-menu):
//!   pages for Weapons / Gadgets / Grenades / Ammo-health-armour / NPCs / Teleport / Level / Toggles /
//!   Screenshot, plus the original tools (spawn a guard at the crosshair by type/mode, force the nearest guard's
//!   next decision, impulse, SpecialDeath, god mode, infinite ammo, teleport save/restore, time scale). Opens
//!   over the frame and PAUSES the sim; arrows/Enter navigate, PageUp/PageDown switch pages, typing filters,
//!   Backspace deletes (or returns to the index), Esc/F2 closes; mouse click + wheel work too. Every give/spawn/
//!   clear command routes through `panel::apply` — the shared command layer.
//! - **`--scenario <name>`**: places Bond and the guards for one of the named test scenarios and prints what to look
//!   for. See `docs/tools/dev-mode.md`.

use crate::gpu::{HudFrame, HudVertex};
use crate::hud::{Anchor, Hud};
use crate::scene::Scene;
use nf_game::drone::{self, state, Drone, Rng};

/// The dev-mode state, lent to the `App`. `None` when `--dev` is off (normal play).
pub struct Dev {
    /// F1: the AI overlay.
    pub overlay: bool,
    /// F2: the dev menu (mouse-clickable + keyboard-navigable when open).
    pub menu: bool,
    /// The time scale: how many 60 Hz ticks to run per render frame. 1 = normal, 0 = paused, fractional = slow.
    pub time_scale: f32,
    /// One-shot: run exactly one tick this frame (the "step" key), whatever the time scale.
    pub step: bool,
    /// God mode: Bond's health is locked at 100, armour at 50 and the hit flash cleared each tick.
    pub god_mode: bool,
    /// Infinite ammo: every OWNED weapon's clip and every spare type is topped back to the most seen
    /// this run, every 30th tick — the recorder's `--ammo` cheat (the trace-session tool), not a
    /// blanket refill.
    pub infinite_ammo: bool,
    /// `--ammo` state: the most clip/spare seen this run per weapon/ammo type, and the level they
    /// belong to (the recorder resets both when the game's BLData pointer changes — a level load).
    pub best_level: Option<u32>,
    pub best_clip: Vec<i16>,
    pub best_spare: Vec<i16>,
    /// No reload: the current weapon's clip is topped to its `weapon_data` clip size each tick, so it never
    /// empties and the game never reloads.
    pub no_reload: bool,
    /// The spawn-guard submenu's current selection (type/mode).
    pub spawn: SpawnPick,
    /// "Forced" markers for the overlay: drones whose next decision was forced this tick (cleared next frame).
    pub forced: Vec<usize>,
    /// The saved teleport spot.
    pub saved_spot: Option<[f32; 3]>,
    /// The mouse position in NDC (−1..1, y up) for the menu's click hit-test.
    pub mouse: (f32, f32),
    /// The chosen scenario (printed at start; `None` in free play). Its "what to check" stays on the menu (B-9).
    pub scenario: Option<&'static str>,
    /// B-9: the keyboard-selected menu row (↑/↓ to move, Enter to activate). Mouse hover updates this too.
    pub menu_sel: usize,
    /// B-9: the last action's result, shown at the bottom of the menu for a few seconds (e.g. "guard 22: forced grenade throw").
    pub result_msg: Option<String>,
    /// B-9: ticks remaining to show `result_msg` (cleared after ~3 s at 60 Hz).
    pub result_timer: u32,
    /// The menu's current page (an index into `PAGES`; 0 = the category index).
    pub page: usize,
    /// Type-to-filter text (lowercase; typed while the menu is open, Backspace deletes).
    pub filter: String,
    /// FPS display (Toggles page): App updates `fps` each frame, `dev::hud` draws it when on.
    pub show_fps: bool,
    /// The last measured render fps (wall-clock, dev display only — never read by the Sim).
    pub fps: f32,
    /// Portal culling on/off (Toggles page): off passes no world to `Renderer::cel_visibility`
    /// (lib.rs's one visibility walk), so every cel draws — the port's stand-in for the game's
    /// portal walk [M 0x1e86e8]. On by default.
    pub portal_cull: bool,
    /// Animation interpolation on/off (Toggles page): off renders whole ticks only (alpha 1.0,
    /// the port's `NF_NOINTERP` behaviour). On by default.
    pub interp: bool,
}

/// The spawn-guard picker: a drone type and AI mode (also the NPCs page's mode selector).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpawnPick { Normal, Sniper, Ambush }

impl Default for Dev {
    fn default() -> Self {
        Dev {
            overlay: false, menu: false, time_scale: 1.0, step: false, god_mode: false, infinite_ammo: false,
            best_level: None, best_clip: Vec::new(), best_spare: Vec::new(), no_reload: false, spawn: SpawnPick::Normal,
            forced: Vec::new(), saved_spot: None, mouse: (0.0, 0.0), scenario: None, menu_sel: 0, result_msg: None, result_timer: 0,
            // New fields on their own lines (the merge rule): the menu opens on the category index,
            // no filter, all rendering aids in their normal play state.
            page: 0,
            filter: String::new(),
            show_fps: false,
            fps: 0.0,
            portal_cull: true,
            interp: true,
        }
    }
}

impl Dev {
    /// The number of ticks to run this frame (the time scale × the elapsed ticks, capped, plus a step). Consumes the
    /// one-shot `step` flag.
    pub fn tick_count(&mut self, elapsed_ticks: u32) -> u32 {
        let mut n = if self.time_scale <= 0.0 { 0 } else { (elapsed_ticks as f32 * self.time_scale) as u32 };
        if self.step { n = n.saturating_add(1).max(1); self.step = false; }
        n.min(8)
    }

    /// The ticks to run while the F2 menu is open: the menu pauses the sim (the task's "opens over
    /// the frame and pauses the sim") — 0 ticks unless F11's one-shot `step` was pressed, which
    /// advances exactly one tick for frame-by-frame inspection. (The old menu left the sim running
    /// under the open panel, so a spawn's weapon switch and the AI raced the open menu.)
    pub fn paused_tick_count(&mut self) -> u32 {
        if self.step { self.step = false; 1 } else { 0 }
    }

    /// Apply god mode / infinite ammo / no reload after a tick (the game's own state, set the way the game
    /// sets it).
    ///
    /// Infinite ammo writes the INVENTORY — the game's own ammo store (`BL+0x170` rounds carried per type,
    /// `BL+0x1B8` rounds in each weapon's clip), which `Sim::run` reads into `gun.clip`/`gun.spare` at the top
    /// of every tick and writes back at its end (scene.rs; the C-10 fix). The old refill only topped the gun's
    /// transient copies, so the next tick re-read the drained inventory: the gun stopped firing while the HUD
    /// still showed a full clip (the user's "infinite ammo isn't always reliable" — and it broke hardest for
    /// other ammo types, after a weapon switch, and across a level carry-over, exactly the cases where the
    /// drained slot differs from the current weapon's). No reload tops only the current weapon's clip to its
    /// `weapon_data+0x92` size.
    ///
    /// The infinite-ammo shape is the TRACE-SESSION cheat, `tools/trace/trace_record.py --ammo` /
    /// `tools/trace/cheats.py` (what the 01-raptor.trace user played with): every 30 frames each **owned**
    /// weapon's clip and each spare type is topped back to the most seen this run — the best only moves up,
    /// and both tables reset when the game's BLData pointer changes (a level load; here: `Sim::level`).
    /// Spares therefore never rise above what a take wrote, so no ammo cap refuses a pickup (the trace:
    /// spare type 3 grows 0→6→13→20→25 by takes alone against the type-3 cap 36 [D ammo_data], every drop
    /// taken), and an unowned gun's clip stays 0, so equipping a dropped gun adds no rounds to the spare
    /// (the trace's first Raptor take: clip 0→9, no spare event). The old blanket refill (`carried.max(200)`
    /// on every type, every clip to its clip size) sat above every ammo cap, so `Player_EquipAmmo`'s cap
    /// check refused every later drop — "some later drops can't be picked up" — and leaked the first drop's
    /// rounds into the spare (fix-pickups-trace).
    pub fn after_tick(&mut self, scene: &mut Scene) {
        if self.god_mode {
            if let Some(sim) = scene.sim.as_mut() {
                sim.drones.health.health = 100.0;
                // The recorder's god writes armour 50.0 (`p.write(bl + 0x8B0, F50, 4)`, cheats.py; the
                // trace's armour column reads 50.0 throughout) — the armour-take refuse threshold.
                sim.drones.health.armour = 50.0;
                sim.drones.health.flash = 0.0;
            }
        }
        if self.infinite_ammo || self.no_reload {
            if let Some(sim) = scene.sim.as_mut() {
                if self.infinite_ammo && sim.drones.frame % 30 == 0 {
                    if self.best_level != Some(sim.level) {
                        self.best_level = Some(sim.level);
                        self.best_clip.clear();
                        self.best_spare.clear();
                    }
                    self.best_clip.resize(sim.drones.inventory.clip.len(), 0);
                    self.best_spare.resize(sim.drones.inventory.carried.len(), 0);
                    // Clips: owned weapons only (`if read(bl+0x1B8+w·0xC+2) != 1: continue`).
                    for (w, c) in sim.drones.inventory.clip.iter_mut().enumerate() {
                        if !sim.drones.inventory.owned.get(w).copied().unwrap_or(false) { continue; }
                        if *c > self.best_clip[w] { self.best_clip[w] = *c; }
                        else if *c < self.best_clip[w] { *c = self.best_clip[w]; }
                    }
                    // Spares: every type (the recorder's loop has no owned gate — spares have none).
                    for (k, n) in sim.drones.inventory.carried.iter_mut().enumerate() {
                        if *n > self.best_spare[k] { self.best_spare[k] = *n; }
                        else if *n < self.best_spare[k] { *n = self.best_spare[k]; }
                    }
                }
                let wi = sim.gun.weapon as usize;
                let cs = sim.weapons.clip_size(wi);
                if self.no_reload && cs > 0 {
                    if let Some(c) = sim.drones.inventory.clip.get_mut(wi) { *c = (*c).max(cs); }
                }
                // Keep the gun's HUD copies in step with the refilled inventory (the HUD and the title read
                // `gun.clip`/`gun.spare`; `Sim::run` re-reads them from the inventory next tick anyway).
                if let (Some(&c), Some(&n)) = (sim.drones.inventory.clip.get(wi), sim.drones.inventory.carried.get(sim.weapons.ammo_type(wi))) {
                    sim.gun.clip = sim.gun.clip.max(c.max(0) as u32);
                    sim.gun.spare = sim.gun.spare.max(n.max(0) as u32);
                }
            }
        }
        self.forced.clear();
        // B-9: the result message fades after ~3 s (180 ticks).
        if self.result_timer > 0 { self.result_timer -= 1; if self.result_timer == 0 { self.result_msg = None; } }
    }

    /// Spawn a guard at `pos` with the picked type/mode, the way the game's spawner does (`Action::Spawn` → present +
    /// enabled + llist). Also adds a `Character` (skin + skeleton + matrix) and a `char_drone` entry so the
    /// renderer draws the guard — without this the drone exists in the AI but is invisible (the user's Round 7
    /// report: "spawning a guard doesn't work in the viewer, even though the test passes"). B-12.
    pub fn spawn_guard(&self, scene: &mut Scene, pos: [f32; 3], yaw: f32) -> Option<usize> {
        let sim = scene.sim.as_mut()?;
        let d = sim.data();
        // B-12: pick the skin of the first active guard on the level (guarantees the mesh is in the level archive).
        // The old hardcoded skins (0x05000006 / 0x05000019) aren't in 07000005's skin table, so spawn returned None.
        let skin = sim.drones.list.iter()
            .find(|x| x.present && x.enabled && !x.corpse && x.health > 0.0 && x.drawn)
            .map(|x| x.skin)
            .unwrap_or(0x0500_0006);
        let mode = match self.spawn {
            SpawnPick::Normal => 0u32,
            SpawnPick::Sniper => 2u32,
            SpawnPick::Ambush => 0x10u32,
        };
        let mut rng = Rng::new(0xdead_beef);
        let param = |k: u32| match k { 0 => skin, 16 => mode, 21 => mode, _ => 0 };
        let mut dr = Drone::new(&d, pos, yaw, param, &mut rng);
        dr.present = true;
        dr.enabled = true;
        dr.started = false;
        dr.grenade_weapon = 0x35;
        dr.grenade_count = 5;
        let idx = sim.drones.list.len();
        sim.drones.list.push(dr);
        sim.drones.llist.push(idx);
        // B-12: add a Character (mesh + skeleton) so the renderer draws the spawned guard. The skin is resolved
        // by hashcode from the level's skin table; the skeleton from the skin's skeleton id. slot_bones is empty
        // (no weapon-attachment bones — the guard's held weapon won't draw, but the body will). This mirrors how
        // `Scene::build` creates a Character for each placement (scene.rs:1114).
        let (skin_obj, skeleton_obj) = {
            let skin_ref = sim.skins.get(&skin)?;
            let skeleton_ref = sim.skeletons.get(&skin_ref.skeleton)?;
            (skin_ref.clone(), skeleton_ref.clone())
        };
        let matrix = nf_formats::placement::object_matrix([0.0, yaw, 0.0], pos);
        let char_idx = scene.characters.len();
        scene.characters.push(crate::scene::Character {
            matrix,
            skin: skin_obj,
            skeleton: skeleton_obj,
            slot_bones: Vec::new(), // no weapon slots — body draws, held weapon does not
            start: None,
            role: nf_formats::drone::SpawnRole::Placed,
            floor: None,
            script: None,
            morph_targets: None,
        });
        // char_drone maps character index → drone index. The renderer's `animate()` uses this to find the drone
        // for each character; without it, the spawned character would pose in the bind pose (not the drone's state).
        sim.char_drone.push(Some(idx));
        let _ = char_idx; // the character is now at scene.characters[char_idx]; char_drone[char_idx] == Some(idx)
        Some(idx)
    }

    /// Force the nearest guard to throw a grenade now: set its state to `GrenadeThrow` and its `pending_grenade` flag.
    pub fn force_grenade(&mut self, scene: &mut Scene, bond: [f32; 3]) -> Option<usize> {
        let sim = scene.sim.as_mut()?;
        let nearest = nearest_guard(&sim.drones, bond)?;
        let dr = &mut sim.drones.list[nearest];
        if dr.grenade_count <= 0 { dr.grenade_count = 5; }
        dr.grenade_weapon = 0x35;
        dr.state = state::GRENADE_THROW;
        dr.next_state = None;
        dr.pending_grenade = true;
        self.forced.push(nearest);
        Some(nearest)
    }

    /// Give the nearest guard an impulse straight up (to test falling off ledges).
    pub fn impulse_guard(&mut self, scene: &mut Scene, bond: [f32; 3]) -> Option<usize> {
        let sim = scene.sim.as_mut()?;
        let nearest = nearest_guard(&sim.drones, bond)?;
        let dr = &mut sim.drones.list[nearest];
        dr.velocity[1] = 8.0;
        dr.in_air = true;
        dr.on_surface = false;
        self.forced.push(nearest);
        Some(nearest)
    }

    /// Force the nearest guard into a SpecialDeath (free root motion, gravity on) — the falling-off-ledge test.
    pub fn special_death(&mut self, scene: &mut Scene, bond: [f32; 3]) -> Option<usize> {
        let sim = scene.sim.as_mut()?;
        let nearest = nearest_guard(&sim.drones, bond)?;
        let dr = &mut sim.drones.list[nearest];
        dr.free_root = true;
        dr.bounded = false;
        dr.velocity[1] = 4.0;
        self.forced.push(nearest);
        Some(nearest)
    }

    /// Teleport Bond to a position (the game's `Player_StandAtNewPosition`).
    pub fn teleport(&self, scene: &mut Scene, pos: [f32; 3], yaw: f32) {
        if let Some(sim) = scene.sim.as_mut() {
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, pos, yaw);
        }
    }
}

/// The nearest present, non-corpse guard to `bond`, within 30 m.
fn nearest_guard(drones: &drone::Drones, bond: [f32; 3]) -> Option<usize> {
    drones.list.iter().enumerate()
        .filter(|(_, x)| x.present && !x.corpse)
        .min_by(|a, b| {
            let da = (a.1.pos[0] - bond[0]).powi(2) + (a.1.pos[2] - bond[2]).powi(2);
            let db = (b.1.pos[0] - bond[0]).powi(2) + (b.1.pos[2] - bond[2]).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .filter(|(_, x)| { let d2 = (x.pos[0] - bond[0]).powi(2) + (x.pos[2] - bond[2]).powi(2); d2 < 900.0 })
        .map(|(i, _)| i)
}

// ---- the overlay render (F1) and the menu (F2) --------------------------------------------------------------

/// Build the dev HUD frame (the AI overlay + the menu), appended to the game's HUD.
pub fn hud(dev: &Dev, scene: &Scene, aspect: f32, view_proj: [[f32; 4]; 4]) -> HudFrame {
    let mut h = Hud { quads: Vec::new(), aspect, textures: &scene.hud_textures };
    if dev.overlay { ai_overlay(&mut h, scene, view_proj); }
    if dev.menu { menu(&mut h, dev, scene); }
    // The Toggles page's fps display (top right; wall-clock — a dev readout, never in the Sim).
    if dev.show_fps && dev.fps > 0.0 {
        if let Some(t) = scene.hud.as_ref() {
            h.text(Anchor::Right, &t.font, 69, &format!("{:.0} fps", dev.fps), 508.0, 4.0, true, [0.9, 0.9, 0.7, 1.0], true);
        }
    }
    h.finish()
}

/// Menu text for the game's bitmap font: ASCII only (the font has no glyphs for dashes or arrows), clipped to `n`
/// characters on a char boundary with ".." when cut.
pub fn fit(s: &str, n: usize) -> String {
    let a: String = s.chars().map(|c| match c { '—' | '–' => '-', '→' => '>', c if c.is_ascii() => c, _ => '?' }).collect();
    if a.len() > n { format!("{}..", &a[..n.saturating_sub(2)]) } else { a }
}

/// Project a world point to NDC screen coordinates (−1..1, y up).
fn project(world: [f32; 3], vp: [[f32; 4]; 4]) -> [f32; 2] {
    let (x, y, z) = (world[0], world[1], world[2]);
    let cx = vp[0][0] * x + vp[1][0] * y + vp[2][0] * z + vp[3][0];
    let cy = vp[0][1] * x + vp[1][1] * y + vp[2][1] * z + vp[3][1];
    let cw = vp[0][3] * x + vp[1][3] * y + vp[2][3] * z + vp[3][3];
    if cw.abs() < 1e-6 { return [2.0, 2.0]; }
    [cx / cw, cy / cw]
}

fn ai_overlay(h: &mut Hud, scene: &Scene, view_proj: [[f32; 4]; 4]) {
    let (Some(sim), Some(t)) = (scene.sim.as_ref(), scene.hud.as_ref()) else { return };
    let font = &t.font;
    let frame = sim.drones.frame;
    let cam = sim.bond.head();
    let mut y = 16.0;
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if !dr.present || dr.corpse { continue; }
        let d2 = (dr.pos[0] - cam[0]).powi(2) + (dr.pos[1] - cam[1]).powi(2) + (dr.pos[2] - cam[2]).powi(2);
        if d2 > 900.0 { continue; }
        let name = state_name(dr.state);
        let stance = if dr.in_air { "AIR" } else if dr.on_surface { "gnd" } else { "?" };
        let line = format!("[{i}] {stance} {name}  hp {:.0}  alert {:.2}  d {:.0}", dr.health, dr.alert, dr.dist);
        h.text(Anchor::Left, font, 60, &line, 4.0, y, false, [1.0, 1.0, 1.0, 1.0], true);
        let mut t2 = format!("  tC {}  tD {}", dr.timer_c.map_or("-".into(), |v| format!("{}", v.saturating_sub(frame))), dr.timer_d.map_or("-".into(), |v| format!("{}", v.saturating_sub(frame))));
        if dr.grenade_count > 0 { t2 += &format!("  gren {}", dr.grenade_count); }
        h.text(Anchor::Left, font, 60, &t2, 4.0, y + 12.0, false, [0.8, 0.8, 0.8, 1.0], false);
        y += 28.0;
        // A line to the target (Bond) or the cover point.
        let target = dr.goal.or(Some(dr.cover_point)).unwrap_or(sim.bond.pos);
        draw_line(h, dr.pos, target, [0.0, 1.0, 0.0, 0.6], view_proj);
    }
    // Live grenades: the arc (a line from pos along velocity) and the explosion radius (a circle).
    for g in &sim.drones.grenades {
        let end = [g.pos[0] + g.velocity[0] * 0.5, g.pos[1] + g.velocity[1] * 0.5, g.pos[2] + g.velocity[2] * 0.5];
        draw_line(h, g.pos, end, [1.0, 0.5, 0.0, 0.8], view_proj);
        draw_circle(h, g.pos, g.radius, [1.0, 0.2, 0.2, 0.4], view_proj);
    }
    // Explosions this tick (a flash circle).
    for &(p, r, _, _, _) in &sim.drones.explosions {
        draw_circle(h, p, r, [1.0, 0.8, 0.2, 0.7], view_proj);
    }
}

/// B-11: the menu's layout constants, derived from the measured font glyph height (17 HUD-px). Pub so the layout
/// test can assert no row overlap + the panel fits under 25% of the screen width, without duplicating magic numbers.
pub const MENU_GLYPH_H: f32 = 17.0;
pub const MENU_ROW_H: f32 = MENU_GLYPH_H + 2.0; // 19 HUD-px — label + gap, no overlap
pub const MENU_PANEL_X: f32 = 4.0;
pub const MENU_PANEL_W: f32 = 190.0;           // ~37% of 512 HUD-px: the panel now carries the game's own weapon names (fix-ingame-dev-menu)
pub const MENU_VISIBLE_ROWS: usize = 7;
pub const MENU_HEADER_H: f32 = MENU_GLYPH_H + 2.0;
pub const MENU_DESC_H: f32 = MENU_GLYPH_H + 2.0;

fn menu(h: &mut Hud, dev: &Dev, scene: &Scene) {
    let Some(t) = scene.hud.as_ref() else { return };
    let font = &t.font;
    let items = menu_items(dev, Some(scene));
    // B-11: layout from the REAL font metrics (17 HUD-px glyphs — measured by `menu_font_metrics_measured`).
    // The old ROW_H=12 caused rows to overlap at ~40 window-px each (the user's screenshot).
    const ROW_H: f32 = MENU_ROW_H;
    const PANEL_X: f32 = MENU_PANEL_X;
    const PANEL_W: f32 = MENU_PANEL_W;
    const HEADER_H: f32 = MENU_HEADER_H;
    const DESC_H: f32 = MENU_DESC_H;
    let n = items.len();
    // The scroll window: shared with the click hit-test (`layout_rows`).
    let (_row_y0, top, visible_end) = layout_rows(dev, n); // the draw fn re-walks the same header steps into `y`
    let mut y = 4.0;
    // The scenario banner (1 line, above the list).
    if let Some(s) = dev.scenario {
        let (label, _) = scenario(s).unwrap_or(("(unknown)", scenarios::door_anti_crush));
        h.quad(Anchor::Left, 70, 0x0300_0003, 0, PANEL_X, y, PANEL_W, HEADER_H, [0.0; 4], (false, false), [0.0, 0.0, 0.2, 0.85]);
        let lbl = fit(&format!("{s}: {label}"), 22);
        h.text(Anchor::Left, font, 69, &lbl, PANEL_X + 2.0, y + 1.0, false, [1.0, 1.0, 0.4, 1.0], true);
        y += HEADER_H + 1.0;
    } else {
        y += 2.0;
    }
    // The page header: which page, how many, and the PgUp/PgDn hint (the dark translucent box the
    // rest of the HUD uses — tex 0x0300_0003 at 0.85 alpha, the HUD font on top).
    let page_name = PAGES.get(dev.page).copied().unwrap_or("?");
    h.quad(Anchor::Left, 70, 0x0300_0003, 0, PANEL_X, y, PANEL_W, HEADER_H, [0.0; 4], (false, false), [0.0, 0.0, 0.2, 0.85]);
    let hdr = fit(&format!("[{}/{}] {} pgup/dn", dev.page, PAGES.len() - 1, page_name), 22);
    h.text(Anchor::Left, font, 69, &hdr, PANEL_X + 2.0, y + 1.0, false, [1.0, 1.0, 0.4, 1.0], true);
    y += HEADER_H + 1.0;
    // The filter line (only when a filter is typed — layout_rows matches).
    if !dev.filter.is_empty() {
        h.quad(Anchor::Left, 70, 0x0300_0003, 0, PANEL_X, y, PANEL_W, ROW_H, [0.0; 4], (false, false), [0.1, 0.1, 0.1, 0.9]);
        let f = fit(&format!("find: {}", dev.filter), 22);
        h.text(Anchor::Left, font, 69, &f, PANEL_X + 4.0, y + 1.0, false, [0.6, 1.0, 0.6, 1.0], true);
        y += ROW_H;
    }
    // The list background (one panel for all visible rows).
    let list_h = (visible_end - top) as f32 * ROW_H;
    h.quad(Anchor::Left, 70, 0x0300_0003, 0, PANEL_X, y, PANEL_W, list_h, [0.0; 4], (false, false), [0.0, 0.0, 0.0, 0.7]);
    // The visible rows - label only (no per-row description; it would overlap the 17px-tall label).
    for vi in top..visible_end {
        let (label, _desc, _) = &items[vi];
        let sel = vi == dev.menu_sel;
        let row_y = y + (vi - top) as f32 * ROW_H;
        if sel {
            h.quad(Anchor::Left, 71, 0x0300_0003, 0, PANEL_X, row_y, PANEL_W, ROW_H, [0.0; 4], (false, false), [0.0, 0.25, 0.0, 0.9]);
            h.quad(Anchor::Left, 72, 0x0300_0003, 0, PANEL_X, row_y, 2.0, ROW_H, [0.0; 4], (false, false), [1.0, 1.0, 0.0, 0.9]);
        }
        let lbl = fit(label, 20);
        h.text(Anchor::Left, font, 69, &lbl, PANEL_X + 4.0, row_y + 1.0, false, if sel { [1.0, 1.0, 0.4, 1.0] } else { [0.95, 0.95, 0.95, 1.0] }, sel);
    }
    y += list_h + 1.0;
    // The selected row's full description (one line, below the list - no overlap with labels).
    if let Some((_, desc, _)) = items.get(dev.menu_sel) {
        let d = fit(desc, 22);
        h.quad(Anchor::Left, 70, 0x0300_0003, 0, PANEL_X, y, PANEL_W, DESC_H, [0.0; 4], (false, false), [0.05, 0.05, 0.05, 0.85]);
        h.text(Anchor::Left, font, 69, &d, PANEL_X + 2.0, y + 1.0, false, [0.75, 0.9, 0.75, 1.0], false);
        y += DESC_H + 1.0;
    }
    // Scroll indicator if there are more rows than visible.
    if n > MENU_VISIBLE_ROWS {
        let more_above = top > 0;
        let more_below = visible_end < n;
        let ind = format!("{} {}/{} {}", if more_above { "^" } else { " " }, dev.menu_sel + 1, n, if more_below { "v" } else { "" });
        h.text(Anchor::Left, font, 69, &ind, PANEL_X + PANEL_W - 36.0, y, false, [0.6, 0.6, 0.6, 1.0], false);
        y += ROW_H;
    }
    // The result message (last action's outcome), ~3 s.
    if dev.result_timer > 0 {
        if let Some(msg) = &dev.result_msg {
            let m = fit(msg, 22);
            h.quad(Anchor::Left, 70, 0x0300_0003, 0, PANEL_X, y, PANEL_W, DESC_H, [0.0; 4], (false, false), [0.15, 0.0, 0.15, 0.9]);
            h.text(Anchor::Left, font, 69, &m, PANEL_X + 2.0, y + 1.0, false, [1.0, 0.9, 1.0, 1.0], true);
            y += DESC_H + 1.0;
        }
    }
    // Footer hint (compact, ASCII). The fire-mode key (the user chose B): B = the game's ○
    // (`Player_Weapon` 0x1a24c8's action 12) — the twin/mode switch (fix-weapon-modes).
    h.text(Anchor::Left, font, 69, "arrows/enter pgup/dn type=bksp F2", PANEL_X, y, false, [0.55, 0.55, 0.55, 1.0], false);
}

// ---- the menu pages (fix-ingame-dev-menu) --------------------------------------------------------------------

/// The menu's pages, in order. Page 0 is the category index; 1-9 are the panel's sections
/// (crates/nf-view/src/panel.rs's page) moved into the window. `--dev-page <name>` renders a page
/// headless through `--shot`.
pub const PAGES: &[&str] = &[
    "menu", "weapons", "gadgets", "grenades", "ammo", "npcs", "teleport", "level", "toggles", "screenshot",
];

/// A `--dev-page`/debug name → page index.
pub fn page_index(name: &str) -> Option<usize> {
    let n = name.to_ascii_lowercase();
    PAGES.iter().position(|p| *p == n)
}

/// Named teleport spots, from the port's own test spots (level id prefix: the mission the spot
/// belongs to). `dist`-independent: fixed world positions, teleported to with `Bond::stand_at`
/// (the game's own placement path), not a relative offset.
pub const TELEPORT_SPOTS: &[(&str, [f32; 3])] = &[
    ("05 Q-Claw point", [16.5, 29.6, 58.6]),
    ("05 cable", [-23.58, 34.0, 2.65]),
    ("08 safe", [155.19, 43.12, -113.38]),
    ("07 Kimiko trigger", [94.2, 44.6, -45.9]),
    ("16 door", [5.0, 0.9, -70.0]),
    ("02 pond bank", [-82.3, 5.3, 32.05]),
    ("0c display vent", [5.8, 15.8, -108.5]),
];

/// A menu row: label, one-line description of what to watch for, action. (B-9's description
/// column; labels are owned Strings now — catalog rows carry the game's own names.)
pub type MenuRow = (String, &'static str, MenuAction);

/// The grenade record ids (the game's own: 0x34 Fragmentation, 0x35 Stun, 0x36 Smoke, 0x37 Remote
/// Mine [L USATxt.dat via Txt_BindLabel(+0x38); panel.rs's consumables]).
const GRENADES: [u8; 4] = [0x34, 0x35, 0x36, 0x37];

/// Is the label row visible under the page's filter? (Case-insensitive substring on the lowercase
/// label; an empty filter shows everything.)
fn passes_filter(dev: &Dev, label: &str) -> bool {
    dev.filter.is_empty() || label.to_ascii_lowercase().contains(&dev.filter)
}

/// The menu rows for the current page. `scene` may be None (headless calls before a level loads):
/// catalog-backed pages then show a single placeholder row.
pub fn menu_items(dev: &Dev, scene: Option<&Scene>) -> Vec<MenuRow> {
    let mut v: Vec<MenuRow> = Vec::new();
    match dev.page {
        // The category index: every panel section one Enter away.
        0 => {
            v.push(("Weapons".into(), "give Bond any weapon by its game name (Enter = equip + draw)", MenuAction::Page(1)));
            v.push(("Gadgets".into(), "gadget inventory: giving one only puts it in the inventory", MenuAction::Page(2)));
            v.push(("Grenades/explosives".into(), "frag/stun/smoke/mine x5 (the game's own ids 0x34-0x37)", MenuAction::Page(3)));
            v.push(("Ammo/health/armour".into(), "full health, full armour, +3 clips per owned weapon", MenuAction::Page(4)));
            v.push(("NPCs".into(), "spawn by skin and mode, clear NPCs, arena mode", MenuAction::Page(5)));
            v.push(("Teleport".into(), "named spots + save/restore Bond's place", MenuAction::Page(6)));
            v.push(("Level".into(), "jump to any level (carries health/weapons)", MenuAction::Page(7)));
            v.push(("Toggles".into(), "god, infinite ammo, noclip, AI overlay, portal cull, interp, fps", MenuAction::Page(8)));
            v.push(("Screenshot".into(), "numbered PNG + the --bond spot to re-render it", MenuAction::Page(9)));
        }
        // Weapons: every non-gadget record (grenades included — the panel's Weapons section), the
        // game's own name per row, Enter gives + equips + draws (the --give path).
        1 => {
            if let Some(sc) = scene {
                if passes_filter(dev, "level kit") {
                    v.push(("level kit".into(), "the game's own loadout for this level (Player_InitWeapon 0x1a7ef8)", MenuAction::LevelKit));
                }
                for e in crate::panel::catalog_entries(sc) {
                    if e.gadget { continue; }
                    let label = format!("0x{:02x} {}{}", e.id, e.name, if e.fp { "" } else { " (no fp model)" });
                    if !passes_filter(dev, &label) { continue; }
                    v.push((label, "Enter: equip + draw (full clip). Ammo in the Ammo page.", MenuAction::GiveWeapon { id: e.id, rounds: None }));
                }
            } else {
                v.push(("(no level loaded)".into(), "load a level first", MenuAction::Back));
            }
        }
        // Gadgets: ids 0x4A and up (Player_WeaponChange's gadget cycle bounds [M 0x1a6f80]).
        2 => {
            if let Some(sc) = scene {
                for e in crate::panel::catalog_entries(sc) {
                    if !e.gadget { continue; }
                    let label = format!("0x{:02x} {}", e.id, e.name);
                    if !passes_filter(dev, &label) { continue; }
                    v.push((label, "Enter: into the inventory (gadget fire-side is per gadget)", MenuAction::GiveWeapon { id: e.id, rounds: None }));
                }
            } else {
                v.push(("(no level loaded)".into(), "load a level first", MenuAction::Back));
            }
        }
        // Grenades/explosives: the four grenade records at x5 rounds (the panel's consumables).
        3 => {
            if let Some(sc) = scene {
                for e in crate::panel::catalog_entries(sc) {
                    if !GRENADES.contains(&e.id) { continue; }
                    let label = format!("0x{:02x} {} x5", e.id, e.name);
                    if !passes_filter(dev, &label) { continue; }
                    v.push((label, "Enter: five into the inventory (frag = 0x34, the game's own id)", MenuAction::GiveWeapon { id: e.id, rounds: Some(5) }));
                }
            } else {
                v.push(("(no level loaded)".into(), "load a level first", MenuAction::Back));
            }
        }
        // Ammo/health/armour: the consumables + per-owned-weapon top-ups.
        4 => {
            v.push(("full health".into(), "health 100 (the recorder cheat's ceiling)", MenuAction::Health));
            v.push(("full armour".into(), "armour 50 (the armour-take refuse threshold)", MenuAction::Armour));
            v.push(("ammo for every owned weapon".into(), "+3 clips into each owned weapon's spare", MenuAction::GiveAmmoAll));
            if let Some(sim) = scene.and_then(|s| s.sim.as_ref()) {
                let sc = scene.unwrap();
                for e in crate::panel::catalog_entries(sc) {
                    if !sim.drones.inventory.owned.get(e.id as usize).copied().unwrap_or(false) { continue; }
                    let label = format!("+ammo 0x{:02x} {}", e.id, e.name);
                    if !passes_filter(dev, &label) { continue; }
                    v.push((label, "Enter: +3 clips of this weapon's ammo", MenuAction::GiveAmmoFor(e.id)));
                }
            }
        }
        // NPCs: mode select + one row per skin (the panel's NPC section: spawn in front of Bond).
        5 => {
            let mode_label = match dev.spawn { SpawnPick::Normal => "normal", SpawnPick::Sniper => "sniper", SpawnPick::Ambush => "ambush" };
            v.push((format!("mode: {mode_label} (Enter cycles)"), "the spawned guard's AI mode (0 normal / 2 sniper / 0x10 ambush)", MenuAction::CycleNpcMode));
            v.push(("spawn at crosshair".into(), "a guard of the level's first active skin, at the crosshair", MenuAction::Spawn(dev.spawn)));
            if let Some(sc) = scene {
                let free = |s: &crate::scene::NpcSlot| -> usize {
                    let Some(sim) = sc.sim.as_ref() else { return 0 };
                    s.chars.iter().filter(|&&c| sim.char_drone.get(c).copied().flatten().is_none()).count()
                };
                let mut seen: Vec<u32> = Vec::new();
                for s in &sc.npc_pool {
                    if seen.contains(&s.skin) { continue; }
                    seen.push(s.skin);
                    let total = sc.npc_pool.iter().filter(|x| x.skin == s.skin).count();
                    let label = format!("skin {:08x} ({}/{})", s.skin, free(s), total);
                    if !passes_filter(dev, &label) { continue; }
                    v.push((label, "Enter: spawn in front of Bond, facing him (a free pool slot)", MenuAction::SpawnNpcSkin(s.skin)));
                }
            }
            v.push(("remove spawned NPCs".into(), "despawn every NPC spawned from the pool", MenuAction::ClearNpcs));
            v.push(("arena mode (clear level)".into(), "remove NPCs, pickups, triggers, spawner templates", MenuAction::Arena));
        }
        // Teleport: the named spots + the old save/restore pair.
        6 => {
            v.push(("teleport: save spot".into(), "save Bond's current spot - restore later", MenuAction::SaveSpot));
            v.push(("teleport: load spot".into(), "return Bond to the saved spot", MenuAction::RestoreSpot));
            for (i, (name, pos)) in TELEPORT_SPOTS.iter().enumerate() {
                let label = format!("{} ({:.2}, {:.2}, {:.2})", name, pos[0], pos[1], pos[2]);
                if !passes_filter(dev, &label) { continue; }
                v.push((label, "Enter: Bond stands there (Player_StandAtNewPosition), grounded on the next ticks", MenuAction::TeleportTo(i)));
            }
        }
        // Level: the disc's level archives (the old level-select rows).
        7 => {
            for (i, &id) in LEVELS.iter().enumerate() {
                let label = LEVEL_LABELS[i].to_string();
                if !passes_filter(dev, &label) { continue; }
                v.push((label, "jump to this level (carries health/weapons)", MenuAction::Level(id)));
            }
        }
        // Toggles: the cheats + the rendering aids, current state in the label.
        8 => {
            v.push(if dev.god_mode { ("god: ON".into(), "Bond takes no damage - toggle off to test death", MenuAction::ToggleGod) } else { ("god: OFF".into(), "Bond takes no damage when ON - toggle to test death", MenuAction::ToggleGod) });
            v.push(if dev.infinite_ammo { ("ammo: ON".into(), "clip refills each tick - toggle off to test reloads", MenuAction::ToggleAmmo) } else { ("ammo: OFF".into(), "clip refills each tick when ON - toggle to test reloads", MenuAction::ToggleAmmo) });
            v.push(if dev.no_reload { ("reload: OFF".into(), "the clip never empties (no reload needed) - toggle to test reloads", MenuAction::ToggleNoReload) } else { ("reload: ON".into(), "the clip never empties while OFF is shown - toggle to test reloads", MenuAction::ToggleNoReload) });
            v.push(("noclip (N key)".into(), "walk through walls - N toggles, like this row", MenuAction::ToggleNoclip));
            v.push(if dev.overlay { ("AI overlay: ON (F1)".into(), "state/alert/health over each guard - toggle or F1", MenuAction::ToggleOverlay) } else { ("AI overlay: OFF (F1)".into(), "state/alert/health over each guard - toggle or F1", MenuAction::ToggleOverlay) });
            v.push(if dev.portal_cull { ("portal culling: ON".into(), "the game's visibility walk (0x1e86e8) - off draws every cel", MenuAction::TogglePortal) } else { ("portal culling: OFF".into(), "the game's visibility walk (0x1e86e8) - off draws every cel", MenuAction::TogglePortal) });
            v.push(if dev.interp { ("interpolation: ON".into(), "render between ticks - off renders whole ticks (NF_NOINTERP)", MenuAction::ToggleInterp) } else { ("interpolation: OFF".into(), "render between ticks - off renders whole ticks (NF_NOINTERP)", MenuAction::ToggleInterp) });
            v.push(if dev.show_fps { ("fps display: ON".into(), "render fps top right - wall-clock display only", MenuAction::ToggleFps) } else { ("fps display: OFF".into(), "render fps top right - wall-clock display only", MenuAction::ToggleFps) });
            v.push(("time: 1x".into(), "normal speed (60 Hz)", MenuAction::TimeScale(1.0)));
            v.push(("time: 0.25x".into(), "quarter speed - watch AI frame-by-frame", MenuAction::TimeScale(0.25)));
            v.push(("time: pause".into(), "freeze the sim; F11 advances one tick", MenuAction::TimeScale(0.0)));
        }
        // Screenshot: the F12 path (a numbered PNG + the --bond spot txt).
        9 => {
            v.push(("screenshot + log spot".into(), "nf-shots/NNN-<level>.png + a .txt with the --bond spot (F12)", MenuAction::Screenshot));
        }
        _ => v.push(("(empty page)".into(), "back to the index", MenuAction::Back)),
    }
    v
}

/// The level-select list (fix-dev T5): the level archives on the user's disc (`FILES.BIN` manifest), in
/// archive order. Jumping feeds `pending_level` — the same warm-transition path a level exit uses.
pub const LEVELS: &[u32] = &[
    0x0700_0001, 0x0700_0002, 0x0700_0003, 0x0700_0004, 0x0700_0005, 0x0700_0006, 0x0700_0007, 0x0700_0008,
    0x0700_0009, 0x0700_000a, 0x0700_000b, 0x0700_000c, 0x0700_000d, 0x0700_0011, 0x0700_0012, 0x0700_0013,
    0x0700_0014, 0x0700_0015, 0x0700_0016, 0x0700_001b, 0x0700_0023, 0x0700_0024, 0x0700_0025, 0x0700_0026,
    0x0700_0027, 0x0700_0028, 0x0700_0029, 0x0700_0041, 0x0700_0043, 0x0700_0046, 0x0700_0048, 0x0700_0049,
    0x0700_004a, 0x0700_004b,
];

/// One label per `LEVELS` entry (static: the menu rows are built every frame and must not allocate).
pub const LEVEL_LABELS: &[&str] = &[
    "level: 07000001", "level: 07000002", "level: 07000003", "level: 07000004", "level: 07000005",
    "level: 07000006", "level: 07000007", "level: 07000008", "level: 07000009", "level: 0700000a",
    "level: 0700000b", "level: 0700000c", "level: 0700000d", "level: 07000011", "level: 07000012",
    "level: 07000013", "level: 07000014", "level: 07000015", "level: 07000016", "level: 0700001b",
    "level: 07000023", "level: 07000024", "level: 07000025", "level: 07000026", "level: 07000027",
    "level: 07000028", "level: 07000029", "level: 07000041", "level: 07000043", "level: 07000046",
    "level: 07000048", "level: 07000049", "level: 0700004a", "level: 0700004b",
];

/// A menu item's action. The give/spawn/arena/health actions route through `panel::apply`
/// (panel.rs is the shared command layer — nothing here re-implements a panel command).
#[derive(Clone, Copy, Debug)]
pub enum MenuAction {
    Close,
    /// Open a page (the index rows; PageUp/PageDown step `Dev::page` directly).
    Page(usize),
    /// Back to the category index (Backspace with an empty filter).
    Back,
    /// Give a weapon/gadget/grenade: `rounds` None = a full clip (the panel's Give button).
    GiveWeapon { id: u8, rounds: Option<i16> },
    GiveAmmoFor(u8), GiveAmmoAll, LevelKit, Health, Armour,
    /// Spawn a pool NPC of `skin` (hex) with the mode picked on the page.
    SpawnNpcSkin(u32), CycleNpcMode, ClearNpcs, Arena,
    /// Teleport Bond to `TELEPORT_SPOTS[idx]`.
    TeleportTo(usize),
    // The original F2 tools (WORKLOG 0081), kept.
    Spawn(SpawnPick), ForceGrenade, Impulse, SpecialDeath, ToggleGod, ToggleAmmo, ToggleNoReload,
    SaveSpot, RestoreSpot, TimeScale(f32), Level(u32),
    // The Toggles/Screenshot pages.
    ToggleNoclip, ToggleOverlay, TogglePortal, ToggleInterp, ToggleFps, Screenshot,
}

/// The side effects a menu action needs the windowed App for (headless tests ignore them).
#[derive(Default)]
pub struct ActionOut {
    /// The result line for the menu's message bar.
    pub status: String,
    /// The weapon to draw (`App.weapon_switch_to` — the panel's switch return).
    pub switch_to: Option<u8>,
    /// The level to load (`App.pending_level`).
    pub pending_level: Option<u32>,
    /// Reset the renderer's viewer cel (a teleport moved Bond — the game's own placement clears
    /// it, `Camera_Enable` 0x1b9920).
    pub viewer_cel_reset: bool,
    /// Take a screenshot (the Screenshot page — App runs its `spot_shot`).
    pub shot: bool,
    /// The new noclip state (the App owns `noclip`; the menu row toggles it like the N key).
    pub noclip_to: Option<bool>,
}

/// Apply a menu action to the scene the way the windowed App does — the REAL path, shared by
/// `App::apply_menu_action` and the headless menu-flow tests. Every give/spawn/clear command goes
/// through `panel::apply` (the shared command layer), so the menu and (with `--panel`) the HTTP
/// page cannot drift apart.
pub fn apply_menu_action_to_scene(dev: &mut Dev, scene: &mut Scene, action: MenuAction, eye: [f32; 3], yaw: f32, noclip: bool) -> ActionOut {
    let mut out = ActionOut::default();
    match action {
        MenuAction::Close => { dev.menu = false; out.status = "menu closed".into(); }
        MenuAction::Page(p) => {
            dev.page = p.min(PAGES.len() - 1);
            dev.menu_sel = 0;
            dev.filter.clear();
            out.status = format!("page: {}", PAGES[dev.page]);
        }
        MenuAction::Back => { dev.page = 0; dev.menu_sel = 0; dev.filter.clear(); out.status = "page: menu".into(); }
        MenuAction::GiveWeapon { id, rounds } => {
            let mut q: Vec<(String, String)> = vec![("do".into(), "weapon".into()), ("id".into(), format!("0x{id:02x}"))];
            if let Some(r) = rounds { q.push(("rounds".into(), format!("{r}"))); }
            let (status, switch) = crate::panel::apply(scene, &q);
            out.status = status;
            out.switch_to = switch;
        }
        MenuAction::GiveAmmoFor(id) => {
            let (status, _) = crate::panel::apply(scene, &[("do".into(), "ammo_for".into()), ("id".into(), format!("0x{id:02x}"))]);
            out.status = status;
        }
        MenuAction::GiveAmmoAll => {
            let (status, _) = crate::panel::apply(scene, &[("do".into(), "ammo_all".into())]);
            out.status = status;
        }
        MenuAction::LevelKit => {
            let (status, _) = crate::panel::apply(scene, &[("do".into(), "levelkit".into())]);
            out.status = status;
        }
        MenuAction::Health => {
            let (status, _) = crate::panel::apply(scene, &[("do".into(), "health".into())]);
            out.status = status;
        }
        MenuAction::Armour => {
            let (status, _) = crate::panel::apply(scene, &[("do".into(), "armour".into())]);
            out.status = status;
        }
        MenuAction::SpawnNpcSkin(skin) => {
            let mode = match dev.spawn { SpawnPick::Normal => "0", SpawnPick::Sniper => "2", SpawnPick::Ambush => "16" };
            let (status, _) = crate::panel::apply(scene, &[("do".into(), "npc".into()), ("skin".into(), format!("{skin:08x}")), ("mode".into(), mode.into())]);
            out.status = status;
        }
        MenuAction::CycleNpcMode => {
            dev.spawn = match dev.spawn { SpawnPick::Normal => SpawnPick::Sniper, SpawnPick::Sniper => SpawnPick::Ambush, SpawnPick::Ambush => SpawnPick::Normal };
            out.status = format!("NPC mode: {}", match dev.spawn { SpawnPick::Normal => "normal", SpawnPick::Sniper => "sniper", SpawnPick::Ambush => "ambush" });
        }
        MenuAction::ClearNpcs => {
            let (status, _) = crate::panel::apply(scene, &[("do".into(), "clear_npcs".into())]);
            out.status = status;
        }
        MenuAction::Arena => {
            let (status, _) = crate::panel::apply(scene, &[("do".into(), "arena".into())]);
            out.status = status;
        }
        MenuAction::TeleportTo(i) => {
            let Some((name, pos)) = TELEPORT_SPOTS.get(i) else { out.status = "bad spot".into(); return out; };
            dev.teleport(scene, *pos, yaw);
            // The game's own placement clears the viewer's cel so the next `Camera_CheckLocation`
            // re-finds it at the placed position [M 0x1b9920].
            out.viewer_cel_reset = true;
            out.status = format!("teleported: {name} ({:.2},{:.2},{:.2})", pos[0], pos[1], pos[2]);
        }
        // ---- the original F2 tools (WORKLOG 0081) ----
        MenuAction::Spawn(pick) => {
            let look = { let s = scene.sim.as_ref(); match s { Some(sim) => sim.bond.look(), None => [0.0, 0.0, 1.0] } };
            let hit = scene.sim.as_ref().and_then(|s| s.world.segment(eye, [eye[0] + look[0] * 30.0, eye[1] + look[1] * 30.0, eye[2] + look[2] * 30.0]));
            let pos = hit.map_or([eye[0] + look[0] * 5.0, eye[1], eye[2] + look[2] * 5.0], |t| [eye[0] + (look[0] * 30.0) * t, eye[1] + (look[1] * 30.0) * t, eye[2] + (look[2] * 30.0) * t]);
            dev.spawn = pick;
            let gi = dev.spawn_guard(scene, pos, yaw);
            let kind = match pick { SpawnPick::Normal => "normal", SpawnPick::Sniper => "sniper", SpawnPick::Ambush => "ambush" };
            out.status = match gi { Some(gi) => format!("guard #{gi}: spawned ({kind}) at ({:.1},{:.1},{:.1})", pos[0], pos[1], pos[2]), None => format!("spawn failed ({kind}) — no free slot or no sim") };
        }
        MenuAction::ForceGrenade => {
            let bond = scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos);
            let gi = dev.force_grenade(scene, bond).or(dev.forced.last().copied());
            out.status = match gi { Some(gi) => format!("guard #{gi}: forced grenade throw — watch the lob"), None => "no guard within 30 m to force".to_string() };
        }
        MenuAction::Impulse => {
            let bond = scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos);
            let gi = dev.impulse_guard(scene, bond).or(dev.forced.last().copied());
            out.status = match gi { Some(gi) => format!("guard #{gi}: impulse applied — watch the hop"), None => "no guard within 30 m to impulse".to_string() };
        }
        MenuAction::SpecialDeath => {
            let bond = scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos);
            let gi = dev.special_death(scene, bond).or(dev.forced.last().copied());
            out.status = match gi { Some(gi) => format!("guard #{gi}: SpecialDeath triggered — watch the fall"), None => "no guard within 30 m".to_string() };
        }
        MenuAction::ToggleGod => { dev.god_mode = !dev.god_mode; out.status = format!("god mode: {}", if dev.god_mode { "ON" } else { "OFF" }); }
        MenuAction::ToggleAmmo => { dev.infinite_ammo = !dev.infinite_ammo; out.status = format!("infinite ammo: {}", if dev.infinite_ammo { "ON" } else { "OFF" }); }
        MenuAction::ToggleNoReload => { dev.no_reload = !dev.no_reload; out.status = format!("no reload: {}", if dev.no_reload { "ON" } else { "OFF" }); }
        MenuAction::SaveSpot => {
            let p = scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos);
            dev.saved_spot = Some(p);
            out.status = format!("spot saved at ({:.1},{:.1},{:.1})", p[0], p[1], p[2]);
        }
        MenuAction::RestoreSpot => {
            match dev.saved_spot {
                Some(p) => { dev.teleport(scene, p, yaw); out.viewer_cel_reset = true; out.status = format!("restored to ({:.1},{:.1},{:.1})", p[0], p[1], p[2]); }
                None => out.status = "no saved spot — use 'save spot' first".to_string(),
            }
        }
        MenuAction::TimeScale(s) => { dev.time_scale = s; out.status = format!("time scale: {}", if s == 0.0 { "paused (F11=step)" } else if s == 1.0 { "1×" } else { "0.25×" }); }
        MenuAction::Level(id) => { out.pending_level = Some(id); out.status = format!("loading {} ..", crate::level_archive_name(id)); }
        // ---- Toggles / Screenshot ----
        MenuAction::ToggleNoclip => { out.noclip_to = Some(!noclip); out.status = format!("noclip: {}", if !noclip { "ON — walk through walls" } else { "OFF" }); }
        MenuAction::ToggleOverlay => { dev.overlay = !dev.overlay; out.status = format!("AI overlay: {}", if dev.overlay { "ON" } else { "OFF" }); }
        MenuAction::TogglePortal => { dev.portal_cull = !dev.portal_cull; out.status = format!("portal culling: {}", if dev.portal_cull { "ON" } else { "OFF" }); }
        MenuAction::ToggleInterp => { dev.interp = !dev.interp; out.status = format!("interpolation: {}", if dev.interp { "ON" } else { "OFF" }); }
        MenuAction::ToggleFps => { dev.show_fps = !dev.show_fps; out.status = format!("fps display: {}", if dev.show_fps { "ON" } else { "OFF" }); }
        MenuAction::Screenshot => { out.shot = true; out.status = "screenshot".into(); }
    }
    out
}

/// The click hit-test: which menu item is under the mouse (NDC −1..1, y up)? Returns the row index.
/// Shares `layout_rows` with the draw fn, so click and paint can never drift.
pub fn menu_hit(dev: &Dev, scene: Option<&Scene>, mouse_x: f32, mouse_y: f32, aspect: f32) -> Option<usize> {
    let s = 2.0 / 512.0 * (4.0 / 3.0) / aspect.max(1e-3);
    let x0 = -1.0 + 4.0 * s;
    let x1 = -1.0 + (4.0 + 150.0) * s;
    if mouse_x < x0 || mouse_x > x1 { return None; }
    let n = menu_items(dev, scene).len();
    if n == 0 { return None; }
    let (y0, top, visible_end) = layout_rows(dev, n);
    for vi in top..visible_end {
        let y_top = y0 + (vi - top) as f32 * MENU_ROW_H;
        let y_bot = y_top + MENU_ROW_H;
        let ndc_top = 1.0 - y_top / 224.0;
        let ndc_bot = 1.0 - y_bot / 224.0;
        if mouse_y >= ndc_bot && mouse_y <= ndc_top { return Some(vi); }
    }
    None
}

/// The menu's row layout, shared by the draw fn and the click hit-test: (the y where the first row
/// starts in HUD px, the first visible row, one past the last). Header = the page tabs line, plus
/// the filter line when a filter is set; the scenario banner sits above both.
fn layout_rows(dev: &Dev, n: usize) -> (f32, usize, usize) {
    const VISIBLE_ROWS: usize = MENU_VISIBLE_ROWS;
    let top = if n <= VISIBLE_ROWS { 0 } else {
        if dev.menu_sel < VISIBLE_ROWS / 2 { 0 }
        else if dev.menu_sel >= n - VISIBLE_ROWS / 2 { n - VISIBLE_ROWS }
        else { dev.menu_sel - VISIBLE_ROWS / 2 }
    };
    let visible_end = (top + VISIBLE_ROWS).min(n);
    // Scenario banner (1 line when a scenario ran) + the page header + the filter line.
    let banner = if dev.scenario.is_some() { MENU_HEADER_H + 1.0 } else { 2.0 };
    let mut y = 4.0 + banner + MENU_HEADER_H + 1.0;
    if !dev.filter.is_empty() { y += MENU_ROW_H; }
    (y, top, visible_end)
}

/// Apply a menu click: returns the action chosen (the caller applies it).
pub fn menu_action(dev: &Dev, scene: Option<&Scene>, idx: usize) -> Option<MenuAction> {
    menu_items(dev, scene).get(idx).map(|(_, _, a)| *a)
}

/// B-9: keyboard navigation for the menu, extended for the paged menu (fix-ingame-dev-menu):
/// ↑/↓ move the selection, Enter activates, PageUp/PageDown switch pages, Backspace deletes a
/// filter char (or returns to the index). Returns the action to apply (if Enter was pressed).
pub fn menu_key(dev: &mut Dev, key: MenuKey) -> Option<MenuAction> {
    let n = menu_items(dev, None).len();
    match key {
        MenuKey::Up => {
            if n == 0 { return None; }
            if dev.menu_sel == 0 { dev.menu_sel = n - 1; } else { dev.menu_sel -= 1; }
            None
        }
        MenuKey::Down => {
            if n == 0 { return None; }
            dev.menu_sel = (dev.menu_sel + 1) % n;
            None
        }
        MenuKey::PageUp => { dev.page = dev.page.saturating_sub(1); dev.menu_sel = 0; dev.filter.clear(); None }
        MenuKey::PageDown => { dev.page = (dev.page + 1).min(PAGES.len() - 1); dev.menu_sel = 0; dev.filter.clear(); None }
        MenuKey::Backspace => {
            if !dev.filter.is_empty() { dev.filter.pop(); }
            else if dev.page != 0 { dev.page = 0; dev.menu_sel = 0; }
            None
        }
        MenuKey::Activate => {
            // No scene here: the static rows (index, toggles, levels) activate fine; the App's
            // Enter uses `menu_activate(d, Some(&scene))` so catalog rows resolve too.
            menu_activate(dev, None)
        }
    }
}

/// Enter on the selected row, with the scene so catalog rows resolve. (Split from `menu_key` so
/// the navigation keys keep their old one-arg shape.)
pub fn menu_activate(dev: &Dev, scene: Option<&Scene>) -> Option<MenuAction> {
    let n = menu_items(dev, scene).len();
    if n == 0 { return None; }
    let i = dev.menu_sel.min(n - 1);
    menu_items(dev, scene).get(i).map(|(_, _, a)| *a)
}

/// A keyboard input to the menu (B-9, extended for pages).
#[derive(Clone, Copy, PartialEq)]
pub enum MenuKey { Up, Down, Activate, PageUp, PageDown, Backspace }

/// Type one character into the filter (the type-to-filter lane; lowercase ASCII).
pub fn menu_type(dev: &mut Dev, c: char) {
    if dev.filter.len() < 24 { dev.filter.push(c.to_ascii_lowercase()); }
    dev.menu_sel = 0;
}

/// Draw a world-space line as a thin NDC quad between the two projected points.
fn draw_line(h: &mut Hud, a: [f32; 3], b: [f32; 3], c: [f32; 4], view_proj: [[f32; 4]; 4]) {
    let pa = project(a, view_proj);
    let pb = project(b, view_proj);
    if pa[0].abs() > 1.5 || pb[0].abs() > 1.5 { return; }
    let (dx, dy) = (pb[0] - pa[0], pb[1] - pa[1]);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (nx, ny) = (-dy / len, dx / len);
    let w = 0.004;
    let v = |p: [f32; 2]| HudVertex { p, uv: [0.0, 0.0], c };
    let q = [
        v([pa[0] + nx * w, pa[1] + ny * w]), v([pb[0] + nx * w, pb[1] + ny * w]), v([pb[0] - nx * w, pb[1] - ny * w]),
        v([pa[0] + nx * w, pa[1] + ny * w]), v([pb[0] - nx * w, pb[1] - ny * w]), v([pa[0] - nx * w, pa[1] - ny * w]),
    ];
    h.ndc_quad(65, q, c);
}

/// Draw a world-space circle (the explosion radius) as a screen-space octagon.
fn draw_circle(h: &mut Hud, centre: [f32; 3], radius: f32, c: [f32; 4], view_proj: [[f32; 4]; 4]) {
    let p0 = project(centre, view_proj);
    if p0[0].abs() > 1.5 { return; }
    let p1 = project([centre[0] + radius, centre[1], centre[2]], view_proj);
    let r = (p1[0] - p0[0]).abs().max(0.01);
    let mut prev = [p0[0] + r, p0[1]];
    for k in 1..=8u32 {
        let a = k as f32 * std::f32::consts::TAU / 8.0;
        let p = [p0[0] + r * a.cos(), p0[1] + r * a.sin()];
        // Draw a line between prev and p (in NDC; z is irrelevant — project ignores it for already-2D points).
        draw_ndc_line(h, prev, p, c);
        prev = p;
    }
}

/// Draw a line between two NDC points (for the circle's segments).
fn draw_ndc_line(h: &mut Hud, pa: [f32; 2], pb: [f32; 2], c: [f32; 4]) {
    let (dx, dy) = (pb[0] - pa[0], pb[1] - pa[1]);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (nx, ny) = (-dy / len, dx / len);
    let w = 0.004;
    let v = |p: [f32; 2]| HudVertex { p, uv: [0.0, 0.0], c };
    let q = [
        v([pa[0] + nx * w, pa[1] + ny * w]), v([pb[0] + nx * w, pb[1] + ny * w]), v([pb[0] - nx * w, pb[1] - ny * w]),
        v([pa[0] + nx * w, pa[1] + ny * w]), v([pb[0] - nx * w, pb[1] - ny * w]), v([pa[0] - nx * w, pa[1] - ny * w]),
    ];
    h.ndc_quad(65, q, c);
}

/// The state name for the overlay.
pub fn state_name(s: u8) -> &'static str {
    use state::*;
    match s {
        GLOBAL => "Global", WAIT_SWITCH => "WaitSwitch", IDLE => "Idle", ALERT => "Alert", INIT_PATROL => "InitPatrol",
        SNIPER_IDLE => "SniperIdle", SNIPER_AIM => "SniperAim", SNIPER_FIRE => "SniperFire", SNIPER_RELOAD => "SniperReload",
        SURRENDER_ANIM => "SurrenderAnim", SURRENDERED => "Surrendered", UNSURRENDER_ANIM => "UnsurrenderAnim", DRAW_WEAPON => "DrawWeapon",
        PATROL => "Patrol", ATTACK => "Attack", COMBAT => "Combat", COMBAT_NO_MOVE => "CombatNoMove",
        COMBAT_OUT_OF_RANGE => "OutOfRange", COMBAT_NEW_SIGHTING => "NewSighting", COMBAT_NO_SIGHT => "NoSight",
        COMBAT_DRONE_AHEAD => "DroneAhead", COMBAT_NO_ROUTE => "NoRoute", NO_OPPONENT => "NoOpponent", DRONE_STUCK => "Stuck",
        GO_TO_GOAL => "GoToGoal", DEATH_ANIM => "DeathAnim", SPECIAL_DEATH => "SpecialDeath", DEAD => "Dead", FADE => "Fade", FADE_FAST => "FadeFast",
        BULLET_IMPACT => "BulletImpact", ALERT_TO_POSITION => "AlertToPos", SEARCH_AREA => "SearchArea",
        HEARD_NOISE => "HeardNoise", HEARD_NOISE_AWARE => "HeardNoiseAware", HEARD_NOISE_SUSPECT => "HeardNoiseSuspect",
        HEARD_NOISE_ALERT => "HeardNoiseAlert",
        RUN_FOR_COVER => "RunForCover", UNDER_COVER_INIT => "CoverInit", UNDER_COVER_IDLE => "CoverIdle",
        UNDER_COVER_AIM => "CoverAim", UNDER_COVER_FIRE => "CoverFire", UNDER_COVER_RETURN => "CoverReturn",
        UNDER_COVER_TYPE_CHANGE => "CoverTypeChange", UNDER_COVER_LEAVE => "CoverLeave", UNDER_COVER_LEAVE_NOW => "CoverLeaveNow",
        UNDER_COVER_SNIPER_FIRE => "Sniper_PopUp", UNDER_COVER_SNIPER_RELOAD => "SniperReload",
        AMBUSH_INIT => "Ambush", AMBUSH_WAIT => "AmbushWait", GRENADE_THROW => "GrenadeThrow",
        AIM_STAND => "AimStand", AIM_STAND_FIRE => "AimStandFire", AIM_STAND_RELOAD => "AimStandReload", AIM_BACKOFF => "AimBackoff",
        AIM_CROUCH => "AimCrouch", AIM_CROUCH_FIRE => "AimCrouchFire", AIM_CROUCH_RELOAD => "AimCrouchReload",
        STEP_AIM_LEFT => "StepAimL", STEP_AIM_RIGHT => "StepAimR", STRAFE_AIM_LEFT => "StrafeAimL", STRAFE_AIM_RIGHT => "StrafeAimR",
        STRAFE_DODGE_LEFT => "StrafeDodgeL", STRAFE_DODGE_RIGHT => "StrafeDodgeR", ROLL_LEFT_CROUCH => "RollL", ROLL_RIGHT_CROUCH => "RollR",
        0x86 => "RecoverFromScary",
        _ => "Unknown",
    }
}

// ---- named scenarios -----------------------------------------------------------------------------------------

/// An active guard (the game's own definition of a guard that runs its AI this frame): present (not a spawner
/// template, not removed), enabled (not gated off), alive (not a corpse), in the llist (the game's tick list — the
/// game won't run its AI otherwise), and drawn (`obj+0xFC`, the renderer's cel visibility flag — a guard the game
/// doesn't draw doesn't act in play). The switch channel is checked against the live channels state so it works
/// pre-tick after the scenario turns channels on.
///
/// B-6/B-7: the old filter was `present` only, which picked guards that exist in the list but never activate in
/// play (WAIT_SWITCH drones, spawner templates) — the user saw "no guard". The drawn + in-llist checks are the
/// user's explicit demand: "Find the game's own definition of an active guard: spawned, in the llist, alive, and
/// the drawn flag (obj+0xFC)." Pub for tests.
pub fn is_active_guard(drones: &nf_game::drone::Drones, gi: usize) -> bool {
    let Some(dr) = drones.list.get(gi) else { return false };
    dr.present && dr.enabled && !dr.corpse && dr.health > 0.0 && dr.drawn
        && drones.llist.contains(&gi)
        && (dr.switch_channel == 0 || drones.channels.on(dr.switch_channel as u32))
}

/// The nearest active guard to `pos` within `max_dist` metres (2-D). Used by tests to find the guard a scenario placed
/// Bond near (the scenario functions return `()`; the test locates the guard by proximity). B-7.
pub fn nearest_active_guard(sim: &crate::scene::Sim, pos: [f32; 3], max_dist: f32) -> Option<usize> {
    sim.drones.list.iter().enumerate()
        .filter(|(i, _)| is_active_guard(&sim.drones, *i))
        .min_by(|(_, a), (_, b)| {
            let da = (a.pos[0] - pos[0]).powi(2) + (a.pos[2] - pos[2]).powi(2);
            let db = (b.pos[0] - pos[0]).powi(2) + (b.pos[2] - pos[2]).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .filter(|(_, d)| ((d.pos[0] - pos[0]).powi(2) + (d.pos[2] - pos[2]).powi(2)).sqrt() <= max_dist)
        .map(|(i, _)| i)
}

/// How many of the 4 cardinal directions Bond can walk `dist` metres from `pos` without hitting a wall. B-6. Pub for
/// tests (asserts the scenario's spot is not sealed in a pocket).
pub fn walkable_directions(world: &nf_game::world::CollisionWorld, pos: [f32; 3], dist: f32) -> usize {
    let eye = [pos[0], pos[1] + 1.0, pos[2]];
    let mut n = 0;
    for [dx, dz] in [[1.0f32, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]] {
        if world.segment(eye, [eye[0] + dx * dist, eye[1], eye[2] + dz * dist]).is_none() { n += 1; }
    }
    n
}

/// The named scenarios. Each places Bond and the guards and returns the instruction text the user should see.
/// (B-3a/b: the five scenarios that used to live in `main.rs::apply_scenario` — weapon-switch / sg5-pickup /
/// wire-music / autoaim / sniper-scope — are merged into this single registry so `--scenario` only has one source
/// of truth.)
pub fn scenario(name: &str) -> Option<(&'static str, fn(&mut Scene))> {
    // B-8/B-9/B-10: the following scenarios are REMOVED until their tests pass. Each is a real gap (see docs/GAPS.md
    // and docs/WORKLOG.md B-10), not a placement issue:
    //  - sniper-cover: the picked guard has no cover nearby → reaches NoSight (0x5c), not 0x97. Needs cover-aware
    //    placement (find a guard near a cover node).
    //  - bullet-flyby: needs the exact fly-by probability from Bullet_DoTrails (decomp).
    //  - friendly-actor: a real game bug — guards outside the llist still fire (B-10).
    //  (searchlight-tracks-bond RE-ADDED by fix-searchlight-remainder: G-036 closed as a misdiagnosis — the
    //   1/7 linked heads on 07000005 are the level's own data and the "beam points up" head is the game's
    //   own mid; 07000006's four heads all carry linked spots and track. Run it on 07000006.)
    //  - grenade-throw / grenade-bounce-explode (B-10): on 07000005 (the only level with active guards at start,
    //    since 07000002's combat guards are switch-gated and turning on all channels starts scripts), the guard's
    //    combat AI keeps entering RUN_FOR_COVER (0x8c) instead of AIM_STAND_FIRE, so `look_for_opponent` never
    //    returns GRENADE_THROW within 600 ticks. Needs cover-aware placement (a guard with NO cover nearby, so it
    //    stays in AIM_STAND_FIRE and throws) — same root as sniper-cover. The scenario fn is kept for when that's done.
    let (n, f): (&'static str, fn(&mut Scene)) = match name {
        "ambush" => ("Ambush (0x37/0x38)", scenarios::ambush),
        "guard-falls-off-ledge" => ("Guard falls off a ledge", scenarios::guard_falls_off_ledge),
        "door-anti-crush" => ("Door anti-crush", scenarios::door_anti_crush),
        "cutscene-ai-pause" => ("Cut-scene AI pause (guards don't fire)", scenarios::cutscene_ai_pause),
        "ghostly-white" => ("Ghostly white (clear colour check)", scenarios::ghostly_white),
        "door-textures" => ("Door texture state", scenarios::door_textures),
        "invisible-walls" => ("Invisible walls (NF_COLL)", scenarios::invisible_walls),
        "level-exit" => ("Level exit trigger", scenarios::level_exit),
        "weapon-switch" => ("Weapon switch (P2K/SG5/sniper)", scenarios::weapon_switch),
        "sg5-pickup" => ("SG5 pickup", scenarios::sg5_pickup),
        "wire-music" => ("Wire music (event 0xD)", scenarios::wire_music),
        "autoaim" => ("Auto-aim (T)", scenarios::autoaim),
        "sniper-scope" => ("Sniper scope (right mouse)", scenarios::sniper_scope),
        "shotgun-reload" => ("Shotgun reload (R, shell-by-shell)", scenarios::shotgun_reload),
        "fusebox-07000014" => ("Fusebox sabotage (07000014, crouched in the vent)", scenarios::fusebox_07000014),
        "searchlight-tracks-bond" => ("Searchlight tracks Bond (07000006)", scenarios::searchlight_tracks_bond),
        _ => return None,
    };
    Some((n, f))
}

/// The list of scenario names (for `--dev` start and `--help`). B-8/B-9/B-10: sniper-cover, bullet-flyby,
/// friendly-actor, grenade-throw and grenade-bounce-explode are removed until their tests pass (see
/// `scenario()` comment). searchlight-tracks-bond is back (G-036 closed as a misdiagnosis; run it on
/// 07000006, whose four heads all carry linked spots).
pub const SCENARIOS: &[&str] = &[
    "ambush", "guard-falls-off-ledge",
    "door-anti-crush", "cutscene-ai-pause",
    "ghostly-white", "door-textures", "invisible-walls", "level-exit",
    "weapon-switch", "sg5-pickup", "wire-music", "autoaim", "sniper-scope", "shotgun-reload",
    "fusebox-07000014", "searchlight-tracks-bond",
];

mod scenarios {
    use super::*;
    use nf_game::player::Bond;

    /// Find a spot for Bond near `guard_pos` at distance `dist` (clamped to [2, 30]) with a clear line of sight to
    /// the guard. Spiral search: 24 angles at 15° intervals (0°..345°), trying the requested distance first then
    /// fall-back distances in [8, 20] m. Each candidate must have a collision floor within 4 m of the guard's y and
    /// a clear LOS from Bond's eye (pos.y + 1.0) to the guard's head (pos.y + 1.5). Returns (bond_pos, yaw) where yaw
    /// faces Bond toward the guard. (B-3a/b.)
    fn spiral_search_for_spot(world: &nf_game::world::CollisionWorld, guard_pos: [f32; 3], dist: f32) -> Option<([f32; 3], f32)> {
        // The requested distance first (it carries the scenario's intent — close for door scenarios, far for sniper
        // scenarios), then fall-back distances in the 8-20 m grenade-throw band so the search always tries that
        // range even when the requested distance is outside it.
        let mut distances: Vec<f32> = vec![dist.clamp(2.0, 30.0)];
        for d in [8.0_f32, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0] {
            if (d - dist).abs() > 0.5 { distances.push(d); }
        }
        let guard_head = [guard_pos[0], guard_pos[1] + 1.5, guard_pos[2]];
        for &d in &distances {
            for k in 0..24u32 {
                // 0° = +z (the original placement direction); angle increases counter-clockwise.
                let angle = std::f32::consts::TAU * k as f32 / 24.0;
                let cx = guard_pos[0] + d * angle.sin();
                let cz = guard_pos[2] + d * angle.cos();
                // Floor below the candidate, starting 0.5 above the guard's y so floors on the guard's level win.
                // B-10: `continue` on no floor (not `?` — the old code aborted the WHOLE spiral on the first
                // candidate without a floor, so every guard failed placement on 07000005).
                let Some(floor_y) = world.floor_below([cx, guard_pos[1], cz], 4.0) else { continue };
                // Reject spots whose floor is far above or below the guard (a different level / a pit).
                if (floor_y - guard_pos[1]).abs() > 3.0 { continue; }
                let bond_pos = [cx, floor_y + 1.0, cz];
                let bond_eye = [bond_pos[0], bond_pos[1] + 1.0, bond_pos[2]];
                if !world.line_of_sight(bond_eye, guard_head) { continue; }
                let dx = guard_pos[0] - bond_pos[0];
                let dz = guard_pos[2] - bond_pos[2];
                let yaw = dx.atan2(dz);
                return Some((bond_pos, yaw));
            }
        }
        None
    }

    /// The game's active-guard definition, mirrored from the pub `is_active_guard` but with the channels state
    /// captured pre-tick (the per-drone `channel_on` cache is only set on tick, so we check the live channels —
    /// the scenario turns them on first). Checks present + enabled + alive + drawn + in-llist + channel.
    fn is_active(x: &nf_game::drone::Drone, i: usize, llist: &[usize], chans: &nf_game::triggers::Channels) -> bool {
        x.present && x.enabled && !x.corpse && x.health > 0.0 && x.drawn
            && llist.contains(&i)
            && (x.switch_channel == 0 || chans.on(x.switch_channel as u32))
    }

    /// Log the chosen guard's activation status — placement index, present / started / enabled / held-by-spawner /
    /// in-llist / drawn — so the user can see WHY a guard was (or wasn't) picked, and verify it is active in play.
    fn log_guard_diagnostics(sim: &crate::scene::Sim, gi: usize) {
        let dr = &sim.drones.list[gi];
        let in_llist = sim.drones.llist.contains(&gi);
        let held_by_spawner = sim.drones.spawners.iter().any(|sp| sp.templates.contains(&gi));
        eprintln!("[scenario] guard #{}: placement={} present={} started={} enabled={} alive={} drawn={} in_llist={} held_by_spawner={} | team={} mode={} anim_set=0x{:x} state=0x{:x} health={:.0} pos=({:.1},{:.1},{:.1})",
            gi, dr.placement, dr.present, dr.started, dr.enabled, !dr.corpse && dr.health > 0.0,
            dr.drawn, in_llist, held_by_spawner, dr.team, dr.mode, dr.anim_set, dr.state, dr.health,
            dr.pos[0], dr.pos[1], dr.pos[2]);
    }

    /// (removed B-10) `turn_on_all_channels` was here — it started scripts (the bridge cut-scene on 07000005),
    /// so scenarios no longer turn on every channel. Only guards already active in play are picked.

    /// How many of the 4 cardinal directions (N/S/E/W) Bond can walk `dist` metres from `pos` without hitting a
    /// wall (a chest-height segment test). The user's rule: the spot must connect to the rest of the level — at least
    /// two clear directions, so Bond isn't sealed in a pocket. B-6.
    fn walkable_directions(world: &nf_game::world::CollisionWorld, pos: [f32; 3], dist: f32) -> usize {
        let eye = [pos[0], pos[1] + 1.0, pos[2]];
        let mut n = 0;
        for [dx, dz] in [[1.0f32, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]] {
            if world.segment(eye, [eye[0] + dx * dist, eye[1], eye[2] + dz * dist]).is_none() { n += 1; }
        }
        n
    }

    /// Whether `a` and `b` are on the same AI path network (so the guard's nav queries can route between them).
    /// A guard whose position is off the nav network cannot path to Bond — the scenario would stall. B-6.
    fn same_nav_path(nav: &nf_game::nav::Network, a: [f32; 3], b: [f32; 3]) -> bool {
        matches!((nav.nav_path_for(a), nav.nav_path_for(b)), (Some(pa), Some(pb)) if pa == pb)
    }

    /// Place Bond near an ACTIVE guard (present + enabled + alive + in-llist), at `dist` metres with a clear LOS to
    /// the guard. Spiral search: 24 angles × multiple distances in [8, 20] m. Each candidate must have:
    ///  - a collision floor within 3 m of the guard's y (same level, not a pit);
    ///  - a clear LOS from Bond's eye to the guard's head;
    ///  - at least 2 walkable cardinal directions (Bond is not sealed in a pocket) [B-6];
    ///  - the guard and Bond on the same AI path network (the guard can path to Bond) [B-6].
    /// Iterates over ALL active guards; the first one with a valid spot wins. Logs the chosen guard's activation
    /// diagnostics. Returns the chosen guard's drone index, or None. (B-3a/b + B-6.)
    fn place_bond_near_guard(scene: &mut Scene, dist: f32) -> Option<usize> {
        let sim = scene.sim.as_mut()?;
        // B-10: do NOT turn on all switch channels — it starts scripts (the bridge cut-scene on 07000005 fired
        // because of this). Only consider guards that are ALREADY active in play: present + enabled + alive +
        // drawn + in-llist, and either no switch_channel or its channel already on at level start. The scenario
        // finds guards the player would actually encounter, not switch-gated ones the level hasn't released yet.
        let chans = sim.drones.channels.clone();
        let llist = sim.drones.llist.clone();
        let candidates: Vec<(usize, [f32; 3])> = sim.drones.list.iter().enumerate()
            .filter(|(i, x)| is_active(x, *i, &llist, &chans))
            .map(|(i, x)| (i, x.pos))
            .collect();
        if candidates.is_empty() {
            eprintln!("[scenario] ERROR: no ACTIVE guard on this level (present+enabled+alive+drawn+in-llist). \
                {} drones total, {} present, {} enabled, {} drawn, {} in-llist.",
                sim.drones.list.len(),
                sim.drones.list.iter().filter(|d| d.present).count(),
                sim.drones.list.iter().filter(|d| d.present && d.enabled).count(),
                sim.drones.list.iter().filter(|d| d.present && d.drawn).count(),
                sim.drones.llist.len());
            return None;
        }
        let mut found: Option<(usize, [f32; 3], [f32; 3], f32)> = None;
        for (gi, g) in &candidates {
            if let Some((bond_pos, yaw)) = spiral_search_for_spot(&sim.world, *g, dist) {
                // B-6: walkability (≥2 clear directions) is a hard requirement (Bond must not be sealed in).
                if walkable_directions(&sim.world, bond_pos, 5.0) < 2 {
                    eprintln!("[scenario] guard #{} spot {:?} rejected: sealed (walkable dirs < 2).", gi, bond_pos);
                    continue;
                }
                // B-10: the same-nav-path check is now a WARNING, not a hard reject — the guard's runtime
                // pathfinder may still route even when the static `nav_path_for` lookup disagrees (the port's
                // nav network is a stand-in). Rejecting on it made every 07000005 guard fail placement.
                if !same_nav_path(&sim.nav, *g, bond_pos) {
                    eprintln!("[scenario] guard #{} spot {:?}: WARNING — guard and Bond not on the same nav path (the guard may not pathfind to Bond).", gi, bond_pos);
                }
                found = Some((*gi, *g, bond_pos, yaw));
                break;
            }
        }
        let Some((gi, g, bond_pos, yaw)) = found else {
            eprintln!("[scenario] ERROR: no valid placement for any active guard (no LOS, no floor, sealed spot, \
                or guard off the nav network). Tried {} guards × 24 angles × 8 distances.",
                candidates.len());
            return None;
        };
        let d = ((bond_pos[0] - g[0]).powi(2) + (bond_pos[1] - g[1]).powi(2) + (bond_pos[2] - g[2]).powi(2)).sqrt();
        sim.bond = Bond::stand_at(&sim.world, bond_pos, yaw);
        // B-10: face the guard toward Bond so its vision cone catches Bond immediately (the game's `DroneFunc_Alert`
        // turns a guard that hears/sees Bond; the scenario places Bond in plain view, so the guard should be facing
        // him). Without this, the guard's spawn yaw may face away and it never sees Bond within the test window.
        let guard_yaw = (bond_pos[0] - g[0]).atan2(bond_pos[2] - g[2]);
        sim.drones.list[gi].yaw = guard_yaw;
        sim.drones.list[gi].desired_yaw = guard_yaw;
        eprintln!("[scenario] Bond at ({:.1}, {:.1}, {:.1}) facing yaw {:.0}°, guard #{} at ({:.1}, {:.1}, {:.1}), distance {:.1} m, walkable dirs {}, guard faces Bond (yaw {:.0}°)",
            bond_pos[0], bond_pos[1], bond_pos[2], yaw.to_degrees(), gi, g[0], g[1], g[2], d,
            walkable_directions(&sim.world, bond_pos, 5.0), guard_yaw.to_degrees());
        log_guard_diagnostics(sim, gi);
        Some(gi)
    }

    /// Place Bond near an ACTIVE guard that carries grenades. Same spiral search + walkability + nav-path checks as
    /// `place_bond_near_guard`, but iterates only over active grenade-carrying guards. On levels with no grenade
    /// guard, prints the levels that have them and returns None. (B-3a/b + B-6.)
    fn place_bond_near_grenade_guard(scene: &mut Scene, dist: f32) -> Option<usize> {
        let sim = scene.sim.as_mut()?;
        let chans = sim.drones.channels.clone();
        let llist = sim.drones.llist.clone();
        let candidates: Vec<(usize, [f32; 3])> = sim.drones.list.iter().enumerate()
            .filter(|(i, x)| is_active(x, *i, &llist, &chans) && x.grenade_count > 0)
            .map(|(i, x)| (i, x.pos))
            .collect();
        if candidates.is_empty() {
            eprintln!("[scenario] ERROR: no ACTIVE guard on this level carries grenades (param 24 = 0).");
            eprintln!("[scenario]   active guards: {}, of which grenade-carrying: 0.", sim.drones.list.iter().enumerate().filter(|(i, x)| is_active(x, *i, &llist, &chans)).count());
            eprintln!("[scenario] Grenade-carrying guards are on levels 07000002, 07000003, 07000009, 0700000a — but those guards are switch-gated (turning on all channels starts scripts). 07000005 has active guards at start; the grenade-throw scenario gives one grenades via the param-24 path.");
            eprintln!("[scenario] Run: nf-view <iso> 07000005 --dev --scenario grenade-throw");
            return None;
        }
        let mut found: Option<(usize, [f32; 3], [f32; 3], f32)> = None;
        for (gi, g) in &candidates {
            if let Some((bond_pos, yaw)) = spiral_search_for_spot(&sim.world, *g, dist) {
                if walkable_directions(&sim.world, bond_pos, 5.0) < 2 {
                    eprintln!("[scenario] grenade guard #{} spot {:?} rejected: sealed (walkable dirs < 2).", gi, bond_pos);
                    continue;
                }
                if !same_nav_path(&sim.nav, *g, bond_pos) {
                    eprintln!("[scenario] grenade guard #{} spot {:?}: WARNING — not on the same nav path (may not pathfind).", gi, bond_pos);
                }
                found = Some((*gi, *g, bond_pos, yaw));
                break;
            }
        }
        let Some((gi, g, bond_pos, yaw)) = found else {
            eprintln!("[scenario] ERROR: no valid placement for any active grenade guard (no LOS, no floor, sealed \
                spot, or guard off the nav network). Tried {} guards.", candidates.len());
            return None;
        };
        let d = ((bond_pos[0] - g[0]).powi(2) + (bond_pos[1] - g[1]).powi(2) + (bond_pos[2] - g[2]).powi(2)).sqrt();
        let (grenade_count, grenade_weapon) = {
            let dr = &sim.drones.list[gi];
            (dr.grenade_count, dr.grenade_weapon)
        };
        sim.bond = Bond::stand_at(&sim.world, bond_pos, yaw);
        eprintln!("[scenario] Bond at ({:.1}, {:.1}, {:.1}) facing yaw {:.0}°, grenade guard #{} at ({:.1}, {:.1}, {:.1}), distance {:.1} m, walkable dirs {}",
            bond_pos[0], bond_pos[1], bond_pos[2], yaw.to_degrees(), gi, g[0], g[1], g[2], d,
            walkable_directions(&sim.world, bond_pos, 5.0));
        eprintln!("[scenario] Guard has {} grenades (weapon {:#x}).", grenade_count, grenade_weapon);
        log_guard_diagnostics(sim, gi);
        Some(gi)
    }

    #[allow(dead_code)] // B-10: removed from registry — guard enters RUN_FOR_COVER instead of throwing. Needs cover-aware placement.
    pub fn grenade_throw(scene: &mut Scene) {
        // B-6: the natively-grenade-carrying guards on 07000002 (placement 464/466) are mode 7 / Hostage and in
        // enclosed spots — no walkable placement exists near them. Instead, pick any ACTIVE accessible guard and give
        // it grenades (the same `NDrone2_DefaultInit` param-24 path: grenade_count + grenade_weapon). The guard then
        // throws from its OWN decision (can_throw_grenade + throw_grenade), not a forced spawn.
        let gi = place_bond_near_guard(scene, 12.0);
        if let Some(gi) = gi {
            if let Some(sim) = scene.sim.as_mut() {
                sim.drones.list[gi].grenade_count = 3;
                sim.drones.list[gi].grenade_weapon = 0x36; // stun (DroneWeap_ThrowGrenade's 0x36)
                // B-10: set alert to max + seen so the guard goes straight to ATTACK/AIM_STAND_FIRE instead of
                // RunForCover (the guard sees Bond immediately and engages, reaching the throw decision).
                sim.drones.list[gi].alert = 1.0;
                sim.drones.list[gi].seen = true;
                sim.drones.list[gi].ever_seen = true;
                sim.drones.list[gi].visibility = 1.0;
            }
            eprintln!("[scenario] grenade-throw: Bond 12 m from an active guard given 3 stun grenades (param-24 path), alert maxed. Watch for the lob (anim 0x49) within ~10 s; F1 shows the state.");
        }
    }
    #[allow(dead_code)] // B-10: removed from registry — same as grenade-throw.
    pub fn grenade_bounce_explode(scene: &mut Scene) {
        // B-7: 12 m, not 15 — the guard's combat state machine throws at 12 m (proven by grenade-throw's test) but
        // at 15 m it takes a different attack path and never throws within 600 ticks. can_throw_grenade allows
        // 8–20 m, but the guard's AIM_STAND_FIRE / StepAim decision picks shooting over throwing at the longer end.
        let gi = place_bond_near_guard(scene, 12.0);
        if let Some(gi) = gi {
            if let Some(sim) = scene.sim.as_mut() {
                sim.drones.list[gi].grenade_count = 3;
                sim.drones.list[gi].grenade_weapon = 0x36;
                sim.drones.list[gi].alert = 1.0;
                sim.drones.list[gi].seen = true;
                sim.drones.list[gi].ever_seen = true;
                sim.drones.list[gi].visibility = 1.0;
            }
            eprintln!("[scenario] grenade-bounce-explode: Bond 12 m from an active guard given 3 stun grenades. The grenade should arc, bounce (sound 0x51C) and explode (sound 0x4A7). F1 shows the arc + explosion radius.");
        }
    }
    #[allow(dead_code)] // B-8: removed from registry until its test passes
    pub fn sniper_cover(scene: &mut Scene) {
        let gi = place_bond_near_guard(scene, 18.0);
        if gi.is_some() {
            if let Some(sim) = scene.sim.as_mut() { sim.drones.list[gi.unwrap()].anim_set = 0x13; }
            eprintln!("[scenario] sniper-cover: the guard's anim set is 0x13 (sniper). When it reaches cover it should pop up once a second (state 0x97), duck after 6 s (0x99), reload when empty (0x98). F1 shows the state.");
        }
    }
    pub fn ambush(scene: &mut Scene) {
        let gi = place_bond_near_guard(scene, 8.0);
        if gi.is_some() {
            if let Some(sim) = scene.sim.as_mut() { sim.drones.list[gi.unwrap()].mode = 0x10; sim.drones.list[gi.unwrap()].alert = 1.0; }
            eprintln!("[scenario] ambush: the guard is in mode 0x10 (ambush). It should crouch behind cover (state 0x37 -> 0x38) and attack when Bond is within 3 m (0x56). F1 shows the state.");
        }
    }
    pub fn guard_falls_off_ledge(scene: &mut Scene) {
        let gi = place_bond_near_guard(scene, 20.0);
        if gi.is_some() {
            if let Some(sim) = scene.sim.as_mut() { sim.drones.list[gi.unwrap()].pos[1] += 3.0; sim.drones.list[gi.unwrap()].velocity[1] = 0.0; }
            eprintln!("[scenario] guard-falls-off-ledge: the guard is lifted 3 m. It should fall under gravity (the Drone_CollisionHandler down-ray) and land on the floor. F1 shows in_air / on_surface.");
        }
    }
    pub fn searchlight_tracks_bond(scene: &mut Scene) {
        // fix-searchlight-remainder: re-added. G-036's "the port is broken (1/7 heads links to a spot; beam
        // points up)" was a misdiagnosis (docs/atlas/searchlight.md): 07000005's placement data simply has
        // 2 spots for 7 heads (the level's own design — 5 heads can never arm, in the game either) and the
        // mid-6.11 head genuinely sweeps ≈10° above horizontal along +x. 07000006's four heads all carry
        // linked spots (placements 489-492, created after their heads) — run it there.
        let mut placed = false;
        if let Some(sim) = scene.sim.as_mut() {
            // B-10: do NOT turn on all channels (it starts scripts). Searchlights with an on-channel that's
            // already on at level start will sweep; others stay off (as they do in normal play before the
            // player triggers them).
            // Bond starts where he is (the scenario moves him after the spot populates). Tick a few frames to
            // transition heads from state 3 (off) → 0 (sweep) via their on-channel, and populate each head's spot
            // (the beam's ground hit). This is a normal game tick (no extra RNG draws).
            for _ in 0..8 { sim.step_bond(Default::default(), false, false, false); }
            // Now find a head whose spot has populated (the beam hits the ground). Place Bond at that spot so the
            // beam catches him immediately (the 0→1 tracking transition needs Bond within 2 m of the spot).
            let chosen: Option<(usize, [f32; 3], [f32; 3])> = { // (head_idx, head_pos, spot_pos)
                let parts = &sim.drones.searchlights.parts;
                let mut hist = [0u32; 5];
                for p in parts { if (p.kind as usize) < 5 { hist[p.kind as usize] += 1; } }
                eprintln!("[scenario] searchlights: {} parts (mounts={}, heads={}, spots={}, beams={}, coronas={})",
                    parts.len(), hist[0], hist[1], hist[2], hist[3], hist[4]);
                let mut chosen: Option<(usize, [f32; 3], [f32; 3])> = None;
                for (j, h) in parts.iter().enumerate().filter(|(_, p)| p.kind == 1) {
                    let Some(sp_idx) = h.links[2] else {
                        eprintln!("[scenario] head #{j} at ({:.1},{:.1},{:.1}) state={} — NO linked spot (cannot track)", h.pos[0], h.pos[1], h.pos[2], h.state);
                        continue;
                    };
                    let sp = &parts[sp_idx];
                    if sp.pos[0] <= -1000.0 {
                        eprintln!("[scenario] head #{j} at ({:.1},{:.1},{:.1}) state={} — spot at −10000 (beam misses)", h.pos[0], h.pos[1], h.pos[2], h.state);
                        continue;
                    }
                    eprintln!("[scenario] head #{j} at ({:.1},{:.1},{:.1}) state={} mid={:.2} → spot at ({:.1},{:.1},{:.1})",
                        h.pos[0], h.pos[1], h.pos[2], h.state, h.mid, sp.pos[0], sp.pos[1], sp.pos[2]);
                    // Prefer the head whose spot is lowest (closest to the ground — the beam points down).
                    if chosen.is_none() || sp.pos[1] < chosen.unwrap().2[1] {
                        chosen = Some((j, h.pos, sp.pos));
                    }
                }
                chosen
            };
            if let Some((head_idx, hpos, sp)) = chosen {
                // Place Bond on the ground at the spot, facing the head.
                if let Some(sfy) = sim.world.floor_below(sp, 4.0) {
                    let face_yaw = (hpos[0] - sp[0]).atan2(hpos[2] - sp[2]);
                    sim.bond = Bond::stand_at(&sim.world, [sp[0], sfy, sp[2]], face_yaw);
                    placed = true;
                    eprintln!("[scenario] searchlight-tracks-bond: Bond at the spot ({:.1}, {:.1}, {:.1}) of head #{head_idx} at ({:.1},{:.1},{:.1}). The beam is on him — it should track (state 1, sound 0xE5 relay). Move and watch the beam follow.",
                        sp[0], sfy, sp[2], hpos[0], hpos[1], hpos[2]);
                } else {
                    eprintln!("[scenario] head #{head_idx} spot at {sp:?} but no floor 4 m below — can't place Bond there.");
                }
            } else {
                eprintln!("[scenario] no searchlight head has a populated spot (every beam misses the ground). Searchlights may be unported or placed wrong.");
            }
        }
        if !placed {
            eprintln!("[scenario] ERROR: no searchlight head (or no ground spot) on this level. Run it on 07000006: all four courtyard heads have linked spots.");
            eprintln!("[scenario] Run: nf-view <iso> 07000006 --dev --scenario searchlight-tracks-bond");
        }
    }
    pub fn door_anti_crush(scene: &mut Scene) {
        let gi = place_bond_near_guard(scene, 5.0);
        if gi.is_some() {
            let has_doors = scene.sim.as_ref().is_some_and(|s| s.drones.doors.list.iter().any(|d| d.glist != 0));
            if !has_doors {
                eprintln!("[scenario] ERROR: no doors on this level. Doors are on 07000005 (WORKLOG 0074).");
                eprintln!("[scenario] Run: nf-view <iso> 07000005 --dev --scenario door-anti-crush");
            } else {
                eprintln!("[scenario] door-anti-crush: Bond near a door. Open it (E), then stand in the doorway -- it should reverse (anti-crush, sound 284). Watch the door.");
            }
        }
    }
    /// B-12: cutscene-ai-pause. Places Bond on the gatehouse wall top (100, 26, 108) — the main agent confirmed
    /// this position triggers the gatehouse cut-scene headless (`nf-view <iso> 07000005 --bond 100,26,108 --frame
    /// 1200 --shot`). The cut-scene fires within ~1 s, runs ~15-20 s, then hands control back (PlayerEnable). An
    /// active guard is placed nearby so the user can see it stands still during the cut-scene (the B-10/B-11 fix:
    /// `Drones::paused` skips the per-drone update while `Sim::disabled`).
    pub fn cutscene_ai_pause(scene: &mut Scene) {
        if let Some(sim) = scene.sim.as_mut() {
            // B-12: the gatehouse plate is at y≈27.05; placing Bond at (100, 26, 108) puts him on the plate.
            // Use Bond::new (not stand_at) to match the main agent's `--bond 100,26,108` path — Bond starts at
            // the exact position and the cut-scene trigger fires within ~1 s of ticking.
            let pos = [100.0, 28.0, 108.0]; // y=28: above the plate (y≈27.05), so Bond falls onto it and triggers the cut-scene
            sim.bond = nf_game::player::Bond::new(pos, sim.bond.yaw);
            eprintln!("[scenario] cutscene-ai-pause: Bond on the gatehouse plate ({:.1}, {:.1}, {:.1}). The cut-scene fires within ~1 s. Guards must NOT fire during it (the AI is paused via Drones::paused). Control returns after ~15-20 s. Check the nearest guard stands still during the cut-scene.",
                pos[0], pos[1], pos[2]);
        }
        // B-12: do NOT call place_bond_near_guard — it would move Bond off the plate and the cut-scene wouldn't fire.
        // The existing guards on the level are the ones the user checks (they're within ~40 m of the plate).
    }
    #[allow(dead_code)] // B-8: removed from registry until its test passes
    pub fn bullet_flyby(scene: &mut Scene) {
        let gi = place_bond_near_guard(scene, 14.0);
        if gi.is_some() {
            if let Some(sim) = scene.sim.as_mut() { for dr in sim.drones.list.iter_mut().filter(|x| x.present) { dr.accuracy = 0; } }
            eprintln!("[scenario] bullet-flyby: a guard fires at Bond from 14 m. A near miss should play the fly-by whiz (0x1dc) when the bullet passes within 3 units. Strafe to make misses pass close.");
        }
    }
    #[allow(dead_code)] // B-8: removed from registry until its test passes
    pub fn friendly_actor(scene: &mut Scene) {
        let gi = place_bond_near_guard(scene, 6.0);
        if gi.is_some() {
            if let Some(sim) = scene.sim.as_mut() { sim.drones.list[gi.unwrap()].team = 3; }
            eprintln!("[scenario] friendly-actor: the guard is on team 3 (friendly). DroneVision_EnemyLookForOpponent (0x177ed0) returns 0 for team 3, so it should not attack Bond -- walk up to it. F1 shows the state stays Idle/Patrol.");
        }
    }
    pub fn ghostly_white(scene: &mut Scene) {
        let _ = place_bond_near_guard(scene, 10.0);
        eprintln!("[scenario] ghostly-white: the non-fogged clear colour is now the game's squeeze-gray. Toggle --no-fog and --no-tex to isolate fog/texture/vertex colour.");
    }
    pub fn door_textures(scene: &mut Scene) {
        let _ = place_bond_near_guard(scene, 3.0);
        eprintln!("[scenario] door-textures: Bond near a door. Open it (E) and watch the door's texture as it opens/closes.");
    }
    pub fn invisible_walls(scene: &mut Scene) {
        let _ = place_bond_near_guard(scene, 10.0);
        eprintln!("[scenario] invisible-walls: Bond near the guard. Press C to name what you're touching. Run with NF_COLL=1 to see collision-only objects.");
    }
    pub fn level_exit(scene: &mut Scene) {
        let _ = place_bond_near_guard(scene, 2.0);
        eprintln!("[scenario] level-exit: Bond near the guard. When he touches the 0xE8 trigger, Music_Event(3,1) fires and the next level loads.");
    }

    // ---- the five scenarios merged from main.rs::apply_scenario (B-3a/b) ------------------------------------
    // Bodies are ported whole; only `sim.foo` is replaced with `if let Some(sim) = scene.sim.as_mut() { sim.foo }`
    // so they take `&mut Scene` like the other scenarios.

    /// Bond at the level start with the full owned arsenal: P2K (2), SG5 (0x16), sniper (0x24), gadgets. The SG5 and
    /// sniper are given owned + ammo so the weapon cycle finds them. (B-3a/b.)
    pub fn weapon_switch(scene: &mut Scene) {
        if let Some(sim) = scene.sim.as_mut() {
            let wi = sim.weapons.records.len();
            // The SG5 and sniper are given owned + ammo so the weapon cycle finds them.
            if wi > 0x16 { sim.drones.inventory.owned[0x16] = true; sim.drones.inventory.clip[0x16] = 30; sim.drones.inventory.carried[5] = 60; }
            if wi > 0x24 { sim.drones.inventory.owned[0x24] = true; sim.drones.inventory.clip[0x24] = 5; sim.drones.inventory.carried[7] = 25; }
        }
        eprintln!("[dev] weapon-switch: scroll the mouse wheel (or 1-3) to cycle WEAPONS (P2K/SG5/sniper). Gadgets are disabled (not ported yet). Watch the holster/draw animation and listen for the sounds. Set NF_SOUNDLOG=1 to see each animation id.");
    }

    /// A dropped SG5 at Bond's feet. Walk forward to pick it up. (B-3a/b.)
    pub fn sg5_pickup(scene: &mut Scene) {
        if let Some(sim) = scene.sim.as_mut() {
            let wi = sim.weapons.records.len();
            if wi > 0x16 {
                let p = sim.bond.pos;
                let m: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [p[0] + 1.0, p[1], p[2] + 1.5, 1.0]];
                if let Some(mut pickup) = nf_game::pickup::Pickup::create_simple(&m, 0x16, 12, nf_game::pickup::kind::WEAPON, &sim.weapons, &sim.models) {
                    pickup.state = 1; // already on the floor
                    sim.drones.pickups.push(pickup);
                }
            }
        }
        eprintln!("[dev] sg5-pickup: a dropped SG5 Commando is at Bond's feet — walk forward to pick it up, then scroll to switch to it");
    }

    /// Bond near the zip wire. Walk in and out — the music section changes (event 0xD).
    pub fn wire_music(scene: &mut Scene) {
        if let Some(sim) = scene.sim.as_mut() {
            if let Some(w) = sim.wires.first() {
                // Search a spiral around the wire's endpoint A for a walkable spot: floor found within 10
                // units of the wire's Y (stay on the same platform, not the cliff base 80 units below),
                // and at least 2 walkable cardinal directions (not a 1-tile ledge or a sealed pocket).
                // Avoid trigger volumes — the wire area on 07000005 has touch triggers that start cut-scenes.
                let (a, _) = w.grab_segment();
                let mut best: Option<([f32; 3], usize)> = None;
                for r in [2.0, 5.0, 8.0, 12.0, 20.0] {
                    for angle in 0..24 {
                        let theta = angle as f32 * std::f32::consts::TAU / 24.0;
                        let px = a[0] + r * theta.cos();
                        let pz = a[2] + r * theta.sin();
                        if let Some(floor_y) = sim.world.floor_below([px, a[1] + 1.0, pz], 10.0) {
                            let pos = [px, floor_y + 1.1, pz];
                            let dirs = walkable_directions(&sim.world, pos, 5.0);
                            if dirs >= 2 {
                                best = Some((pos, dirs));
                                break;
                            }
                            if best.is_none() && dirs >= 1 {
                                best = Some((pos, dirs));
                            }
                        }
                    }
                    if best.is_some_and(|(_, d)| d >= 2) { break; }
                }
                if let Some((pos, _)) = best {
                    // Face Bond toward the wire.
                    let dx = a[0] - pos[0]; let dz = a[2] - pos[2];
                    let yaw = dx.atan2(dz);
                    sim.bond = nf_game::player::Bond::new(pos, yaw);
                } else {
                    eprintln!("[dev] wire-music: no walkable spot found near the wire");
                }
            }
        }
        eprintln!("[dev] wire-music: Bond is near the zip wire — walk toward it; when close the music changes to the stealth section, walk away and it returns");
    }

    /// A guard in view. T toggles auto-aim — the crosshair snaps to the guard. (B-3a/b.)
    pub fn autoaim(scene: &mut Scene) {
        if let Some(sim) = scene.sim.as_mut() {
            sim.auto_aim = false; // off by default (D-001)
            // Find the nearest alive guard and face Bond toward it.
            if let Some((i, dr)) = sim.drones.list.iter().enumerate().filter(|(_, d)| d.present && d.health > 0.0).min_by_key(|(_, d)| {
                let dx = d.pos[0] - sim.bond.pos[0]; let dz = d.pos[2] - sim.bond.pos[2];
                (dx * dx + dz * dz) as u64
            }) {
                let dx = dr.pos[0] - sim.bond.pos[0]; let dz = dr.pos[2] - sim.bond.pos[2];
                sim.bond.yaw = dx.atan2(dz);
                eprintln!("[dev] autoaim: guard #{i} is in view — press T to toggle auto-aim (the crosshair snaps to the guard when on)");
            } else {
                eprintln!("[dev] autoaim: no guard found — press T to toggle auto-aim");
            }
            return;
        }
        eprintln!("[dev] autoaim: no sim available.");
    }

    /// Give Bond the sniper (0x24) and select it. Hold right mouse to zoom. (B-3a/b.)
    pub fn sniper_scope(scene: &mut Scene) {
        if let Some(sim) = scene.sim.as_mut() {
            let wi = sim.weapons.records.len();
            if wi > 0x24 {
                sim.drones.inventory.owned[0x24] = true;
                sim.drones.inventory.clip[0x24] = 5;
                sim.drones.inventory.carried[7] = 25;
                sim.gun.weapon = 0x24;
                sim.gun.switch_request = 0x24;
                sim.view_script = sim.weapons.idle_script(0x24);
                sim.view_time = 0.0;
                sim.view_speed = 1.0;
            }
        }
        eprintln!("[dev] sniper-scope: Bond has the sniper rifle. Hold RIGHT MOUSE to zoom — the FOV narrows and the sensitivity drops. Release to return to normal.");
    }

    /// Give Bond the SG5 Commando (0x16) with an empty clip and spare ammo. Press R to reload.
    /// The SG5 is in 07000005's level archive (its FP skin 0x050000c0 is present). The actual shotguns
    /// (0x1c/0x1d) are not in 07000005's archive, so they can't be tested here. The SG5 loads all rounds
    /// at once (not shell-by-shell); the shell-by-shell loop can only be tested on a level that has the
    /// shotgun's FP skin.
    pub fn shotgun_reload(scene: &mut Scene) {
        if let Some(sim) = scene.sim.as_mut() {
            let wi = sim.weapons.records.len();
            if wi > 0x16 {
                sim.drones.inventory.owned[0x16] = true;
                sim.drones.inventory.clip[0x16] = 0; // empty clip
                sim.drones.inventory.carried[sim.weapons.ammo_type(0x16)] = 60; // spare ammo
                sim.gun.weapon = 0x16;
                sim.gun.switch_request = 0x16;
                sim.view_script = sim.weapons.idle_script(0x16);
                sim.view_time = 0.0;
                sim.view_speed = 1.0;
            }
        }
        eprintln!("[dev] shotgun-reload: Bond has the SG5 Commando (0x16) with an empty clip. Press R to reload (loads all rounds at once — the shell-by-shell loop needs a shotgun 0x1c which is not in this level's archive).");
    }

    /// fix-wire2 — the 07000014 fusebox sabotage demo. The user's playtest: the fusebox spot spawned
    /// him ABOVE the vent he was supposed to be in. This scenario starts Bond CROUCHED INSIDE the
    /// level's crawl vent — the [D]-proven crawl tunnel system at x 31-41 (headroom 1.44-1.46 over
    /// floor 2.15-2.40; the cold start (40.81, 2.40, 65.08) is the tunnel's north end). The spot
    /// (41.0, 2.45, 62.0) is inside the north run's 1.46 headroom [D probe_vent2]. From there the user
    /// crawls south (hold C) to the vent drop (TouchOnce #301 at 39.68..41.75, 2.07..4.14,
    /// 37.62..40.40 [D]) and plays the chain to the sabotage. The Laser (0x4E — the disc's cutting
    /// weapon, bullet category 0x0002 [L weapon_data; M Collide_FilterBullets 0x1ece30]) is owned,
    /// loaded and selected so the wires can be burned on arrival (fix-fusebox-cut's cut path).
    pub fn fusebox_07000014(scene: &mut Scene) {
        let Some(sim) = scene.sim.as_mut() else { return };
        if sim.level != 0x0700_0014 {
            eprintln!("[scenario] fusebox-07000014: this scenario is 07000014-only (the fusebox sabotage level).");
            eprintln!("[scenario] Run: nf-view <iso> 07000014 --dev --scenario fusebox-07000014");
            return;
        }
        let pos = [41.0, 2.45, 62.0];
        sim.bond = nf_game::player::Bond::new(pos, std::f32::consts::PI);
        // Start CROUCHED (the crawl): `Player_HandleJump`'s crouch set — `crouched`, cc = the 0.55
        // crouch root height [M 0x1924c0; the port's CROUCH_ROOT_HEIGHT]. crouch_drop eases in on tick.
        sim.bond.crouched = true;
        sim.bond.cc = nf_game::player::CROUCH_ROOT_HEIGHT;
        let wi = sim.weapons.records.len();
        if wi > 0x4e {
            sim.drones.inventory.owned[0x4e] = true;
            sim.drones.inventory.clip[0x4e] = 100;
            sim.gun.weapon = 0x4e;
            sim.gun.switch_request = 0x4e;
            sim.view_script = sim.weapons.idle_script(0x4e);
            sim.view_time = 0.0;
            sim.view_speed = 1.0;
        }
        eprintln!("[scenario] fusebox-07000014: Bond crouched INSIDE the vent at (41.0, 2.45, 62.0), facing the drop (south). Hold C to stay crouched and W to crawl south to the vent drop; the chain then runs to the generator room. The Laser (0x4E) is out — burn the fusebox wires with left mouse.");
    }
}
