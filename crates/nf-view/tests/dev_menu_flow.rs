//! Headless NF_ISO tests for the in-window dev menu (fix-ingame-dev-menu): the F2 menu now carries
//! ALL of the browser panel's functions, so each panel flow must work through the REAL menu path —
//! `dev::menu_key` / `dev::menu_type` / `dev::menu_activate` (the exact functions the window's key
//! handler calls), applied through `dev::apply_menu_action_to_scene` (the same router
//! `App::apply_menu_action` uses), with every give/spawn/clear command landing in `panel::apply`
//! (the shared command layer — the menu cannot drift from `--panel`'s HTTP page).
//!
//! - `menu_open_weapons_enter_on_frag_gives_0x34_and_draws_it` — the task's flow: open, navigate to
//!   Weapons, Enter on the frag grenade: the inventory has 0x34 and it's drawn (the gun becomes it).
//! - `typing_laser_finds_the_q_laser` — type-to-filter narrows the weapons page to the laser rows.
//! - `menu_teleport_page_moves_bond_to_the_display_vent_and_he_grounds` — the Teleport page moves
//!   Bond to the named spot and he's grounded after 60 ticks.
//! - `spawn_with_the_menu_open_while_paused_is_safe` — the user's 07000015 crash (pausing to spawn
//!   grenades from the panel): give grenades + spawn an NPC with the menu OPEN (sim paused), then
//!   resume — no panic, the inventory and the NPC are right.
//! - `every_panel_command_has_a_menu_row` — nothing from the browser panel is left behind.
//!
//! Opt-in via `NF_ISO`.

use nf_view::dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A loaded scene (Bond at the level start) — the same path the viewer uses.
fn loaded(name: &str) -> Option<nf_view::scene::Scene> {
    let iso = iso()?;
    Some(nf_view::load(&iso, name, false).expect(name))
}

/// The router's eye/yaw arguments, from the loaded sim (what App::apply_menu_action passes).
fn eye_yaw(scene: &nf_view::scene::Scene) -> ([f32; 3], f32) {
    let sim = scene.sim.as_ref().expect("sim");
    (sim.bond.head(), sim.bond.yaw)
}

/// The task's flow: open the menu, navigate to Weapons, type to the frag grenade, Enter — the
/// inventory has 0x34 and it's drawn.
#[test]
fn menu_open_weapons_enter_on_frag_gives_0x34_and_draws_it() {
    let Some(mut scene) = loaded("07000005.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let mut d = dev::Dev::default();
    d.menu = true;
    // The menu opens on the category index; row 0 is Weapons. Enter (the key handler's
    // `menu_activate`) opens the page.
    assert!(matches!(dev::menu_activate(&d, Some(&scene)), Some(dev::MenuAction::Page(1))), "index row 0 = Weapons");
    // Enter (menu_key returns the action; the key handler applies it — here through the shared router).
    let (eye, yaw) = eye_yaw(&scene);
    let a = dev::menu_key(&mut d, dev::MenuKey::Activate).expect("the Weapons row action");
    dev::apply_menu_action_to_scene(&mut d, &mut scene, a, eye, yaw, false);
    assert_eq!(d.page, 1, "the Weapons page");
    // Type-to-filter: 'f','r','a','g' through the same `menu_type` the key handler feeds.
    for c in "frag".chars() { dev::menu_type(&mut d, c); }
    let rows = dev::menu_items(&d, Some(&scene));
    let row = rows.iter().find(|(l, _, _)| l.contains("0x34"))
        .unwrap_or_else(|| panic!("no frag grenade row under filter {:?}: rows {:?}", d.filter, rows.iter().map(|(l, _, _)| l).collect::<Vec<_>>()));
    // The game's own name (Txt_BindLabel(+0x38)), not a hardcoded table: 0x34 is the FRAG grenade.
    assert!(row.0.to_ascii_lowercase().contains("frag"), "the 0x34 row carries the game's own name: {}", row.0);
    assert!(matches!(row.2, dev::MenuAction::GiveWeapon { id: 0x34, .. }), "the row gives 0x34");
    // Enter on it (the filtered list's first row — menu_type reset the selection).
    let action = dev::menu_activate(&d, Some(&scene)).expect("an action on the filtered row");
    let dev::MenuAction::GiveWeapon { id, rounds } = action else { panic!("expected GiveWeapon, got {action:?}") };
    assert_eq!(id, 0x34);
    // Apply through the shared router (what App::apply_menu_action does) — it must land in
    // panel::apply's "weapon" arm: equip + full clip + the switch request back.
    let out = dev::apply_menu_action_to_scene(&mut d, &mut scene, action, eye, yaw, false);
    assert_eq!(out.switch_to, Some(0x34), "the router returns the weapon to draw");
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.inventory.owned.get(0x34).copied().unwrap_or(false), "the inventory has 0x34");
    assert!(sim.drones.inventory.clip.get(0x34).copied().unwrap_or(0) > 0, "the clip slot is filled");
    // Sister: the STUN grenade (0x35) is NOT owned — the old panel's "Frag" button gave 0x35; the
    // menu's rows are the bank's own names, so the mix-up cannot come back.
    assert!(!sim.drones.inventory.owned.get(0x35).copied().unwrap_or(false), "0x35 (stun) untouched");
    // It's drawn: the switch request consumed by the real tick path puts the grenade in Bond's hands.
    let drawn = {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = nf_game::player::Input::default();
        input.switch_to = Some(0x34);
        sim.step_bond(input, false, false, false);
        for _ in 0..30 {
            if sim.gun.weapon == 0x34 { break; }
            sim.step_bond(Default::default(), false, false, false);
        }
        sim.gun.weapon
    };
    assert_eq!(drawn, 0x34, "the frag grenade is drawn after the switch");
    if scene.view_models.contains_key(&0x34) {
        eprintln!("[dev-menu] 07000005 archives the 0x34 first-person model (it renders)");
    } else {
        eprintln!("[dev-menu] note: 07000005 has no 0x34 FP skin in its archive (the switch still drew it)");
    }
}

