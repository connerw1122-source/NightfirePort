//! fix-satchel-tripbomb: the satchel charge (0x3b) and the laser tripbomb (0x3a) through the real
//! Sim path (nf-view load + `Sim::step_bond`, the grenade list the game fills from `Bullet_init`
//! 0x12c5a0). The facts under test, all off the disc/decompile:
//! - The records [L dump of all 115 `weapon_data` entries]: 0x3a 'Laser Tripbomb' = 8/100,
//!   `+0x70` = 0x080100 (sticky 0x100 + the laser bit 0x80000, NO +0x200 fuse bit), base 0x3a;
//!   0x3b 'Satchel Charge' = 8/2000, `+0x70` = 0x300 (sticky + the fuse bit), base 0x3b.
//! - The fuse [M `Bullet_init` 0x12c5a0:199-208]: base 0x3b → `((id − base)·5 + 5)·60` = 300 for
//!   the fired 0x3b (every satchel bases to itself [L]; only 0x3b is selectable).
//! - The stick [M `Bullet_CollisionHandler` 0x12ce80:232-394]: `+0x70 & 0x100` + a world hit →
//!   state 3, the matrix aligned to the NEGATED hit normal, parked `hit + normal·offset` (0.04 for
//!   the 0x3b + Sound 0x4a7 [:319-323]; 0.002 for the 0x80000 tripbomb + Sound 0x446 +
//!   `BU+0xFC = 120.0` [:325-334]); the tripbomb's 6-placed walk [:335-370].
//! - The stuck fuse [M `Bullet_DoTrails` 0x12d890:262-285 runs in state 3 — `Bullet_update` 0x12efd8
//!   passes the step 0.0 but DoTrails' first gate tests the OBJ pointer (a0) [M disasm
//!   0x12d890-0x12d8c8 + 0x12efe8 `mov.s $f12, $f21` with $f21 = 0.0]]; the 0x3b tail's beeps
//!   (Sound 0x447 on the quarter-second change [:441-443]) and the digits [L SatchelDigits
//!   @0x2bed90 = 0x2000791..0x200079a; M :485-553].
//! - The tripbomb's laser [M `Bullet_update` case 3, the 0x80000 arm 0x12efa4-0x12ef78]: the fuse
//!   counts down 120, then the beam object (gfx 0x2000133) + Sound 0x27c once, then the ray out
//!   along +normal (`pos − 200·Z`, Z = −normal [M 0x12ce80:266]) — a char (2/3) on it →
//!   `Bullet_handle_object_destruction` 0x12cd98 → the 8/100 blast [L record] and the beam dies.
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_game::grenade::Grenade;
use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, "07000005.bin", false).expect("load 07000005")
}

/// A wall within 4 m of `pos`: (hit point, unit normal) from the level's own collision, wall-ish
/// (the normal mostly horizontal), whose outward beam line (a 10 m sample along +normal) stays
/// > 2 m clear of Bond and every live guard — so the armed laser can't blow on a bystander before
/// the test's own trigger.
fn find_wall(sim: &nf_view::scene::Sim, pos: [f32; 3]) -> Option<([f32; 3], [f32; 3])> {
    for k in 0..24u32 {
        let a = k as f32 * std::f32::consts::TAU / 24.0;
        let dir = [a.sin(), 0.0, a.cos()];
        let far = [pos[0] + dir[0] * 4.0, pos[1], pos[2] + dir[2] * 4.0];
        if let Some((t, n)) = sim.world.segment_hit(pos, far) {
            if n[1].abs() < 0.5 {
                let hit = [pos[0] + (far[0] - pos[0]) * t, pos[1] + (far[1] - pos[1]) * t, pos[2] + (far[2] - pos[2]) * t];
                // The 10 m beam sample out of the wall must stay > 2 m from Bond and the live guards.
                let mut clear = true;
                let mut check = |p: [f32; 3]| {
                    let ab = [n[0] * 10.0, n[1] * 10.0, n[2] * 10.0];
                    let t2 = (((p[0] - hit[0]) * ab[0] + (p[1] - hit[1]) * ab[1] + (p[2] - hit[2]) * ab[2]) / 100.0).clamp(0.0, 1.0);
                    let dx = hit[0] + ab[0] * t2 - p[0];
                    let dy = hit[1] + ab[1] * t2 - p[1];
                    let dz = hit[2] + ab[2] * t2 - p[2];
                    if dx * dx + dy * dy + dz * dz < 4.0 { clear = false; }
                };
                check(sim.bond.pos);
                for dr in &sim.drones.list {
                    if dr.present && dr.enabled && !dr.corpse && dr.health > 0.0 { check(dr.pos); }
                }
                if clear { return Some((hit, n)); }
            }
        }
    }
    None
}

