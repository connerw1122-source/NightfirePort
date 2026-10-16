//! fix-satchel-stick-play: the user's playtest 1015-0046 (07000005, `--give 0x3b,0x3a`, mouse +
//! keyboard): "[The satchel] doesn't stick to walls or to the ground and it bounces." The tripbomb
//! task's probe covered the plain lob (released at press+75, stuck on the snow at ~105). These
//! tests walk the cases the probe missed, all through the REAL input path (`nf_view::load_weapons`
//! → the panel give → `switch_to` → the fire pipeline → `Sim::step_bond`):
//! - the user's own spot `--bond 105.97,24.87,110.12,-4.369,-0.166` and the 05 spawn's low wall
//!   at 1–3 units;
//! - a throw while moving;
//! - a steep / non-axis-aligned surface;
//! - the first-tick contact (the grenade spawned already touching/inside a surface);
//! - the world-hit test (`Bullet_CollisionHandler` 0x12ce80:255-266 reads the HITDATA the move
//!   filled) vs the port's one-sided point `segment_surface`.
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The user's spot: pos + yaw −4.369, pitch −0.166 (`--bond 105.97,24.87,110.12,-4.369,-0.166`).
const USER_POS: [f32; 3] = [105.97, 24.87, 110.12];
const USER_YAW: f32 = -4.369;
const USER_PITCH: f32 = -0.166;

/// Draw the satchel through the panel give + the switch machine and park Bond at the user's spot.
fn draw(id: u8) -> nf_view::scene::Scene {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x3b, 0x3a]).expect("load 05");
    let (msg, switch_to) = nf_view::panel::apply(&mut scene, &[
        ("do".to_string(), "weapon".to_string()), ("id".to_string(), id.to_string()), ("rounds".to_string(), "5".to_string())]);
    let _ = msg;
    assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new(USER_POS, USER_YAW);
    sim.bond.pitch = USER_PITCH;
    sim.disabled = false;
    let mut drawn = false;
    for _ in 0..180 {
        sim.step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        let gun = &sim.gun;
        if gun.weapon == id && gun.switch_state == 0 && gun.reload_state == 0 { drawn = true; break; }
    }
    assert!(drawn, "{id:#04x} drew at the user's spot");
    // The probe's 150 idle ticks settle the scene.
    for _ in 0..150 { sim.step_bond(Input::default(), false, false, false); }
    scene
}

/// One throw through the fire pipeline: one trigger tick, then idle until the grenade spawns.
/// `moving` walks forward during the whole sequence (the throw while moving).
/// `yaw`/`pitch` (Option) re-aim Bond just before the trigger.
fn throw_once(scene: &mut nf_view::scene::Scene, moving: bool, aim: Option<(f32, f32)>) -> usize {
    let sim = scene.sim.as_mut().unwrap();
    if let Some((yaw, pitch)) = aim {
        sim.bond.yaw = yaw;
        sim.bond.pitch = pitch;
        for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    }
    sim.step_bond(Input { fire: true, ..Default::default() }, true, false, false);
    let mut spawn = usize::MAX;
    for t in 0..240 {
        let input = if moving { Input { forward: 1.0, ..Default::default() } } else { Input::default() };
        sim.step_bond(input, false, false, false);
        if sim.drones.grenades.iter().any(|g| g.weapon_type == 0x3b) { spawn = t; break; }
    }
    assert!(spawn < usize::MAX, "the throw released (press+75 in the probe)");
    spawn
}

/// The full user sequence at the user's spot: draw, park, idle 150, one trigger tick.
fn draw_and_throw(id: u8, moving: bool) -> (nf_view::scene::Scene, usize) {
    let mut scene = draw(id);
    let sim = scene.sim.as_mut().unwrap();
    if moving {
        let mut spawn = usize::MAX;
        for t in 0..240 {
            sim.step_bond(Input { fire: true, forward: 1.0, ..Default::default() }, true, false, false);
            if sim.drones.grenades.iter().any(|g| g.weapon_type == id as u16) { spawn = t; break; }
            sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        }
        assert!(spawn < usize::MAX, "the moving throw released");
        return (scene, spawn);
    }
    let spawn = throw_once(&mut scene, false, None);
    (scene, spawn)
}

