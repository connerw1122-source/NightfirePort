//! The walkreal walker: drives Bond with the REAL input path only (`Sim::step_bond` + `Input`) —
//! no `bond.pos` writes after the spawn. The route is a list of waypoints; each waypoint carries an
//! optional action (E-press, wait, ladder climb, wire ride, crouch). Stuck detection reports the
//! neighbourhood (doors, triggers, floor) so a walk failure reads like a bug report.
//!
//! Task walkreal-a: the castle (07000001–04) and Mayhew (07000005–08) routes.

use nf_game::player::Input;
use nf_game::triggers::kind;
use nf_game::world::V3;
use nf_view::scene::Scene;

pub const TICKS_PER_SEC: u32 = 60;

/// Wrap an angle into −π..π.
pub fn wrap_pi(a: f32) -> f32 {
    let mut a = a;
    while a > std::f32::consts::PI { a -= std::f32::consts::TAU; }
    while a < -std::f32::consts::PI { a += std::f32::consts::TAU; }
    a
}

/// XZ distance.
pub fn dxz(a: V3, b: V3) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// What to do once the waypoint is reached (or while approaching it).
#[derive(Clone, Copy, Debug)]
pub enum Act {
    /// Just arrival.
    None,
    /// Face the point and hold the use key until it reacts (door opens / trigger fires), ≤ `use_ticks`.
    Use { use_ticks: u32 },
    /// Stand still N ticks on arrival (cut-scenes, door swings).
    Wait(u32),
    /// Approach the ladder's base and hold forward until Bond's feet pass `top_y` (the top lunge
    /// dismounts him [M Player_Climb]).
    Ladder { top_y: f32 },
    /// Ride the grabbed wire toward its B end with `input.strafe = 1` until dropped (mode 7
    /// [M Player_Wire 0x1977c0]).
    WireRide,
    /// Jump once on arrival.
    Jump,
    /// Toggle crouch on (hold) for the next leg.
    CrouchOn,
    /// Release crouch.
    CrouchOff,
}

#[derive(Clone, Copy, Debug)]
pub struct Wp {
    pub at: V3,
    pub act: Act,
    /// Arrival radius (xz), default 0.9.
    pub arrive: f32,
}

impl Wp {
    pub fn new(x: f32, y: f32, z: f32) -> Wp { Wp { at: [x, y, z], act: Act::None, arrive: 0.9 } }
    pub fn at(x: f32, y: f32, z: f32, act: Act) -> Wp { Wp { at: [x, y, z], act, arrive: 0.9 } }
    pub fn r(mut self, r: f32) -> Wp { self.arrive = r; self }
}

pub struct Report {
    /// Per-leg notes (label → ticks, end pos).
    pub legs: Vec<(String, u32, V3)>,
    pub stuck_events: Vec<String>,
    pub falls: Vec<String>,
}

/// The walker over one loaded scene.
pub struct Walker {
    pub scene: Scene,
    /// Top Bond's health up every tick (the "enemies disabled for the walk" allowance; the pain
    /// pipeline still runs — flash, dir bits — so the walk is not a no-op).
    pub top_up_health: bool,
    /// Stop the walk the moment `level_exit` is requested.
    pub stop_on_exit: bool,
    /// The cut-scene skip (the game's own skip action) while `disabled`. On by default (the walk
    /// cannot watch cut-scenes); a route that must NOT skip (a script whose channel writes the
    /// route depends on — walkreal-c's 07000016 launch chain and 0700001b defeat NIS) turns it
    /// off and drives `step_bond` itself for that segment.
    pub skip_cutscenes: bool,
    pub report: Report,
    pub total_ticks: u32,
    pos: V3,
    last_sample: V3,
    sample_tick: u32,
    stuck: u32,
    stuck_cycles: u32,
    sidestep: i32,
    max_y: f32,
    lenient: bool,
}

/// The stuck dump, public for the walkreal tests' own aim loops (walkreal-b2).
#[allow(dead_code)]
pub fn fail_dump_pub(scene: &Scene, target: V3, label: &str) -> String { fail_dump(scene, target, label) }

fn fail_dump(scene: &Scene, target: V3, label: &str) -> String {
    let sim = scene.sim.as_ref().unwrap();
    let p = sim.bond.pos;
    let seg = sim.world.segment(p, target);
    let mut s = format!(
        "STUCK at {label}: bond=({:.2},{:.2},{:.2}) target=({:.2},{:.2},{:.2}) d={:.2} grounded={} substate={} crouched={} head_bump={} floor_ny={:.2} disabled={} health={:.1} segment_to_target={:?}\n",
        p[0], p[1], p[2], target[0], target[1], target[2], dxz(p, target),
        sim.bond.grounded, sim.bond.substate, sim.bond.crouched, sim.bond.head_bump, sim.bond.floor_ny,
        sim.disabled, sim.drones.health.health, seg
    );
    let doors: Vec<String> = sim.drones.doors.list.iter()
        .filter(|d| dxz(d.pos, p) < 4.0)
        .map(|d| format!("door@({:.1},{:.1},{:.1}) kind={} state={:#x} pos01={:.2} ch_open={} ch_lock={} lock_flag={} approach={} lock_on={}",
            d.pos[0], d.pos[1], d.pos[2], d.kind, d.state, d.pos01, d.ch_open, d.ch_lock, d.locked_flag, d.approach,
            d.ch_lock != 0 && sim.drones.channels.on(d.ch_lock as u32)))
        .collect();
    s.push_str(&format!("doors nearby: {}\n", if doors.is_empty() { "none".into() } else { doors.join("; ") }));
    let trigs: Vec<String> = sim.drones.triggers.iter()
        .filter(|t| !t.tris.is_empty() && dxz(t.centre(), p) < 4.0 && !t.gone)
        .map(|t| format!("trig kind={} out={:#x} mask={:#x} at({:.1},{:.1},{:.1})", t.kind, t.out, t.mask, t.centre()[0], t.centre()[1], t.centre()[2]))
        .collect();
    s.push_str(&format!("triggers nearby: {}\n", if trigs.is_empty() { "none".into() } else { trigs.join("; ") }));
    let ground = sim.world.floor_below(p, 6.0);
    s.push_str(&format!("floor_below: {:?}\n", ground));
    s
}

impl Walker {
    pub fn new(scene: Scene) -> Walker {
        let pos = scene.sim.as_ref().unwrap().bond.pos;
        Walker {
            scene, top_up_health: true, stop_on_exit: true, skip_cutscenes: true,
            report: Report { legs: Vec::new(), stuck_events: Vec::new(), falls: Vec::new() },
            total_ticks: 0, pos, last_sample: pos, sample_tick: 0, stuck: 0, stuck_cycles: 0,
            sidestep: 0, max_y: pos[1], lenient: false,
        }
    }

