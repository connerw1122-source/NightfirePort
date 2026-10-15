//! fix-blast-scripts live tests: each blast plays ITS OWN effect script.
//!
//! The game's `Explode_Create` 0x17cbd0 takes a per-source effect script (its `Script_Load` arg, a3
//! at every call site) — the port's `spawn_blast` played the generic one-shot 0x06000052 for every
//! explosion. The census (docs/reports/fix-blast-scripts.md):
//! - `Bullet_CollisionHandler` 0x12ce80 contact arm (+0x70 & 4): 0x6000052 [M disasm 0x12d088-0x12d0c0]
//! - `Bullet_CollisionHandler` 0x12ce80 bolt arms (+0x70 & 0x100000 / & 0x200000): 0x60007c4 [M
//!   disasm 0x12d0e8-0x12d124, 0x12d14c-0x12d188 — the Phoenix Samurai's bolt 0x32: radius 8,
//!   damage 256, script 0x060007c4; radius float f13 = weapon_data+0x08, damage f14 = +0x0C]
//! - the fuse death `Bullet_handle_object_destruction` 0x12cd98: 0x6000052 [M disasm 0x12cdf0-0x12ce2c]
//! - `Destroy_Smash` 0x12bb60: the tag's type byte (tag+0x16 = placement param 2) picks the script —
//!   1 → 0x6000027 (×4 radius), 2 → 0x600004f (×2), any other non-zero → 0x6000052 (×3); type 0
//!   plays NO script [M disasm 0x12bc50-0x12bd20]. The blast damage is the tag's +0x08 float
//!   (placement param 5) [M disasm 0x12bd04].
//!
//! All four scripts ship in every story archive [D disc census — probe over FILES.BIN kind-7 NIS].
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Give a weapon through the dev panel (the user's `--give` path) and wait for the switch.
fn give_and_switch(scene: &mut nf_view::scene::Scene, id: u8) {
    let (msg, switch_to) = nf_view::panel::apply(scene, &q(&[("do", "weapon"), ("id", &id.to_string()), ("rounds", "5")]));
    eprintln!("[give] {id:#04x}: {msg} -> {switch_to:?}");
    assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id && scene.sim.as_ref().unwrap().gun.switch_state == 0 { return; }
    }
    panic!("the switch to {id:#04x} never completed");
}

/// The one-shot blast players (placement usize::MAX — level script players carry real placements).
fn blast_players(scene: &nf_view::scene::Scene, script: u32) -> Vec<f32> {
    scene.sim.as_ref().unwrap().drones.scripts.players.iter()
        .filter(|p| p.scripts[0] == script && p.placement == usize::MAX)
        .map(|p| p.scale).collect()
}

/// Point the camera at a world point through Bond's yaw/pitch (dir = [cos·sin(yaw), sin(pitch), cos·cos(yaw)] [M player.rs aim]).
fn aim_at(scene: &mut nf_view::scene::Scene, target: [f32; 3], eye_h: f32) {
    let sim = scene.sim.as_mut().unwrap();
    let eye = [sim.bond.pos[0], sim.bond.pos[1] + eye_h, sim.bond.pos[2]];
    let d = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).asin();
}

