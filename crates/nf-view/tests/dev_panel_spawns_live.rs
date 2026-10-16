//! dev-panel-spawns — the dev panel spawns what it says. The catalog rows (id, the game's own
//! label name, class grouping, projectile mesh) are checked against the game's own `weapon_data`
//! table [L ee_start_05.bin @0x2bf150, 115 records × 0x10C; docs/fields/weapon_data.md], the F2
//! menu's pages select by the game's own two cycles (`Player_WeaponChange` 0x1a6f80: weapons
//! 0x01..0x46, gadgets 0x4A..0x5E — disasm 0x1a707c/0x1a72ac-b0 [M]), the give runs the level
//! pickups' own take path (`Inventory::take`, the port shape of `Pickup_Handler` 0x18e650), and
//! the thrown grenades draw their own records' +0x5C mesh (`Bullet_update` 0x12e7b0 disasm
//! 0x12e8f0 `lw a1,0x5c(v0)` → `hashtable_set_object_to_entity_gfx` [M]): the frag gives 0x34
//! with the frag mesh 0x020002af, the stun 0x35 with 0x02000726 [L].

use nf_view::panel;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A minimal WEAPON-kind pickup — the take path reads kind/weapon/rounds only [M 0x18e650 case 0].
fn weapon_pickup(id: u8, rounds: i16) -> nf_game::pickup::Pickup {
    nf_game::pickup::Pickup {
        kind: nf_game::pickup::kind::WEAPON,
        weapon: id as u16,
        rounds,
        model: 0,
        bounds: nf_game::pickup::Model { centre: [0.0; 3], radius: 0.0 },
        m: [[0.0; 4]; 4],
        start: [0.0; 3],
        vel: [0.0; 3],
        state: 0,
        lifetime: 0,
        switch_channel: 0,
        sound: None,
        hidden: false,
        gone: false,
        from: None,
        obj: None,
        respawn: 0,
        respawn_ticks: 0,
        spin: false,
        label: 0,
    }
}

/// Every panel row matches its `weapon_data` record: the game's own label name, the record's own
/// meshes, the game's own class grouping, and the raw +0x06 class byte; the AI/drop records
/// outside the two cycles are not rows at all.
#[test]
fn panel_entries_match_the_weapon_table() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let entries = panel::catalog_entries(&scene);
    assert!(entries.len() >= 60, "the catalog lists {} entries", entries.len());
    let sim = scene.sim.as_ref().unwrap();
    let wd = &sim.weapons;
    let norm = |h: u32| if h == u32::MAX { 0 } else { h };
    let fallback = |e: &panel::Entry| if e.id == 0x01 { "Fists".to_string() } else { format!("{} {:#04x}", if e.gadget { "gadget" } else { "weapon" }, e.id) };
    let ids: Vec<u8> = entries.iter().map(|e| e.id).collect();
    for e in &entries {
        let rec = &wd.records[e.id as usize];
        let u32_at = |o: usize| u32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
        // The name is the game's own label text (`Txt_BindLabel(+0x38)` — the SP pickup/HUD lane,
        // Pickup_Handler 0x18e650:246 / HUD_UpdateAmmoPane 0x19a7f0 [M]).
        let label = u32_at(0x38);
        let want = if label != 0 && label != 0xFFFFFFFF {
            let t = sim.lang.as_ref().and_then(|l| l.bind(label)).unwrap_or("").trim().to_string();
            if t.is_empty() { fallback(e) } else { t }
        } else {
            fallback(e)
        };
        assert_eq!(e.name, want, "entry {:#04x}: the name must be the game's own label", e.id);
        assert_eq!(e.held, norm(u32_at(0x7c)), "entry {:#04x}: held is the record's +0x7C", e.id);
        assert_eq!(e.tp, wd.model(e.id as usize), "entry {:#04x}: tp is the record's +0x80", e.id);
        assert_eq!(e.proj, norm(wd.projectile_mesh(e.id as usize)), "entry {:#04x}: proj is the record's +0x5C", e.id);
        assert_eq!(e.cls, rec[6], "entry {:#04x}: cls is the record's +0x06", e.id);
        // The class grouping is the game's own cycles [M 0x1a6f80].
        assert_eq!(e.gadget, (0x4a..=0x5e).contains(&e.id), "entry {:#04x}: the gadget flag must be the game's gadget cycle", e.id);
        assert!((1..=0x46).contains(&e.id) || (0x4a..=0x5e).contains(&e.id), "entry {:#04x} sits outside both cycles", e.id);
    }
    // The records the game never equips on Bond are not rows (the user's "I can't tell what's
    // what"): the padding, the AI rocket record 0x62, and the 0x68..0x6F drop/AI variants that
    // borrow OTHER weapons' name labels (0x69 binds the Smoke Grenade label, 0x6A/0x6E the
    // Q-Claw's, 0x6D the AT-600 Scorpion's, 0x6F the Militek MGL's [L ee_start_05.bin 0x2bf150]).
    for noise in [0x48u8, 0x49, 0x62, 0x68, 0x69, 0x6a, 0x6d, 0x6e, 0x6f] {
        assert!(!ids.contains(&noise), "{noise:#04x} must not be a panel row (outside both cycles)");
    }
    // The four throwables say the game's own names, carry the explosive class and their own meshes.
    let gren = |id: u8| entries.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("{id:#04x} must be a panel row"));
    assert_eq!(gren(0x34).name, "Fragmentation Grenade");
    assert_eq!(gren(0x35).name, "Stun Grenade");
    assert_eq!(gren(0x36).name, "Smoke Grenade");
    assert_eq!(gren(0x37).name, "Remote Mine");
    for id in [0x34u8, 0x35, 0x36, 0x37] {
        assert_eq!(gren(id).cls, 4, "{id:#04x}: the game's class byte is 4 = explosive [L +0x06; BOTSTATE_isPreferredWeapon 0x129580's pref-5 arm]");
    }
    assert_eq!(gren(0x34).proj, 0x0200_02af, "the frag's projectile mesh [L +0x5C]");
    assert_eq!(gren(0x35).proj, 0x0200_0726, "the stun's projectile mesh [L +0x5C]");
    assert_ne!(gren(0x34).proj, gren(0x35).proj, "the frag and the stun draw different bodies");
    // The rocket launchers say their own names (the user: "no clue what the rocket launcher is").
    assert_eq!(gren(0x2c).name, "AT-420 Sentinel");
    assert_eq!(gren(0x2e).name, "AT-600 Scorpion");
    assert_eq!(gren(0x2c).proj, 0x0200_0432, "the Sentinel's rocket mesh [L +0x5C]");
}