    fn sim(&mut self) -> &mut nf_view::scene::Sim { self.scene.sim.as_mut().unwrap() }

    fn note_stuck(&mut self, msg: String) {
        self.report.stuck_events.push(msg);
        self.stuck_cycles += 1;
    }

    /// One tick with the given input; keeps Bond alive when `top_up_health` (damage suppressed —
    /// the "enemies may be disabled for the walk" allowance; falls are tracked in the report).
    /// A script cut-scene holding Bond (`disabled`) is SKIPPED through the game's own skip action
    /// (`Scripts::skip_request` = `Input_Action(−1, 0x19, 1)` [M nis.rs; fix-dev T1] — the walk
    /// cannot watch cut-scenes, and the skip is the real player's button).
    /// Walk a nav route by stepping EVERY node as its own waypoint (the direct final legs of
    /// `walk_nav_to` wedge on furniture the AI's node chain threads between). Returns false when
    /// no route exists. The per-node walks are lenient: a wedged node is skipped and the next
    /// node re-aims, so one bad node doesn't strand the walk.
    pub fn follow_nav_route(&mut self, target: V3, label: &str) -> bool {
        let pts = {
            let sim = self.scene.sim.as_ref().unwrap();
            nav_route(sim, self.pos, target)
        };
        let pts = match pts {
            Some(p) => p,
            None => {
                eprintln!("[walk] {label}: no nav route — the direct chain");
                let sim = self.scene.sim.as_ref().unwrap();
                let mut v = leg(sim, self.pos, target);
                v.pop();
                v
            }
        };
        for (k, p) in pts.iter().enumerate() {
            let _ = self.walk_to(Wp::new(p[0], p[1], p[2]).r(1.3), &format!("{label} node {k}"));
        }
        let _ = self.walk_to(Wp::new(target[0], target[1], target[2]).r(1.3), &format!("{label} end"));
        true
    }

    /// Walk to a tall-step target with a jump every 30 ticks (the player's own stair move when
    /// the step exceeds the walk's step-up): turn, run forward, hop. Returns on arrival or the
    /// y/xy cap.
    pub fn climb_stairs(&mut self, target: V3, label: &str, arrive: f32, max_ticks: u32) -> bool {
        eprintln!("[walk] {label}: climb begin (bond {:?})", self.scene.sim.as_ref().unwrap().bond.pos);
        let start = self.total_ticks;
        loop {
            if self.total_ticks - start >= max_ticks {
                eprintln!("[walk] {label}: climb cap (bond {:?} target {target:?})", self.scene.sim.as_ref().unwrap().bond.pos);
                return false;
            }
            if self.stop_on_exit && self.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { return true; }
            let p = self.scene.sim.as_ref().unwrap().bond.pos;
            let d = dxz(p, target);
            if d < arrive { return true; }
            let sim = self.scene.sim.as_ref().unwrap();
            let desired = (target[0] - p[0]).atan2(target[2] - p[2]);
            let err = wrap_pi(desired - sim.bond.yaw);
            let jump = self.total_ticks % 30 == 0 && sim.bond.grounded;
            let input = Input {
                turn: err.clamp(-0.08, 0.08),
                forward: if err.abs() > 1.0 { 0.0 } else { 1.0 },
                jump,
                ..Default::default()
            };
            self.tick(input, false);
        }
    }

    pub fn tick(&mut self, input: Input, use_key: bool) {
        self.tick_inner(input, use_key, false);
    }

    /// One tick with the trigger HELD (walkreal-b2): the gadget burns come from the held fire's refire
    /// [M Player_WeaponInitBullet 0x1a4bf0 → Player_MuzzleFlash 0x1a4298 → Draw_LaserBeam 0x1a4560; the
    /// held-fire loop 0x1a5460:601-694]; `Lock_Update` 0x1ae3d8:194 burns the lock the beam touches.
    pub fn tick_fire(&mut self, input: Input) {
        self.tick_inner(input, false, true);
    }

    fn tick_inner(&mut self, input: Input, use_key: bool, fire: bool) {
        if self.skip_cutscenes && self.scene.sim.as_ref().unwrap().disabled {
            self.scene.sim.as_mut().unwrap().drones.scripts.skip_request = true;
        }
        let y_before = self.scene.sim.as_ref().unwrap().bond.pos[1];
        self.scene.sim.as_mut().unwrap().step_bond(input, fire, false, use_key);
        let sim = self.scene.sim.as_ref().unwrap();
        self.total_ticks += 1;
        if sim.bond.pos[1] < y_before - 0.4 && !sim.bond.grounded {
            // falling: record the drop
            let drop = self.max_y - sim.bond.pos[1];
            if drop > 4.0 && !self.report.falls.iter().any(|f| f.ends_with("(open)")) {
                self.report.falls.push(format!("fall from y {:.2} at tick {} (open)", self.max_y, self.total_ticks));
            }
        }
        if sim.bond.grounded {
            self.max_y = sim.bond.pos[1];
            // close any open fall entry
            for f in self.report.falls.iter_mut() { if f.ends_with("(open)") { f.push_str(" landed"); } }
        }
        if self.top_up_health {
            let h = &mut self.scene.sim.as_mut().unwrap().drones.health;
            h.health = 100.0;
        }
    }

    /// Step with no input until Bond is not `disabled` (cut-scene wait), capped.
    pub fn wait_enabled(&mut self, max_ticks: u32, what: &str) -> bool {
        for _ in 0..max_ticks {
            self.tick(Input::default(), false);
            if !self.scene.sim.as_ref().unwrap().disabled { return true; }
        }
        panic!("Bond still disabled after {max_ticks} ticks ({what})");
    }

    /// Stand still N ticks.
    pub fn wait(&mut self, ticks: u32) {
        for _ in 0..ticks { self.tick(Input::default(), false); }
    }

