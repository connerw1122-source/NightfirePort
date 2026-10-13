//! Headless NF_ISO tests for the external dev panel (fix-dev-panel): the user's main test tool must be
//! exact — "a frag grenade spawns what looks to be a stun grenade that explodes like a frag grenade.
//! I cant tell whats what. no clue what the rocket launcher is."
//!
//! The game's own names come from `USATxt.dat` bound through the weapon's name label
//! (`Txt_BindLabel(weapon_data[w]+0x38)`, the SP pickup-name lane — `Pickup_Handler` 0x18e650:246
//! [M]; the verified map is docs/fields/weapon_data.md +0x30..0x3F: the HUD's fire-mode lane reads
//! +0x30+mode*4, +0x38 = the SP name). The old panel hardcoded a partial name table that disagrees
//! with the bank (it put "Frag Grenade" on 0x34 and "Stun Grenade" on 0x35 while the atlas pins
//! 0x35 = Frag Grenade [L+M]) — the direct source of "can't tell what's what".
//!
//! Meshes per record (docs/fields/weapon_data.md, the verified map): **+0x7C** = the held view-entity
//! model (`AnimDatumSetEntity` per draw, `Player_WeaponFiring` 0x1a5460:106-107 [M] — the atlas's
//! "fp_flashlight_model" name for it is wrong), **+0x80** = the third-person/pickup model, **+0x5C**
//! = the flying projectile's mesh (`Bullet_update` 0x12e7b0 disasm 0x12e8f0 `lw $a1, 0x5c($v0)` →
//! `hashtable_set_object_to_entity_gfx` [M]).
//!
//! Tests:
//! - `panel_entries_carry_the_record_meshes` — every catalog entry exposes held/tp/proj hashcodes
//!   equal to the weapon_data record (the old catalog carried none of them).
//! - `panel_names_are_the_games_own` — every entry's name is the bank's `Txt_BindLabel(+0x38)`
//!   string (trimmed); no label → the honest "weapon 0xNN" fallback. Fails on the old hardcoded
//!   table wherever it disagrees with the bank.
//! - `panel_give_spawns_exactly_the_record` — the Give path owns exactly the clicked id and fills
//!   the record's own clip slot to the record's clip size, and the switch request is that id.
//! - `level_kit_grants_the_games_loadout` — `do=levelkit` reproduces `Player_InitWeapon`
//!   (0x1a7ef8) for the current level exactly (the port's `nf_game::player::init_level_loadout`
//!   against a cleared inventory), so a playtest starts like the real game.
//! - `panel_shows_the_current_fire_mode_and_ammo_type` — the catalog exposes the current weapon's
//!   fire-mode label text (`+0x30 + mode*4`, the HUD's own read — `HUD_UpdateAmmoPane` 0x19a7f0:229)
//!   and ammo type (its `ammo_data` name label + the spare/clip counts).
//!
//! `NF_PANEL_DUMP=1` additionally prints the whole ground-truth table (the name/mesh dump the
//! assertions were read from).
//!
//! Opt-in via `NF_ISO`.

