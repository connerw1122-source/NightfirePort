//! fix-pickups-remainder: the pickup census over the disc's on-foot levels, driven through the
//! real Sim path (`nf_view::load` → `Sim`, the same creation the game's
//! `parsemap_create_dynamic_objects` 0x1d04f0 + `Script_CreateEntity` 0x1d8968 walks).
//!
//! For every on-foot level:
//! - every type-0xF0 placement's pickup is created by the port with the placement's own
//!   kind/id/rounds (the params buffer read at `level_tag+0x2C+4k` [M 0x1d04f0:457-462]);
//! - every created pickup is takeable from where a player stands: the game's own touch
//!   predicate (`Pickup_Update` 0x18e148:203-214, radius + 1.1, LOS from the sphere centre) and
//!   `Pickup_Handler`'s consume rules (armour ≥ 50 refuses; ammo at cap refuses; a first-person
//!   model missing from the level archive refuses — the user's "can't pick up the rocket
//!   launcher" on 07000008);
//! - the kind census: which PICKUPINFO kinds the disc's SP levels actually place (7-11 =
//!   `Pickup_CreateFromSet` 0x18f1b0 are the MP weapon sets; SP use would need porting).
//!
//! Opt-in via NF_ISO. NF_PICKUPCENSUS=1 prints the per-level table.

