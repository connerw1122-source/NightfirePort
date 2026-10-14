//! fix-pickups-ammo: the pickup layer's take-path fixes, driven through the real Sim path
//! (`nf_view::load` + `Sim::step_bond`, the same walks `Pickup_Update` 0x18e148 /
//! `Pickup_Handler` 0x18e650 run). Each test is failing-first (the bug it pins was live on
//! master 4ea550b7):
//!
//! 1. The take sound for kinds 0/1 is FORCED to 0xF5 — the message switch's cases 0/1 both land
//!    on `LAB_0018ee80: *(ushort*)(info+0x2a) = 0xf5` [M 0x18e650:290-300]. The disc places
//!    weapon/ammo pickups with other sounds (the 07000002 sniper 0x13F, the 0700000c/d 0x1f boxes
//!    0xFFFF, the 07000001/05 id-6 boxes 0x13F [D the placement census]) — the port used to play
//!    the placement's word (or nothing).
//! 2. The ammo path's id is the COMPOSITION `upgrade_ammo_id(upgrade_weapon(raw))` [M
//!    0x18e650:142-144 + case 1; 0x1a8d18:11-19]: the id-6 ammo boxes (07000001/05) credit
//!    weapon_data[2]'s pool — type 1, cap 70 — not record 6's type 2 [L weapon_data]. The
//!    message's ammo NAME stays the RAW id's type (a game quirk [M 0x18e650 case 1 reads
//!    `weapon_data[raw+0x24]+0x90`]).
//! 3. `Pickup_Create`'s level-conditional rounds override [M 0x18dda0: GameState+0x12 ==
//!    0x7000008 && id == 0x2c → rounds = 4]: the vault's rocket pickup is created with 4, not the
//!    placement's 5 — the take grants 4 loaded, 0 spare (cap 14 = 4 [L ammo_data]), not 4+1.
//! 4. A hidden pickup is still takeable — `Pickup_Update`'s take path never reads `obj+0xF0` [M
//!    0x18e148]: the AR drop's invisible 0x1B grenade pile (created with obj+0xF0 |= 0x10 by
//!    `DroneWeap_DropWeapon` 0x17a2a0) is collected walking over the drop.
//! 5. The kind-4 label rides the pickup (+0x34, the placement's param 6) and is the take message
//!    when non-zero [M 0x18e650 case 4]: 0700000b's id-0x47 item carries 0x2000036 [D].
//!
//! Opt-in via `NF_ISO`.

use nf_view::dev::Dev;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    // 07000001's level-start intro (SP #277, 0x06000087) takes control; skip it the game's way
    // (`Script_FFwd` 0x1dac78) so the tests start in control.
    let intro: &[u32] = match level { "07000001.bin" => &[0x0600_0087], _ => &[] };
    if !intro.is_empty() {
        for t in 0..600u32 {
            let sim = scene.sim.as_mut().unwrap();
            let pending = sim.drones.scripts.players.iter().any(|p| intro.contains(&p.scripts[0]) && (matches!(p.state, 1 | 2 | 7) || p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)));
            if t >= 2 && !pending && !sim.disabled { break; }
            sim.drones.scripts.skip_request = true;
            sim.step_bond(Default::default(), false, false, false);
        }
    }
    scene
}

/// God mode only — the infinite-ammo topping would defeat the cap assertions.
fn god() -> Dev { let mut d = Dev::default(); d.god_mode = true; d }

fn step(scene: &mut Scene, trigger: bool, dev: Option<&mut Dev>) {
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), trigger, false, false);
    }
    if let Some(d) = dev { d.after_tick(scene); }
}