/// A horizontal vector perpendicular to `n` (the beam line's side direction).
fn side_of(n: [f32; 3]) -> [f32; 3] {
    let s = [n[1], -n[0], 0.0];
    let l = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt().max(1e-6);
    [s[0] / l, s[1] / l, s[2] / l]
}

/// A thrown gadget: 1 m off the wall along `−normal`, lobbed INTO it at the record's throw speed
/// (the port's grenade list is exactly what `Bullet_init` fills — the Sim's fire pipeline spawns
/// the same struct, scene.rs's gravity arm).
fn thrown(sim: &nf_view::scene::Sim, hit: [f32; 3], normal: [f32; 3], id: u16) -> Grenade {
    let r = &sim.weapons.records[id as usize];
    let f32_at = |o: usize| f32::from_le_bytes(r[o..o + 4].try_into().unwrap());
    let speed = f32_at(0x20).max(0.05);
    let pos = [hit[0] - normal[0] * 0.6, hit[1] - normal[1] * 0.6, hit[2] - normal[2] * 0.6];
    Grenade {
        pos, velocity: [-normal[0] * speed, -normal[1] * speed, -normal[2] * speed], speed,
        travelled: 0.0,
        range: if id == 0x3b { 300.0 } else { (f32_at(0x1c) / speed).max(1.0) },
        damage: f32_at(0x0c), radius: f32_at(0x08), weapon_type: id,
        mesh: sim.weapons.projectile_mesh(id as usize), owner: usize::MAX, state: 0,
        bounces: 0, smoke_popped: false, resting: false, stuck: false, seek_target: None,
        phase: Some(Box::new(nf_game::grenade::StuckGadget { normal: [0.0, 1.0, 0.0], arm: 0.0, armed: false, last_qs: 0, digits: None })),
    }
}

/// The satchel thrown at a wall STICKS and blows at the game's tick: the Stick lands within a few
/// ticks of the throw, the fuse totals 300 ticks FROM THE THROW (the family arm
/// `((0x3b − 0x3b)·5 + 5)·60` [M 0x12c5a0:199-208] — the pre-fix port defaulted to 210 and never
/// stuck), and the blast goes through the shared 0x6000052 blast path.
#[test]
fn satchel_thrown_at_a_wall_sticks_and_blows_at_the_game_tick() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    sim.disabled = false;
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    let Some((hit, normal)) = find_wall(sim, sim.bond.pos) else { eprintln!("skip: no wall near Bond's start"); return };
    sim.drones.grenades.push(thrown(sim, hit, normal, 0x3b));
    // The stick: 0.6 m at the record's 0.067 u/tick throw speed + gravity → under 20 ticks.
    let mut stuck = false;
    for _ in 0..25 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.grenades.first().is_some_and(|g| g.stuck) { stuck = true; break; }
    }
    assert!(stuck, "the satchel sticks at the wall (pre-fix it bounced)");
    let g = &sim.drones.grenades[0];
    let p = g.phase.as_ref().expect("the phase box");
    for k in 0..3 { assert!((p.normal[k] - normal[k]).abs() < 1e-3, "the phase normal is the hit normal"); }
    assert!((g.range - 300.0).abs() < 1e-4, "the family fuse is 300 [M 0x12c5a0:204], got {}", g.range);
    // The blast: at travelled 301 — within 310 ticks of the stick (the fuse ticks through the flight).
    let mut boom = false;
    for _ in 0..310 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.grenades.is_empty() { boom = true; break; }
        if sim.drones.scripts.players.iter().any(|p| p.scripts[0] == 0x0600_0052) { boom = true; break; }
    }
    assert!(boom, "the stuck satchel blows at its 300-tick fuse (pre-fix: 210 and mid-air)");
    assert!(sim.drones.scripts.players.iter().any(|p| p.scripts[0] == 0x0600_0052),
        "the 0x6000052 blast script plays (the satchel's death is the generic fuse blast [M 0x12d890:391-394])");
}

/// The satchel's countdown while stuck: the display digits (SatchelDigits meshes
/// 0x2000791+sec/10 and +sec%10 [L 0x2bed90]) read the remaining seconds, and the 0x447 beep
/// sounds land in the Sim's sound list on the quarter-second changes [M 0x12d890:441-443].
#[test]
fn stuck_satchel_shows_the_countdown_and_beeps() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    sim.disabled = false;
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    let Some((hit, normal)) = find_wall(sim, sim.bond.pos) else { eprintln!("skip: no wall near Bond's start"); return };
    sim.drones.grenades.push(thrown(sim, hit, normal, 0x3b));
    for _ in 0..25 { sim.step_bond(Input::default(), false, false, false); }
    let Some(g) = sim.drones.grenades.first() else { panic!("the satchel is alive") };
    assert!(g.stuck, "stuck before the countdown assertions");
    // Stuck ticks: the digits carry (0, 4) (fuse ~285 → sec 4) and the 0x447 beeps appear.
    let mut beeps = 0;
    let mut digits = None;
    for _ in 0..20 {
        sim.step_bond(Input::default(), false, false, false);
        if let Some(g) = sim.drones.grenades.first() {
            if let Some(p) = g.phase.as_ref() { digits = digits.or(p.digits); }
        }
        if sim.drones.sounds.iter().any(|s| s.id == 0x447) { beeps += 1; }
    }
    assert_eq!(digits, Some((0, 4)), "the display reads the remaining seconds (04) [M 0x12d890:421-422]");
    assert!(beeps >= 1, "the 0x447 countdown beeps reach the sound list");
}

