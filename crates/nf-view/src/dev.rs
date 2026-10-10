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
        h.quad(Anchor::Left, 70, 0x0300_0003, false, 4.0, y, 280.0, 14.0, [0.0, 0.0, 1.0, 1.0], (false, false), [0.0, 0.0, 0.0, 0.7]);
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
        "friendly-actor" => ("Friendly actor", scenarios::friendly_actor),
        _ => return None,
    };
    Some((n, f))
}

/// The list of scenario names (for `--dev` start and `--help`).
pub const SCENARIOS: &[&str] = &[
    "grenade-throw", "grenade-bounce-explode", "sniper-cover", "ambush", "guard-falls-off-ledge",
    "searchlight-tracks-bond", "door-anti-crush", "bullet-flyby", "friendly-actor",
];

mod scenarios {
    use super::*;
    use nf_game::player::Bond;

    pub fn grenade_throw(scene: &mut Scene) {
        place_bond_near_gate(scene, 12.0);
        if let Some(sim) = scene.sim.as_mut() { if let Some(i) = sim.drones.list.iter().position(|x| x.present) { sim.drones.list[i].grenade_weapon = 0x35; sim.drones.list[i].grenade_count = 5; } }
        eprintln!("[scenario] grenade-throw: Bond 12 m from the gate guard. The guard has grenades. Watch for the lob (anim 0x49) within ~10 s; F1 shows the state.");
    }
    pub fn grenade_bounce_explode(scene: &mut Scene) {
        place_bond_near_gate(scene, 15.0);
        if let Some(sim) = scene.sim.as_mut() { if let Some(i) = sim.drones.list.iter().position(|x| x.present) { sim.drones.list[i].grenade_weapon = 0x35; sim.drones.list[i].grenade_count = 5; } }
        eprintln!("[scenario] grenade-bounce-explode: Bond 15 m away. The grenade should arc, bounce (sound 0x51C) and explode (sound 0x4A7). F1 shows the arc + explosion radius.");
    }
    pub fn sniper_cover(scene: &mut Scene) {
        place_bond_near_gate(scene, 18.0);
        if let Some(sim) = scene.sim.as_mut() { if let Some(i) = sim.drones.list.iter().position(|x| x.present) { sim.drones.list[i].anim_set = 0x13; } }
        eprintln!("[scenario] sniper-cover: the guard's anim set is 0x13 (sniper). When it reaches cover it should pop up once a second (state 0x97), duck after 6 s (0x99), reload when empty (0x98). F1 shows the state.");
    }
    pub fn ambush(scene: &mut Scene) {
        place_bond_near_gate(scene, 8.0);
        if let Some(sim) = scene.sim.as_mut() { if let Some(i) = sim.drones.list.iter().position(|x| x.present) { sim.drones.list[i].mode = 0x10; sim.drones.list[i].alert = 1.0; } }
        eprintln!("[scenario] ambush: the guard is in mode 0x10 (ambush). It should crouch behind cover (state 0x37 → 0x38) and attack when Bond is within 3 m (0x56). F1 shows the state.");
    }
    pub fn guard_falls_off_ledge(scene: &mut Scene) {
        place_bond_near_gate(scene, 20.0);
        if let Some(sim) = scene.sim.as_mut() { if let Some(i) = sim.drones.list.iter().position(|x| x.present) { sim.drones.list[i].pos[1] += 3.0; sim.drones.list[i].velocity[1] = 0.0; } }
        eprintln!("[scenario] guard-falls-off-ledge: the guard is lifted 3 m. It should fall under gravity (the Drone_CollisionHandler down-ray) and land on the floor. F1 shows in_air / on_surface.");
    }
    pub fn searchlight_tracks_bond(scene: &mut Scene) {
        place_bond_near_gate(scene, 10.0);
        eprintln!("[scenario] searchlight-tracks-bond: Bond near the gate. A searchlight should sweep and, when it sees Bond, track him (sound 0xE5 relay). Watch the beam.");
    }
    pub fn door_anti_crush(scene: &mut Scene) {
        place_bond_near_gate(scene, 5.0);
        eprintln!("[scenario] door-anti-crush: Bond near a door. Open it (E), then stand in the doorway — it should reverse (anti-crush, sound 284). Watch the door.");
    }
    pub fn bullet_flyby(scene: &mut Scene) {
        place_bond_near_gate(scene, 14.0);
        if let Some(sim) = scene.sim.as_mut() { for dr in sim.drones.list.iter_mut().filter(|x| x.present) { dr.accuracy = 0; } }
        eprintln!("[scenario] bullet-flyby: a guard fires at Bond from 14 m. A near miss should play the fly-by whiz (0x1dc) when the bullet passes within 3 units. Strafe to make misses pass close.");
    }
    pub fn friendly_actor(scene: &mut Scene) {
        place_bond_near_gate(scene, 6.0);
        if let Some(sim) = scene.sim.as_mut() { if let Some(i) = sim.drones.list.iter().position(|x| x.present) { sim.drones.list[i].team = 2; } }
        eprintln!("[scenario] friendly-actor: the guard is on team 2 (friendly). It should not attack Bond — walk up to it. F1 shows the state stays Idle/Patrol.");
    }

    fn place_bond_near_gate(scene: &mut Scene, dist: f32) {
        if let Some(sim) = scene.sim.as_mut() {
            let g = sim.drones.list.iter().find(|x| x.present).map(|x| x.pos).unwrap_or([0.0, 0.0, 0.0]);
            sim.bond = Bond::stand_at(&sim.world, [g[0] + dist, g[1], g[2]], std::f32::consts::PI);
        }
    }
}
