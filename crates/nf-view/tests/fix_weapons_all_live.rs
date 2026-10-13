//! fix-weapons-all: the 2026-10-12 playtest weapon bugs, through the real input path (the dev panel's
//! give commands + `Sim::step_bond` fire input). Opt-in: set `NF_ISO` to the disc image.
//!
//! - **nades05** "a frag grenade spawns what looks to be a stun grenade … no clue what the rocket
//!   launcher is": the panel's quick buttons give decimal ids 53/54 = 0x35/0x36 (correct since the
//!   panel parses decimal), so the audit locks the give ids, the pinned projectile meshes
//!   [L ee_start_05.bin 0x2bf150: 0x35 → 0x02000726, 0x36 → 0x0200071f, 0x3A → 0x0200074f] and the
//!   panel NAMES (0x3A had no name at all — the user could not find the rocket launcher).
//! - **The frag "blows up immediately as soon as it hits the ground"**: `Bullet_CollisionHandler`
//!   0x12ce80 NEVER explodes a bouncing grenade — a slow bounce zeroes the speed (LAB_0012d4bc tail:
//!   `speed < FRAME_RATE_MUL·0.01 → speed = 0`, state stays 1) and the 20th bounce (count reaching
//!   0x14) also just zeroes the speed. Only the tick fuse (BU+0x3F, 210 for the frag [M Bullet_init
//!   0x12c5a0:103]) explodes it. The port exploded on both — fail-first below.
//! - **Bounce feel**: the game scales the bounce by the per-material coefficient table at 0x3037dc
//!   (record stride 0x88; 0.5 for materials 0..12/15, 0.0 for 13/14) [M 0x12ce80 `*(float *)lVar10`],
//!   not a flat constant.
//! - **The flashbang must not kill** [L 0x36: explosion_radius +0x08 = 0.0, dmg_mul +0x0C = 0.0]:
//!   `Explode_Propagate` (0x17c518) with radius 0 damages nobody. Regression lock through the throw.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

/// Equips `id` through the dev panel's own `apply` and switches to it with the real weapon-switch
/// input (`Player_WeaponChange` via `switch_to`). Returns the number of ticks the switch took.
fn give_and_switch(scene: &mut nf_view::scene::Scene, id: u8, label: &str) -> usize {
    let (msg, switch_to) = nf_view::panel::apply(scene, &q(&[("do", "weapon"), ("id", &id.to_string()), ("rounds", "5")]));
    eprintln!("[panel] {label}: {msg}");
    assert_eq!(switch_to, Some(id), "the panel must switch to the weapon it gave");
    let mut ticks = 0;
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        ticks += 1;
        if scene.sim.as_ref().unwrap().gun.weapon == id { return ticks; }
    }
    panic!("the switch to {id:#04x} never completed ({label})");
}

/// Throws the held weapon through the real fire path: hold the trigger until the projectile spawns
/// (the release frame = the fire script's `fire_anim_len` [M Player_SetWeaponAnimObj case 9]),
/// then release. Returns the tick the projectile appeared.
fn throw_held(scene: &mut nf_view::scene::Scene, expect_grenade: bool) -> Option<usize> {
    for t in 0..200 {
        let had = scene.sim.as_ref().unwrap().drones.grenades.len();
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        let now = scene.sim.as_ref().unwrap().drones.grenades.len();
        if now > had {
            assert!(expect_grenade, "expected a grenade projectile");
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
            return Some(t);
        }
    }
    None
}