/// Stand Bond on pickup `k` (then at the cardinal offsets, for pickups that rest against
/// geometry) and tick until it is taken (`drones.taken` grows). The real take path: the same
/// walk `Pickup_Update`'s touch test runs every tick.
fn take_pickup(scene: &mut Scene, k: usize, mut dev: Option<&mut Dev>) -> Option<nf_game::pickup::Taken> {
    let at = {
        let sim = scene.sim.as_ref().unwrap();
        let p = &sim.drones.pickups[k];
        [p.m[3][0], p.m[3][1] + 0.4, p.m[3][2]]
    };
    for (dx, dz) in [(0.0f32, 0.0f32), (1.5, 0.0), (-1.5, 0.0), (0.0, 1.5), (0.0, -1.5)] {
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, [at[0] + dx, at[1], at[2] + dz], 0.0);
        }
        for _ in 0..40 {
            let before = scene.sim.as_ref().unwrap().drones.taken.len();
            step(scene, false, dev.as_deref_mut());
            let sim = scene.sim.as_ref().unwrap();
            if sim.drones.taken.len() > before { return sim.drones.taken.last().map(|(_, t)| *t); }
        }
    }
    None
}

/// 1 + 2: a dropped guard gun taken twice — the second take is ammo up to the cap — and the
/// id-6 ammo box credits the P2K's type-1 pool while the message names the RAW id's type.
#[test]
fn drops_and_boxes_take_through_the_real_path_with_the_game_numbers() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let dev = god();

    // -- The id-6 ammo boxes (two placements, rounds 20, sound word 0x13F [D the census]). --
    let boxes: Vec<usize> = scene.sim.as_ref().unwrap().drones.pickups.iter().enumerate()
        .filter(|(_, p)| !p.gone && p.kind == nf_game::pickup::kind::AMMO && p.weapon == 6)
        .map(|(k, _)| k).collect();
    assert_eq!(boxes.len(), 2, "07000001 places two id-6 ammo boxes [D]");
    assert_eq!(scene.sim.as_ref().unwrap().drones.pickups[boxes[0]].sound, Some(0x13f), "the placement's sound word is 0x13F [D]");

    let mut dev = dev;
    let t1 = take_pickup(&mut scene, boxes[0], Some(&mut dev)).expect("the first box is taken");
    {
        let sim = scene.sim.as_ref().unwrap();
        // The COMPOSITION: Upgrade_Weapon(6) = 2 [L UpgradedHandguns[0]] → type 1, cap 70. The old
        // port credited type 2 (cap 90) — record 6's own type [L weapon_data].
        assert_eq!(t1, nf_game::pickup::Taken::Ammo { weapon: 6, rounds: 20 }, "41 + 20 < cap 70: the whole box is taken");
        assert_eq!(sim.drones.inventory.carried[1], 61, "the rounds land in the type-1 pool");
        assert_eq!(sim.drones.inventory.carried[2], 0, "sister: the type-2 pool stays untouched");
        // The take sound is FORCED 0xF5 (the placement said 0x13F).
        assert!(sim.drones.sounds.iter().any(|s| s.id == 0xf5), "the take plays sound 0xF5, not the placement's 0x13F: {:?}", sim.drones.sounds.iter().map(|s| s.id).collect::<Vec<_>>());
        // The message names the RAW id's ammo type (a game quirk [M 0x18e650 case 1]) with the
        // taken count — the same string `lang.format(0x200000E, [rounds, raw-type name])` builds.
        let sim = scene.sim.as_ref().unwrap();
        let lang = sim.lang.as_ref().expect("the level's text table");
        let raw_type = sim.weapons.ammo_type(6);
        let name = lang.bind(sim.weapons.ammo_name_label(raw_type)).unwrap_or("").to_string();
        let expected = lang.format(0x200_000E, &["20".to_string(), name]);
        assert!(sim.hud.msgs.iter().any(|m| m.text == expected), "the ammo message = the raw id's type name with the taken count: {:?} vs [{expected}]", sim.hud.msgs.iter().map(|m| &m.text).collect::<Vec<_>>());
    }
    // The second box: 61 + 20 clamps at the type-1 cap 70 — the take returns 9, not 20.
    let t2 = take_pickup(&mut scene, boxes[1], Some(&mut dev)).expect("the second box is taken");
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(t2, nf_game::pickup::Taken::Ammo { weapon: 6, rounds: 9 }, "the take returns the rounds actually carried (the cap clamp) [M 0x1a8d18]");
        assert_eq!(sim.drones.inventory.carried[1], 70, "clamped at the type-1 cap 70 [L ammo_data]");
        assert_eq!(sim.drones.inventory.carried[2], 0, "sister: still no type-2 rounds");
    }

    // -- A dropped guard gun taken twice: the second take is ammo up to the cap. --
    // 07000001's Raptor guards hold 0xE (clip 9, type 3, cap 36 [L weapon_data + ammo_data]).
    let raptor = |scene: &Scene| scene.sim.as_ref().unwrap().drones.list.iter().enumerate()
        .find(|(_, dr)| dr.present && !dr.corpse && dr.health > 0.0 && dr.weapon == 0xe)
        .map(|(i, _)| i);
    let mut kills = 0u32;
    let mut first_take = None;
    let mut second_take = None;
    while kills < 8 {
        let Some(gi) = raptor(&scene) else { break };
        if !kill_raptor(&mut scene, gi, Some(&mut dev)) { kills += 1; continue; }
        kills += 1;
        // The drop: the newest non-gone kind-0 pickup of 0xE.
        let drop = scene.sim.as_ref().unwrap().drones.pickups.iter().enumerate()
            .find(|(_, p)| !p.gone && p.kind == nf_game::pickup::kind::WEAPON && p.weapon == 0xe)
            .map(|(k, _)| k);
        let Some(drop) = drop else { continue };
        let t = take_pickup(&mut scene, drop, Some(&mut dev));
        if first_take.is_none() { first_take = t; } else if second_take.is_none() { second_take = t; break; }
    }
    assert_eq!(first_take, Some(nf_game::pickup::Taken::Weapon { weapon: 0xe }), "the first take is the gun (owned, clip 9)");
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.inventory.owned[0xe], "0xE owned after the first take");
        assert_eq!(sim.drones.inventory.clip[0xe], 9, "a full clip [M 0x1a8a58]");
    }
    let Some(nf_game::pickup::Taken::Ammo { weapon, rounds }) = second_take else {
        panic!("the second drop must be ammo (the gun is owned): {second_take:?}");
    };
    assert_eq!(weapon, 0xe);
    assert!((4..=8).contains(&rounds), "the drop rounds = clip 9 · (FRand(0.5)+0.5) truncated [M 0x17a2a0:88-98]: {rounds}");
    {
        let sim = scene.sim.as_ref().unwrap();
        let spare = sim.drones.inventory.carried[3] as i32;
        assert!(spare >= rounds as i32 && spare < 36, "the spare grew by the take, below the type-3 cap 36: {spare}");
        // Take drops to the cap: seed 1 under it (the test's stand-in for many kills) and the next
        // drop is clamped to exactly the missing round; a full pool refuses (the pickup stays).
        let mut sim = scene.sim.as_mut().unwrap();
        sim.drones.inventory.carried[3] = 35;
    }
    let raptor_now = raptor(&scene);
    if let Some(gi) = raptor_now {
        if kill_raptor(&mut scene, gi, Some(&mut dev)) {
            let drop = scene.sim.as_ref().unwrap().drones.pickups.iter().enumerate()
                .find(|(_, p)| !p.gone && p.kind == nf_game::pickup::kind::WEAPON && p.weapon == 0xe)
                .map(|(k, _)| k);
            if let Some(drop) = drop {
                let t = take_pickup(&mut scene, drop, Some(&mut dev));
                let sim = scene.sim.as_ref().unwrap();
                if sim.drones.inventory.carried[3] >= 35 {
                    match t {
                        Some(nf_game::pickup::Taken::Ammo { rounds: 1, .. }) => assert_eq!(sim.drones.inventory.carried[3], 36, "the take is clamped to the cap's last round"),
                        None => assert_eq!(sim.drones.inventory.carried[3], 35, "a full pool refuses: the pickup stays"),
                        other => panic!("at 35/36 the take is 1 round or refused: {other:?}"),
                    }
                }
            }
        }
    }
    assert!(scene.sim.as_ref().unwrap().drones.sounds.iter().any(|s| s.id == 0xf5), "the drop takes play 0xF5 (CreateSimple's own sound, the same forced value)");
}