    /// Walk to the nearest live pickups (ammo/guns on the floor — placed or guard drops) until the
    /// ammo pool grows. Returns whether any ammo arrived.
    pub fn resupply(&mut self, label: &str) -> bool {
        let ammo = |w: &Walker| {
            let sim = w.scene.sim.as_ref().unwrap();
            sim.gun.clip as i32 + sim.gun.spare as i32
        };
        let before = ammo(self);
        for attempt in 0..4 {
            let Some(target) = self.scene.sim.as_ref().unwrap().drones.pickups.iter()
                .filter(|p| !p.gone)
                // Ground-level pickups only: the upstairs placed boxes are a detour the walker
                // cannot always climb; the drops land at the guards' feet [L drop positions].
                .filter(|p| (p.m[3][1] - self.pos[1]).abs() < 1.5)
                .map(|p| [p.m[3][0], p.m[3][1], p.m[3][2]])
                .min_by(|a, b| dxz(*a, self.pos).total_cmp(&dxz(*b, self.pos)))
            else { break };
            let d = dxz(target, self.pos);
            if d > 45.0 { break; }
            eprintln!("[walk] {label}: resupply attempt {attempt} at ({:.1},{:.1},{:.1}) (d {:.1})", target[0], target[1], target[2], d);
            self.walk_nav_to(target, &format!("{label} resupply {attempt}"), 1.8);
            if ammo(self) > before { break; }
            // The nav's 1.8 arrive radius can stop short of a pickup sitting just behind a cycling
            // doorway (the 01 west-hall ammo box sat 1.33 away through a door that opened and shut
            // while the nav believed it had arrived). Close the last stretch directly — walk_to's
            // stuck handler opens the doors on the way; lenient so a wedged doorway gives up.
            if dxz(target, self.pos) > 0.9 {
                self.lenient = true;
                self.walk_to(Wp { at: target, act: Act::None, arrive: 0.7 }, &format!("{label} resupply {attempt} close"));
                self.lenient = false;
            }
            if ammo(self) > before { break; }
        }
        let got = ammo(self) > before;
        eprintln!("[walk] {label}: resupply done ({got}) clip {} spare {}", self.scene.sim.as_ref().unwrap().gun.clip, self.scene.sim.as_ref().unwrap().gun.spare);
        if !got {
            // The pickups may be DROPPED GUNS (another weapon's ammo): cycle weapons until one is
            // armed [M Player_GetBestWeapon's first-owned-with-ammo scan, through the real switch].
            if self.switch_to_armed(label) { return true; }
        }
        got
    }

    /// Cycle the weapon wheel until the gun in hand has ammo (or give up after 12 switches).
    pub fn switch_to_armed(&mut self, label: &str) -> bool {
        for k in 0..12 {
            let (c, s) = {
                let sim = self.scene.sim.as_ref().unwrap();
                (sim.gun.clip, sim.gun.spare)
            };
            if c + s > 0 { return true; }
            eprintln!("[walk] {label}: cycling weapons ({k})");
            self.tick(Input { weapon_cycle: 1, ..Default::default() }, false);
            for _ in 0..60 { self.tick(Input::default(), false); } // the holster/draw cycle
        }
        let (c, s) = {
            let sim = self.scene.sim.as_ref().unwrap();
            (sim.gun.clip, sim.gun.spare)
        };
        c + s > 0
    }