/// The F2 menu's pages select by the game's own class: Weapons = the non-gadget rows, Gadgets =
/// the gadget cycle, Grenades = the four throwables at x5.
#[test]
fn the_f2_pages_group_by_the_games_own_cycles() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let mut dev = nf_view::dev::Dev::default();
    dev.page = 1; // Weapons
    let rows = nf_view::dev::menu_items(&dev, Some(&scene));
    for (label, _, action) in &rows {
        if let nf_view::dev::MenuAction::GiveWeapon { id, .. } = action {
            let e = panel::catalog_entries(&scene).into_iter().find(|e| e.id == *id).expect("row's record is in the catalog");
            assert!(!e.gadget, "weapons page row {label} must not be a gadget");
            assert!((1..=0x46).contains(id), "weapons page row {label} outside the game's weapon cycle");
        }
    }
    dev.page = 2; // Gadgets
    let rows = nf_view::dev::menu_items(&dev, Some(&scene));
    assert!(rows.len() >= 8, "the gadget page lists the gadget cycle's rows ({})", rows.len());
    for (label, _, action) in &rows {
        if let nf_view::dev::MenuAction::GiveWeapon { id, .. } = action {
            assert!((0x4a..=0x5e).contains(id), "gadgets page row {label} outside the game's gadget cycle [M 0x1a6f80]");
        }
    }
    dev.page = 3; // Grenades/explosives
    let rows = nf_view::dev::menu_items(&dev, Some(&scene));
    let mut given = Vec::new();
    for (_, _, action) in &rows {
        if let nf_view::dev::MenuAction::GiveWeapon { id, rounds: Some(5) } = action {
            given.push(*id);
        }
    }
    given.sort();
    assert_eq!(given, vec![0x34, 0x35, 0x36, 0x37], "the grenades page gives exactly the game's four throwables at x5");
}

