//! fix-weapon-modes: the fire-mode button through the real input path (`Sim::step_bond`).
//!
//! The game's switch is `Player_Weapon` 0x1a24c8's ○ block: `Input_Action(slot, 0xc, 4)` with the
//! view anim idle and not (aiming ∧ +0x68 & 0x40); a twin-linked record (+0x05) switches to
//! `w + twin` through `Player_WeaponHasAmmo` 0x1a16c0 (carried rounds / clip / ammo type 0 — NOT
//! the owned byte), the rest cycles the mode byte `BL + w*0xC + 0x1BB` up to the count at +0x2E.
//! The keyboard binding is M (the pad's ○, action 12).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load_with_carry(iso, "07000005.bin", false, None).expect("load 07000005")
}

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// A mode press on the P2K (weapon 0x02 'Semi', twin +1) requests the silenced twin 0x03
/// ('Silenced'); a second press requests 0x02 back. The gate is has-ammo only: the pair shares
/// ammo type 1, so the carried rounds pass it [M 0x1a16c0 — no owned byte read].
#[test]
fn p2k_mode_button_toggles_the_silenced_twin() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    {
        let sim = scene.sim.as_mut().unwrap();
        // Bond at the level start holds the P2K (2) [L ee_start_05.bin owned = {1,2,0x4a,0x4e,0x50,0x54}].
        sim.gun.weapon = 2;
        sim.gun.switch_request = 2;
        sim.gun.switch_state = 0;
        // Give the twin a clean slate (no switch in flight, view idle).
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x03, "○ on the P2K requests the silenced twin 0x03 (0x02 + twin 1) [M 0x1a24c8:246-260]");
    }
    tick(&mut scene, 200);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.gun.weapon, 0x03, "the switch completes onto the silenced record");
        assert_eq!(sim.gun.switch_state, 0, "and settles idle");
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x02, "○ again toggles back (0x03 + twin −1) [M 0x1a16c0 twin −1]");
    }
}

/// Sister: a weapon without a twin link and with a mode count of 1 (0x0c, the Frinesi's neighbour
/// record — +0x05 = 0, +0x2E = 1 [L ee_start_05.bin]) ignores the button: no request, no mode move.
#[test]
fn a_weapon_without_twin_or_modes_ignores_the_button() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.weapon = 0x0c;
    sim.gun.switch_request = 0x0c;
    sim.gun.reload_state = 0;
    sim.gun.fire_state = false;
    let mut input = Input::default();
    input.mode_switch = true;
    sim.step_bond(input, false, false, false);
    assert_eq!(sim.gun.switch_request, 0x0c, "no twin: no switch request");
    assert_eq!(sim.drones.inventory.fire_mode[0x0c as usize], 0, "mode count 1: the mode byte does not move [M 0x1a24c8:210-215]");
}

/// The Scorpion heat-seeker: the AT-600's 'Heatseeker' record 0x2E carries `+0x6C & 0x20000`
/// [L ee_start_05.bin], so its bullets home — `Bullet_homing` 0x12f358 via `Bullet_update`
/// 0x12e7b0:483. Bond fires past a guard (off to the side, inside the 0.66-dot cone, LOS clear);
/// the bullet's direction bends toward the guard over the next ticks and the guard takes the hit.
/// Sister: the same shot with the 'Unguided' twin 0x2F (no homing bit) flies straight and misses.
#[test]
fn the_heat_seeker_homes_onto_a_guard_and_the_unguided_twin_does_not() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // Two runs: the homing record first, the straight twin as the sister.
    for (weapon, expect_hit) in [(0x2eu8, true), (0x2fu8, false)] {
        let mut scene = load(&iso);
        tick(&mut scene, 30);
        let mut spot = [0.0f32; 3];
        {
            let sim = scene.sim.as_mut().unwrap();
            // Grant the launcher record and its ammo (type 14) through the inventory the game keys.
            let t = sim.weapons.ammo_type(weapon as usize);
            sim.drones.inventory.owned[weapon as usize] = true;
            sim.drones.inventory.clip[weapon as usize] = 4;
            sim.drones.inventory.carried[t] = 20;
            sim.gun.weapon = weapon;
            sim.gun.switch_request = weapon;
            sim.gun.clip = 4;
            sim.gun.spare = 20;
            // Face +Z (the look() convention: yaw 0 → (0,0,1)). The start room's gate pillar walls
            // off everything at dx ≤ +4 from z+6 on [D probe: segment sweep], and the missile's
            // first ticks fly the straight aim ray — no bend can clear a wall 6 units out. So stand
            // Bond past the pillar mouth (the corridor at dx+6 is open from z+4 on) and run the
            // duel there; the guard spot below is scanned at the MUZZLE height (+0.7, the bullet's
            // flight line), not the eye — the gate lintel the eye-ray passes over still blocks the
            // shot.
            let p0 = sim.bond.pos;
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, [p0[0] + 6.0, p0[1], p0[2] + 4.0], 0.0);
            sim.bond.yaw = 0.0;
            sim.bond.pitch = 0.0;
            let p = sim.bond.pos;
            let eye = [p[0], p[1] + 0.7, p[2]];
            let mut placed = false;
            for back in 0..4 {
                let cand = [p[0] + 1.2, p[1] + 1.0, p[2] + 14.0 - back as f32];
                if sim.world.segment(eye, cand).is_none() {
                    if let Some(dr) = sim.drones.list.get_mut(0) {
                        dr.pos = cand;
                        dr.present = true;
                        dr.enabled = true;
                        dr.drawn = true;
                        dr.corpse = false;
                        dr.health = 10.0;
                        // The game's seekable gate needs a live object and a non-WAIT_SWITCH view
                        // state [M 0x12f248: +0xfc != 0, health > 0, +0xf4 != 1]. This level's
                        // drone 0 is a spawner-held template parked in WAIT_SWITCH (1) — stand him
                        // up (ATTACK), or the ENTER arm disdains him (enabled=false) and no class-2
                        // object exists for the scan.
                        dr.state = nf_game::drone::state::ATTACK;
                    }
                    placed = true;
                    break;
                }
            }
            assert!(placed, "no LOS-clear spot for the guard — the test spot needs a different origin");
            spot = sim.drones.list[0].pos;
        }
        // Fire once through the real trigger (press + hold until the round spawns). The guard is
        // pinned to his spot: the teleport stands him on a floorless point (no navmesh there — a
        // real guard's floor snap is placement-driven), so without the pin he falls out of the
        // cone through the floor and no 400-tick chase can catch him. The pin stands in for the
        // floor; the AI keeps him enabled/live around it.
        let mut hit_guard = false;
        for i in 0..400 {
            let fire = i < 12;
            let input = Input::default();
            let sim = scene.sim.as_mut().unwrap();
            sim.step_bond(input, fire, false, false);
            sim.drones.list[0].pos = spot;
            if let Some(imp) = sim.drones.player_impacts.iter().find(|p| matches!(p, nf_game::drone::PlayerImpact::Drone { drone: 0, .. })) {
                let _ = imp;
                hit_guard = true;
                break;
            }
        }
        assert_eq!(hit_guard, expect_hit, "weapon {weapon:#x}: the guard hit verdict (homing record vs the straight twin)");
    }
}