/// 3: the vault's rocket pickup is created with 4 rounds (not the placement's 5) and the take
/// grants 4 loaded + 0 spare — the HUD's 4/4, not 4/5.
#[test]
fn the_vault_rocket_pickup_carries_four_rounds_not_the_placements_five() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    let mut dev = god();
    {
        let sim = scene.sim.as_ref().unwrap();
        let rocket = sim.drones.pickups.iter()
            .find(|p| !p.gone && p.kind == nf_game::pickup::kind::WEAPON && p.weapon == 0x2c)
            .expect("07000008 places the rocket launcher");
        assert_eq!(rocket.rounds, 4, "Pickup_Create's override [M 0x18dda0]: 4, not the placement's 5 [D]");
    }
    let k = scene.sim.as_ref().unwrap().drones.pickups.iter()
        .position(|p| !p.gone && p.kind == nf_game::pickup::kind::WEAPON && p.weapon == 0x2c).unwrap();
    let t = take_pickup(&mut scene, k, Some(&mut dev)).expect("the rocket is taken");
    assert_eq!(t, nf_game::pickup::Taken::Weapon { weapon: 0x2c });
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.inventory.owned[0x2c], "owned");
    assert_eq!(sim.drones.inventory.clip[0x2c], 4, "a full clip = clip_size 4 [L weapon_data]");
    assert_eq!(sim.drones.inventory.carried[14], 0, "no spare: 4 + 0 old − 4 clip = 0 [M 0x1a8a58] — the old port showed 1 (4/5)");
    // Sister: the take sound is the forced 0xF5 (the placement says 0x13F).
    assert!(sim.drones.sounds.iter().any(|s| s.id == 0xf5), "the rocket take plays 0xF5");
}

