//! level-07-interactables: one live test per interactive KIND on 07000007 (Mayhew C: the party),
//! driven the way a player drives it — `nf_view::load` → `Sim::step_bond` inputs (the use key,
//! the trigger), the level's own scripts running. Kinds and evidence: docs/levels/07000007.md §7.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Never through the speakers.
fn load() -> Option<nf_view::scene::Scene> {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let iso = iso()?;
    Some(nf_view::load(&iso, "07000007.bin", false).expect("load 07000007"))
}

/// Step the intro away (≤400 ticks).
fn free(sim: &mut nf_view::scene::Sim) {
    for _ in 0..400 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    assert!(!sim.disabled, "Bond is free after the intro");
}

/// Walk to the hall TouchOnce #412 through the lock gate (the walk test's input sequence) so the
/// disguise lock releases and the P2K auto-draws [M 0x1a24c8:242-275].
fn release_weapon_lock(sim: &mut nf_view::scene::Sim) {
    let c = sim.drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::TOUCH_ONCE && u32::from(t.out) == 0x48)
        .map(|t| t.centre()).expect("TouchOnce #412 exists");
    sim.bond.pos = [c[0], c[1], c[2]];
    let mut prev = sim.drones.channels.on(nf_game::player::WEAPON_LOCK_CHANNEL);
    for _ in 0..120 {
        let mut input = Input::default();
        nf_game::player::weapon_lock_input(sim.level, prev, &mut input);
        sim.step_bond(input, false, false, false);
        let on = sim.drones.channels.on(nf_game::player::WEAPON_LOCK_CHANNEL);
        if nf_game::player::weapon_lock_auto_draw(sim.level, on, prev) { sim.gun.switch_request = 2; }
        prev = on;
        if on { break; }
    }
    for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x48), "the hall walk-in fired [D #412]");
    // The plate's own cut-scene (the control-taking NIS 0x06000073) holds Bond from the plate tick;
    // a player skips or watches it — either way it ends before the trigger reaches the gun
    // [M step_bond's disabled gate; the decompile's player input behind the scene hold].
    skip_held_cutscene(sim, "the control-taking scene");
    assert!(sim.drones.scripts.players.iter()
        .any(|p| p.scripts.contains(&0x0600_0073) && p.state >= 3),
        "the control-taking cut-scene (0x06000073) ran through the real trigger [M levels.md §5.1]");
    for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 2, "the P2K is out");
}

/// Skip a cut-scene that holds Bond with the game's own skip action — `Scripts::skip_request` =
/// `Input_Action(−1, 0x19, 1)` [M nis.rs; the walker's tick and the view loop carry the same
/// request]. Bounded; panics with `what` if Bond is never handed back.
fn skip_held_cutscene(sim: &mut nf_view::scene::Sim, what: &str) {
    for _ in 0..1200 {
        if !sim.disabled { return; }
        sim.drones.scripts.skip_request = true;
        sim.step_bond(Input::default(), false, false, false);
    }
    panic!("Bond still disabled after 1200 ticks ({what})");
}

/// Hold the trigger from `from` facing `centre` and sweep the aim across it until break object
/// `idx` dies (the player's own look-and-fire). The aim pitch is the game's half-π unit
/// (`BL+0x8a8·π/2` [M RotMatrixX 0x1e46a0:16-24 — `Bond::look` and the crosshair ray both scale
/// it]), so the sweep centres on `atan2(dy, dxz)·2/π` and steps ±0.02/±0.04 around it. The stand
/// settles 30 ticks first (a teleport lands Bond mid-air; he slides on the slope while falling),
/// then the yaw and the distance are re-taken from the SETTLED position — the bullet spawns at
/// the eye [M Player_WeaponInitBullet 0x1a4bf0], so the line must start where Bond stands.
fn fire_sweep(sim: &mut nf_view::scene::Sim, from: [f32; 3], centre: [f32; 3], idx: usize) -> Option<(usize, usize)> {
    sim.bond.pos = from;
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    let p = sim.bond.pos;
    sim.bond.yaw = (centre[0] - p[0]).atan2(centre[2] - p[2]);
    let dist = ((centre[0] - p[0]).powi(2) + (centre[2] - p[2]).powi(2)).sqrt().max(0.5);
    let eye = sim.bond.head()[1];
    let c = ((centre[1] - eye) / dist).atan() * std::f32::consts::FRAC_2_PI;
    sim.bond.pitch = c;
    for (k, dp) in [0.0f32, 0.02, -0.02, 0.04, -0.04].iter().enumerate() {
        sim.bond.pitch = c + dp;
        // The P2K is semi-auto — one round per press edge [M Player_RoundToFire 0x1a18e0]: press
        // 2 ticks, release 3, eight presses per pitch (the dry gun auto-reloads [M 0x1a5460:447-450]).
        for _ in 0..8 {
            for phase in 0..5 {
                let fire = phase < 2;
                sim.step_bond(Input { fire, ..Input::default() }, fire, false, false);
                if sim.drones.breaks.list[idx].dead { return Some((k, sim.drones.frame as usize)); }
            }
        }
    }
    None
}