/// The tripbomb on a wall ARMS after the game's 120-tick delay, draws its beam out along the
/// surface normal (the sim's `tripbomb_beams`, mesh 0x2000133 [M the laser block]), and a guard
/// walking through the beam sets it off: the 8/100 blast [L record] kills/hurts him and the beam
/// dies with the tripbomb [M 0x12cd98's 0x80000 tail]. Sister: a guard parked OFF the beam line
/// leaves the tripbomb armed with the beam up.
#[test]
fn tripbomb_arms_draws_the_beam_and_a_guard_crossing_sets_it_off() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    sim.disabled = false;
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    let Some((hit, normal)) = find_wall(sim, sim.bond.pos) else { eprintln!("skip: no wall near Bond's start"); return };
    // Bond steps 3 m sideways off the future beam line (he holds still — no input).
    let side = side_of(normal);
    sim.bond.pos = [sim.bond.pos[0] + side[0] * 3.0, sim.bond.pos[1], sim.bond.pos[2] + side[2] * 3.0];
    sim.drones.grenades.push(thrown(sim, hit, normal, 0x3a));
    let mut stuck = false;
    for _ in 0..20 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.grenades.first().is_some_and(|g| g.stuck) { stuck = true; break; }
    }
    assert!(stuck, "the tripbomb sticks");
    let g0 = sim.drones.grenades[0].clone();
    let (pos, n) = (g0.pos, g0.phase.as_ref().unwrap().normal);
    assert!(sim.drones.tripbomb_beams.is_empty(), "no beam during the arm delay");
    // The arm: 120 stuck ticks (+ slack for the flight) — then the beam is up, out along +normal.
    let mut armed = false;
    for _ in 0..140 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.grenades.first().is_some_and(|g| g.phase.as_ref().is_some_and(|p| p.armed)) { armed = true; break; }
    }
    assert!(armed, "the laser arms 120 stuck ticks after the stick [M 0x12ce80:330 + case 3]");
    assert!(!sim.drones.tripbomb_beams.is_empty(), "the beam is drawn once armed");
    let (a, b) = sim.drones.tripbomb_beams[0];
    for k in 0..3 {
        assert!((a[k] - pos[k]).abs() < 1e-3, "the beam starts at the stuck tripbomb");
        let along = (b[k] - a[k]) * if n[k].abs() > 1e-6 { 1.0 / n[k] } else { 0.0 };
        assert!(along > 0.5, "the beam extends OUT along +normal (k={k}, along={along})");
    }
    // A guard walks through: park a live guard ON the beam line, 1 m out from the wall.
    let Some(gi) = sim.drones.list.iter().position(|d| d.present && d.enabled && !d.corpse && d.health > 0.0) else {
        eprintln!("skip: no live guard"); return;
    };
    let hp0 = sim.drones.list[gi].health;
    let spot = [pos[0] + n[0] * 1.0, pos[1] + n[1] * 1.0, pos[2] + n[2] * 1.0];
    sim.drones.list[gi].pos = spot;
    let mut blown = false;
    for _ in 0..4 {
        sim.drones.list[gi].pos = spot; // the AI's own motion must not walk him off the line
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.list[gi].corpse || sim.drones.list[gi].health < hp0 { blown = true; break; }
    }
    assert!(blown, "the guard crossing the beam takes the 8/100 blast [L record 0x3a] ({} → {})", hp0, sim.drones.list[gi].health);
    assert!(sim.drones.grenades.iter().all(|g| g.weapon_type != 0x3a),
        "the tripbomb is gone with its beam [M 0x12cd98's 0x80000 tail]");
    assert!(sim.drones.tripbomb_beams.is_empty(), "the beam dies with the tripbomb");
    // Sister: a fresh tripbomb with the guard parked 2 m OFF the line stays armed, beam up.
    sim.drones.grenades.push(thrown(sim, hit, normal, 0x3a));
    for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.grenades.iter().any(|g| g.weapon_type == 0x3a && g.stuck), "the sister tripbomb sticks");
    let off_spot = [pos[0] + side[0] * 2.0, pos[1], pos[2] + side[2] * 2.0];
    let mut armed2 = false;
    for _ in 0..150 {
        sim.drones.list[gi].pos = off_spot;
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.grenades.iter().any(|g| g.weapon_type == 0x3a && g.phase.as_ref().is_some_and(|p| p.armed)) { armed2 = true; break; }
    }
    assert!(armed2, "the sister tripbomb arms with the guard off the line");
    assert!(sim.drones.grenades.iter().any(|g| g.weapon_type == 0x3a && g.state < 2),
        "the off-beam guard triggers nothing — the tripbomb still stands");
    assert!(!sim.drones.list[gi].corpse, "the off-beam guard is untouched");
}