/// ── The Samurai's bolt plays the game's cyan blast script 0x060007c4 ───────────────────────────
/// The player-reachable Samurai is the beam twin 0x33 (a direct select lands 0x33 — 0x32's owned
/// slot is its base [D census]); the ○ mode switch lands the bolt record 0x32 [M fire_mode_switch:
/// 0x33's twin −1 → 0x32], whose +0x70 = 0x100000 [L the ACTION.ELF records]: `Bullet_Collision
/// Handler`'s 0x100000 arm explodes the hit with `Explode_Create(radius f13 = weapon_data+0x08 =
/// 8, damage f14 = +0x0C = 256, script a3 = 0x60007c4)` [M disasm 0x12d0e8-0x12d124]. Pre-fix the
/// port played NO blast for the bolt (the only world-hit explode is the generic-script bit-2 arm
/// and 0x32 has no bit 2).
#[test]
fn the_samurai_bolt_plays_the_cyan_blast_script() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x32, 0x33]).expect("load");
    give_and_switch(&mut scene, 0x33);
    // The meter weapon ships no spare rounds (the give fills the 0x33 meter only) — top the bolt's
    // slots up directly (samurai_no_reload_script's carried-ammo precedent): ammo type 31 shared,
    // and the port's meter slot for 0x32 (the game's own refill gates only cover clip[0x33] [M
    // Player_Update 0x193eb8-0x19405c — a twin-meter gap documented in the report]).
    {
        let sim = scene.sim.as_mut().unwrap();
        let t = sim.weapons.ammo_type(0x32);
        if let Some(n) = sim.drones.inventory.carried.get_mut(t) { *n = (*n).max(16); }
        if let Some(c) = sim.drones.inventory.clip.get_mut(0x32) { *c = (*c).max(100); }
    }
    // ○ to the bolt twin 0x32 (the MGL 'Delay' test's twin-switch shape [M 0x1a24c8:246-260]).
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x32, "○ on the beam requests the bolt twin 0x32");
    }
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x32 && scene.sim.as_ref().unwrap().gun.switch_state == 0 { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x32, "the bolt twin never landed");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
        sim.bond.pitch = -1.0; // straight down: the floor is the wall
    }
    let mut exploded = None;
    let mut armed = false;
    for t in 0..400u32 {
        let had = scene.sim.as_ref().unwrap().drones.player_bullets.len();
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), !armed, false, false);
        let now = scene.sim.as_ref().unwrap().drones.player_bullets.len();
        if now > had { armed = true; } // the bolt is out: release (the game's own pull shape)
        if !scene.sim.as_ref().unwrap().drones.explosions.is_empty() { exploded = Some(scene.sim.as_ref().unwrap().drones.explosions.clone()); break; }
    }
    let ex = exploded.expect("the Samurai bolt must explode on the floor contact [M 0x12ce80:207-215]");
    assert_eq!(ex.len(), 1, "exactly one blast");
    assert!((ex[0].1 - 8.0).abs() < 1e-3, "the bolt blast radius is weapon_data+0x08 = 8.0 [L 0x32], got {}", ex[0].1);
    assert!((ex[0].2 - 256.0).abs() < 1e-3, "the bolt blast damage is weapon_data+0x0C = 256.0 [L 0x32], got {}", ex[0].2);
    // The blast plays the game's script 0x060007c4, scale = the game's f12 = 1.0 [M disasm
    // 0x12d0ec-0x12d0f0 (lui $at, 0x3f80; mtc1 $at, $f12) + 0x12d118 (lwc1 $f13, 8($t2))]: the
    // bolt's 0x100000 arm passes Explode_Create(scale f12 = 1.0, radius f13 = +0x08 = 8.0,
    // damage f14 = +0x0C) — the script scale is 1.0 while the blast radius stays 8.0. (The 0x4
    // arms pass f12 = +0x08 — scale = radius — which is why the frag/rocket sisters below pin 8.0.)
    let cyan = blast_players(&scene, 0x0600_07c4);
    assert_eq!(cyan.len(), 1, "exactly one 0x060007c4 one-shot player (the bolt's cyan blast)");
    assert!((cyan[0] - 1.0).abs() < 1e-3, "the blast player's scale is the game's f12 = 1.0 [M 0x12d0ec-0x12d0f0], got {}", cyan[0]);
    // Sister: the generic blast script is NOT what the bolt spawned (the old hardcoded one-shot;
    // the fire-modes3-shaped first world arm caught the bolt bits and pushed 0x06000052 — this is
    // the assert that fails on the pre-fix merged tree).
    assert!(blast_players(&scene, 0x0600_0052).is_empty(), "the bolt must not play the generic 0x06000052");
}