    /// The exposed-part aim point: enumerate the drone's hit boxes (the same boxes the port's
    /// bullets test against, `hit_boxes` [L hit-box dump]; the game's per-part damage groups are
    /// `NDrone2_HitDamage` 0x145558 — head 5 ×100, the arms 53–56 ×1.0, legs ×0.75) and return the
    /// clearest one's centre: the eye→centre world segment must miss everything. A player pinned by
    /// a crouched guard in cover re-aims at whatever sticks out instead of shooting the wall. The
    /// preference is the clear box closest to the torso anchor (root + 0.3 — the old aim), so a
    /// clear torso wins and a head/arm pick only happens when the torso is buried.
    pub fn exposed_aim(&self, di: usize) -> Option<V3> {
        let sim = self.scene.sim.as_ref().unwrap();
        let d = &sim.drones.list[di];
        let bp = sim.bond.pos;
        let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
        let data = nf_game::drone::Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
            skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
            hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
            surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
        };
        let torso = [d.pos[0], d.pos[1] + 0.3, d.pos[2]];
        let mut best: Option<(f32, V3)> = None;
        for b in d.hit_boxes(&data) {
            let c = [b.m[3][0], b.m[3][1], b.m[3][2]];
            if sim.world.segment(eye, c).is_some() { continue; }
            let dev = dxz(c, torso);
            if best.map_or(true, |(bd, _)| dev < bd) { best = Some((dev, c)); }
        }
        best.map(|(_, c)| c)
    }

    /// Aim at a drone (its exposed part — see `exposed_aim`) with the view (turn + look through the
    /// input) and tap the trigger until it dies or the cap. The real fire path (the gun, the
    /// bullets, the hit boxes). The port's semi-autos fire once per trigger press [M the fire
    /// script gates Player_WeaponFiring], so the trigger pulses; an empty clip pulses the reload
    /// key (the game's R).
    ///
    /// Cover behaviour (a player's, not a seed dodge): while NO part's segment is clear the trigger
    /// stays DOWN — firing into the wall only burns the clip — and Bond flanks: alternate sidesteps
    /// every 90 ticks, plus a forward push while beyond 8 units; after 9 s of that failing (a
    /// corner pins the sidestep) he walks the NAV GRAPH to a flank point — perpendicular to the
    /// guard bearing, alternating the side and growing the offset 3→5→7 — and re-checks. The
    /// 900-tick no-damage fallback (4 repositions) stays for clear-aim-but-missing cases.
    pub fn shoot(&mut self, di: usize, label: &str) -> bool {
        let mut hp0 = self.scene.sim.as_ref().unwrap().drones.list[di].health;
        let mut repositions = 0i32;
        let mut last_progress = 0u32;
        let mut last_tap = 0u32;
        let mut aim = self.exposed_aim(di);
        let mut blocked_streak = 0u32;
        let mut flanks = 0i32;
        for t in 0..10800u32 {
            {
                let sim = self.scene.sim.as_ref().unwrap();
                if sim.drones.list[di].corpse || sim.drones.list[di].health <= 0.0 { return true; }
                // Re-pick the exposed part periodically: the guard animates, a part clears or buries.
                if t % 30 == 0 { aim = self.exposed_aim(di); }
                let (bp, dp) = (sim.bond.pos, sim.drones.list[di].pos);
                let at = aim.unwrap_or([dp[0], dp[1] + 0.3, dp[2]]);
                let dx = dp[0] - bp[0];
                let dz = dp[2] - bp[2];
                let dy = at[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
                let desired = dx.atan2(dz);
                let err = wrap_pi(desired - sim.bond.yaw);
                let want_pitch = (dy / dxz(bp, dp).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
                let perr = want_pitch - sim.bond.pitch;
                let clip_empty = sim.gun.clip == 0 && sim.gun.spare > 0;
                let reload = clip_empty && t % 20 == 0;
                let blocked = aim.is_none();
                blocked_streak = if blocked { blocked_streak + 1 } else { 0 };
                // Tap only when the view is ON the part (the boxes are ~0.3 wide; the fixed-period
                // taps sprayed ammo at a drifting aim) and SOMETHING is exposed.
                let on_target = err.abs() < 0.02 && perr.abs() < 0.028 && t.saturating_sub(last_tap) > 15;
                let trigger = !clip_empty && !blocked && on_target;
                if trigger { last_tap = t; }
                // The flank: while blocked, sidestep (alternating every 90 ticks) and push forward
                // in the first half of each window when the guard is far.
                let far = dxz(bp, dp) > 8.0;
                let window = (blocked_streak / 90) % 2 == 0;
                let input = Input {
                    turn: err.clamp(-0.08, 0.08),
                    look: perr.clamp(-0.08, 0.08),
                    forward: if blocked && far && window { 1.0 } else { 0.0 },
                    strafe: if blocked { if window { 1.0 } else { -1.0 } } else if repositions % 2 != 0 && t - last_progress < 60 { -1.0 } else if repositions % 2 != 0 { 0.0 } else if t - last_progress < 60 { 1.0 } else { 0.0 },
                    ..Default::default()
                };
                self.scene.sim.as_mut().unwrap().step_bond(input, trigger, reload, false);
            }
            self.total_ticks += 1;
            if self.top_up_health { self.scene.sim.as_mut().unwrap().drones.health.health = 100.0; }
            // Out of ammo: give up (the caller resupplies and retries).
            if self.scene.sim.as_ref().unwrap().gun.clip == 0
                && self.scene.sim.as_ref().unwrap().gun.spare == 0 && t > 60 {
                eprintln!("[walk] {label}: out of ammo at +{t}");
                return false;
            }
            // Progress check: no damage in 900 ticks → sidestep (alternating sides), max 4.
            let hp = self.scene.sim.as_ref().unwrap().drones.list[di].health;
            if hp < hp0 { hp0 = hp; last_progress = t; }
            if t - last_progress == 900 && repositions < 4 {
                repositions += 1;
                last_progress = t;
                eprintln!("[walk] {label}: no damage for 900 ticks — repositioning ({repositions})");
            }
            // 9 s fully blocked: the local sidestep is pinned (a corner, a column) — walk the nav
            // graph to a flank point: perpendicular to the guard bearing, alternating the side,
            // growing the offset 3→5→7. This is the player's walk-around-the-cover; it ends when a
            // part's segment clears (the loop above picks it up).
            if blocked_streak >= 540 && flanks < 6 {
                flanks += 1;
                let side = if flanks % 2 == 1 { 1.0 } else { -1.0 };
                let dist = 2.0 + flanks as f32 * 1.5;
                let sim = self.scene.sim.as_ref().unwrap();
                let (bp, dp) = (sim.bond.pos, sim.drones.list[di].pos);
                let (vx, vz) = (dp[0] - bp[0], dp[2] - bp[2]);
                let len = (vx * vx + vz * vz).sqrt().max(0.001);
                let (px, pz) = (-vz / len, vx / len);
                let fp = [dp[0] + px * side * dist, dp[1], dp[2] + pz * side * dist];
                drop(sim);
                eprintln!("[walk] {label}: no exposed part for {blocked_streak} ticks — flanking to ({:.1},{:.1},{:.1}) (side {side}, offset {dist:.1})", fp[0], fp[1], fp[2]);
                self.lenient = true;
                self.walk_nav_to(fp, &format!("{label} flank {flanks}"), 2.0);
                self.lenient = false;
                blocked_streak = 0;
                aim = self.exposed_aim(di);
                last_progress = t; // the flank is the progress for now
            }
            if t % 1200 == 0 {
                let sim = self.scene.sim.as_ref().unwrap();
                eprintln!("[walk] {label}: shooting drone {di} at +{t} (health {:.0}, dist {:.1}, clip {} spare {}, exposed {})",
                    sim.drones.list[di].health, dxz(sim.bond.pos, sim.drones.list[di].pos), sim.gun.clip, sim.gun.spare, aim.is_some());
            }
        }
        let sim = self.scene.sim.as_ref().unwrap();
        eprintln!("[walk] {label}: drone {di} still alive after 10800 ticks (health {:.0})", sim.drones.list[di].health);
        false
    }

    /// Follow a moving drone (the escort): steer toward its current position until it settles near
    /// `goal` (the route's end), or truly stops (under 0.15 u/s for 15 s) short of it, or the level
    /// exits. The escort's walk crawls (~0.9 u/s [L probe 14]) — the stall test must be slower.
    pub fn follow(&mut self, di: usize, label: &str, max_ticks: u32, goal: V3) -> V3 {
        let start_ticks = self.total_ticks;
        let mut last = self.scene.sim.as_ref().unwrap().drones.list[di].pos;
        let mut still = 0u32;
        loop {
            assert!(self.total_ticks - start_ticks < max_ticks, "follow {label}: cap\n{}",
                fail_dump(&self.scene, self.scene.sim.as_ref().unwrap().drones.list[di].pos, label));
            if self.stop_on_exit && self.scene.sim.as_ref().unwrap().drones.level_exit.is_some() {
                return last;
            }
            if self.scene.sim.as_ref().unwrap().disabled { self.tick(Input::default(), false); continue; }
            let target = self.scene.sim.as_ref().unwrap().drones.list[di].pos;
            let moved = dxz(target, last);
            last = target;
            still = if moved < 0.0025 { still + 1 } else { 0 };
            let d = dxz(self.scene.sim.as_ref().unwrap().bond.pos, target);
            if dxz(target, goal) < 5.0 {
                eprintln!("[walk] {label}: the drone reached the goal at ({:.1},{:.1},{:.1}) after +{} ticks", target[0], target[1], target[2], self.total_ticks - start_ticks);
                return target;
            }
            if still > 900 {
                eprintln!("[walk] {label}: the drone stopped short of the goal at ({:.1},{:.1},{:.1}) after +{} ticks (10 s still)", target[0], target[1], target[2], self.total_ticks - start_ticks);
                return target;
            }
            // Steer toward the drone, keeping OUT of the lead's "Bond in the way" band: the escort
            // parks in PlayerInWay while Bond is within 3.0 [M 0x15ea28 case 3], and waits when Bond
            // is beyond 12 [M 0x15e698 case 3]. The escort band is 4.5..10.
            let sim = self.scene.sim.as_ref().unwrap();
            let desired = (target[0] - sim.bond.pos[0]).atan2(target[2] - sim.bond.pos[2]);
            let err = wrap_pi(desired - sim.bond.yaw);
            let input = Input {
                turn: err.clamp(-0.07, 0.07),
                forward: if d > 5.0 { 1.0 } else if d > 4.0 { 0.45 } else { 0.0 },
                ..Default::default()
            };
            self.tick(input, false);
        }
    }

    /// `walk_to` that reports a wedge instead of panicking (resupply / guard approach skip the
    /// unreachable target and the caller carries on).
    pub fn try_walk_to(&mut self, wp: Wp, label: &str) -> bool {
        self.lenient = true;
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.walk_to(wp, label)));
        self.lenient = false;
        r.is_ok()
    }

    /// Walk to a target THROUGH the nav graph (the guard approaches and resupply runs cross rooms;
    /// the direct steer wedges on walls between them). `final_arrive`: the last leg's radius.
    pub fn walk_nav_to(&mut self, target: V3, label: &str, final_arrive: f32) {
        self.lenient = true;
        let pts = {
            let sim = self.scene.sim.as_ref().unwrap();
            leg(sim, self.pos, target)
        };
        for (k, p) in pts.iter().enumerate().skip(1) {
            // A wedged nav node is SKIPPED (the next node re-aims); the route continues.
            let _ = self.walk_to(Wp::new(p[0], p[1], p[2]).r(1.3), &format!("{label} nav {k}"));
        }
        self.walk_to(Wp { at: target, act: Act::None, arrive: final_arrive }, label);
        self.lenient = false;
    }

    /// Steer Bond to one waypoint; returns the ticks it took. Panics with a dump when genuinely
    /// wedged (3 stuck-mitigation cycles without progress).
    pub fn walk_to(&mut self, wp: Wp, label: &str) -> u32 {
        let start_ticks = self.total_ticks;
        let mut use_left = 0u32;
        let mut ladder = false;
        let mut riding = false;
        // The last door the stuck handler used: the second use of the SAME door without Bond
        // crossing means the door is not the blocker — the wedge is the geometry beside it
        // (the 01 stairwell head's threshold door sat on the lane, opened, auto-closed, and
        // re-hijacked the mitigation forever, so the sidestep/jump ladder never ran).
        let mut last_door: Option<usize> = None;
        loop {
            if self.total_ticks - start_ticks >= 40_000 && !self.lenient {
                let dump = fail_dump(&self.scene, wp.at, label);
                panic!("walk_to {label}: 40000-tick cap\n{dump}");
            }
            if self.total_ticks - start_ticks >= 6_000 {
                // Lenient or not: a 100-second leg is a wedge. Report and bail.
                let dump = fail_dump(&self.scene, wp.at, label);
                if self.lenient { eprintln!("[walk] {label}: gave up at the 6000-tick cap\n{dump}"); return u32::MAX; }
                panic!("walk_to {label}: 6000-tick cap\n{dump}");
            }
            let sim = self.scene.sim.as_ref().unwrap();
            if self.stop_on_exit && sim.drones.level_exit.is_some() {
                self.report.legs.push((label.to_string(), self.total_ticks - start_ticks, self.pos));
                eprintln!("[walk] {label}: level_exit reached at +{} ticks", self.total_ticks - start_ticks);
                return self.total_ticks - start_ticks;
            }
            if self.scene.sim.as_ref().unwrap().disabled {
                self.tick(Input::default(), false);
                continue;
            }
            let p = self.scene.sim.as_ref().unwrap().bond.pos;
            self.pos = p;
            let d = dxz(p, wp.at);

            // Riding: strafe toward B until the wire drops us.
            if riding {
                let on = self.scene.sim.as_ref().unwrap().bond.substate;
                if on != nf_game::player::SS_WIRE && on != nf_game::player::SS_ZIPLINE {
                    riding = false;
                    self.report.legs.push((format!("{label} (wire ride)"), self.total_ticks - start_ticks, self.pos));
                    eprintln!("[walk] {label}: wire ride ended at +{} ticks, pos {:?}", self.total_ticks - start_ticks, self.pos);
                    return self.total_ticks - start_ticks;
                }
                self.tick(Input { strafe: 1.0, ..Default::default() }, false);
                continue;
            }
            // Ladder climb: hold forward until the top.
            if ladder {
                let sim = self.scene.sim.as_ref().unwrap();
                let y = sim.bond.pos[1];
                if sim.bond.substate != nf_game::player::SS_CLIMB && y > wp.at[1] - 1.2 && sim.bond.grounded {
                    ladder = false;
                    self.report.legs.push((format!("{label} (climb)"), self.total_ticks - start_ticks, self.pos));
                    eprintln!("[walk] {label}: ladder climbed to y {y:.2} at +{} ticks", self.total_ticks - start_ticks);
                    return self.total_ticks - start_ticks;
                }
                // Facing the ladder is forced by the climb; just push up.
                self.tick(Input { forward: 1.0, ..Default::default() }, false);
                continue;
            }

            // Arrival?
            if d < wp.arrive {
                match wp.act {
                    Act::None => {
                        for _ in 0..6 { self.tick(Input::default(), false); }
                        self.report.legs.push((label.to_string(), self.total_ticks - start_ticks, self.pos));
                        eprintln!("[walk] {label}: arrived at +{} ticks pos ({:.2},{:.2},{:.2})",
                            self.total_ticks - start_ticks, self.pos[0], self.pos[1], self.pos[2]);
                        return self.total_ticks - start_ticks;
                    }
                    Act::Jump => {
                        for k in 0..30 {
                            self.tick(Input { jump: k == 0, ..Default::default() }, false);
                        }
                        self.report.legs.push((label.to_string(), self.total_ticks - start_ticks, self.pos));
                        return self.total_ticks - start_ticks;
                    }
                    Act::Wait(n) => {
                        self.wait(n);
                        self.report.legs.push((label.to_string(), self.total_ticks - start_ticks, self.pos));
                        eprintln!("[walk] {label}: waited {n} at pos ({:.2},{:.2},{:.2})", self.pos[0], self.pos[1], self.pos[2]);
                        return self.total_ticks - start_ticks;
                    }
                    Act::Use { use_ticks } => {
                        if use_left == 0 { use_left = use_ticks; }
                        // Face the target. The E press is EDGE-triggered in the game (Input_Action
                        // mask 4) and a TOGGLE on doors [M Door_Activate] — pulse one tick per 6.
                        let sim = self.scene.sim.as_ref().unwrap();
                        let desired = (wp.at[0] - sim.bond.pos[0]).atan2(wp.at[2] - sim.bond.pos[2]);
                        let err = wrap_pi(desired - sim.bond.yaw);
                        let near_door = sim.drones.doors.list.iter().position(|d| dxz(d.pos, wp.at) < 3.0);
                        let (door_closed, before) = match near_door {
                            Some(i) => { let d = &sim.drones.doors.list[i]; (d.state & 0xc == 0, Some((d.pos01, d.state))) }
                            None => (true, None),
                        };
                        let input = Input { turn: err.clamp(-0.08, 0.08), ..Default::default() };
                        // Pulse: E on the first tick of every 6 (an edge), only while the door is closed.
                        let press = door_closed && use_left % 6 == 0;
                        self.tick(input, press);
                        use_left -= 1;
                        let sim = self.scene.sim.as_ref().unwrap();
                        let reacted = match (near_door, before) {
                            (Some(i), Some(b)) => {
                                let d = &sim.drones.doors.list[i];
                                // Reacted = the state moved off closed (0x8 opening / 0x4 open).
                                d.state & 0xc != 0 && b.1 & 0xc == 0
                            }
                            _ => sim.drones.level_exit.is_some(),
                        };
                        if use_left == 0 || reacted {
                            self.report.legs.push((format!("{label} (use)"), self.total_ticks - start_ticks, self.pos));
                            eprintln!("[walk] {label}: use done (reacted {reacted}) at pos ({:.2},{:.2},{:.2})", self.pos[0], self.pos[1], self.pos[2]);
                            return self.total_ticks - start_ticks;
                        }
                        continue;
                    }
                    Act::Ladder { top_y } => {
                        // Walk INTO the ladder centre; the grab runs in the walk tick. The grab needs
                        // Bond facing the wall (the ladder yaw's back [M Player_CollLadder]).
                        let desired = (wp.at[0] - self.pos[0]).atan2(wp.at[2] - self.pos[2]);
                        let err = wrap_pi(desired - self.scene.sim.as_ref().unwrap().bond.yaw);
                        let input = Input {
                            turn: if err.abs() > 0.2 { err.clamp(-0.08, 0.08) } else { 0.0 },
                            forward: if err.abs() > 0.2 { 0.3 } else { 1.0 },
                            ..Default::default()
                        };
                        self.tick(input, false);
                        if self.scene.sim.as_ref().unwrap().bond.substate == nf_game::player::SS_CLIMB {
                            ladder = true;
                            eprintln!("[walk] {label}: ladder grabbed at +{} ticks", self.total_ticks - start_ticks);
                        }
                        if self.total_ticks - start_ticks > 1200 {
                            panic!("ladder grab failed at {label}\n{}", fail_dump(&self.scene, wp.at, label));
                        }
                        let _ = top_y;
                        continue;
                    }
                    Act::WireRide => {
                        if self.scene.sim.as_ref().unwrap().bond.substate == nf_game::player::SS_WIRE
                            || self.scene.sim.as_ref().unwrap().bond.substate == nf_game::player::SS_ZIPLINE {
                            riding = true;
                            eprintln!("[walk] {label}: wire grabbed at +{} ticks", self.total_ticks - start_ticks);
                            continue;
                        }
                        // Walk toward the wire point and PULSE E — the game grabs a wire only on the
                        // action key (Player_HandleJump 0x1916b0:126-147: the action edge scans for
                        // the nearest class-0x2C object; no walk-in auto-grab).
                        let desired = (wp.at[0] - self.pos[0]).atan2(wp.at[2] - self.pos[2]);
                        let err = wrap_pi(desired - self.scene.sim.as_ref().unwrap().bond.yaw);
                        let input = Input {
                            turn: err.clamp(-0.08, 0.08),
                            forward: 1.0,
                            ..Default::default()
                        };
                        let press = self.total_ticks % 6 == 0;
                        self.tick(input, press);
                        if self.total_ticks - start_ticks > 2400 {
                            panic!("wire grab failed at {label}\n{}", fail_dump(&self.scene, wp.at, label));
                        }
                        continue;
                    }
                    Act::CrouchOn | Act::CrouchOff => unreachable!(),
                }
            }

            // Steering: turn toward the target, walk forward. Stuck detection every 60 ticks.
            let sim = self.scene.sim.as_ref().unwrap();
            let desired = (wp.at[0] - p[0]).atan2(wp.at[2] - p[2]);
            let err = wrap_pi(desired - sim.bond.yaw);
            let mut input = Input {
                turn: err.clamp(-0.07, 0.07),
                forward: if d > 2.2 { 1.0 } else { 0.45 },
                ..Default::default()
            };
            if err.abs() > 1.2 { input.forward = 0.0; }
            if self.sidestep != 0 {
                // Pure lateral (no forward): the diagonal walk grinds into the same wall corner.
                input.strafe = if self.sidestep > 0 { 1.0 } else { -1.0 };
                input.forward = 0.0;
                self.sidestep -= 1;
            }
            self.tick(input, false);

            // Stuck bookkeeping.
            self.sample_tick += 1;
            if self.sample_tick >= 60 {
                let moved = dxz(self.pos, self.last_sample);
                self.sample_tick = 0;
                self.last_sample = self.pos;
                if moved < 0.3 {
                    self.stuck += 1;
                    // The real player's reaction: a closed door in reach → press E until it moves,
                    // then let the swing finish. Anything else: sidestep, jump, then report.
                    // The door decision in its own scope (the immutable sim borrow must end
                    // before use_door's &mut self, and before the ladder's re-borrows).
                    let door_i = {
                        let sim = self.scene.sim.as_ref().unwrap();
                        sim.drones.doors.list.iter().position(|d| {
                            dxz(d.pos, self.pos) < 2.8
                                && (d.pos[1] - self.pos[1]).abs() < 2.5  // the door's own floor (the 15 crossing's lower gantry doors sit 17.6 below the lane)
                                && !(d.ch_lock != 0 && !sim.drones.channels.on(d.ch_lock as u32))  // a LOCKED door: the E thuds, never opens (the 15 keypad doors)
                                && sim.world.segment(self.pos, d.pos).is_some()
                        })
                    };
                    let repeat = door_i.is_some() && last_door == door_i;
                    let mut used_door = false;
                    if let Some(di) = door_i {
                        if !repeat {
                            last_door = Some(di);
                            self.use_door(di, label);
                            self.stuck = 0;
                            used_door = true;
                        } else {
                            // The same door again: not the blocker — clear the memory and let the
                            // plain wedge ladder below run (back up, sidestep, jump).
                            last_door = None;
                        }
                    }
                    if !used_door {
                    if self.stuck == 1 {
                        // No world wall on the hip line: a knee-high rail — hop first.
                        let rail = self.scene.sim.as_ref().unwrap().world.segment([self.pos[0], self.pos[1] - 0.7, self.pos[2]], [wp.at[0], wp.at[1] - 0.7, wp.at[2]]).is_some();
                        if rail {
                            let sim = self.scene.sim.as_mut().unwrap();
                            if sim.bond.grounded { self.tick(Input { jump: true, forward: 1.0, ..Default::default() }, false); }
                        } else {
                            self.sidestep = 60; // nudge right
                        }
                    } else if self.stuck == 2 {
                        self.sidestep = -60; // nudge left
                    } else if self.stuck == 3 {
                        // jump once — with forward: a knee-high ledge (the gatehouse floor) needs the
                        // hop INTO it, a jump in place only re-tests the wall
                        let sim = self.scene.sim.as_mut().unwrap();
                        let j = sim.bond.grounded;
                        if j { self.tick(Input { jump: true, forward: 1.0, ..Default::default() }, false); }
                    } else if self.stuck == 4 {
                        // back up, sidestep, and jump (crate-top pickups need the hop)
                        self.tick(Input { forward: -0.6, ..Default::default() }, false);
                        self.tick(Input { forward: -0.6, ..Default::default() }, false);
                        self.sidestep = 40;
                        let sim = self.scene.sim.as_mut().unwrap();
                        if sim.bond.grounded { self.tick(Input { jump: true, forward: 1.0, ..Default::default() }, false); }
                    } else if self.stuck == 5 {
                        self.tick(Input { forward: -0.6, ..Default::default() }, false);
                        self.tick(Input { forward: -0.6, ..Default::default() }, false);
                        self.sidestep = -40;
                        let sim = self.scene.sim.as_mut().unwrap();
                        if sim.bond.grounded { self.tick(Input { jump: true, forward: 1.0, ..Default::default() }, false); }
                    } else {
                        let dump = fail_dump(&self.scene, wp.at, label);
                        self.note_stuck(dump.clone());
                        if self.stuck_cycles >= 6 {
                            if self.lenient { eprintln!("[walk] {label}: wedged after 6 mitigation cycles\n{dump}"); return u32::MAX; }
                            panic!("walk_to {label}: wedged after 6 mitigation cycles (stuck events: {:?})\n{}",
                                self.report.stuck_events, dump);
                        }
                        self.stuck = 0;
                    }
                    }
                } else {
                    self.stuck = 0;
                }
            }
        }
    }

    /// Press E at a nearby door until it opens, then keep walking (no wait — the auto-close is 360
    /// ticks [M Door_Update]; the doorway is straight ahead). E is EDGE-triggerED in the game
    /// (Input_Action mask 4, docs/systems/input.md §+0x12c) and a TOGGLE on doors [M Door_Activate:
    /// open or opening closes] — pulse one tick per 6, never hold.
    pub fn use_door(&mut self, di: usize, label: &str) {
        let (dpos, state) = {
            let sim = self.scene.sim.as_ref().unwrap();
            (sim.drones.doors.list[di].pos, sim.drones.doors.list[di].state)
        };
        if state & 0xc != 0 {
            // Already open/opening — just walk through.
            return;
        }
        eprintln!("[walk] {label}: opening door ({:.1},{:.1},{:.1}) state {:#x}", dpos[0], dpos[1], dpos[2], state);
        for k in 0..60u32 {
            let sim = self.scene.sim.as_ref().unwrap();
            let desired = (dpos[0] - sim.bond.pos[0]).atan2(dpos[2] - sim.bond.pos[2]);
            let err = wrap_pi(desired - sim.bond.yaw);
            // The use point is head + look (radius 1, doors' reach 2.5 [M 0x1a5170; door.rs
            // player_use]): aim the LOOK at the door's centre (the placement pos is its base — a
            // door a floor below the approach needs the pitched look, or the E misses).
            let centre = [dpos[0], dpos[1] + 1.0, dpos[2]];
            let to = [centre[0] - sim.bond.pos[0], centre[1] - (sim.bond.pos[1] + sim.bond.eye - sim.bond.crouch_drop), centre[2] - sim.bond.pos[2]];
            let want_pitch = (to[1] / to[0].hypot(to[2]).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            let perr = want_pitch - sim.bond.pitch;
            let d = &sim.drones.doors.list[di];
            let far = dxz(sim.bond.pos, dpos) > 1.7;
            let pressing = d.state & 0xc == 0 && k % 6 == 0; // one edge per 6 ticks
            let input = Input {
                turn: err.clamp(-0.08, 0.08),
                look: perr.clamp(-0.06, 0.06),
                forward: if far { 0.5 } else { 0.0 },
                ..Default::default()
            };
            self.tick(input, pressing);
            if self.scene.sim.as_ref().unwrap().drones.doors.list[di].state & 0x8 != 0 { break; } // opening
        }
    }

    /// A whole route. Crouch toggles are actions on their waypoint.
    pub fn walk_route(&mut self, wps: &[(&str, Wp)]) {
        for (label, wp) in wps {
            self.walk_to(*wp, label);
        }
    }
}