/// The give and the level pickups' take land the same inventory — the give IS `Inventory::take`
/// (the port shape of `Pickup_Handler` 0x18e650, the call drone.rs runs for every level pickup).
#[test]
fn the_panel_give_is_the_level_pickup_take() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let wd = scene.sim.as_ref().unwrap().weapons.clone();
    for id in [0x34u8, 0x16, 0x2c] {
        let (n, a) = { let sim = scene.sim.as_ref().unwrap(); (sim.drones.inventory.clip.len(), sim.drones.inventory.carried.len()) };
        // (a) the panel give on a fresh inventory (the page's decimal id + the default rounds).
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.drones.inventory = nf_game::pickup::Inventory::empty(n, a);
            sim.gun.weapon = 0x47;
        }
        let (status, switch) = panel::apply(&mut scene, &[("do".to_string(), "weapon".to_string()), ("id".to_string(), id.to_string())]);
        eprintln!("[panel] {status}");
        assert_eq!(switch, Some(id), "{id:#04x}: the give switches onto it");
        let after_give = {
            let sim = scene.sim.as_ref().unwrap();
            (sim.drones.inventory.owned.clone(), sim.drones.inventory.clip.clone(), sim.drones.inventory.carried.clone())
        };
        // (b) the same WEAPON pickup through `Inventory::take` — the level pickups' own path
        // (`Pickup_Handler` 0x18e650; the game-side call is drone.rs's take with the sim clock).
        let rounds = wd.clip_size(id as usize).max(1);
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.drones.inventory = nf_game::pickup::Inventory::empty(n, a);
        }
        let clock = scene.sim.as_ref().unwrap().drones.frame;
        let mut armour = 0.0f32;
        let taken = scene.sim.as_mut().unwrap().drones.inventory.take(&wd, &weapon_pickup(id, rounds), true, &mut armour, clock);
        assert!(matches!(taken, Some(nf_game::pickup::Taken::Weapon { .. })), "{id:#04x}: the reference take must succeed");
        // The panel's ONE documented affordance on top of the take — the twin-clip fill
        // (fix-aims20-modes-lights: the AIMS-20 0x1a's mode press needs the twin 0x1b's slot; the
        // game gives the twin's rounds as a separate pickup) — applied to the reference too, so
        // the comparison pins take+affordance == give and any OTHER drift still fails.
        {
            let sim = scene.sim.as_mut().unwrap();
            let twin = wd.records.get(id as usize).map(|r| r[5] as i8).unwrap_or(0);
            if twin != 0 {
                let t = (id as i32 + twin as i32) as usize;
                if let Some(c) = sim.drones.inventory.clip.get_mut(t) {
                    if *c == 0 { *c = wd.clip_size(t).max(1); }
                }
            }
        }
        let after_take = {
            let sim = scene.sim.as_ref().unwrap();
            (sim.drones.inventory.owned.clone(), sim.drones.inventory.clip.clone(), sim.drones.inventory.carried.clone())
        };
        assert_eq!(after_give, after_take, "{id:#04x}: the panel give and the pickup take land the same inventory");
    }
}

/// The throw through the real input path: the panel-given frag flies with the FRAG's mesh, the
/// stun with the stun's (`Bullet_update` 0x12e8f0 sets `weapon_data[fired]+0x5C` on the bullet
/// object [M]; the port's throw is scene.rs's Grenade { mesh: projectile_mesh(fired) }).
/// Levels follow the throw anims' own availability: the frag's scripts (0x06000703/0x704) live
/// only in some level archives — 07000015 has them, 07000005 does not [D scan; fix-weapons-all],
/// while the stun throws fine on 07000005 (models-weapons' thrown-grenade test).
#[test]
fn the_frag_gives_the_frag_mesh_and_the_stun_the_stun_mesh() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (dec, id, mesh, what, level) in [
        ("52", 0x34u8, 0x0200_02afu32, "frag", "07000015.bin"),
        ("53", 0x35u8, 0x0200_0726, "stun", "07000005.bin"),
    ] {
        let mut scene = if what == "frag" {
            nf_view::load_dev(&iso, level, false).unwrap_or_else(|_| panic!("load {level}"))
        } else {
            nf_view::load(&iso, level, false).unwrap_or_else(|_| panic!("load {level}"))
        };
        let (status, switch) = panel::apply(&mut scene, &[("do".to_string(), "weapon".to_string()), ("id".to_string(), dec.to_string()), ("rounds".to_string(), "5".to_string())]);
        eprintln!("[panel] {status}");
        assert_eq!(switch, Some(id), "{what}: the give switches onto {id:#04x}");
        let sim = scene.sim.as_mut().unwrap();
        assert!(sim.drones.inventory.owned.get(id as usize).copied().unwrap_or(false), "{what}: owned after the give");
        let mut input = nf_game::player::Input { switch_to: switch, ..Default::default() };
        for _ in 0..90 { sim.step_bond(input, false, false, false); if sim.gun.weapon == id { break; } input = Default::default(); }
        assert_eq!(sim.gun.weapon, id, "{what}: drawn");
        let mut thrown = false;
        for _ in 0..180 {
            sim.step_bond(nf_game::player::Input::default(), true, false, false);
            if !sim.drones.grenades.is_empty() { thrown = true; break; }
        }
        assert!(thrown, "{what}: the throw spawned a grenade");
        assert_eq!(sim.drones.grenades[0].weapon_type, id as u16, "{what}: the flying grenade carries the fired id");
        assert_eq!(sim.drones.grenades[0].mesh, mesh, "{what}: the flying grenade draws its own record's +0x5C mesh [M Bullet_update 0x12e8f0]");
    }
}