/// ── Sister: the frag's fuse blast and the Sentinel rocket's contact blast keep the generic script ─
/// The fuse death 0x12cd98 and the contact arm 0x12ce80:194-206 both pass 0x6000052 [M disasm
/// 0x12cdf0-0x12ce2c, 0x12d088-0x12d0c0]; the frag record 0x34 = radius 8 / damage 100 [L].
#[test]
fn the_frag_and_the_rocket_keep_the_generic_blast_script() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The frag: a live Grenade (the sim's own struct, the fuse expiry the input throw feeds) with
    // the record's 8/100, fuse 10.
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
        let at = [sim.bond.pos[0] + 2.0, sim.bond.pos[1] + 1.5, sim.bond.pos[2]];
        let mesh = sim.weapons.model(0x34);
        sim.drones.grenades.push(nf_game::grenade::Grenade {
            pos: at, velocity: [0.05, 0.0, 0.0], speed: 0.05, travelled: 0.0, range: 10.0,
            damage: 100.0, radius: 8.0, weapon_type: 0x34, owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, resting: false, stuck: false, seek_target: None, mesh, phase: None,
        });
    }
    let mut frag_exploded = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if !scene.sim.as_ref().unwrap().drones.explosions.is_empty() { frag_exploded = true; break; }
    }
    assert!(frag_exploded, "the frag never exploded");
    let generic = blast_players(&scene, 0x0600_0052);
    assert_eq!(generic.len(), 1, "the frag's fuse blast plays 0x06000052 [M 0x12cd98 disasm]");
    assert!((generic[0] - 8.0).abs() < 1e-3, "the frag blast scale is the radius 8.0, got {}", generic[0]);
    assert!(blast_players(&scene, 0x0600_07c4).is_empty(), "no cyan blast for the frag");

    // The Sentinel rocket (0x2c, +0x70 bit 2): the contact explode plays the same generic script.
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    give_and_switch(&mut scene, 0x2c);
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
        sim.bond.pitch = -1.0;
    }
    let mut rocket_exploded = false;
    let mut armed = false;
    for t in 0..400u32 {
        let had = scene.sim.as_ref().unwrap().drones.player_bullets.len();
        scene.sim.as_mut().unwrap().step_bond(Input::default(), !armed, false, false);
        if scene.sim.as_ref().unwrap().drones.player_bullets.len() > had { armed = true; }
        if !scene.sim.as_ref().unwrap().drones.explosions.is_empty() { rocket_exploded = true; break; }
    }
    assert!(rocket_exploded, "the Sentinel's rocket must explode on the floor contact [M 0x12d0bc]");
    let generic = blast_players(&scene, 0x0600_0052);
    assert_eq!(generic.len(), 1, "the rocket's contact blast plays 0x06000052 [M 0x12d088-0x12d0c0]");
    assert!((generic[0] - 8.0).abs() < 1e-3, "the rocket blast scale is the radius 8.0 [L 0x2c], got {}", generic[0]);
    assert!(blast_players(&scene, 0x0600_07c4).is_empty(), "no cyan blast for the rocket");
}