/// nades05: the panel's give list hands out exactly the named weapon, with its own view model and
/// the pinned projectile meshes. Sister assertion inside: the catalog names 0x3A (the user could not
/// find the rocket launcher because it showed as "weapon 0x3a").
#[test]
fn panel_give_list_matches_weapon_data() {
    let iso_v = iso();
    let mut scene = nf_view::load_dev(&iso_v, "07000012.bin", false).expect("load 07000012 (the user's level)");
    let wd = scene.sim.as_ref().unwrap().weapons.clone();
    // The pinned projectile meshes [L ee_start_05.bin @ 0x2bf150 + w*0x10C + 0x5C].
    assert_eq!(wd.projectile_mesh(0x35), 0x0200_0726, "frag grenade projectile mesh [L]");
    assert_eq!(wd.projectile_mesh(0x36), 0x0200_071f, "stun grenade projectile mesh [L]");
    assert_eq!(wd.projectile_mesh(0x3a), 0x0200_074f, "rocket projectile mesh [L]");
    // Distinct first-person models (the user: "a frag grenade spawns what looks to be a stun grenade").
    assert_ne!(wd.first_person(0x35), wd.first_person(0x36), "frag and stun have their own view skins [L]");
    for &(id, label) in &[(0x35u8, "Frag Grenade"), (0x36, "Stun Grenade"), (0x3a, "Scorpion (rocket launcher)")] {
        let ticks = give_and_switch(&mut scene, id, label);
        eprintln!("[panel] {label} (0x{id:02x}) equipped in {ticks} ticks, view model: {}",
            scene.view_models.contains_key(&id));
        assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "{label} must be the equipped weapon");
        assert!(scene.view_models.contains_key(&id), "{label} (0x{id:02x}) needs a first-person view model (the dev panel's 'Every single weapon needs to have a first person model')");
    }
    // The catalog JSON must not leave the rocket launcher unnamed (the user's "no clue what the
    // rocket launcher is"): no bare "weapon 0x" fallback for the ids the panel lists.
    let cat = nf_view::panel::catalog(&scene);
    assert!(!cat.contains("weapon 0x3a"), "the rocket launcher (0x3A) must be named in the panel: {cat}");
}

/// The frag must bounce and then SIT on the ground until its 210-tick fuse [M Bullet_CollisionHandler
/// 0x12ce80: a slow bounce or the 20th bounce zeroes the speed; only the fuse explodes]. The old port
/// exploded on both — fail-first below. 07000015 because the throw anim scripts (0x06000703/0x704)
/// live only in some level archives [D scan: 07000001 has neither; 07000015 has both] — on a level
/// without them the release frame (fire_anim_len 26 [L 0x35 +0x44]) can never arrive.
#[test]
fn frag_bounces_and_waits_for_the_fuse() {
    let iso_v = iso();
    let mut scene = nf_view::load_dev(&iso_v, "07000015.bin", false).expect("load 07000015");
    give_and_switch(&mut scene, 0x35, "Frag Grenade");
    let thrown_at = throw_held(&mut scene, true).expect("the frag throw must spawn a grenade projectile");
    eprintln!("[frag] thrown at tick {thrown_at}");
    let sim = scene.sim.as_mut().unwrap();
    let g0 = sim.drones.grenades.last().unwrap().clone();
    eprintln!("[frag] spawn pos {:?} vel {:?} fuse {} radius {}", g0.pos, g0.velocity, g0.range, g0.radius);
    assert!((g0.radius - 10.0).abs() < 1e-4, "the frag blast radius is weapon_data+0x08 = 10.0 [L 0x35]");
    // Tick until the explosion (the sim drops exploded grenades from the list; the tick's explosion
    // surfaces in `drones.explosions`), watching the bounce count.
    let mut first_bounce = None;
    let mut exploded_at = None;
    for t in 0..400u32 {
        let b_before = sim.drones.grenades.last().map(|g| g.bounces).unwrap_or(0);
        sim.step_bond(Input::default(), false, false, false);
        if !sim.drones.explosions.is_empty() { exploded_at = Some(t); break; }
        if let Some(g) = sim.drones.grenades.last() {
            if g.bounces > b_before && first_bounce.is_none() { first_bounce = Some((t, g.bounces)); }
        }
    }
    let fb = first_bounce.expect("the thrown frag must bounce off the ground at least once");
    eprintln!("[frag] first bounce at tick {} (count {})", fb.0, fb.1);
    let exploded_at = exploded_at.expect("the frag must explode at its fuse");
    eprintln!("[frag] exploded at tick {exploded_at} (fuse {})", g0.range);
    // The game: a bounced grenade NEVER explodes from the bounce — it sits and pops at the fuse
    // [M 0x12ce80]. The fuse is 210 ticks [M Bullet_init 0x12c5a0:103]. The old port exploded on the
    // decayed bounce (well before the fuse on flat ground).
    assert!(exploded_at >= (g0.range as u32).saturating_sub(10),
        "the frag must explode at its ~{}-tick fuse, not at tick {} (the bounce must never explode it)",
        g0.range as u32, exploded_at);
}

