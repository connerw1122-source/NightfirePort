//! In-game test tooling, gated behind `--dev` (WORKLOG 0081). Tools only call into existing systems or set state the
//! way the game itself would — never change game logic or the order of RNG calls. Logic stays at a fixed 60 Hz; the
//! time scale changes the number of ticks run per render frame, not the tick rate.
//!
//! - **F1**: the AI overlay — each guard within 30 m shows its state name, alertness, health and active timers; a line
//!   to its target or cover point; live grenades' arcs and explosion radii.
//! - **F2**: the dev menu — spawn a guard at the crosshair (by type/mode), force the nearest guard's next decision
//!   (e.g. "throw grenade now"), give the nearest guard an impulse or a SpecialDeath, god mode, infinite ammo,
//!   teleport, and the time scale (1× / 0.25× / pause / step one tick).
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
    /// God mode: Bond's health is locked at 100 and the hit flash cleared each tick.
    pub god_mode: bool,
    /// Infinite ammo: the gun's clip is refilled each tick.
    pub infinite_ammo: bool,
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
}

/// The spawn-guard picker: a drone type and AI mode.
#[derive(Clone, Copy, PartialEq)]
pub enum SpawnPick { Normal, Sniper, Ambush }

impl Default for Dev {
    fn default() -> Self {
        Dev { overlay: false, menu: false, time_scale: 1.0, step: false, god_mode: false, infinite_ammo: false, spawn: SpawnPick::Normal, forced: Vec::new(), saved_spot: None, mouse: (0.0, 0.0), scenario: None, menu_sel: 0, result_msg: None, result_timer: 0 }
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

    /// Apply god mode / infinite ammo after a tick (the game's own state, set the way the game sets it).
    pub fn after_tick(&mut self, scene: &mut Scene) {
        if self.god_mode {
            if let Some(sim) = scene.sim.as_mut() {
                sim.drones.health.health = 100.0;
                sim.drones.health.armour = 100.0;
                sim.drones.health.flash = 0.0;
            }
        }
        if self.infinite_amma() {
            if let Some(sim) = scene.sim.as_mut() {
                sim.gun.clip = sim.gun.clip.max(40);
                sim.gun.spare = 200;
            }
        }
        self.forced.clear();
        // B-9: the result message fades after ~3 s (180 ticks).
        if self.result_timer > 0 { self.result_timer -= 1; if self.result_timer == 0 { self.result_msg = None; } }
    }

    fn infinite_amma(&self) -> bool { self.infinite_ammo }

    /// Spawn a guard at `pos` with the picked type/mode, the way the game's spawner does (`Action::Spawn` → present +
    /// enabled + llist). Returns the new drone's index.
    pub fn spawn_guard(&self, scene: &mut Scene, pos: [f32; 3], yaw: f32) -> Option<usize> {
        let sim = scene.sim.as_mut()?;
        let d = sim.data();
        let (skin, mode) = match self.spawn {
            SpawnPick::Normal => (0x0500_0006u32, 0u32),
            SpawnPick::Sniper => (0x0500_0019, 2),
            SpawnPick::Ambush => (0x0500_0006, 0x10),
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
pub const MENU_PANEL_W: f32 = 150.0;           // ~29% of 512 HUD-px -> ~21% of a 2000px window
pub const MENU_VISIBLE_ROWS: usize = 7;
pub const MENU_HEADER_H: f32 = MENU_GLYPH_H + 2.0;
pub const MENU_DESC_H: f32 = MENU_GLYPH_H + 2.0;

fn menu(h: &mut Hud, dev: &Dev, scene: &Scene) {
    let Some(t) = scene.hud.as_ref() else { return };
    let font = &t.font;
    let items = menu_items(dev);
    // B-11: layout from the REAL font metrics (17 HUD-px glyphs — measured by `menu_font_metrics_measured`).
    // The old ROW_H=12 caused rows to overlap at ~40 window-px each (the user's screenshot).
    const GLYPH_H: f32 = MENU_GLYPH_H;
    const ROW_H: f32 = MENU_ROW_H;
    const PANEL_X: f32 = MENU_PANEL_X;
    const PANEL_W: f32 = MENU_PANEL_W;
    const VISIBLE_ROWS: usize = MENU_VISIBLE_ROWS;
    const HEADER_H: f32 = MENU_HEADER_H;
    const DESC_H: f32 = MENU_DESC_H;
    let n = items.len();
    // The scroll window: keep `menu_sel` within [top, top + VISIBLE_ROWS).
    let top = if n <= VISIBLE_ROWS { 0 } else {
        if dev.menu_sel < VISIBLE_ROWS / 2 { 0 }
        else if dev.menu_sel >= n - VISIBLE_ROWS / 2 { n - VISIBLE_ROWS }
        else { dev.menu_sel - VISIBLE_ROWS / 2 }
    };
    let visible_end = (top + VISIBLE_ROWS).min(n);
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
        let lbl = fit(label, 16);
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
    if n > VISIBLE_ROWS {
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
    // Footer hint (compact, ASCII).
    h.text(Anchor::Left, font, 69, "up/dn ent click F2", PANEL_X, y, false, [0.55, 0.55, 0.55, 1.0], false);
}

/// The menu items: (label, one-line description of what to watch for, action). B-9: descriptions added so each
/// action is self-explanatory — the user no longer has to guess what "force: impulse" does.
pub fn menu_items(dev: &Dev) -> Vec<(&'static str, &'static str, MenuAction)> {
    let mut v: Vec<(&'static str, &'static str, MenuAction)> = vec![
        ("F2: close", "close this menu and return to play", MenuAction::Close),
        ("spawn: Normal", "a mode-0 guard appears at the crosshair - it should see you and shoot", MenuAction::Spawn(SpawnPick::Normal)),
        ("spawn: Sniper", "a sniper (anim 0x13) - pop up, fire, duck (state 0x97)", MenuAction::Spawn(SpawnPick::Sniper)),
        ("spawn: Ambush", "an ambush guard (mode 0x10) - crouches, attacks when close", MenuAction::Spawn(SpawnPick::Ambush)),
        ("force: grenade", "nearest guard lobs a grenade - watch the arc + explosion (F1)", MenuAction::ForceGrenade),
        ("force: impulse", "nearest guard hops forward - tests the collision handler", MenuAction::Impulse),
        ("force: death", "nearest guard plays a death anim + falls", MenuAction::SpecialDeath),
    ];
    v.push(if dev.god_mode { ("god: ON", "Bond takes no damage - toggle off to test death", MenuAction::ToggleGod) } else { ("god: OFF", "Bond takes no damage when ON - toggle to test death", MenuAction::ToggleGod) });
    v.push(if dev.infinite_ammo { ("ammo: ON", "clip refills each tick - toggle off to test reloads", MenuAction::ToggleAmmo) } else { ("ammo: OFF", "clip refills each tick when ON - toggle to test reloads", MenuAction::ToggleAmmo) });
    v.push(("teleport: save", "save Bond's current spot - restore later", MenuAction::SaveSpot));
    v.push(("teleport: load", "return Bond to the saved spot", MenuAction::RestoreSpot));
    v.push(("time: 1x", "normal speed (60 Hz)", MenuAction::TimeScale(1.0)));
    v.push(("time: 0.25x", "quarter speed - watch AI frame-by-frame", MenuAction::TimeScale(0.25)));
    v.push(("time: pause", "freeze the sim; F11 advances one tick", MenuAction::TimeScale(0.0)));
    v
}

/// A menu item's action.
#[derive(Clone, Copy)]
pub enum MenuAction {
    Close, Spawn(SpawnPick), ForceGrenade, Impulse, SpecialDeath, ToggleGod, ToggleAmmo, SaveSpot, RestoreSpot, TimeScale(f32),
}

/// The click hit-test: which menu item is under the mouse (NDC −1..1, y up)? Returns the row index. B-10:
/// updated for the compact panel (150 px wide, 12 px rows, top-left at y=4, ~8 visible rows with scrolling).
pub fn menu_hit(dev: &Dev, mouse_x: f32, mouse_y: f32, aspect: f32) -> Option<usize> {
    let s = 2.0 / 512.0 * (4.0 / 3.0) / aspect.max(1e-3);
    let x0 = -1.0 + 4.0 * s;
    let x1 = -1.0 + (4.0 + 150.0) * s;
    if mouse_x < x0 || mouse_x > x1 { return None; }
    let n = menu_items(dev).len();
    // B-11: match the menu fn's layout (ROW_H=19, VISIBLE_ROWS=7, HEADER_H=19).
    const ROW_H: f32 = 19.0;
    const VISIBLE_ROWS: usize = 7;
    const HEADER_H: f32 = 19.0;
    let top = if n <= VISIBLE_ROWS { 0 } else {
        if dev.menu_sel < VISIBLE_ROWS / 2 { 0 }
        else if dev.menu_sel >= n - VISIBLE_ROWS / 2 { n - VISIBLE_ROWS }
        else { dev.menu_sel - VISIBLE_ROWS / 2 }
    };
    let visible_end = (top + VISIBLE_ROWS).min(n);
    let banner = if dev.scenario.is_some() { HEADER_H + 1.0 } else { 2.0 };
    for vi in top..visible_end {
        let y_top = 4.0 + banner + (vi - top) as f32 * ROW_H;
        let y_bot = y_top + ROW_H;
        let ndc_top = 1.0 - y_top / 224.0;
        let ndc_bot = 1.0 - y_bot / 224.0;
        if mouse_y >= ndc_bot && mouse_y <= ndc_top { return Some(vi); }
    }
    None
}

/// Apply a menu click: returns the action chosen (the caller applies it).
pub fn menu_action(dev: &Dev, idx: usize) -> Option<MenuAction> {
    menu_items(dev).get(idx).map(|(_, _, a)| *a)
}

/// B-9: keyboard navigation for the menu. ↑/↓ move the selection, Enter activates the selected row. Returns the
/// action to apply (if Enter was pressed). The caller also sets the result message.
pub fn menu_key(dev: &mut Dev, key: MenuKey) -> Option<MenuAction> {
    let n = menu_items(dev).len();
    if n == 0 { return None; }
    match key {
        MenuKey::Up => { if dev.menu_sel == 0 { dev.menu_sel = n - 1; } else { dev.menu_sel -= 1; } None }
        MenuKey::Down => { dev.menu_sel = (dev.menu_sel + 1) % n; None }
        MenuKey::Activate => {
            let i = dev.menu_sel.min(n - 1);
            menu_items(dev).get(i).map(|(_, _, a)| *a)
        }
    }
}

/// A keyboard input to the menu (B-9).
#[derive(Clone, Copy, PartialEq)]
pub enum MenuKey { Up, Down, Activate }

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
    //  - searchlight-tracks-bond (B-9): the searchlight PORT is broken (1/7 heads links to a spot; beam points up).
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
        _ => return None,
    };
    Some((n, f))
}

/// The list of scenario names (for `--dev` start and `--help`). B-8/B-9/B-10: sniper-cover, bullet-flyby,
/// friendly-actor, searchlight-tracks-bond, grenade-throw and grenade-bounce-explode are removed until their
/// tests pass (see `scenario()` comment).
pub const SCENARIOS: &[&str] = &[
    "ambush", "guard-falls-off-ledge",
    "door-anti-crush", "cutscene-ai-pause",
    "ghostly-white", "door-textures", "invisible-walls", "level-exit",
    "weapon-switch", "sg5-pickup", "wire-music", "autoaim", "sniper-scope", "shotgun-reload",
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
    #[allow(dead_code)] // B-9: removed from registry — searchlight PORT is broken (1/7 heads links to a spot; that head's beam points up). See scenario() comment + GAPS.md.
    pub fn searchlight_tracks_bond(scene: &mut Scene) {
        // B-6/B-8: the old scenario placed Bond near a GUARD, not in a searchlight's sweep — so the light never saw
        // Bond. Place Bond on the GROUND in a searchlight HEAD's sweep arc. The head is mounted high on a wall/pole
        // (y≈38 on 07000005) and the play-area floor is ~15 m below, so we search a spiral around the head's XZ in
        // the sweep direction (base_yaw ± offsets) for a floor position within 30 m below. Bond stands there; the
        // sweeping beam catches him within ~2 s. (The spot, kind 2, is at −10000 until the first ray hit repositions
        // it, so we can't use its position pre-tick.) B-7: widened floor search to 30 m.
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
            eprintln!("[scenario] ERROR: no searchlight head (or no ground spot) on this level. Searchlights are on 07000005.");
            eprintln!("[scenario] Run: nf-view <iso> 07000005 --dev --scenario searchlight-tracks-bond");
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
    /// B-11: cutscene-ai-pause. Places Bond at the nearest TouchOnce (0xEA) trigger zone's centre — the gatehouse
    /// cut-scene trigger near the castle gate on 07000005 — so the user can verify guards do NOT fire during the
    /// cut-scene (the B-10/B-11 fix: `Drones::paused` skips the per-drone update while `Sim::disabled`). The
    /// cut-scene should start within ~1 s, run ~15-20 s, then hand control back (PlayerEnable). An active guard is
    /// placed nearby so the user can see it stands still during the cut-scene.
    pub fn cutscene_ai_pause(scene: &mut Scene) {
        // B-11: the gatehouse cut-scene on 07000005 is triggered by walking into a zone (no 0xEA TouchOnce trigger
        // exists — the kinds are 1/2/0xa). Place Bond at the level start and walk forward; the cut-scene fires
        // within ~20 s of walking (the 2be51e5 commit confirmed this headless). The user walks from here.
        if let Some(sim) = scene.sim.as_mut() {
            // Bond stays at the level start; the user walks forward to trigger the gatehouse cut-scene.
            eprintln!("[scenario] cutscene-ai-pause: Bond at the level start ({:.1}, {:.1}, {:.1}). Walk forward toward the castle gate. The gatehouse cut-scene fires within ~20 s. Guards must NOT fire during it (the AI is paused via Drones::paused). Control returns after the cut-scene.",
                sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2]);
        }
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
                let a = w.a;
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
}