/// ── The destroyable's smash plays its type's script; a type-0 smash plays none ──────────────────
/// `Destroy_Smash` 0x12bb60 explodes the smash with the tag type byte's script [M disasm
/// 0x12bc50-0x12bd20]: type 1 → 0x6000027, type 2 → 0x600004f, other non-zero → 0x6000052, and a
/// type-0 tag plays NO script. 07000003's destroy[0] (placement #266) is type 1 with health 0 [D
/// the archive's 0xe1 placements' params 2/3]; 0700000d's destroy[10] (placement #470) is type 0
/// with health 0 [D].
#[test]
fn the_destroyable_smash_plays_its_type_script() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // 07000003: stand on the floor 3 units from destroy[0] (type 1) and shoot it with the P2K
    // (health 0: the first contact smashes [M 0x12bd48 health < 0]; no fire while falling — the
    // teleport lands, then the trigger).
    let mut scene = nf_view::load(&iso, "07000003.bin", false).expect("load");
    give_and_switch(&mut scene, 0x02);
    let t1 = [30.454449, 6.177959, 5.269394]; // destroy[0]'s bounds centre
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.disabled = false; // the boot script's freeze zeroes the trigger (models-misc precedent)
        sim.drones.health.health = 500.0; // the fix's own smash blast propagates to Bond at 3 units (game-accurate) — survive it
        sim.bond.pos = [28.0262, 5.778, 3.5114]; // the floor spot 3 units out (probe-measured)
        sim.bond.vel = [0.0; 3];
    }
    // Land first (no fire while falling): step until grounded, like the probe.
    for _ in 0..120 {
        let sim = scene.sim.as_mut().unwrap();
        if sim.bond.grounded { break; }
        sim.bond.vel = [0.0; 3];
        sim.step_bond(Input::default(), false, false, false);
    }
    assert!(scene.sim.as_ref().unwrap().bond.grounded, "Bond never landed on the floor spot");
    aim_at(&mut scene, t1, 1.6);
    let mut smashed = false;
    for t in 0..60u32 {
        aim_at(&mut scene, t1, 1.6);
        scene.sim.as_mut().unwrap().step_bond(Input::default(), t < 3, false, false);
        if scene.sim.as_ref().unwrap().drones.props.destroy.iter().filter(|d| d.dead).count() > 0 { smashed = true; break; }
    }
    assert!(smashed, "the type-1 destroyable never smashed");
    let monitor = blast_players(&scene, 0x0600_0027);
    assert_eq!(monitor.len(), 1, "the type-1 smash plays 0x0600027 [M 0x12bc74-0x12bc7c]");
    assert!(blast_players(&scene, 0x0600_07c4).is_empty() && blast_players(&scene, 0x0600_004f).is_empty(), "no stray blast scripts on the type-1 smash");

    // Sister on 0700000d: the type-0 destroyable's smash spawns NO blast player at all [M 0x12bc50
    // `if (cVar4 != 0)`] — the smash (dead + channel) still runs.
    let mut scene = nf_view::load(&iso, "0700000d.bin", false).expect("load");
    // The panel give (the switch itself may not land — the boot script re-requests — but the give
    // resets the view state the fire gate needs; the probe measured the smash through this shape).
    let _ = nf_view::panel::apply(&mut scene, &q(&[("do", "weapon"), ("id", "2"), ("rounds", "5")]));
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(2), ..Default::default() }, false, false, false);
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.disabled = false; // the boot script's freeze zeroes the trigger (models-misc precedent)
        sim.drones.health.health = 500.0;
        sim.bond.pos = [1.9717, -0.4, -2.256]; // 3 units from destroy[10]'s box at the origin
        sim.bond.vel = [0.0; 3];
    }
    let t0 = [0.0, 0.0, 0.0];
    aim_at(&mut scene, t0, 1.6);
    for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); aim_at(&mut scene, t0, 1.6); }
    let mut smashed0 = false;
    for t in 0..60u32 {
        aim_at(&mut scene, t0, 1.6);
        scene.sim.as_mut().unwrap().step_bond(Input::default(), t < 30, false, false);
        if scene.sim.as_ref().unwrap().drones.props.destroy.iter().filter(|d| d.dead).count() > 0 { smashed0 = true; break; }
    }
    assert!(smashed0, "the type-0 destroyable never smashed");
    assert!(blast_players(&scene, 0x0600_0027).is_empty(), "the type-0 smash spawned no 0x0600027");
    assert!(blast_players(&scene, 0x0600_0052).is_empty(), "the type-0 smash spawned no generic blast either [M 0x12bc50]");
    assert!(blast_players(&scene, 0x0600_07c4).is_empty() && blast_players(&scene, 0x0600_004f).is_empty(), "no stray blast scripts");
}