use nf_view::panel;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A loaded scene (Bond at the level start) — the same path the viewer uses.
fn loaded() -> Option<nf_view::scene::Scene> {
    let iso = iso()?;
    Some(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"))
}

/// Pull every `{"id":N, ...}` object out of the catalog JSON with a minimal scanner (the panel's
/// JSON is machine-generated: flat objects, no nesting inside an entry).
fn scan_entries(json: &str) -> Vec<std::collections::BTreeMap<String, String>> {
    let mut out = Vec::new();
    let mut rest = json;
    while let Some(p) = rest.find("{\"id\":") {
        let obj = &rest[p..];
        let end = obj.find('}').map(|e| e + 1).unwrap_or(obj.len());
        let obj = &obj[..end];
        let mut fields = std::collections::BTreeMap::new();
        // Split the flat "key":value pairs (values: number, true/false or "string"). Backslash
        // escapes are honoured — the bank's ammo names carry literal inches marks (`20" crossbow
        // bolt`), which arrive JSON-escaped.
        let inner = obj.trim_start_matches('{').trim_end_matches('}');
        let mut cur = String::new();
        let mut parts: Vec<String> = Vec::new();
        let (mut in_str, mut esc) = (false, false);
        for c in inner.chars() {
            if esc { esc = false; cur.push(c); }
            else if c == '\\' { esc = true; cur.push(c); }
            else if c == '"' { in_str = !in_str; cur.push(c); }
            else if c == ',' && !in_str { parts.push(std::mem::take(&mut cur)); }
            else { cur.push(c); }
        }
        parts.push(cur);
        for part in parts {
            let Some((k, v)) = part.split_once(':') else { continue };
            fields.insert(k.trim_matches('"').to_string(), v.trim_matches('"').to_string());
        }
        out.push(fields);
        rest = &rest[p + end..];
    }
    out
}

/// The ground-truth dump (run with `NF_PANEL_DUMP=1 ... -- --nocapture`): the bank names, the
/// fire-mode labels and the mesh hashes for every record the panel lists.
#[test]
fn dump_panel_ground_truth() {
    if std::env::var_os("NF_PANEL_DUMP").is_none() { eprintln!("NF_PANEL_DUMP not set — skipping"); return; }
    let Some(mut scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let sim = scene.sim.as_ref().unwrap();
    let wd = &sim.weapons;
    let lang = sim.lang.as_ref().expect("USATxt.dat loaded into the Sim");
    eprintln!("id  | sel | clip | ammo | name_label  | bank name                       | fire0                  | held(+7C)  tp(+80)  proj(+5C)");
    for id in 1..0x72u8 {
        let rec = &wd.records[id as usize];
        let u32_at = |o: usize| u32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
        let held = u32_at(0x7c);
        let tp = u32_at(0x80);
        let proj = u32_at(0x5c);
        let nl = u32_at(0x38);
        let bound = (nl != 0xFFFFFFFF).then(|| lang.bind(nl)).flatten().unwrap_or("");
        let f0 = wd.fire_mode_label(id as usize, 0);
        let f0t = (f0 != 0).then(|| lang.bind(f0)).flatten().unwrap_or("").to_string();
        eprintln!("{id:#04x} |  {}  | {:4} | {:4} | 0x{nl:08x} | {bound:31} | {f0t:22} | {held:#010x} {tp:#010x} {proj:#010x}",
            wd.selectable(id) as u8, wd.clip_size(id as usize), wd.ammo_type(id as usize));
    }
    for t in 0..34usize {
        let al = wd.ammo_name_label(t);
        if al != 0 { eprintln!("ammo {t:2}: 0x{al:08x} = {:?}", lang.bind(al).unwrap_or("")); }
    }
    // The grenades' behaviour fields: the explosion radius/damage (+0x08/+0x0C) and the first-person
    // model (+0xDC) — which record explodes frag-grade, and which can `Player_EquipWeapon` at all.
    for id in [0x34u8, 0x35, 0x36, 0x37, 0x45, 0x2a, 0x2c, 0x2e, 0x3a] {
        let rec = &wd.records[id as usize];
        let f32_at = |o: usize| f32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
        let u32_at = |o: usize| u32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
        eprintln!("behaviour {id:#04x}: explosion_radius {:?} dmg_mul {:?} fp_model {:#010x} flags68 {:#010x}",
            f32_at(0x08), f32_at(0x0c), u32_at(0xdc), u32_at(0x68));
    }
}

/// Every catalog entry carries the weapon_data record's own meshes: the held view-entity model
/// (+0x7C), the third-person/pickup model (+0x80) and the flying projectile's mesh (+0x5C) —
/// `Bullet_update` 0x12e7b0:483 sets +0x5C on the bullet object (`hashtable_set_object_to_entity_gfx`)
/// and the draw machine sets +0x7C on the view entity (`AnimDatumSetEntity`, 0x1a5460:106) [M].
/// The old catalog had none of the three: the user literally could not tell what a row would spawn.
#[test]
fn panel_entries_carry_the_record_meshes() {
    let Some(scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let sim = scene.sim.as_ref().unwrap();
    let wd = &sim.weapons;
    let entries = scan_entries(&panel::catalog(&scene));
    assert!(!entries.is_empty(), "the catalog lists no entries");
    let mut checked = 0;
    for e in &entries {
        let id = e["id"].parse::<u32>().expect("entry id");
        let rec = &wd.records[id as usize];
        let u32_at = |o: usize| u32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
        let norm = |h: u32| if h == u32::MAX { 0 } else { h };
        // The entry must expose all three record hashes.
        let held = e.get("held").unwrap_or_else(|| panic!("entry {id:#04x} has no held (+0x7C) hash")).parse::<u32>().unwrap();
        let tp = e.get("tp").unwrap_or_else(|| panic!("entry {id:#04x} has no tp (+0x80) hash")).parse::<u32>().unwrap();
        let proj = e.get("proj").unwrap_or_else(|| panic!("entry {id:#04x} has no proj (+0x5C) hash")).parse::<u32>().unwrap();
        assert_eq!(held, norm(u32_at(0x7c)), "entry {id:#04x} held mesh (+0x7C) != the record");
        assert_eq!(tp, u32_at(0x80), "entry {id:#04x} third-person/pickup mesh (+0x80) != the record");
        assert_eq!(proj, norm(u32_at(0x5c)), "entry {id:#04x} projectile mesh (+0x5C) != the record");
        // The raw label id travels too (the page can cite it; the report's live checks use it).
        assert_eq!(e["label"].parse::<u32>().unwrap(), u32_at(0x38), "entry {id:#04x} name label id != the record");
        checked += 1;
    }
    eprintln!("[meshes] {checked} entries checked against the records");
    assert!(checked >= 60, "the catalog should list the populated records (got {checked})");
}

/// Every entry shows the game's own name: `Txt_BindLabel(weapon_data[w]+0x38)` from USATxt.dat
/// (`Pickup_Handler` 0x18e650:246 binds the same lane for the pickup message; `HUD_UpdateAmmoPane`
/// 0x19a7f0 draws it in the ammo pane) [M]. The old hardcoded table disagreed with the bank — its
/// "Frag Grenade" sat on 0x34 while the atlas pins 0x35 = Frag Grenade [L+M], the user's
/// "a frag grenade spawns what looks to be a stun grenade".
#[test]
fn panel_names_are_the_games_own() {
    let Some(scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let sim = scene.sim.as_ref().unwrap();
    let wd = &sim.weapons;
    let lang = sim.lang.as_ref().expect("USATxt.dat loaded into the Sim");
    let entries = scan_entries(&panel::catalog(&scene));
    assert!(!entries.is_empty(), "the catalog lists no entries");
    let mut named = 0;
    for e in &entries {
        let id = e["id"].parse::<u32>().expect("entry id");
        let rec = &wd.records[id as usize];
        let nl = u32::from_le_bytes(rec[0x38..0x3c].try_into().unwrap());
        let want = (nl != 0xFFFFFFFF).then(|| lang.bind(nl)).flatten().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let got = &e["name"];
        match want {
            Some(want) => {
                assert_eq!(got, &want, "entry {id:#04x}: the panel's name is not the game's own (bank label 0x{nl:08x})");
                named += 1;
            }
            None => assert!(got.starts_with("weapon 0x") || got.starts_with("gadget 0x") || (id == 1 && got == "Fists"),
                "entry {id:#04x}: no bank name (label 0x{nl:08x}) — the fallback must be the honest id, got \"{got}\""),
        }
    }
    eprintln!("[names] {named} entries carry bank names");
    assert!(named >= 40, "the bank should name most records (got {named})");
    // The exact rows the user called out: the grenades and the rocket launcher. Whatever the bank
    // binds for them IS the panel's string (pinned here so a regression can't silently shift them).
    let name_of = |id: u8| entries.iter().find(|e| e["id"] == format!("{id}")).map(|e| e["name"].clone()).unwrap_or_default();
    for id in [0x34u8, 0x35, 0x36, 0x37, 0x3a] {
        let nl = u32::from_le_bytes(wd.records[id as usize][0x38..0x3c].try_into().unwrap());
        let bank = lang.bind(nl).unwrap_or("").trim().to_string();
        assert!(!bank.is_empty(), "the bank has no name for {id:#04x} (label 0x{nl:08x}) — dump with NF_PANEL_DUMP=1");
        assert_eq!(name_of(id), bank, "the {id:#04x} row must show the game's own name");
        eprintln!("[names] {id:#04x} = \"{bank}\" (label 0x{nl:08x})");
    }
}

/// The Give path spawns exactly the clicked record: Bond owns that id, its clip slot holds the
/// record's clip size (the clip slot is the base's when +0x05 shares it — `Player_EquipWeapon`
/// 0x1a8a58:44 [M]) and the draw request is that id. Run on a cleared inventory so every give is
/// the fresh-own path.
#[test]
fn panel_give_spawns_exactly_the_record() {
    let Some(mut scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let ids: Vec<u8> = [2u8, 0x06, 0x0e, 0x16, 0x1c, 0x24, 0x34, 0x35, 0x36, 0x37, 0x3a, 0x45, 0x4a, 0x4e, 0x50, 0x54]
        .iter().copied().collect();
    for id in ids {
        {
            let sim = scene.sim.as_mut().unwrap();
            let n = sim.drones.inventory.clip.len();
            let a = sim.drones.inventory.carried.len();
            sim.drones.inventory = nf_game::pickup::Inventory::empty(n, a);
            sim.gun.weapon = 0x47; // hands — the give must pull Bond onto the new record
        }
        let q = [("do".to_string(), "weapon".to_string()), ("id".to_string(), format!("{id}"))];
        let (status, switch) = panel::apply(&mut scene, &q);
        assert_eq!(switch, Some(id), "giving {id:#04x} must switch Bond onto it ({status})");
        let sim = scene.sim.as_ref().unwrap();
        let wd = &sim.weapons;
        assert!(sim.drones.inventory.owned.get(id as usize).copied().unwrap_or(false), "giving {id:#04x}: not owned ({status})");
        // The record's own clip slot (base's when shared) is filled to the record's clip size.
        let base = wd.base(id as usize) as usize;
        let slot = if wd.shares_base_clip(id as usize) && wd.ammo_type(id as usize) == wd.ammo_type(base) { base } else { id as usize };
        let clip = sim.drones.inventory.clip.get(slot).copied().unwrap_or(-1);
        assert_eq!(clip, wd.clip_size(id as usize), "giving {id:#04x}: clip slot {slot} holds {clip}, the record's size is {} ({status})", wd.clip_size(id as usize));
    }
}

/// The "level kit" button grants exactly the loadout `Player_InitWeapon` (0x1a7ef8) gives on a cold
/// load of the current level — the port's `nf_game::player::init_level_loadout` against a cleared
/// inventory is the reference. 07000005's case: P2K 48 rounds, the Q-Claw 999, the laser 0, the
/// watch 0 + the common tail (the Q-Decryptor 0x4E 999, the fists) [M 0x1a7ef8 cases + :318-325].
#[test]
fn level_kit_grants_the_games_loadout() {
    let Some(mut scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    // Wipe the inventory first (the level load already granted the loadout — the test must prove
    // the BUTTON rebuilds it, not that the load did).
    {
        let sim = scene.sim.as_mut().unwrap();
        let n = sim.drones.inventory.clip.len();
        let a = sim.drones.inventory.carried.len();
        sim.drones.inventory = nf_game::pickup::Inventory::empty(n, a);
        sim.gun.switch_request = 0;
    }
    let q = [("do".to_string(), "levelkit".to_string())];
    let (status, _) = panel::apply(&mut scene, &q);
    let sim = scene.sim.as_ref().unwrap();
    // The reference: the same function the level-load path runs, on a cleared inventory, with the
    // sim's own switch channels (the game reads them behind the warm-reset branch) [M 0x1957a8].
    let mut ref_inv = nf_game::pickup::Inventory::empty(sim.drones.inventory.clip.len(), sim.drones.inventory.carried.len());
    let mut ref_health = sim.drones.health;
    let mut ref_ch = sim.drones.channels.clone();
    let wd = sim.weapons.clone();
    let level = sim.level;
    let loaded = |w: u8| sim.skins.contains_key(&wd.first_person(w as usize));
    let _req = nf_game::player::init_level_loadout(level, None, &mut ref_health, &mut ref_inv, &mut ref_ch, &wd, loaded);
    assert!(sim.drones.inventory.owned.get(2).copied().unwrap_or(false), "level kit: the P2K must be owned ({status})");
    for w in [2usize, 0x4a, 0x4e, 0x50, 0x54] {
        assert!(sim.drones.inventory.owned.get(w).copied().unwrap_or(false), "level kit: weapon {w:#04x} must be owned ({status}) — the game's loadout owns it");
    }
    // Exact-match the inventory against the reference loadout (the panel must grant what the game grants).
    assert_eq!(sim.drones.inventory.owned, ref_inv.owned, "level kit: owned set != the game's ({status})");
    assert_eq!(sim.drones.inventory.clip, ref_inv.clip, "level kit: clips != the game's ({status})");
    assert_eq!(sim.drones.inventory.carried, ref_inv.carried, "level kit: carried rounds != the game's ({status})");
    // The common tail owns the fists (`equip 1 with 0 rounds` [M 0x1a7ef8:319]); a fresh own fills
    // the clip slot to the record's clip size (`Player_EquipWeapon` 0x1a8a58:44-48 [M]).
    assert!(sim.drones.inventory.owned[1], "level kit: the fists tail is missing ({status})");
    assert_eq!(sim.drones.inventory.clip[1], wd.clip_size(1), "level kit: the fists clip slot ({status})");
    // The draw request is the loadout's: the P2K on 07000005 (`Upgrade_Weapon(6)` = 2 [L]).
    assert_eq!(sim.gun.switch_request, 2, "level kit: the draw request must be the game's ({status})");
}

/// The panel shows the current weapon's fire mode and ammo type: the catalog carries a "current"
/// object with the weapon id, its fire-mode label TEXT (`weapon_data+0x30 + mode*4` — the HUD's own
/// read, `HUD_UpdateAmmoPane` 0x19a7f0:229; the port's trigger runs mode 0), the ammo type's
/// `ammo_data` name text (`Pickup_Handler` 0x18e650 case 1 binds `ammo_data[type]+8` [M]) and the
/// clip/spare counts.
#[test]
fn panel_shows_the_current_fire_mode_and_ammo_type() {
    let Some(scene) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let json = panel::catalog(&scene);
    let cur = json.find("\"current\":").expect("the catalog must carry the current weapon's state");
    let obj = &json[cur..json[cur..].find('}').map(|e| cur + e + 1).unwrap_or(json.len())];
    for key in ["weapon", "fire_mode", "ammo_type", "clip", "spare"] {
        assert!(obj.contains(&format!("\"{key}\":")), "the current-weapon object lacks \"{key}\": {obj}");
    }
    // The fire mode and ammo type are the game's own texts, not invented labels.
    let sim = scene.sim.as_ref().unwrap();
    let lang = sim.lang.as_ref().unwrap();
    let wi = sim.gun.weapon as usize;
    let fl = sim.weapons.fire_mode_label(wi, 0);
    let fm_text = lang.bind(fl).unwrap_or("").trim().to_string();
    assert!(!fm_text.is_empty(), "weapon {wi:#04x} has a fire-mode label the bank binds (0x{fl:08x})");
    assert!(obj.contains(&format!("\"fire_mode\":\"{}\"", fm_text.replace('"', "\\\""))), "the fire mode text must be the bank's \"{fm_text}\": {obj}");
    let at = sim.weapons.ammo_type(wi);
    let al = sim.weapons.ammo_name_label(at);
    let ammo_text = lang.bind(al).unwrap_or("").trim().to_string();
    assert!(!ammo_text.is_empty(), "ammo type {at} has a name the bank binds (0x{al:08x})");
    assert!(obj.contains(&format!("\"ammo_type\":\"{}\"", ammo_text.replace('"', "\\\""))), "the ammo type text must be the bank's \"{ammo_text}\": {obj}");
}