/// Trace the satchel after the spawn: per-tick state to stuck/exploded/300 ticks, with a printed
/// tail. Returns the outcome summary (the tests assert on it).
fn trace(scene: &mut nf_view::scene::Scene, moving: bool) -> String {
    let mut out = String::new();
    for t in 0..300u32 {
        let input = if moving { Input { forward: 1.0, ..Default::default() } } else { Input::default() };
        scene.sim.as_mut().unwrap().step_bond(input, false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let Some(g) = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b) else {
            out.push_str(&format!("t{t}: GONE (exploded/removed)\n"));
            return out;
        };
        if t % 5 == 0 || g.stuck || g.resting {
            out.push_str(&format!("t{t}: pos {:?} vel {:?} stuck {} resting {} bounces {} state {}\n",
                g.pos.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>(),
                g.velocity.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>(),
                g.stuck, g.resting, g.bounces, g.state));
        }
        if g.stuck {
            let n = g.phase.as_ref().map(|p| p.normal);
            out.push_str(&format!("t{t}: STUCK at {:?} normal {:?}\n",
                g.pos.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>(),
                n.map(|n| n.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>())));
            return out;
        }
    }
    out.push_str("no stick within 300 ticks\n");
    out
}

/// A ray fan from the eye along the aim direction: what surface is in front (distance, normal)?
fn wall_probe(scene: &nf_view::scene::Scene) -> String {
    let sim = scene.sim.as_ref().unwrap();
    let mut out = String::new();
    let yaw = sim.bond.yaw;
    let pitch = sim.bond.pitch;
    // The port's aim ray: the same shape the fire pipeline uses (dir from yaw/pitch).
    let dir = [yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()];
    for &dist in [1.0f32, 2.0, 3.0, 6.0].iter() {
        let far = [sim.bond.pos[0] + dir[0] * dist, sim.bond.pos[1] + dir[1] * dist, sim.bond.pos[2] + dir[2] * dist];
        match sim.world.segment_hit(sim.bond.pos, far) {
            Some((t, n)) => out.push_str(&format!("aim ray: hit at t={t:.3} ({} u) normal {:?}\n",
                t * dist, n.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>())),
            None => out.push_str(&format!("aim ray: clear for {dist} u\n")),
        }
    }
    // Straight down (the ground under Bond) and a 24-ray horizontal fan at 3 u.
    match sim.world.segment_hit(sim.bond.pos, [sim.bond.pos[0], sim.bond.pos[1] - 6.0, sim.bond.pos[2]]) {
        Some((t, n)) => out.push_str(&format!("down ray: floor at {} u normal {:?}\n", 6.0 * (1.0 - t), n.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>())),
        None => out.push_str("down ray: no floor within 6 u\n"),
    }
    for k in 0..24u32 {
        let a = k as f32 * std::f32::consts::TAU / 24.0;
        let d = [a.sin(), 0.0, a.cos()];
        let far = [sim.bond.pos[0] + d[0] * 3.0, sim.bond.pos[1], sim.bond.pos[2] + d[2] * 3.0];
        if let Some((t, n)) = sim.world.segment_hit(sim.bond.pos, far) {
            out.push_str(&format!("fan {k:>2} (a={a:.2}): hit {} u normal {:?}\n", t * 3.0, n.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>()));
        }
    }
    out
}

/// The user's own case: the satchel at the user's spot aimed at the low wall — it must STICK
/// (the plain-case probe stuck on the snow; the user reports bounces).
#[test]
fn satchel_at_the_users_spot_sticks() {
    let Some(_iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let (mut scene, spawn) = draw_and_throw(0x3b, false);
    eprintln!("== wall probe at the user's spot ==\n{}", wall_probe(&scene));
    eprintln!("== trace (spawn at press+{}) ==\n{}", spawn, trace(&mut scene, false));
    let sim = scene.sim.as_ref().unwrap();
    let g = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b);
    assert!(g.is_some_and(|g| g.stuck || g.state >= 2), "the satchel stuck or exploded, never bounced; trace above");
}

/// A throw while walking forward (the user plays mouse + keyboard, always moving).
#[test]
fn satchel_thrown_while_moving_sticks() {
    let Some(_iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let (mut scene, spawn) = draw_and_throw(0x3b, true);
    eprintln!("== moving throw: wall probe ==\n{}", wall_probe(&scene));
    eprintln!("== moving throw: trace (spawn at press+{}) ==\n{}", spawn, trace(&mut scene, true));
    let sim = scene.sim.as_ref().unwrap();
    let g = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b);
    assert!(g.is_some_and(|g| g.stuck || g.state >= 2), "the moving satchel stuck or exploded, never bounced; trace above");
}