/// Find a trigger by kind + out channel; its bbox centre.
pub fn trigger_centre(sim: &nf_view::scene::Sim, k: u8, out: u8) -> Option<V3> {
    sim.drones.triggers.iter().filter(|t| t.kind == k && t.out == out && !t.tris.is_empty())
        .map(|t| t.centre()).next()
}

/// The nearest door position to a point.
pub fn door_near(sim: &nf_view::scene::Sim, p: V3) -> Option<(V3, usize)> {
    let mut best: Option<(f32, V3, usize)> = None;
    for (i, d) in sim.drones.doors.list.iter().enumerate() {
        let dd = dxz(d.pos, p);
        if best.map_or(true, |(bd, _, _)| dd < bd) { best = Some((dd, d.pos, i)); }
    }
    best.map(|(_, pos, i)| (pos, i))
}

/// All pickups' positions (`m` row 3) with their switch channels.
pub fn pickups(sim: &nf_view::scene::Sim) -> Vec<(V3, u16, u16)> {
    sim.drones.pickups.iter().filter(|p| !p.gone && !p.hidden)
        .map(|p| ([p.m[3][0], p.m[3][1], p.m[3][2]], p.kind, p.switch_channel)).collect()
}

/// Ladders: centres + top/bottom.
pub fn ladders(sim: &nf_view::scene::Sim) -> Vec<(V3, f32, f32)> {
    sim.ladders.iter().map(|l| (l.centre, l.top(), l.bottom())).collect()
}