/// The stun grenade explodes with radius 0.0 [L 0x36 +0x08] — `Explode_Propagate` (0x17c518) damages
/// nobody, so a flashbang can never kill ("The flashbang kills enemies" must stay impossible here).
/// Sister: the same throw with the FRAG carries radius 10.0 [L 0x35 +0x08] and its explosion damages
/// a guard standing at the blast.
#[test]
fn stun_grenade_never_kills_but_frag_does() {
    let iso_v = iso();
    let mut scene = nf_view::load_dev(&iso_v, "07000015.bin", false).expect("load");
    give_and_switch(&mut scene, 0x36, "Stun Grenade");
    scene.sim.as_mut().unwrap().bond.pitch = -0.25;
    throw_held(&mut scene, true).expect("stun throw");
    let mut stun_exploded = false;
    let mut stun_radius = f32::NAN;
    for _ in 0..400u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        if let Some(&(_, r, _, _, wt)) = sim.drones.explosions.iter().find(|&&(_, _, _, _, wt)| wt == 0x36) {
            stun_exploded = true; stun_radius = r; break;
        }
    }
    eprintln!("[stun-vs-frag] stun exploded: {stun_exploded} radius {stun_radius}");
    assert!(stun_exploded, "the stun grenade must explode at its fuse");
    assert!(stun_radius == 0.0, "the stun blast radius is weapon_data+0x08 = 0.0 [L 0x36]");
    // Radius 0 → `Explode_Propagate` damages nobody: a guard AT the blast takes nothing.
    let hits = nf_game::grenade::explode_propagate([0.0; 3], stun_radius, 100.0, &[(0, [0.5, 0.0, 0.5])], [50.0; 3], 1.8);
    assert!(hits.drones.is_empty() && hits.player == 0.0, "radius 0 must damage nobody [M 0x17c518]");
    // Sister: the frag's radius 10.0 [L] at the same spot damages the guard (the harness discriminates).
    let hits = nf_game::grenade::explode_propagate([0.0; 3], 10.0, 100.0, &[(0, [0.5, 0.0, 0.5])], [50.0; 3], 1.8);
    assert_eq!(hits.drones.len(), 1, "the frag's blast (radius 10) reaches the guard 1 unit away");
    assert!(hits.drones[0].1 > 0.0);
}

/// "Every single weapon needs to have a first person model": every selectable weapon whose record
/// carries a first-person skin (+0xDC) must have a view model after the dev merge on the user's
/// level. Prints the full gap list (the playtest's "still missing models").
#[test]
fn every_fp_weapon_has_a_view_model_on_07000012() {
    let iso_v = iso();
    let scene = nf_view::load_dev(&iso_v, "07000012.bin", false).expect("load 07000012");
    let wd = &scene.sim.as_ref().unwrap().weapons;
    let mut missing = Vec::new();
    let mut checked = 0;
    for w in 1..0x73u8 {
        if !wd.selectable(w) || wd.first_person(w as usize) == 0 { continue; }
        checked += 1;
        if !scene.view_models.contains_key(&w) { missing.push(w); }
    }
    eprintln!("[view-models] {checked} selectable weapons with fp skins, {} without a view model: {:?}",
        missing.len(), missing.iter().map(|w| format!("0x{w:02x}")).collect::<Vec<_>>());
    // The user's daily drivers must never be missing.
    for w in [0x02u8, 0x35, 0x36, 0x3a] {
        assert!(scene.view_models.contains_key(&w), "weapon 0x{w:02x} has no first-person view model on 07000012");
    }
}