/// 4: the AR drop's hidden 0x1B grenade pile is collected walking over the drop — the take path
/// never reads the hidden bit [M 0x18e148]. The pile is constructed exactly like
/// `DroneWeap_DropWeapon` 0x17a2a0 builds it (kind 1, hidden) and the take runs the real walk.
#[test]
fn the_ar_drops_hidden_grenade_pile_is_still_taken() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let mut dev = god();
    // Build the pile at Bond's feet the way DropWeapon does (the same constructor, the same kind).
    let k = {
        let sim = scene.sim.as_mut().unwrap();
        let m = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [sim.bond.pos[0], sim.bond.pos[1] + 1.0, sim.bond.pos[2], 1.0]];
        let mut pile = nf_game::pickup::Pickup::create_simple(&m, 0x1b, 3, nf_game::pickup::kind::AMMO, &sim.weapons, &sim.models)
            .expect("the 0x1B pile builds");
        pile.hidden = true; // DroneWeap_DropWeapon: `obj+0xF0 |= 0x10` [M 0x17a2a0:110-112]
        sim.drones.pickups.push(pile);
        (sim.drones.pickups.len() - 1)
    };
    let _ = k;
    let t = take_pickup(&mut scene, k, Some(&mut dev))
        .expect("the hidden pile is taken (the game's take path has no hidden check)");
    assert!(matches!(t, nf_game::pickup::Taken::Ammo { weapon: 0x1b, .. }), "{t:?}");
    let sim = scene.sim.as_ref().unwrap();
    // `Player_EquipAmmo`'s 0x10-flag clip fill: 0x1B's clip is empty, so the 3 rounds fill the
    // clip slot (0x1B's own — its ammo type 12 differs from the AR base's 5 [L weapon_data]) and
    // the return of 3 leaves no spare [M 0x1a8d18].
    assert_eq!(sim.drones.inventory.clip[0x1b], 3, "the empty clip fills first [M 0x1a8d18]");
    assert_eq!(sim.drones.inventory.carried[12], 0, "no spare: 3 − clip 6 < 1 [M 0x1a8d18]");
    assert!(sim.drones.sounds.iter().any(|s| s.id == 0xf5), "the pile's take plays 0xF5");
    // The renderer still skips it until taken: the tombstone path (`gone`) is set by the respawn
    // tail (respawn 0 for drops) — the pile is gone, not hidden-and-stuck.
    assert!(sim.drones.pickups[k].gone, "consumed like every kind-1 take");
}