/// The kind filter helpers re-exported for the tests.
pub use nf_game::triggers::kind as trigger_kind;

/// The level's LOAD_LEVEL triggers with their gate/key/fallback params.
pub fn load_levels(sim: &nf_view::scene::Sim) -> Vec<(V3, u8, u8, u8, u32)> {
    sim.drones.triggers.iter().filter(|t| t.kind == kind::LOAD_LEVEL)
        .map(|t| (t.centre(), t.gate, t.key_gate, t.fallback, t.next_level)).collect()
}

/// The AI path graph's node route from `start` to `dest` (`AINetwork_CalcRoute` + A*): every path is
/// tried (the graph is per-path; a cross-path walk needs the caller's own anchors). Returns the node
/// positions between start and dest (exclusive of both), or None when no path connects them.
pub fn nav_route(sim: &nf_view::scene::Sim, start: V3, dest: V3) -> Option<Vec<V3>> {
    let mut order: Vec<usize> = (0..sim.nav.paths.len()).collect();
    // Prefer the path whose nearest node to dest sits at dest's height.
    order.sort_by_key(|&pi| {
        let best = sim.nav.paths[pi].nodes.iter()
            .map(|n| (if (n.pos[1] - dest[1]).abs() < 4.0 { 0u8 } else { 1 }, dxz(n.pos, dest)))
            .min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)))
            .unwrap_or((2, f32::INFINITY));
        (best.0, std::cmp::Reverse(best.1.is_finite()))
    });
    for pi in order {
        if sim.nav.paths[pi].nodes.is_empty() { continue; }
        let goal = sim.nav.target_nodes(&sim.world, pi, dest, 0.0);
        let used = vec![0u16; sim.nav.paths[pi].links.len()];
        if let Ok((_, route)) = nf_game::nav::Route::to_position(&sim.nav, &sim.world, Some(pi), start, dest, &goal, &used) {
            let pts: Vec<V3> = route.nodes.iter().map(|&n| sim.nav.paths[pi].nodes[n as usize].pos).collect();
            if pts.len() > 2 { return Some(pts[1..pts.len() - 1].to_vec()); }
        }
    }
    None
}