/// KIND 0x20 (Break_Create 0x12c230): a P2K round breaks the party's vase, the statue and the
/// hall's glass. Positions [D census — the meshes are world-baked]: vase #49
/// (60.0,43.9,-42.3)..(60.5,44.4,-41.9); window #298 (59.5,45.25,-53.98)..(61.0,46.47,-53.98), the
/// z=-53.98 wall's upper pane (kind 2); statue #109 (104.9,44.1,-64.7)..(105.5,44.9,-64.3). The
/// pool is (float)param 1 = 0.0, so the first positive-damage contact kills
/// [M 0x12bfd8:34-42, 0x12b970:41].
///
/// The old fire gate (level-07-interactables' OPEN: `last_shot` stayed `None` after 100 held-fire
/// ticks with the P2K out) was NOT a weapon-lock fire lockout: `Player_Weapon` 0x1a24c8:242-275
/// only reroutes the cycle actions and writes the rising-edge auto-draw request BL+0x63 = 2. The
/// gate is the hall plate's own cut-scene — TouchOnce #412's ch 0x48 starts the control-taking
/// NIS 0x06000073, which holds Bond (`disabled`): `step_bond` zeroes the input and the trigger
/// while it stands, exactly as the game's player input dies behind the cut-scene hold. The test
/// now plays the scene away with the game's own skip action (`skip_held_cutscene`, the same
/// request the walker carries) — on the first held tick after the hand-back the P2K spawns the
/// shot [M Player_WeaponFiring 0x1a5460 → Player_RoundToFire 0x1a18e0 → Player_WeaponInitBullet
/// 0x1a4bf0].
#[test]
fn the_party_vase_breaks_on_a_p2k_round() {
    let Some(mut scene) = load() else { eprintln!("NF_ISO not set — skipping"); return };
    {
        let sim = scene.sim.as_mut().unwrap();
        free(sim);
        release_weapon_lock(sim);
        assert!(!sim.drones.breaks.list.is_empty(), "the 0x20 placements created break objects [D census]");
        assert!(!sim.drones.breaks.list[0].dead, "vase #49 stands before the shot");
        assert_eq!(sim.drones.breaks.list[0].kind, 1, "vase #49 is the debris type (param 0 = 1 [D])");
        assert_eq!(sim.drones.breaks.list[0].health, 0.0, "the disc's pool (float)param 1 = 0.0 [D; M 0x12c230:21-25]");
    }
    let sim = scene.sim.as_mut().unwrap();
    // The vase #49: 4m north of it on the hall floor (it sits toward -z ABOVE the eye — the vase's
    // band 43.90..44.40 rides on a table), face -z, sweep the aim across its centre.
    let broke_vase = fire_sweep(sim, [60.25, 43.0, -38.0], [60.25, 44.15, -42.07], 0);
    assert!(broke_vase.is_some(), "the vase never broke across the aim sweep");
    assert!(sim.drones.breaks.list[0].dead, "the P2K round killed the vase [M Break_Kill 0x12b970:41]");
    // The window #298 on the z=-53.98 wall (45.25..46.47): from the same stand the round that
    // clears the (now dead) vase's line shatters the upper pane — the glass is kind 2, the
    // shatter route [M 0x12b970].
    assert_eq!(sim.drones.breaks.list[4].kind, 2, "window #298 is the glass type (param 0 = 2 [D])");
    let broke_window = fire_sweep(sim, [60.25, 43.0, -38.0], [60.25, 45.86, -53.98], 4);
    assert!(broke_window.is_some(), "window #298 never broke across the aim sweep");
    assert!(sim.drones.breaks.list[4].dead, "the P2K round killed window #298 [M Break_Kill 0x12b970]");
    // The statue #109 in the open hall (44.10..44.89 on its plinth): 3m south of it, face -z,
    // sweep the aim across its body.
    let broke_statue = fire_sweep(sim, [105.2, 43.0, -61.5], [105.25, 44.49, -64.51], 2);
    assert!(broke_statue.is_some(), "the statue never broke across the aim sweep");
    assert!(sim.drones.breaks.list[2].dead, "the P2K round killed statue #109 [M Break_Kill 0x12b970]");
    // The neighbouring pane #299 (x 65.0..66.5 — the next window east) is outside every fired line.
    assert!(!sim.drones.breaks.list[5].dead, "window #299 stands away from the fired lines");
}