/// The 6-placed cap [M `Bullet_CollisionHandler` 0x12ce80:335-370]: never more than 6 placed
/// tripbombs — the 7th placement removes the oldest (the port trims to the last 6 in spawn order
/// [*suggested*, obj+0xEC]).
#[test]
fn the_seventh_tripbomb_removes_the_oldest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    sim.disabled = false;
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    let Some((_hit, normal)) = find_wall(sim, sim.bond.pos) else { eprintln!("skip: no wall near Bond's start"); return };
    // 7 stuck + armed tripbombs lifted 3 m above the floor and spread along the beam's side
    // direction, so no beam line comes within the 1.0 trigger radius of Bond (feet) or a guard.
    let side = side_of(normal);
    let base = sim.bond.pos;
    for k in 0..7 {
        let mut g = thrown(sim, base, normal, 0x3a);
        g.pos = [base[0] + side[0] * k as f32 * 0.5, base[1] + 3.0, base[2] + side[2] * k as f32 * 0.5];
        g.velocity = [0.0; 3];
        g.stuck = true;
        g.state = 1;
        g.phase = Some(Box::new(nf_game::grenade::StuckGadget { normal, arm: 0.0, armed: true, last_qs: 0, digits: None }));
        sim.drones.grenades.push(g);
    }
    for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    let placed = sim.drones.grenades.iter().filter(|g| g.weapon_type == 0x3a && g.stuck && g.state < 2).count();
    assert_eq!(placed, 6, "the 6-placed cap [M 0x12ce80:335-370] — pre-fix the port kept all 7, got {placed}");
}

/// The real fire path (the dev panel's own give → the game's loadout grant, the same path the
/// hat test throws through): Bond draws the satchel and fires — the grenade the fire pipeline
/// spawns carries the FAMILY fuse `((0x3b − 0x3b)·5 + 5)·60 = 300` [M `Bullet_init`
/// 0x12c5a0:199-208], not the pre-fix default 210. Sister: the tripbomb (0x3a, base 0x3a, no
/// +0x200 fuse bit) gets the range-out fuse `+0x1C/speed` [M the same formula's else-arm].
#[test]
fn the_fire_pipeline_spawns_the_family_fuse() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    for &id in [0x3bu8, 0x3a].iter() {
        let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[id]).expect("load 05");
        let (msg, switch_to) = nf_view::panel::apply(&mut scene, &[
            ("do".to_string(), "weapon".to_string()), ("id".to_string(), id.to_string()), ("rounds".to_string(), "5".to_string())]);
        let _ = msg;
        assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
        let mut drawn = false;
        for _ in 0..180 {
            scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
            let gun = &scene.sim.as_ref().unwrap().gun;
            if gun.weapon == id && gun.switch_state == 0 && gun.reload_state == 0 { drawn = true; break; }
        }
        assert!(drawn, "{id:#04x} drew");
        // Hold fire until the release frame spawns the grenade (the fire-script pipeline).
        let mut spawned = false;
        for _ in 0..240 {
            scene.sim.as_mut().unwrap().step_bond(Input { fire: true, ..Input::default() }, true, false, false);
            if scene.sim.as_ref().unwrap().drones.grenades.iter().any(|g| g.weapon_type == id as u16) { spawned = true; break; }
        }
        let sim = scene.sim.as_ref().unwrap();
        assert!(spawned, "firing {id:#04x} spawned its grenade (clip {})", sim.gun.clip);
        let g = sim.drones.grenades.iter().find(|g| g.weapon_type == id as u16).unwrap();
        if id == 0x3b {
            assert!((g.range - 300.0).abs() < 1e-4,
                "the satchel's spawned fuse is the family arm's 300 [M 0x12c5a0:204], got {}", g.range);
        } else {
            let r = &sim.weapons.records[0x3a];
            let spd = f32::from_le_bytes(r[0x20..0x24].try_into().unwrap());
            let want = (f32::from_le_bytes(r[0x1c..0x20].try_into().unwrap()) / spd).max(1.0);
            assert!((g.range - want).abs() < 1.0,
                "the tripbomb's spawned fuse is the range-out arm (+0x1C/speed = {want}), got {}", g.range);
        }
    }
}