use nf_formats::iso9660::IsoImage;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Every level archive in FILES.BIN whose name is 0700xxxx.bin, minus the driving missions
/// (07000023-29) and the shared packs (07000500, 07000999) — the levels-sweep's own filter.
fn on_foot_levels(img: &IsoImage) -> Vec<String> {
    let (entries, _) = nf_formats::disc::files_bin_entries(img).unwrap();
    let mut names: Vec<String> = entries.iter()
        .map(|e| e.name.to_ascii_lowercase())
        .filter(|n| n.starts_with("0700") && n.ends_with(".bin"))
        .filter(|n| {
            let code = u32::from_str_radix(&n[..8], 16).unwrap_or(0) & 0xff;
            !(0x23..=0x29).contains(&code) && code != 0x00 && n != "07000500.bin" && n != "07000999.bin"
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The placement's param k (`level_tag+0x2C+4k`, absent reads 0) — the game's own lookup [M 0x1d04f0:457-462].
fn param(p: &nf_formats::placement::Placement, idx: u32) -> u32 {
    p.params.iter().find(|(i, _)| *i == idx).map(|(_, v)| *v).unwrap_or(0)
}

/// The level archive's bytes (the wire_live helper).
fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

#[test]
fn pickup_census_every_level_creates_and_offers_its_pickups() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = IsoImage::open(&iso).unwrap();
    let levels = on_foot_levels(&img);
    eprintln!("[census] {} on-foot levels", levels.len());
    let mut failures: Vec<String> = Vec::new();
    let mut kind_counts: std::collections::BTreeMap<u16, usize> = Default::default();

    for lv in &levels {
        let scene = std::panic::catch_unwind(|| nf_view::load(&iso, lv, false));
        let Ok(Ok(mut scene)) = scene else {
            failures.push(format!("{lv}: load failed"));
            continue;
        };
        let Some(sim) = scene.sim.as_mut() else { continue };
        let level_name = lv.clone();

        // The expected set: the main map's 0xF0 placements, (kind, id, rounds) from the params.
        let expected: Vec<(u16, u16, i16, [f32; 3])> = match nf_formats::level::Level::parse(&read_archive(&iso, lv)) {
            Ok(lv2) => lv2.maps[lv2.main_map().unwrap_or(0)].placements.iter()
                .filter(|p| p.flags & 0xff == 0xf0)
                .map(|p| (param(p, 0) as u16, param(p, 1) as u16, param(p, 2) as i16, p.translation))
                .collect(),
            Err(e) => { failures.push(format!("{level_name}: level re-read failed {e}")); Vec::new() }
        };

        // The sim's pickups = the placed pickups + the scripts' reserved 0xF0 slots (gone = true
        // tombstones until the script spawns, fix-safe08) — the census counts only the live ones.
        let placed = sim.drones.pickups.iter().filter(|p| !p.gone).count();
        let mut matched = 0usize;
        for (kind, id, rounds, tr) in &expected {
            // Kinds 7-11 never become pickups: `Pickup_Create` intercepts them into
            // `Pickup_CreateFromSet` — a kind-0 pickup of the matrix weapon [M 0x18dda0:104-105].
            let level_code = u32::from_str_radix(&level_name[..8], 16).unwrap_or(0);
            let (ekind, eid, erounds) = if (7..=11).contains(kind) {
                let set_w = nf_game::pickup::PICKUP_MATRIX_ROW0[(*kind - 7) as usize];
                let base = sim.weapons.base(set_w as usize) as u16;
                (0u16, base, (sim.weapons.clip_size(base as usize) as i16) * 2)
            } else {
                // `Pickup_Create`'s level-conditional rounds override [M 0x18dda0]: 07000008's
                // 0x2c rocket pickup is created with 4 rounds whatever the placement says (5).
                (*kind, *id, nf_game::pickup::Pickup::create_rounds_override(level_code, *id, *rounds))
            };
            let hit = sim.drones.pickups.iter().find(|p|
                p.kind == ekind && p.weapon == eid && p.rounds == erounds
                && (p.m[3][0] - tr[0]).abs() < 0.5 && (p.m[3][1] - tr[1]).abs() < 0.5 && (p.m[3][2] - tr[2]).abs() < 0.5);
            match hit {
                Some(_) => matched += 1,
                None => failures.push(format!("{level_name}: placement pickup kind {kind} id {id:#x} rounds {rounds} at ({:.1},{:.1},{:.1}) not created by the port (expected kind {ekind} id {eid:#x} rounds {erounds})", tr[0], tr[1], tr[2])),
            }
        }

        // Takeability: a player standing at each live pickup (the game's own predicate), and the
        // consume rules of Pickup_Handler. `loaded` = the level archive has the first-person model
        // (the game's hashtable_getitem [M 0x1a8a58:8-11]).
        let mut taken = 0usize;
        for k in 0..placed {
            let p = sim.drones.pickups[k].clone();
            if p.gone { continue; }
            *kind_counts.entry(p.kind).or_default() += 1;
            let t = [p.m[3][0], p.m[3][1], p.m[3][2]];
            let loaded = sim.skins.contains_key(&sim.weapons.first_person(p.weapon as usize));
            // Takeable from where a player stands: the game's own predicate from a ring of standing
            // spots around the pickup (its centre's height, on the ground the pickup rests on) — any
            // clear spot takes it. One synthetic spot fights the pedestal geometry a real player
            // walks around.
            let c = p.centre();
            let mut touches = false;
            for s in 0..8 {
                let (dy, dx) = (s as f32 * 0.785398, 0.45);
                // At the pickup's own height and 1/2 below it (a high-mounted pickup is taken from
                // the floor below — the 07000014 kind-6 bonuses hang ~3 up).
                for drop in [0.0f32, -1.0, -2.0] {
                    let pos = [c[0] + dx * dy.cos(), t[1] + drop, c[2] + dx * dy.sin()];
                    if p.touches(&sim.world, None, pos, [pos[0], pos[1] + 0.9, pos[2]]) { touches = true; break; }
                }
                if touches { break; }
            }
            let mut armour = 0.0f32;
            // A player standing on a pickup touches it EVERY tick [M Pickup_Update 0x18e148:203-214],
            // and a refused take still mutates the inventory (Player_EquipAmmo's clip-fill can return
            // 0 — the game's own quirk [M 0x1a8d18] — the next touch then takes it). Three takes =
            // three ticks of standing on it.
            let mut take = sim.drones.inventory.take(&sim.weapons, &p, loaded, &mut armour, 1);
            for _ in 0..2 { if take.is_none() { take = sim.drones.inventory.take(&sim.weapons, &p, loaded, &mut armour, 1); } }
            if take.is_none() && (p.kind == 0 || p.kind == 1) {
                let id = nf_game::pickup::Inventory::upgrade_ammo_id(p.weapon, 0, 1) as usize;
                let at = sim.weapons.ammo_type(id);
                eprintln!("[census-debug] {level_name} #{k} id {:#x} kind {} upgrade→{id:#x} ammo_type {at} carried {:?} cap {:?} clip {:?} owned {:?} rounds {}", p.weapon, p.kind,
                    sim.drones.inventory.carried.get(at), sim.weapons.ammo_cap.get(at), sim.drones.inventory.clip.get(id), sim.drones.inventory.owned.get(id), p.rounds);
            }
            let legit_refusal = match p.kind {
                // Armour at 50 refuses; an owned weapon's ammo at cap refuses [M 0x18e650]. The
                // slot is the COMPOSED id's ammo type — `Upgrade_Weapon` first, then
                // `Player_EquipAmmo`'s own 0x1f/0x25 remap [M 0x18e650:142-144 + 0x1a8d18:11-19;
                // fix-pickups-ammo: the id-6 boxes credit type 1, cap 70].
                3 => armour >= 50.0,
                0 | 1 => !loaded || take.is_none() && {
                    let id = nf_game::pickup::Inventory::upgrade_ammo_id(
                        nf_game::pickup::Inventory::upgrade_weapon(p.weapon, 0, 1), 0, 1) as usize;
                    let at = sim.weapons.ammo_type(id);
                    sim.drones.inventory.carried.get(at).copied().unwrap_or(0) >= sim.weapons.ammo_cap.get(at).copied().unwrap_or(0)
                },
                _ => false,
            };
            let ok = touches && (take.is_some() || legit_refusal);
            if std::env::var_os("NF_PICKUPCENSUS").is_some() {
                eprintln!("[census] {level_name} #{k} kind {} id {:#x} rounds {} loaded {loaded} touch {touches} take {take:?} ok {ok}", p.kind, p.weapon, p.rounds);
            }
            if !ok {
                failures.push(format!(
                    "{level_name} #{k} kind {} id {:#x} rounds {} at ({:.1},{:.1},{:.1}): loaded {loaded} touch {touches} take {take:?}",
                    p.kind, p.weapon, p.rounds, t[0], t[1], t[2]));
            } else if take.is_some() {
                taken += 1;
            }
        }
        if std::env::var_os("NF_PICKUPCENSUS").is_some() {
            eprintln!("[census] {level_name}: expected {} placed {placed} matched {matched} takenable {taken}", expected.len());
        }
        // Every 0xF0 placement the port could resolve must exist as a pickup.
        assert_eq!(matched, expected.len(), "{level_name}: placements the port dropped (of {})", expected.len());
        assert_eq!(placed, expected.len(), "{level_name}: the sim's live placed pickup count vs the 0xF0 placements (script spawns appear later)");
    }
    eprintln!("[census] kinds placed across the disc: {kind_counts:?}");
    assert!(failures.is_empty(), "{} pickup census failures:\n{}", failures.len(), failures.join("\n"));
}