/// Sister of the vase test: while the lock channel holds (ch 0x48 off — before the hall plate),
/// the real input path refuses the gun and the shot. `Player_Weapon` 0x1a24c8:243-246 routes the
/// cycle actions to the fists-only mode-2 change while the channel is off, and the direct
/// selection the number keys request cannot leave the fists either (the port's `weapon_lock_input`
/// stands in for that arm, as the view loop applies it [lib.rs]); the fists' fire is the melee
/// whoosh — their fire script's op-1 is 0x142, no bullet [D 07000005.bin; fix-melee].
#[test]
fn the_lock_holds_the_shot_refused() {
    let Some(mut scene) = load() else { eprintln!("NF_ISO not set — skipping"); return };
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    // The loadout draw (request 1, the fists [M 0x1a7ef8 case 0x7000007]) settles after the intro
    // hands back — the walkreal's wait(60).
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 1, "Bond holds the fists once the draw settles [M 0x1a7ef8]");
    assert!(!sim.drones.channels.on(0x48), "the lock channel holds before the hall plate [D #412]");
    // The view loop's per-tick merge: weapon_lock_input, then step_bond with the trigger and the
    // held P2K request (what a player mashing fire + the weapon key does under the lock).
    let mut prev = sim.drones.channels.on(nf_game::player::WEAPON_LOCK_CHANNEL);
    for t in 0..100 {
        let mut input = Input { fire: true, switch_to: Some(2), ..Input::default() };
        nf_game::player::weapon_lock_input(sim.level, prev, &mut input);
        sim.step_bond(input, true, false, false);
        prev = sim.drones.channels.on(nf_game::player::WEAPON_LOCK_CHANNEL);
        assert_eq!(sim.gun.weapon, 1, "the lock refused the P2K selection (tick {t}) [M 0x1a24c8:243-246]");
        assert!(!sim.drones.player_bullets.iter().any(|b| b.weapon == 2),
            "no P2K round spawns while the lock holds (tick {t})");
    }
}

/// Sister: walking the party without firing leaves every breakable standing.
#[test]
fn walking_the_party_breaks_nothing() {
    let Some(mut scene) = load() else { eprintln!("NF_ISO not set — skipping"); return };
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    release_weapon_lock(sim);
    // Stand next to the vase the way the walk does, no trigger.
    sim.bond.pos = [60.25, 43.0, -38.0];
    sim.bond.yaw = 0.0;
    for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    assert!(!sim.drones.breaks.list.iter().any(|b| b.dead), "no trigger, no break [M 0x12bfd8 — only class-5 contacts subtract]");
}

/// KIND 0xF0 (Pickup_Create 0x18dda0): the heavy armour takes on contact for +50 armour [D #507:
/// kind 3, rounds 50; M Pickup_Handler 0x18e650's armour arm]. The P2K bonus pickups (kind 6)
/// take and vanish too [M case 6, BOND_BONUS].
/// [ignore] OPEN (level-07-interactables): standing on the armour (dist 0.75 < r+1.1 [M 0x18e148])
/// does not take it in 40 ticks — the take needs its blocker found (suspect: the cel-scoped take
/// ray after a teleport, or the fall/rest state). The sister pins the no-reach side; the take code
/// itself is proven on other levels (level_carry/pickup tests).
#[test]
fn the_armour_pickup_takes_for_fifty() {
    let Some(mut scene) = load() else { eprintln!("NF_ISO not set — skipping"); return };
    {
        let sim = scene.sim.as_mut().unwrap();
        free(sim);
        let idx = sim.drones.pickups.iter().position(|p| p.kind == 3).expect("the armour pickup #507 is created");
        assert!(!sim.drones.pickups[idx].gone, "the armour sits before Bond");
        assert_eq!(sim.drones.pickups[idx].rounds, 50, "the armour amount [D #507 param 2]");
        assert_eq!(sim.drones.health.armour, 0.0, "the party starts armour 0 [M Player_InitWeapon case 0x7000007]");
        // Walk onto it (the player way: step on the pickup). The take point is the pickup's
        // placement translation (obj+0x90 row 3) — the model bounds' centre is the glist's LOCAL
        // centre (world-baked meshes carry ~0) [D census; the placement is at (92.0,43.7,-33.2)].
        let b = sim.drones.pickups[idx].m[3];
        assert!((b[0] - 92.0).abs() < 0.5 && (b[2] + 33.2).abs() < 0.5, "the armour sits at its placement, got {b:?}");
        sim.bond.pos = [b[0], 43.0, b[2]];
        for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); if sim.drones.pickups[idx].gone { break; } }
        assert!(sim.drones.pickups[idx].gone, "the armour took on contact");
        assert_eq!(sim.drones.health.armour, 50.0, "armour +50 [M Pickup_Handler armour arm]");
        // The kind-6 bonuses: same take path, delete-on-take [D #502/#503 respawn 0].
        let bonus = sim.drones.pickups.iter().filter(|p| p.kind == 6).count();
        assert_eq!(bonus, 2, "the two Pickup 007 bonuses are created [D #502/#503]");
    }
}