/// A route leg: two anchors, filled with nav nodes between them (per-path A* first, then the
/// cross-path roadmap).
pub fn leg(sim: &nf_view::scene::Sim, from: V3, to: V3) -> Vec<V3> {
    let mut pts = vec![from];
    pts.extend(nav_route(sim, from, to).or_else(|| nav_route_global(sim, from, to)).unwrap_or_default());
    pts.push(to);
    pts
}

/// Cross-path roadmap: Dijkstra over EVERY path's nodes (within-path links + cross-path hops where
/// two nodes stand within 3 units). The game's per-path A* cannot span rooms the AI paths split
/// across; the walk harness can. Returns node positions from `start` to `dest` (exclusive ends).
pub fn nav_route_global(sim: &nf_view::scene::Sim, start: V3, dest: V3) -> Option<Vec<V3>> {
    use std::collections::BinaryHeap;
    #[derive(PartialEq)]
    struct Q { cost: f32, node: usize }
    impl Eq for Q {}
    impl Ord for Q { fn cmp(&self, o: &Self) -> std::cmp::Ordering { o.cost.total_cmp(&self.cost) } }
    impl PartialOrd for Q { fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) } }

    let mut nodes: Vec<V3> = Vec::new();
    let mut owner: Vec<usize> = Vec::new(); // path index per node
    for (pi, p) in sim.nav.paths.iter().enumerate() {
        if p.kind & 0xd != 0 { continue; } // boundary/mission-only kinds are not walk roads
        for n in &p.nodes { nodes.push(n.pos); owner.push(pi); }
    }
    if nodes.is_empty() { return None; }
    let n = nodes.len();
    // Adjacency: within-path links, then cross-path proximity hops.
    let mut adj: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
    for (pi, p) in sim.nav.paths.iter().enumerate() {
        if p.kind & 0xd != 0 { continue; }
        for l in &p.links {
            let (a, b) = (l.a as usize, l.b as usize);
            // Global node index: the path's nodes were pushed in order; find the offset.
            let off: usize = sim.nav.paths.iter().take(pi).map(|q| if q.kind & 0xd != 0 { 0 } else { q.nodes.len() }).sum();
            let (ga, gb) = (off + a, off + b);
            adj[ga].push((gb, l.len));
            adj[gb].push((ga, l.len));
        }
    }
    for a in 0..n {
        for b in (a + 1)..n {
            if owner[a] == owner[b] { continue; }
            let d = ((nodes[a][0] - nodes[b][0]).powi(2) + (nodes[a][1] - nodes[b][1]).powi(2) + (nodes[a][2] - nodes[b][2]).powi(2)).sqrt();
            if d < 3.0 && sim.nav.move_test(&sim.world, nodes[a], nodes[b], false, false).0 == 1 {
                adj[a].push((b, d));
                adj[b].push((a, d));
            }
        }
    }
    let nearest = |p: V3| -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, q) in nodes.iter().enumerate() {
            let dy = (q[1] - p[1]).abs();
            let d = dxz(*q, p) + dy * dy * 8.0;
            if best.is_none_or(|(bd, _)| d < bd) { best = Some((d, i)); }
        }
        best.map(|(_, i)| i).filter(|&i| dxz(nodes[i], p) < 12.0)
    };
    let s = nearest(start)?;
    let g = nearest(dest)?;
    let mut dist = vec![f32::INFINITY; n];
    let mut prev = vec![usize::MAX; n];
    let mut heap = BinaryHeap::new();
    dist[s] = 0.0;
    heap.push(Q { cost: 0.0, node: s });
    while let Some(Q { cost, node }) = heap.pop() {
        if cost > dist[node] { continue; }
        if node == g { break; }
        for &(nb, w) in &adj[node] {
            let c = cost + w;
            if c < dist[nb] { dist[nb] = c; prev[nb] = node; heap.push(Q { cost: c, node: nb }); }
        }
    }
    if dist[g].is_infinite() { return None; }
    let mut path = Vec::new();
    let mut k = g;
    while k != usize::MAX {
        path.push(nodes[k]);
        if k == s { break; }
        k = prev[k];
    }
    path.reverse();
    // Trim the ends (the start/dest nodes stand in for the endpoints).
    if path.len() > 2 { Some(path[1..path.len() - 1].to_vec()) } else { Some(Vec::new()) }
}

/// The smallest-volume cel bbox containing `p` (the index, for probes).
pub fn cel_of(cels: &[nf_view::scene::Cel], p: [f32; 3]) -> Option<usize> {
    nf_view::scene::find_cel(cels, p)
}