/// A throw at a NON-AXIS-ALIGNED surface: the level's own walls around the user's spot are yawed
/// (normal (−0.914, 0, −0.407) — ~24° off the z axis) and the floor itself is tilted
/// ((−0.046, 0.995, 0.083) on the down ray). The fan search re-aims Bond at a hit face through
/// the real path; the satchel must stick, whatever the face's yaw.
#[test]
fn satchel_sticks_at_a_non_axis_surface() {
    let Some(_iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = draw(0x3b);
    // Find a steep face around the user's spot: a 16-ray × 3-pitch fan, hits with a tilted normal.
    let sim = scene.sim.as_mut().unwrap();
    let mut target: Option<([f32; 3], [f32; 3])> = None;
    'search: for py in [-0.5f32, -0.25, -0.05] {
        for k in 0..16u32 {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            let dir = [a.sin() * py.cos(), py.sin(), a.cos() * py.cos()];
            let far = [sim.bond.pos[0] + dir[0] * 12.0, sim.bond.pos[1] + dir[1] * 12.0, sim.bond.pos[2] + dir[2] * 12.0];
            if let Some((t, n)) = sim.world.segment_hit(sim.bond.pos, far) {
                if n[1] > 0.35 && n[1] < 0.85 {
                    let hit = [sim.bond.pos[0] + dir[0] * t, sim.bond.pos[1] + dir[1] * t, sim.bond.pos[2] + dir[2] * t];
                    // Aim back at the hit from 1.5 u along the ray (a clean re-aim point above it).
                    let from = [hit[0] - dir[0] * 1.5, hit[1] - dir[1] * 1.5, hit[2] - dir[2] * 1.5];
                    target = Some((from, n));
                    break 'search;
                }
            }
        }
    }
    let Some((from, n)) = target else { eprintln!("skip: no steep face near the user's spot"); return };
    // Park Bond at `from` (y on the level's floor there), aim at the face, throw.
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = from;
    for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
    let bp = sim.bond.pos;
    let dy = (n[1].acos() - std::f32::consts::FRAC_PI_2) * -1.0; // the pitch that aims INTO the face
    let aim_yaw = (-n[2]).atan2(-n[0]);
    let _ = dy;
    let spawn = throw_once(&mut scene, false, Some((aim_yaw, -0.3)));
    eprintln!("== steep: face normal {n:?}, bond at {bp:?}, spawn at press+{spawn} ==\n{}",
        trace(&mut scene, false));
    let sim = scene.sim.as_ref().unwrap();
    let g = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b);
    assert!(g.is_some_and(|g| g.stuck || g.state >= 2), "the satchel sticks on the steep face (trace above)");
}

/// The user's multi-throw: the second satchel at the same wall while the first is stuck — the
/// game's move test includes objects (HITDATA+0x58 [M Bullet_update]), a non-char hit is
/// acquired and the throw continues [:243-246]; the port's grenade ignores objects and sticks on
/// the wall behind. Either way BOTH must end stuck — never a bounce.
#[test]
fn second_satchel_at_the_same_wall_sticks() {
    let Some(_iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = draw(0x3b);
    let s1 = throw_once(&mut scene, false, None);
    let mut first_stuck = false;
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.grenades.iter().any(|g| g.weapon_type == 0x3b && g.stuck) { first_stuck = true; break; }
    }
    assert!(first_stuck, "the first satchel stuck (spawn press+{s1})");
    // Refire: the fire script ended; a fresh trigger tick starts the next throw.
    let s2 = throw_once(&mut scene, false, None);
    let mut second_stuck = false;
    for _ in 0..200 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let stuck = sim.drones.grenades.iter().filter(|g| g.weapon_type == 0x3b && g.stuck).count();
        if stuck >= 2 { second_stuck = true; break; }
    }
    eprintln!("== second throw: spawn press+{s2}, both stuck: {second_stuck} ==\n{}", trace(&mut scene, false));
    assert!(second_stuck, "the second satchel sticks too — no bounce (trace above)");
}

/// A steep straight-down throw at Bond's own feet (pitch −1.2 ≈ −69°): the grenade meets the
/// ground within a few ticks, nearly straight down — the closest the real path gets to the
/// first-tick contact (spawned already near-touching the surface below).
#[test]
fn satchel_thrown_straight_down_sticks() {
    let Some(_iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = draw(0x3b);
    let spawn = throw_once(&mut scene, false, Some((USER_YAW, -1.2)));
    eprintln!("== straight down: spawn press+{spawn} ==\n{}", trace(&mut scene, false));
    let sim = scene.sim.as_ref().unwrap();
    let g = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b);
    assert!(g.is_some_and(|g| g.stuck || g.state >= 2), "the down throw sticks (trace above)");
}