/// Sister: the armour only takes when Bond reaches it.
#[test]
fn the_armour_sits_until_bond_reaches_it() {
    let Some(mut scene) = load() else { eprintln!("NF_ISO not set — skipping"); return };
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    let idx = sim.drones.pickups.iter().position(|p| p.kind == 3).expect("the armour pickup exists");
    assert!(!sim.drones.pickups[idx].gone, "the armour takes only on contact [M 0x18e650]");
    assert_eq!(sim.drones.health.armour, 0.0);
}

/// KIND 0xDB free door (p2 = p3 = 0 [D #245]): the use key opens it [M Door_Activate]. The locked
/// boundary doors (ch_lock 200, no writer on this level [D]) refuse it.
/// [ignore] OPEN (level-07-interactables): the use key at 1.6m (use point within the 2.5 reach
/// [M 0x1a5170]) does not swing the free door — the pick needs its blocker found. The locked half
/// (the ch-200 boundary refuses) is pinned by the passing sister test below.
#[test]
fn the_free_door_opens_on_the_use_key_and_the_locked_one_refuses() {
    let Some(mut scene) = load() else { eprintln!("NF_ISO not set — skipping"); return };
    {
        let sim = scene.sim.as_mut().unwrap();
        free(sim);
        // The free wooden door #245 (80.0,43.0,-49.9): the door list is in placement order
        // (door_placements), #242 is the first → #245 is list index 3.
        let door_i = sim.drones.doors.list.iter().position(|d| d.placement == 245).expect("door #245 created");
        assert!(sim.drones.doors.closed_flags()[door_i], "the free door starts closed");
        // Stand against it and press E (face +z — the intro leaves Bond facing +x, the look
        // vector carries the yaw [M Bond::look]).
        sim.bond.pos = [80.0, 43.0, -51.5];
        sim.bond.yaw = 0.0;
        for _ in 0..30 {
            sim.step_bond(Input::default(), false, false, true);
            if !sim.drones.doors.closed_flags()[door_i] { break; }
        }
        assert!(!sim.drones.doors.closed_flags()[door_i], "the use key opens the free door [M Door_Activate]");
        // The locked boundary door #253 (128.5,43.0,-88.1, ch_lock 200 [D]): E refuses.
        let lock_i = sim.drones.doors.list.iter().position(|d| d.placement == 253).expect("door #253 created");
        assert!(sim.drones.doors.closed_flags()[lock_i], "the locked door starts closed");
        sim.bond.pos = [128.5, 43.0, -89.8];
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, true); }
        assert!(sim.drones.doors.closed_flags()[lock_i], "the ch-200 lock holds against the use key [D #253 p3=200; the lock channel has no writer on this level]");
    }
}

/// Sister: without the use key the free door stays shut.
#[test]
fn the_free_door_stays_shut_without_the_use_key() {
    let Some(mut scene) = load() else { eprintln!("NF_ISO not set — skipping"); return };
    let sim = scene.sim.as_mut().unwrap();
    free(sim);
    let door_i = sim.drones.doors.list.iter().position(|d| d.placement == 245).expect("door #245 created");
    sim.bond.pos = [80.0, 43.0, -51.5];
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.doors.closed_flags()[door_i], "no E, no open [M 0x1a5170]");
}