/// 5: the kind-4 item pickup carries the placement's label word (param 6 → PICKUPINFO+0x34) and
/// the take's message is the bound label — 0700000b's id-0x47 item carries 0x2000036 [D].
#[test]
fn the_kind_4_item_pickup_shows_its_label_message() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "0700000b.bin");
    let mut dev = god();
    let k = {
        let sim = scene.sim.as_ref().unwrap();
        sim.drones.pickups.iter().position(|p| !p.gone && p.kind == nf_game::pickup::kind::LABEL && p.weapon == 0x47)
            .expect("0700000b places the id-0x47 item (kind 4)")
    };
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.pickups[k].label, 0x2000036, "the placement's label word rides the pickup [D the params]");
    }
    let t = take_pickup(&mut scene, k, Some(&mut dev)).expect("the item is taken (case 4 always consumes)");
    assert_eq!(t, nf_game::pickup::Taken::Label { label: 0x2000036 });
    // The message is the bound label text (non-zero label → Text_AddMsg of the bound string).
    let sim = scene.sim.as_ref().unwrap();
    let lang = sim.lang.as_ref().expect("the level's text table");
    let expected = lang.bind(0x2000036).unwrap_or("").to_string();
    assert!(!expected.is_empty(), "the label binds to text on the disc");
    assert!(sim.hud.msgs.iter().any(|m| m.text == expected), "the take shows the bound label: {:?} vs [{expected}]", sim.hud.msgs.iter().map(|m| &m.text).collect::<Vec<_>>());
}

// ---- helpers ----

/// Kill the Raptor guard `gi` through the real fire path (fix-pickups-trace's kill_guard, god
/// mode only), then let the drop fall and land.
fn kill_raptor(scene: &mut Scene, gi: usize, mut dev: Option<&mut Dev>) -> bool {
    let spots = |gp: [f32; 3], floor: f32| -> Vec<[f32; 3]> {
        let mut v = Vec::new();
        for r in [3.0f32, 4.5] {
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                v.push([gp[0] + r * a.sin(), floor + 1.1, gp[2] + r * a.cos()]);
            }
        }
        v
    };
    let mut spot = 0usize;
    for t in 0..2000 {
        let (gp, gchest) = { let dr = &scene.sim.as_ref().unwrap().drones.list[gi]; (dr.pos, [dr.pos[0], dr.pos[1] + 0.3, dr.pos[2]]) };
        let los = |from: [f32; 3]| scene.sim.as_ref().unwrap().world.line_of_sight([from[0], from[1] + 1.0, from[2]], gchest);
        let floor = scene.sim.as_ref().unwrap().world.floor_below(gp, 6.0).unwrap_or(gp[1]);
        let cands = spots(gp, floor);
        if t == 0 || !los(cands[spot % cands.len()]) {
            spot += 1;
            let Some(next) = (spot..spot + cands.len()).map(|i| i % cands.len()).find(|&i| los(cands[i])) else { continue };
            spot = next;
            let sim = scene.sim.as_mut().unwrap();
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, cands[spot], 0.0);
        }
        {
            let sim = scene.sim.as_mut().unwrap();
            let e = sim.bond.head();
            let d = [gchest[0] - e[0], gchest[1] - e[1], gchest[2] - e[2]];
            let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
            sim.bond.yaw = d[0].atan2(d[2]);
            sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
        }
        step(scene, t % 9 == 0, dev.as_deref_mut());
        if scene.sim.as_ref().unwrap().drones.list[gi].health <= 0.0 { break; }
    }
    let dead = scene.sim.as_ref().unwrap().drones.list[gi].health <= 0.0;
    for _ in 0..90 { step(scene, false, dev.as_deref_mut()); } // the drop falls and lands
    dead
}