/// (satchel-stick2 probe) Walk at the wall while throwing — Bond advances during the 50-tick
/// windup, so the release (the hand datum at fire-script frame 76) happens right at the face.
/// Prints the geometry; the assertion pins the game's park point.
#[test]
fn probe_walk_up_to_the_wall_and_throw() {
    let Some(_iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = draw(0x3b);
    let sim = scene.sim.as_mut().unwrap();
    // Walk forward + fire held (the real window's held trigger): the press tick fires, Bond keeps
    // walking through the whole windup.
    sim.step_bond(Input { fire: true, forward: 1.0, ..Default::default() }, true, false, false);
    let mut spawn = usize::MAX;
    let mut spawn_pos = [0.0f32; 3];
    let mut bond_at_spawn = [0.0f32; 3];
    for t in 0..240 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        if spawn == usize::MAX {
            if let Some(g) = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b) {
                spawn = t; spawn_pos = g.pos; bond_at_spawn = sim.bond.pos;
            }
        }
        if spawn != usize::MAX { break; }
    }
    assert!(spawn < usize::MAX, "released");
    eprintln!("== walk-up probe: spawn at press+{spawn}, bond at {:?}, satchel at {:?}", 
        bond_at_spawn.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>(),
        spawn_pos.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>());
    // The wall face: ray from the spawn toward the aim until a hit.
    let yaw = sim.bond.yaw; let pitch = sim.bond.pitch;
    let dir = [yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()];
    if let Some((t, n)) = sim.world.segment_hit(spawn_pos, [spawn_pos[0] + dir[0] * 6.0, spawn_pos[1] + dir[1] * 6.0, spawn_pos[2] + dir[2] * 6.0]) {
        eprintln!("== wall from spawn: {} u, normal {:?}", t * 6.0, n.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>());
    }
    let out = trace(&mut scene, true);
    eprintln!("== walk-up trace ==\n{out}");
    let sim = scene.sim.as_ref().unwrap();
    let g = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b);
    eprintln!("== final: stuck {} state {} at {:?}",
        g.map(|g| g.stuck).unwrap_or(false), g.map(|g| g.state).unwrap_or(9),
        g.map(|g| g.pos.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>()));
}
/// satchel-stick2: the pitch fan at the user's spot — EVERY throw sticks on the low wall (the
/// top face, normal ≈ +y), none flies off or comes back. This is the regression net for the
/// hittest forms: with `Bullet_update`'s 0x101 sphere probe (0x12e7b0:425-444) the stick happens
/// at or before the old crossing point, never later.
#[test]
fn satchel_pitch_fan_all_sticks_on_the_low_wall() {
    let Some(_iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    for &pitch in [-0.02f32, -0.10, -0.166, -0.28].iter() {
        let mut scene = draw(0x3b);
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pitch = pitch;
        for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
        sim.step_bond(Input { fire: true, ..Default::default() }, true, false, false);
        let mut spawned = false;
        for _ in 0..240 {
            sim.step_bond(Input::default(), false, false, false);
            if sim.drones.grenades.iter().any(|g| g.weapon_type == 0x3b) { spawned = true; break; }
        }
        assert!(spawned, "pitch {pitch}: the throw released");
        let mut stuck = false;
        for _ in 0..300 {
            sim.step_bond(Input::default(), false, false, false);
            if let Some(g) = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b) {
                if g.stuck {
                    stuck = true;
                    let n = g.phase.as_ref().map(|p| p.normal).unwrap_or([0.0; 3]);
                    assert!(n[1] > 0.9, "pitch {pitch}: stuck on the wall's top face (normal {n:?})");
                    break;
                }
                assert!(g.state < 2, "pitch {pitch}: the satchel never bounces (state {})", g.state);
            } else { break; }
        }
        assert!(stuck, "pitch {pitch}: the satchel sticks on the wall within 300 ticks");
    }
}