/// Type-to-filter: typing "laser" finds the Q-Laser row (the game's own name from the bank), and
/// the filter actually narrows.
#[test]
fn typing_laser_finds_the_q_laser() {
    let Some(mut scene) = loaded("07000005.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let mut d = dev::Dev::default();
    d.menu = true;
    dev::menu_key(&mut d, dev::MenuKey::PageDown); // index → weapons
    assert_eq!(d.page, 1);
    let unfiltered = dev::menu_items(&d, Some(&scene)).len();
    assert!(unfiltered > 5, "the weapons page lists the level's weapons ({unfiltered} rows)");
    for c in "laser".chars() { dev::menu_type(&mut d, c); }
    let rows = dev::menu_items(&d, Some(&scene));
    let laser_rows: Vec<&String> = rows.iter().map(|(l, _, _)| l).filter(|l| l.to_ascii_lowercase().contains("aser")).collect();
    assert!(!laser_rows.is_empty(), "typing 'laser' finds the laser row(s); got {:?}", rows.iter().map(|(l, _, _)| l).collect::<Vec<_>>());
    for l in &laser_rows { eprintln!("[dev-menu] laser row: {l}"); }
    // The Q-Laser by name: the bank's own label contains "Q-Laser" or "Laser" — assert one row
    // names a laser (not a hardcoded id guess: the id is whatever the bank says it is).
    assert!(rows.iter().any(|(l, _, a)| l.to_ascii_lowercase().contains("aser") && matches!(a, dev::MenuAction::GiveWeapon { .. })));
    // Sister: the filter narrows (a case-insensitive substring on the game's own names).
    assert!(rows.len() < unfiltered, "the filter narrows the page: {} -> {}", unfiltered, rows.len());
    // Sister: a filter nothing matches leaves the page empty (the filter gates every row).
    for c in "zzzz".chars() { dev::menu_type(&mut d, c); }
    let none = dev::menu_items(&d, Some(&scene));
    assert!(none.iter().all(|(l, _, _)| !l.contains("0x")), "a dead filter shows no weapon rows");
}

/// The Teleport page: Enter on the named spot moves Bond there and he's grounded after 60 ticks.
#[test]
fn menu_teleport_page_moves_bond_to_the_display_vent_and_he_grounds() {
    let Some(mut scene) = loaded("0700000c.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let mut d = dev::Dev::default();
    d.menu = true;
    // PageDown six times: index → teleport (the key handler's page stepping).
    for _ in 0..6 { dev::menu_key(&mut d, dev::MenuKey::PageDown); }
    assert_eq!(d.page, 6, "the Teleport page");
    assert_eq!(dev::PAGES[d.page], "teleport");
    // The named spot table carries the task's spots.
    assert!(dev::TELEPORT_SPOTS.iter().any(|(n, p)| *n == "0c display vent" && *p == [5.8, 15.8, -108.5]));
    let rows = dev::menu_items(&d, Some(&scene));
    let idx = rows.iter().position(|(l, _, _)| l.starts_with("0c display vent"))
        .unwrap_or_else(|| panic!("no display vent row: {:?}", rows.iter().map(|(l, _, _)| l).collect::<Vec<_>>()));
    d.menu_sel = idx;
    let action = dev::menu_activate(&d, Some(&scene)).expect("the teleport action");
    let dev::MenuAction::TeleportTo(i) = action else { panic!("expected TeleportTo, got {action:?}") };
    let spot = dev::TELEPORT_SPOTS[i].1;
    let (eye, yaw) = eye_yaw(&scene);
    let out = dev::apply_menu_action_to_scene(&mut d, &mut scene, action, eye, yaw, false);
    assert!(out.viewer_cel_reset, "a teleport clears the viewer cel (the game's own placement order)");
    {
        let sim = scene.sim.as_ref().unwrap();
        // x/z land exactly; y = the floor BELOW the spot (Bond::stand_at grounds him at the spot's
        // floor — the spot's y is the stand height, the floor is up to ~1 m under it).
        assert!((sim.bond.pos[0] - spot[0]).abs() < 0.01 && (sim.bond.pos[2] - spot[2]).abs() < 0.01
            && sim.bond.pos[1] <= spot[1] + 0.01 && spot[1] - sim.bond.pos[1] < 2.0,
            "Bond stands at the spot's floor: {:?} vs {:?}", sim.bond.pos, spot);
    }
    // Grounded after 60 ticks: the sim runs through the real step path, Bond settles on the vent
    // floor instead of falling through or drifting.
    let (grounded, y) = {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
        (sim.bond.grounded, sim.bond.pos[1])
    };
    assert!(grounded, "Bond is grounded after 60 ticks at the display vent");
    assert!((y - spot[1]).abs() < 2.0, "he stays at the vent's height (y {y} vs spot {})", spot[1]);
    // Sister: Backspace from a page returns to the index (the key handler's back lane).
    dev::menu_key(&mut d, dev::MenuKey::Backspace);
    assert_eq!(d.page, 0, "Backspace with an empty filter returns to the index");
}

/// The user's 07000015 crash: pausing to spawn grenades from the panel hard-crashed the game. In
/// the port the menu PAUSES the sim while open (`paused_tick_count`), the give/spawn actions land
/// through panel::apply between frames, and resuming runs the real tick path over the new state —
/// no panic, the inventory and the spawned NPC are right.
#[test]
fn spawn_with_the_menu_open_while_paused_is_safe() {
    let Some(mut scene) = loaded("07000015.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let mut d = dev::Dev::default();
    d.menu = true;
    // The menu pauses the sim: 0 ticks per frame while open (F11's one-shot still steps).
    assert_eq!(d.paused_tick_count(), 0, "the open menu runs no ticks");
    d.step = true;
    assert_eq!(d.paused_tick_count(), 1, "F11 steps exactly one tick under the open menu");
    // The Grenades page (index → 3 × PageDown), type "frag", Enter: x5 fragmentation grenades.
    for _ in 0..3 { dev::menu_key(&mut d, dev::MenuKey::PageDown); }
    assert_eq!(d.page, 3, "the Grenades page");
    for c in "frag".chars() { dev::menu_type(&mut d, c); }
    let action = dev::menu_activate(&d, Some(&scene)).expect("the frag x5 row");
    let dev::MenuAction::GiveWeapon { id, rounds } = action else { panic!("expected GiveWeapon, got {action:?}") };
    assert_eq!((id, rounds), (0x34, Some(5)), "frag = 0x34 at five rounds (the panel's own consumable row)");
    let (eye, yaw) = eye_yaw(&scene);
    let out = dev::apply_menu_action_to_scene(&mut d, &mut scene, action, eye, yaw, false);
    assert_eq!(out.switch_to, Some(0x34));
    // And an NPC spawn through the NPCs page (spawn by skin and mode): page 5, first skin row.
    for _ in 0..2 { dev::menu_key(&mut d, dev::MenuKey::PageDown); }
    assert_eq!(d.page, 5, "the NPCs page");
    let rows = dev::menu_items(&d, Some(&scene));
    let skin_row = rows.iter().position(|(l, _, a)| matches!(a, dev::MenuAction::SpawnNpcSkin(_)))
        .unwrap_or_else(|| panic!("no skin rows on 07000015: {:?}", rows.iter().map(|(l, _, _)| l).collect::<Vec<_>>()));
    d.menu_sel = skin_row;
    let action = dev::menu_activate(&d, Some(&scene)).expect("the skin spawn action");
    let before = scene.sim.as_ref().unwrap().drones.list.len();
    let out2 = dev::apply_menu_action_to_scene(&mut d, &mut scene, action, eye, yaw, false);
    eprintln!("[dev-menu] npc: {}", out2.status);
    // Still paused, nothing ran: the drone is placed but no tick touched it.
    assert_eq!(scene.sim.as_ref().unwrap().drones.list.len(), before + 1, "the NPC joined the list");
    assert_eq!(d.paused_tick_count(), 0, "the menu is still open — still paused");
    // Close the menu (F2's lane) and run the real tick path over the new state: no panic, the
    // grenade stack is in the inventory, and the drawn weapon settles on 0x34.
    d.menu = false;
    let (weapon, clip, npc_ok) = {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = nf_game::player::Input::default();
        input.switch_to = Some(0x34);
        sim.step_bond(input, false, false, false);
        for _ in 0..119 { sim.step_bond(Default::default(), false, false, false); }
        (sim.gun.weapon, sim.drones.inventory.clip.get(0x34).copied().unwrap_or(0), sim.drones.list.last().map_or(false, |dr| dr.present))
    };
    // The grenade's clip slot holds the one in Bond's hand (the record's clip size); the x5 lands
    // in the spare (the panel's own give: clip = clip_size, carry = max(3 clips, rounds)).
    assert!(clip >= 1, "a frag grenade is in the hand slot (clip slot {})", clip);
    let spare = scene.sim.as_ref().unwrap().drones.inventory.carried.get(scene.sim.as_ref().unwrap().weapons.ammo_type(0x34)).copied().unwrap_or(0);
    assert!(clip + spare >= 5, "the 5 fragmentation grenades are in the inventory (hand {clip} + spare {spare})");
    assert!(npc_ok, "the spawned NPC is still present after 120 ticks");
    if weapon == 0x34 { eprintln!("[dev-menu] the frag grenade is drawn on 07000015"); }
}

/// Nothing from the browser panel is left behind: every panel command (`panel::apply`'s arms and
/// poll_panel's specials) is reachable from a menu row.
#[test]
fn every_panel_command_has_a_menu_row() {
    let Some(scene) = loaded("07000005.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let mut d = dev::Dev::default();
    d.menu = true;
    // Walk every page and collect the actions.
    let mut seen: Vec<dev::MenuAction> = Vec::new();
    for p in 0..dev::PAGES.len() {
        d.page = p;
        for (_, _, a) in dev::menu_items(&d, Some(&scene)) { seen.push(a); }
    }
    let has = |f: &dyn Fn(&dev::MenuAction) -> bool| seen.iter().any(|a| f(a));
    // panel::apply's arms → their menu actions.
    assert!(has(&|a| matches!(a, dev::MenuAction::GiveWeapon { .. })), "do=weapon");
    assert!(has(&|a| matches!(a, dev::MenuAction::GiveAmmoFor(_))), "do=ammo_for");
    assert!(has(&|a| matches!(a, dev::MenuAction::GiveAmmoAll)), "do=ammo_all");
    assert!(has(&|a| matches!(a, dev::MenuAction::LevelKit)), "do=levelkit");
    assert!(has(&|a| matches!(a, dev::MenuAction::Health)), "do=health");
    assert!(has(&|a| matches!(a, dev::MenuAction::Armour)), "do=armour");
    assert!(has(&|a| matches!(a, dev::MenuAction::SpawnNpcSkin(_))), "do=npc");
    assert!(has(&|a| matches!(a, dev::MenuAction::ClearNpcs)), "do=clear_npcs");
    assert!(has(&|a| matches!(a, dev::MenuAction::Arena)), "do=arena");
    // poll_panel's specials → the Toggles/Screenshot rows.
    assert!(has(&|a| matches!(a, dev::MenuAction::ToggleGod)), "do=god");
    assert!(has(&|a| matches!(a, dev::MenuAction::ToggleAmmo)), "do=ammo");
    assert!(has(&|a| matches!(a, dev::MenuAction::Screenshot)), "do=shot");
    // The index page names all nine sections.
    d.page = 0;
    let idx = dev::menu_items(&d, Some(&scene));
    assert_eq!(idx.len(), 9, "the index lists the nine panel sections");
    for (l, _, a) in &idx { assert!(matches!(a, dev::MenuAction::Page(_)), "index row {l} opens a page"); }
    // --dev-page resolves every page name (the render lane for the main agent).
    for name in dev::PAGES { assert_eq!(dev::page_index(name), Some(dev::PAGES.iter().position(|p| p == name).unwrap()), "page name {name}"); }
    assert!(dev::page_index("nope").is_none());
}