/// The fire-mode HUD label follows the mode byte (`HUD_UpdateAmmoPane` 0x19a7f0 reads
/// `BL + w*0xC + 0x1BB` and binds `weapon_data[w]+0x30 + mode*4`): the launcher records' lane 0
/// differs from their twins' ('Heatseeker' 0x05000047 on 0x2E, 'Unguided' 0x05000015 on 0x2F —
/// [L USATxt.dat via the port's Lang::bind]).
#[test]
fn fire_mode_labels_follow_the_record() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load(&iso);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.weapons.fire_mode_label(0x2e, 0), 0x0500_0047, "'Heatseeker'");
    assert_eq!(sim.weapons.fire_mode_label(0x2f, 0), 0x0500_0015, "'Unguided'");
    assert_eq!(sim.weapons.fire_mode_label(0x02, 0), 0x0500_000e, "the P2K's mode-0 label 'Semi'");
    assert_eq!(sim.weapons.fire_mode_label(0x03, 0), 0x0500_000f, "the silenced twin's 'Silenced'");
}

/// The island loadout grants the Q-Worm: `Player_InitWeapon` 0x1a7ef8 case 0x0700009 equips
/// 0x58 (0) beside the Stunner/Decryptor/Q-Claw/dart/Shaver, and 0x070000a/b re-run it when the
/// handgun is unowned. Sister: 0x07000005 (whose case grants no gadgets beyond the start set).
#[test]
fn the_island_loadout_grants_the_q_worm() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load_with_carry(&iso, "0700000a.bin", false, None).expect("load 0700000a cold");
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.inventory.owned[0x58usize], "the Q-Worm (0x58) is owned on a cold 0700000a [M 0x1a7ef8 case 9]");
    assert!(sim.drones.inventory.owned[0x56usize], "the Decryptor (0x56) too");
    assert!(sim.drones.inventory.owned[0x5busize], "the Shaver (0x5b) too");
}

/// The op-1 per-weapon fire-sound remap (`AnimProcessScriptCmds` 0x1c1b38:251-277) — the id table
/// against the decompile, and the P2K's real id: weapon 0x02 is NOT in the table, so its fire
/// scripts' 0x403 (0x06000823/0x06000824 frame 1 [M WORKLOG 0064]) plays as-is.
#[test]
fn op1_remap_table_matches_the_decompile() {
    let cases: &[(u32, u8, u32)] = &[
        (1, 7, 2), (1, 9, 2),          // id 1 on the 0x07/0x09 twins → 2
        (0x403, 3, 2), (0x403, 5, 2),  // 0x403 on 3/5 → 2
        (0x403, 2, 0x403),             // the P2K (0x02): no remap — 0x403 is the real id
        (0xfa, 0x14, 0x1fb),
        (0x106, 0x1e, 0x284), (0x106, 0x23, 0x284), (0x106, 0x24, 0x106), // 0x1e..0x23 only
        (0x1e1, 0x10, 0x602),
        (4, 0x16, 0x604), (4, 0x17, 0x604), (4, 0x18, 4),
        (0x403, 0x2e, 0x403),          // a weapon outside the table keeps its script id
    ];
    for &(id, w, want) in cases {
        assert_eq!(nf_view::scene::remap_op1_sound_for_test(id, w), want, "op-1 {id:#x} on weapon {w:#x}");
    }
}