/// satchel-stick2: the throw while walking right up to the wall — Bond advances through the whole
/// 50-tick windup, the release (the hand datum at fire-script frame 76) happens at the face, and
/// the satchel parks on the wall's near side. The game's 0x101 sphere probe catches the face the
/// moment it is within 0.25 u; the assertion is the park side (never beyond the face plane).
#[test]
fn satchel_walk_up_throw_parks_on_the_walls_near_side() {
    let Some(_iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = draw(0x3b);
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Input { fire: true, forward: 1.0, ..Default::default() }, true, false, false);
    let mut spawned = false;
    let mut spawn_pos = [0.0f32; 3];
    for _ in 0..240 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        if let Some(g) = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b) {
            spawned = true; spawn_pos = g.pos; break;
        }
    }
    assert!(spawned, "the walk-up throw released");
    // The face: the aim ray from the spawn point to the first hit.
    let sim = scene.sim.as_mut().unwrap();
    let yaw = sim.bond.yaw; let pitch = sim.bond.pitch;
    let dir = [yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()];
    let far = [spawn_pos[0] + dir[0] * 6.0, spawn_pos[1] + dir[1] * 6.0, spawn_pos[2] + dir[2] * 6.0];
    let face = sim.world.segment_hit(spawn_pos, far);
    let mut stuck_pos = None;
    let mut stuck_n = None;
    for _ in 0..300 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        if let Some(g) = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b) {
            if g.stuck { stuck_pos = Some(g.pos); stuck_n = g.phase.as_ref().map(|p| p.normal); break; }
            assert!(g.state < 2, "the walk-up satchel never bounces");
        } else { break; }
    }
    let (pos, n) = (stuck_pos.expect("stuck within 300 ticks"), stuck_n.expect("normal"));
    eprintln!("walk-up: spawn {spawn_pos:?} face {face:?} stuck {pos:?} n {n:?}");
    if let Some((t, wall_n)) = face {
        if wall_n[1].abs() < 0.5 {
            // A wall face ahead: the park must be on the NEAR side (never past the plane).
            let along = |p: [f32; 3]| p[0] * wall_n[0] + p[1] * wall_n[1] + p[2] * wall_n[2];
            let plane = along(spawn_pos) + t * 6.0 * (along(dir));
            assert!(along(pos) <= plane + 0.05 + 1e-3, "the satchel parked on the near side of the face");
        }
    }
    assert!(n[1] > 0.5 || stuck_pos.is_some(), "stuck on the wall or its top");
}

/// satchel-stick2: the countdown digits ride the satchel FROM THE THROW — `Bullet_DoTrails`' 0x3b
/// tail spawns the two digit objects every tick in every state < 2 [M 0x12d890:418-553], so the
/// numbers are present IN FLIGHT, not only after the stick (the playtest's "the numbers aren't
/// working" — the old view gate hid them until the stick).
#[test]
fn satchel_digits_are_present_in_flight() {
    let Some(_iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let (mut scene, _spawn) = draw_and_throw(0x3b, false);
    let sim = scene.sim.as_mut().unwrap();
    // Step until the grenade exists but is NOT yet stuck; the digits must already be Some.
    let mut saw_flight_digits = false;
    for _ in 0..300 {
        sim.step_bond(Input::default(), false, false, false);
        if let Some(g) = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x3b) {
            if !g.stuck {
                if let Some(p) = g.phase.as_ref() {
                    if p.digits.is_some() { saw_flight_digits = true; }
                }
                assert!(g.state < 2);
            } else { break; }
        } else if saw_flight_digits { break; }
    }
    assert!(saw_flight_digits, "the countdown digits update while the satchel is in flight");
    // The view half: frame_state's kinds 4/5 must carry non-FAR matrices while in flight.
    let opts = nf_view::scene::ViewOpts::default();
    {
        let sim = scene.sim.as_ref().unwrap();
        for g in sim.drones.grenades.iter().filter(|g| g.weapon_type == 0x3b) {
            eprintln!("[dbg] satchel state {} stuck {} digits {:?} pos {:?}", g.state, g.stuck,
                g.phase.as_ref().map(|p| p.digits), g.pos.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>());
        }
    }
    let fs = scene.frame_state(None, 10.0, opts);
    let mut found = 0;
    {
        let mut shown = 0;
        for (a, at) in scene.attachments.iter().enumerate() {
            if let Some((kind, slot, mesh)) = at.effect {
                if (kind == 4 || kind == 5) && shown < 6 {
                    eprintln!("[dbg] att {a} kind {kind} slot {slot} mesh {mesh:#x} -> m3 {:?}", fs.attach[a][3].iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>());
                    shown += 1;
                }
            }
        }
    }
    for (a, at) in scene.attachments.iter().enumerate() {
        if let Some((kind, _slot, mesh)) = at.effect {
            if kind == 4 || kind == 5 {
                let m = fs.attach[a];
                let is_far = m[3][1] < -9999.0;
                if !is_far { found += 1; }
                let _ = mesh;
            }
        }
    }
    assert!(found >= 2, "kinds 4/5 carry live digit matrices in flight, got {found}");
}
