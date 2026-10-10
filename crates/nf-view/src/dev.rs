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
    /// F2: the dev menu (mouse-clickable when open).
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
    /// The chosen scenario (printed at start; `None` in free play).
    pub scenario: Option<&'static str>,
}

/// The spawn-guard picker: a drone type and AI mode.
#[derive(Clone, Copy, PartialEq)]
pub enum SpawnPick { Normal, Sniper, Ambush }

impl Default for Dev {
    fn default() -> Self {
        Dev { overlay: false, menu: false, time_scale: 1.0, step: false, god_mode: false, infinite_ammo: false, spawn: SpawnPick::Normal, forced: Vec::new(), saved_spot: None, mouse: (0.0, 0.0), scenario: None }
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

fn menu(h: &mut Hud, dev: &Dev, scene: &Scene) {
    let Some(t) = scene.hud.as_ref() else { return };
    let font = &t.font;
    let items = menu_items(dev);
    let mut y = 60.0;
    for (label, _) in &items {
        h.quad(Anchor::Left, 70, 0x0300_0003, 0, 4.0, y, 280.0, 14.0, [0.0, 0.0, 1.0, 1.0], (false, false), [0.0, 0.0, 0.0, 0.7]);
        h.text(Anchor::Left, font, 69, label, 8.0, y + 2.0, false, [1.0, 1.0, 1.0, 1.0], false);
        y += 16.0;
    }
}

fn menu_items(dev: &Dev) -> Vec<(&'static str, MenuAction)> {
    let mut v = vec![
        ("F2: close menu", MenuAction::Close),
        ("spawn: Normal guard", MenuAction::Spawn(SpawnPick::Normal)),
        ("spawn: Sniper", MenuAction::Spawn(SpawnPick::Sniper)),
        ("spawn: Ambush", MenuAction::Spawn(SpawnPick::Ambush)),
        ("force: throw grenade now", MenuAction::ForceGrenade),
        ("force: impulse (hop up)", MenuAction::Impulse),
        ("force: SpecialDeath (fall)", MenuAction::SpecialDeath),
    ];
    v.push(if dev.god_mode { ("god mode: ON", MenuAction::ToggleGod) } else { ("god mode: OFF", MenuAction::ToggleGod) });
    v.push(if dev.infinite_ammo { ("infinite ammo: ON", MenuAction::ToggleAmmo) } else { ("infinite ammo: OFF", MenuAction::ToggleAmmo) });
    v.push(("teleport: save spot", MenuAction::SaveSpot));
    v.push(("teleport: restore spot", MenuAction::RestoreSpot));
    v.push(("time: 1x", MenuAction::TimeScale(1.0)));
    v.push(("time: 0.25x", MenuAction::TimeScale(0.25)));
    v.push(("time: pause (Step=F11)", MenuAction::TimeScale(0.0)));
    v
}

/// A menu item's action.
#[derive(Clone, Copy)]
pub enum MenuAction {
    Close, Spawn(SpawnPick), ForceGrenade, Impulse, SpecialDeath, ToggleGod, ToggleAmmo, SaveSpot, RestoreSpot, TimeScale(f32),
}

/// The click hit-test: which menu item is under the mouse (NDC −1..1, y up)? Returns the row index.
pub fn menu_hit(dev: &Dev, mouse_x: f32, mouse_y: f32, aspect: f32) -> Option<usize> {
    let s = 2.0 / 512.0 * (4.0 / 3.0) / aspect.max(1e-3);
    let x0 = -1.0 + 4.0 * s;
    let x1 = -1.0 + (4.0 + 280.0) * s;
    if mouse_x < x0 || mouse_x > x1 { return None; }
    for i in 0..menu_items(dev).len() {
        let y_top = 60.0 + i as f32 * 16.0;
        let y_bot = y_top + 14.0;
        let ndc_top = 1.0 - y_top / 224.0;
        let ndc_bot = 1.0 - y_bot / 224.0;
        if mouse_y >= ndc_bot && mouse_y <= ndc_top { return Some(i); }
    }
    None
}

/// Apply a menu click: returns the action chosen (the caller applies it).
pub fn menu_action(dev: &Dev, idx: usize) -> Option<MenuAction> {
    menu_items(dev).get(idx).map(|(_, a)| *a)
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

/// The named scenarios. Each places Bond and the guards and returns the instruction text the user should see.
/// (B-3a/b: the five scenarios that used to live in `main.rs::apply_scenario` — weapon-switch / sg5-pickup /
/// wire-music / autoaim / sniper-scope — are merged into this single registry so `--scenario` only has one source
/// of truth.)
pub fn scenario(name: &str) -> Option<(&'static str, fn(&mut Scene))> {
    let (n, f): (&'static str, fn(&mut Scene)) = match name {
        "grenade-throw" => ("Grenade throw", scenarios::grenade_throw),
        "grenade-bounce-explode" => ("Grenade bounce and explosion", scenarios::grenade_bounce_explode),
        "sniper-cover" => ("Sniper cover (0x97/0x98)", scenarios::sniper_cover),
        "ambush" => ("Ambush (0x37/0x38)", scenarios::ambush),
        "guard-falls-off-ledge" => ("Guard falls off a ledge", scenarios::guard_falls_off_ledge),
        "searchlight-tracks-bond" => ("Searchlight tracks Bond", scenarios::searchlight_tracks_bond),
        "door-anti-crush" => ("Door anti-crush", scenarios::door_anti_crush),
        "bullet-flyby" => ("Bullet fly-by", scenarios::bullet_flyby),
        "friendly-actor" => ("Friendly actor (team 3)", scenarios::friendly_actor),
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

/// The list of scenario names (for `--dev` start and `--help`).
pub const SCENARIOS: &[&str] = &[
    "grenade-throw", "grenade-bounce-explode", "sniper-cover", "ambush", "guard-falls-off-ledge",
    "searchlight-tracks-bond", "door-anti-crush", "friendly-actor",
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
                let floor_y = world.floor_below([cx, guard_pos[1], cz], 4.0)?;
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

    /// Place Bond near a present guard, at `dist` units in +z if that spot has LOS to the guard; otherwise a spiral
    /// search at 24 angles × multiple distances in [8, 20] m. Iterates over ALL present guards (the first one with a
    /// valid spot wins). Returns the chosen guard's drone index, or None if no spot worked for any guard. (B-3a/b.)
    fn place_bond_near_guard(scene: &mut Scene, dist: f32) -> Option<usize> {
        let sim = scene.sim.as_mut()?;
        let candidates: Vec<(usize, [f32; 3])> = sim.drones.list.iter().enumerate()
            .filter(|(_, x)| x.present)
            .map(|(i, x)| (i, x.pos))
            .collect();
        if candidates.is_empty() {
            eprintln!("[scenario] ERROR: no present guard on this level.");
            return None;
        }
        let mut found: Option<(usize, [f32; 3], [f32; 3], f32)> = None;
        for (gi, g) in &candidates {
            if let Some((bond_pos, yaw)) = spiral_search_for_spot(&sim.world, *g, dist) {
                found = Some((*gi, *g, bond_pos, yaw));
                break;
            }
        }
        let Some((gi, g, bond_pos, yaw)) = found else {
            eprintln!("[scenario] ERROR: no valid placement for any guard (no LOS or no floor at 8-20 m, 24 directions). The scenario will not start.");
            return None;
        };
        let d = ((bond_pos[0] - g[0]).powi(2) + (bond_pos[1] - g[1]).powi(2) + (bond_pos[2] - g[2]).powi(2)).sqrt();
        sim.bond = Bond::stand_at(&sim.world, bond_pos, yaw);
        eprintln!("[scenario] Bond at ({:.1}, {:.1}, {:.1}) facing yaw {:.0}°, guard #{} at ({:.1}, {:.1}, {:.1}), distance {:.1} m",
            bond_pos[0], bond_pos[1], bond_pos[2], yaw.to_degrees(), gi, g[0], g[1], g[2], d);
        Some(gi)
    }

    /// Place Bond near a guard that carries grenades. Same spiral search as `place_bond_near_guard` but iterates only
    /// over grenade-carrying guards. On levels with no grenade guard (e.g. 07000005), prints the levels that have them
    /// and returns None. (B-3a/b.)
    fn place_bond_near_grenade_guard(scene: &mut Scene, dist: f32) -> Option<usize> {
        let sim = scene.sim.as_mut()?;
        let candidates: Vec<(usize, [f32; 3])> = sim.drones.list.iter().enumerate()
            .filter(|(_, x)| x.present && x.grenade_count > 0)
            .map(|(i, x)| (i, x.pos))
            .collect();
        if candidates.is_empty() {
            eprintln!("[scenario] ERROR: no guard on this level carries grenades (param 24 = 0).");
            eprintln!("[scenario] Grenade-carrying guards are on levels 07000002, 07000003, 07000009, 0700000a.");
            eprintln!("[scenario] Run: nf-view <iso> 07000002 --dev --scenario grenade-throw");
            return None;
        }
        let mut found: Option<(usize, [f32; 3], [f32; 3], f32)> = None;
        for (gi, g) in &candidates {
            if let Some((bond_pos, yaw)) = spiral_search_for_spot(&sim.world, *g, dist) {
                found = Some((*gi, *g, bond_pos, yaw));
                break;
            }
        }
        let Some((gi, g, bond_pos, yaw)) = found else {
            eprintln!("[scenario] ERROR: no valid placement for any grenade guard (no LOS or no floor at 8-20 m, 24 directions). The scenario will not start.");
            return None;
        };
        let d = ((bond_pos[0] - g[0]).powi(2) + (bond_pos[1] - g[1]).powi(2) + (bond_pos[2] - g[2]).powi(2)).sqrt();
        let (grenade_count, grenade_weapon) = {
            let dr = &sim.drones.list[gi];
            (dr.grenade_count, dr.grenade_weapon)
        };
        sim.bond = Bond::stand_at(&sim.world, bond_pos, yaw);
        eprintln!("[scenario] Bond at ({:.1}, {:.1}, {:.1}) facing yaw {:.0}°, grenade guard #{} at ({:.1}, {:.1}, {:.1}), distance {:.1} m",
            bond_pos[0], bond_pos[1], bond_pos[2], yaw.to_degrees(), gi, g[0], g[1], g[2], d);
        eprintln!("[scenario] Guard has {} grenades (weapon {:#x}).", grenade_count, grenade_weapon);
        Some(gi)
    }

    pub fn grenade_throw(scene: &mut Scene) {
        let gi = place_bond_near_grenade_guard(scene, 12.0);
        if gi.is_some() {
            eprintln!("[scenario] grenade-throw: Bond 12 m from a guard with stun grenades. Watch for the lob (anim 0x49) within ~10 s; F1 shows the state.");
        }
    }
    pub fn grenade_bounce_explode(scene: &mut Scene) {
        let gi = place_bond_near_grenade_guard(scene, 15.0);
        if gi.is_some() {
            eprintln!("[scenario] grenade-bounce-explode: Bond 15 m away. The grenade should arc, bounce (sound 0x51C) and explode (sound 0x4A7). F1 shows the arc + explosion radius.");
        }
    }
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
        let gi = place_bond_near_guard(scene, 10.0);
        if gi.is_some() {
            let has_searchlight = scene.sim.as_ref().is_some_and(|s| !s.drones.searchlights.parts.is_empty());
            if !has_searchlight {
                eprintln!("[scenario] ERROR: no searchlights on this level. Searchlights are on 07000005 (WORKLOG 0072).");
                eprintln!("[scenario] Run: nf-view <iso> 07000005 --dev --scenario searchlight-tracks-bond");
            } else {
                eprintln!("[scenario] searchlight-tracks-bond: Bond near a guard. A searchlight should sweep and, when it sees Bond, track him (sound 0xE5 relay). Watch the beam.");
            }
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
    pub fn bullet_flyby(scene: &mut Scene) {
        let gi = place_bond_near_guard(scene, 14.0);
        if gi.is_some() {
            if let Some(sim) = scene.sim.as_mut() { for dr in sim.drones.list.iter_mut().filter(|x| x.present) { dr.accuracy = 0; } }
            eprintln!("[scenario] bullet-flyby: a guard fires at Bond from 14 m. A near miss should play the fly-by whiz (0x1dc) when the bullet passes within 3 units. Strafe to make misses pass close.");
        }
    }
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

    /// Bond 5 m from the zip wire. Walk in and out — the music section changes (event 0xD). (B-3a/b.)
    pub fn wire_music(scene: &mut Scene) {
        if let Some(sim) = scene.sim.as_mut() {
            if let Some(w) = sim.wires.first() {
                // Place Bond 5 m from the wire's endpoint A, searching for a floor at that XZ.
                // The wire may be high above the floor (Y=42.94 vs floor at Y=22.93 on 07000005), and the
                // floor may not exist at every XZ near the wire. Search a small spiral around the wire's
                // endpoint for an XZ where floor_below succeeds.
                let a = w.a;
                let mut placed = false;
                for r in [2.0, 5.0, 8.0, 12.0, 20.0] {
                    for angle in [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5] {
                        let theta = angle * std::f32::consts::FRAC_PI_4;
                        let px = a[0] + r * theta.cos();
                        let pz = a[2] + r * theta.sin();
                        if let Some(floor_y) = sim.world.floor_below([px, a[1] + 1.0, pz], 80.0) {
                            sim.bond = nf_game::player::Bond::new([px, floor_y + 1.1, pz], sim.bond.yaw);
                            placed = true;
                            break;
                        }
                    }
                    if placed { break; }
                }
                if !placed {
                    // Fallback: place at the wire's endpoint Y (Bond will fall, but at least he's near the wire).
                    sim.bond = nf_game::player::Bond::new(a, sim.bond.yaw);
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
