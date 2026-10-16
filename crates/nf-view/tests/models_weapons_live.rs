//! models-weapons-items: every weapon's first-person and third-person model, every gadget's models, every
//! pickup's model, dropped weapons and the projectiles (grenades, rockets, darts, the Q-Worm).
//!
//! - The per-level inventory (the golden): on every story level every armed drone's third-person gun resolves
//!   (mesh + datum), every placed pickup's model resolves, and every projectile mesh the level can fire resolves.
//! - The projectile draw (`weapon_data+0x5C`, `Bullet_update` 0x12e7b0 disasm 0x12e8f0): a flying grenade shows
//!   its weapon's mesh at the flight matrix — through the real input path (`Sim::step_bond` with the throw input).
//! - The dev-mode view models (folded from 190-fix-dev2): with `--dev`, every selectable weapon has a first-person
//!   model on every level (the missing FP skins merge from their holding archives, the way the game's session
//!   hash tables accumulate — `AnimLoadFile` 0x1d8248). Normal play (no flag) must be unchanged.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The 20 story levels (docs/models/characters.md's inventory uses the same set).
pub const STORY: [&str; 20] = ["07000001", "07000002", "07000003", "07000004", "07000005", "07000006", "07000007",
    "07000008", "07000009", "0700000a", "0700000b", "0700000c", "0700000d", "07000011", "07000012", "07000013",
    "07000014", "07000015", "07000016", "0700001b"];

struct Disc {
    img: nf_formats::iso9660::IsoImage<std::fs::File>,
}

impl Disc {
    fn open(iso: &str) -> Disc {
        Disc { img: nf_formats::iso9660::IsoImage::open(iso).expect("open ISO") }
    }
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let (entries, bin) = nf_formats::disc::files_bin_entries(&self.img).ok()?;
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
        self.img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()
    }
    fn level(&self, name: &str) -> nf_formats::level::Level {
        let data = self.read(name).unwrap_or_else(|| panic!("no {name} in FILES.BIN"));
        nf_formats::level::Level::parse(&data).expect("parse level")
    }
    fn weapons(&self) -> nf_formats::weapon::WeaponData {
        let e = self.img.find("/ACTION.ELF").expect("ACTION.ELF on the disc");
        let elf = self.img.read_file(e).expect("read ACTION.ELF");
        nf_formats::weapon::WeaponData::from_action_elf(&elf).expect("weapon_data")
    }
}

/// One level's weapon-model inventory, as the golden row.
struct Inv {
    /// Armed drone placements (type 0x0F, param 9 weapon ≥ 1).
    drones: usize,
    /// Their third-person guns that resolve (mesh in by_hash AND the datum on the skin).
    tp_ok: usize,
    /// Their third-person guns that DON'T (a mesh or datum miss is a never-drawn gun).
    tp_miss: usize,
    /// Their muzzle-flash meshes (+0x4C) that resolve.
    flash_ok: usize,
    flash_miss: usize,
    /// Level-placed pickups (type 0xF0) whose placement object resolves.
    pickups: usize,
    pickup_miss: usize,
    /// Distinct projectile meshes (+0x5C) of the weapons this level can fire that resolve in by_hash.
    proj: usize,
    proj_miss: usize,
    /// First-person skins of the selectable weapons that resolve in this archive (the normal-play
    /// view models; the dev-mode task merges the rest).
    fp_present: usize,
}

/// The weapons that can appear on a level: the drones' param-9 weapons, each drone's grenade weapon
/// (placement param 11, the decode `Drone::new` mirrors — `NDrone2_DefaultInit` 0x14b300 lines 348-376),
/// the placed pickups' weapon ids (kinds 0/1) and the level's cold-load loadout (`Player_InitWeapon`'s
/// per-level switch, ported in `init_level_loadout`). A projectile mesh (`weapon_data+0x5C`) of any of
/// these must resolve in the level's own archive: on a cold boot into the level the session hash table
/// holds nothing else, and the game ships the level playable.
fn level_weapon_set(level: &nf_formats::level::Level, weapons: &nf_formats::weapon::WeaponData) -> Vec<usize> {
    let mut set: Vec<usize> = Vec::new();
    let mm = match level.main_map() { Some(m) => m, None => return set };
    let mut push = |w: usize, set: &mut Vec<usize>| { if (1..0x73).contains(&w) && !set.contains(&w) { set.push(w); } };
    let level_id = u32::from_str_radix(&level.maps[mm].name.trim().trim_end_matches(".bin"), 16).unwrap_or(0);
    for p in &level.maps[mm].placements {
        match p.flags & 0xff {
            0x0f => {
                let param = |i: u32| p.params.iter().find(|(k, _)| *k == i).map(|(_, v)| *v).unwrap_or(0);
                push(param(9) as usize, &mut set);
                // The grenade weapon (param 11's selection nibble; the 0x07000014 clamp at 0x14b300:374-376).
                let p11 = param(11);
                let gw = match ((p11 >> 4) & 0xffff) as u16 {
                    1 => 0x36, 2 => 0x35, 4 => 0x34, 5 => ((p11 & 0xf).wrapping_add(0x3a)) as u16, 6 => 0x6c, _ => 0,
                };
                let gw = if level_id == 0x0700_0014 && (0x34..0x37).contains(&gw) { 0x34 } else { gw };
                if gw != 0 { eprintln!("  placement at {:?} throws weapon {gw:#04x} (p11 {p11:#x})", p.translation); push(gw as usize, &mut set); }
            }
            0xf0 => {
                let param = |i: u32| p.params.iter().find(|(k, _)| *k == i).map(|(_, v)| *v).unwrap_or(0) as usize;
                let (kind, id) = (param(0), param(1));
                if matches!(kind, 0 | 1) { push(id, &mut set); }
            }
            _ => {}
        }
    }
    // The level's starting loadout (cold load): every owned weapon can be fired on the spot.
    let mut inv = nf_game::pickup::Inventory::empty(0x73, 40);
    let _ = nf_game::player::init_level_loadout(level_id, None, &mut nf_game::bullet::PlayerHealth::default(),
        &mut inv, &mut nf_game::triggers::Channels::default(), weapons, |w| weapons.first_person(w as usize) != 0);
    for (w, &o) in inv.owned.iter().enumerate() { if o { push(w, &mut set); } }
    set
}

fn inventory(d: &Disc, name: &str, weapons: &nf_formats::weapon::WeaponData, session: &std::collections::HashSet<u32>) -> Inv {
    let level = d.level(&format!("{name}.bin"));
    let mm = level.main_map().expect("main map");
    let mut inv = Inv { drones: 0, tp_ok: 0, tp_miss: 0, flash_ok: 0, flash_miss: 0, pickups: 0, pickup_miss: 0, proj: 0, proj_miss: 0, fp_present: 0 };
    for p in &level.maps[mm].placements {
        match p.flags & 0xff {
            0x0f => {
                inv.drones += 1;
                let skin = p.params.iter().find(|(k, _)| *k == 0).map(|(_, v)| *v).unwrap_or(0);
                let w = p.params.iter().find(|(k, _)| *k == 9).map(|(_, v)| *v).unwrap_or(0) as usize;
                if !(1..0x73).contains(&w) { continue; }
                let Some((mesh, datum)) = weapons.third_person(w as u8) else { continue };
                let datum = if datum == 0xff { 0 } else { datum as u32 };
                let mut ok = level.by_hash.contains_key(&mesh) || session.contains(&mesh);
                // The datum hangs on the placement's skin. A datum the skin doesn't carry is the game's
                // own no-op: `AnimDatumSetEntity` (0x1c7848) walks the skin's datum list and returns
                // without attaching when the id is absent [M] — that guard is unarmed in the shipped
                // game too (07000004/16's skin 0x0500001e has no datum 4 for the SMG [D]).
                if let Some(sk) = level.skins.get(&skin) {
                    ok &= sk.datums.iter().any(|dt| dt.id == datum && dt.bone >= 0);
                }
                if ok { inv.tp_ok += 1 } else { eprintln!("  tp miss [{name}] w{w:#04x} mesh {mesh:08x} skin {skin:08x} datum {datum}"); inv.tp_miss += 1 }
                if let Some((fmesh, _)) = weapons.muzzle_flash(w as u8) {
                    if level.by_hash.contains_key(&fmesh) || session.contains(&fmesh) { inv.flash_ok += 1 } else { eprintln!("  flash miss [{name}] w{w:#04x} mesh {fmesh:08x}"); inv.flash_miss += 1 }
                }
            }
            0xf0 => {
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[mm].objects.len() { Some((mm, p.index as usize)) } else { None };
                match target {
                    Some((om, oi)) => {
                        let obj = &level.maps[om].objects[oi];
                        // `Pickup_Create` draws the placement's glist; the bounds come from the object
                        // itself (fix-objects). Resolve = the object exists with a real glist.
                        if obj.gfx.leaves.iter().any(|l| !l.batches.is_empty()) || obj.centre != [0.0; 3] { inv.pickups += 1 } else { inv.pickup_miss += 1 }
                    }
                    None => inv.pickup_miss += 1,
                }
            }
            _ => {}
        }
    }
    for w in level_weapon_set(&level, weapons) {
        let m = weapons.projectile_mesh(w);
        if m == 0 { continue; }
        if level.by_hash.contains_key(&m) || session.contains(&m) { inv.proj += 1 } else { eprintln!("  proj miss [{name}] w{w:#04x} mesh {m:08x}"); inv.proj_miss += 1 }
    }
    for w in 1..0x73usize {
        if !weapons.selectable(w as u8) { continue; }
        let fp = weapons.first_person(w);
        if fp != 0 && level.skins.contains_key(&fp) { inv.fp_present += 1; }
    }
    inv
}

fn row(name: &str, i: &Inv) -> String {
    let f = [i.drones, i.tp_ok, i.tp_miss, i.flash_ok, i.flash_miss, i.pickups, i.pickup_miss, i.proj, i.proj_miss, i.fp_present];
    let cols: Vec<String> = f.iter().map(|x| x.to_string()).collect();
    format!("{name}\t{}", cols.join("\t"))
}

const GOLDEN_HEADER: &str = "level\tdrones\ttp_ok\ttp_miss\tflash_ok\tflash_miss\tpickups\tpickup_miss\tproj\tproj_miss\tfp_present";

/// The per-level golden: every armed drone's gun, every placed pickup and every projectile mesh the
/// level can fire resolves. The counts are pinned (numbers only); a change means a model stopped or
/// started resolving and the lane doc must be updated with it.
#[test]
fn golden_weapon_models_resolve_on_every_story_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let d = Disc::open(&iso);
    let weapons = d.weapons();
    // The session union: every story archive before this one, in story order (the level codes ascend
    // in mission order — docs/systems/levels.md §1). The game's object hashtable accumulates exactly
    // these (see the merge comment in nf_view::load_impl).
    let mut session: std::collections::HashSet<u32> = Default::default();
    let mut actual: String = format!("{GOLDEN_HEADER}\n");
    for name in STORY {
        let inv = inventory(&d, name, &weapons, &session);
        // add THIS level's objects to the session for the next one
        let level = d.level(&format!("{name}.bin"));
        for h in level.by_hash.keys() { session.insert(*h); }
        eprintln!("[{}] tp {}/{} flash {}/{} pickups {}/{} proj {}/{} fp {}", name,
            inv.tp_ok, inv.tp_ok + inv.tp_miss, inv.flash_ok, inv.flash_ok + inv.flash_miss,
            inv.pickups, inv.pickups + inv.pickup_miss, inv.proj, inv.proj + inv.proj_miss, inv.fp_present);
        actual.push_str(&row(name, &inv));
        actual.push('\n');
    }
    eprintln!("---- actual table ----\n{actual}---- end ----");
    // The pinned golden (numbers only, measured with the session union above). The nonzero
    // miss columns are the game's own artifacts, each measured:
    // - proj/proj_miss (re-measured 2026-10-15, fix-start-loadouts): the proj lanes count the
    //   START LOADOUT's own projectile meshes (Player_InitAmmoWeapons 0x1a17f8 + the per-level
    //   Player_InitWeapon 0x1a7ef8 kit) on top of the placements'; the misses are kit meshes the
    //   session union has not loaded by that level — `Bullet_update` skips the draw when the
    //   hashtable misses (the `iVar14 == 0` arm [M 0x12e7b0]), so the port skips them too.
    // - 07000001/03's guard proj_miss: the stun-throwing guards (param 11 selects weapon 0x36 [M
    //   NDrone2_DefaultInit 0x14b300]) whose stun mesh 0x0200071f is on NO archive the session has
    //   loaded by those levels [D scan] — `Bullet_update` skips the draw when the hashtable misses
    //   (the `iVar14 == 0` arm [M 0x12e7b0]), so the port skips it too.
    // - 07000004/16 tp_miss 1+2: skin 0x0500001e carries no datum 4 (the SMG's +0x50), and
    //   `AnimDatumSetEntity` (0x1c7848) no-ops on a missing datum id [M] — the game draws that
    //   guard unarmed.
    let expected = format!("{GOLDEN_HEADER}
\
07000001	20	19	0	19	0	6	0	10	7	19
\
07000002	25	21	0	21	0	8	0	10	7	19
\
07000003	20	17	0	17	0	5	0	10	7	19
\
07000004	1	0	1	1	0	4	0	10	7	20
\
07000005	16	16	0	13	0	6	0	10	7	22
\
07000006	32	31	0	23	0	11	0	10	7	22
\
07000007	58	8	0	8	0	3	0	10	7	15
\
07000008	26	26	0	26	0	8	0	10	7	18
\
07000009	6	0	0	0	0	3	0	10	7	19
\
0700000a	20	0	0	0	0	5	0	10	7	19
\
0700000b	10	10	0	10	0	5	0	10	7	25
\
0700000c	40	40	0	40	0	15	0	12	5	23
\
0700000d	41	41	0	41	0	16	0	12	5	23
\
07000011	37	32	0	32	0	13	0	14	3	11
\
07000012	32	28	0	28	0	18	0	14	3	15
\
07000013	12	12	0	12	0	7	0	15	2	16
\
07000014	61	61	0	61	0	32	0	18	1	19
\
07000015	20	20	0	20	0	12	0	16	1	20
\
07000016	20	16	2	18	0	17	0	16	1	20
\
0700001b	20	19	0	18	0	7	0	17	1	2
");
    let diffs = nf_formats::golden::diff_tsv(&expected, &actual);
    assert!(diffs.is_empty(), "the weapon-model golden changed:\n{}\nactual:\n{actual}", diffs.join("\n"));
}




/// The stun grenade's drawn body (`weapon_data[0x35]+0x5C` = 0x02000726 [L ee_start_05.bin @0x2bf150
/// + 0x35·0x10C + 0x5C]): `Bullet_update` state 0 sets the mesh on the bullet object
/// (`hashtable_set_object_to_entity_gfx` disasm 0x12e8f0-0x12e900 [M]) and `View_SetDrawInAllViews`
/// draws it at the flight matrix every tick. Driven through the real input path: the panel gives the
/// weapon, `Sim::step_bond` pulls the trigger.
#[test]
fn a_thrown_grenade_draws_its_weapon_mesh() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    // Give Bond the stun grenade (the panel's `do=weapon` command — the dev path a player uses) and
    // switch to it the way the viewer does (`Input.switch_to` → the holster/draw cycle).
    let (status, switch) = nf_view::panel::apply(&mut scene, &[("do".into(), "weapon".into()), ("id".into(), "53".into())]);
    eprintln!("[panel] {status}");
    assert!(scene.view_models.contains_key(&0x35), "the stun grenade got a view model");
    assert_eq!(switch, Some(0x35));
    let sim = scene.sim.as_mut().unwrap();
    let mut input = nf_game::player::Input { switch_to: switch, ..Default::default() };
    for _ in 0..90 { sim.step_bond(input, false, false, false); if sim.gun.weapon == 0x35 { break; } input = Default::default(); }
    assert_eq!(sim.gun.weapon, 0x35, "the stun grenade is drawn");
    // Pull the trigger until the grenade spawns (the fire anim runs first; the round spawns when the
    // fire script's frame reaches `fire_anim_len` +0x44 [M Player_SetWeaponAnimObj case 9]).
    let mut thrown = false;
    for _ in 0..180 {
        sim.step_bond(nf_game::player::Input::default(), true, false, false);
        if !sim.drones.grenades.is_empty() { thrown = true; break; }
    }
    assert!(thrown, "the throw spawned a grenade");
    // The scene must carry the projectile-mesh instances (the stun's mesh + the frag's 0x020002af both land in the pool — the walk covers every weapon).
    let slot0 = scene.attachments.iter().find_map(|a| a.projectile.map(|(s, _)| s))
        .expect("projectile mesh instances built");
    assert_eq!(slot0, 0, "slot 0 is the first grenade slot");
    let stun = sim.weapons.projectile_mesh(0x35);
    assert_eq!(stun, 0x0200_0726, "the stun's +0x5C mesh [L] (0x35 = Stun Grenade; the frag is 0x34 → 0x020002af)");
    assert!(scene.attachments.iter().any(|a| a.projectile == Some((0, stun))), "a stun-mesh instance for grenade slot 0");
    // While the grenade flies, its instance is posed AT the grenade (not collapsed to the FAR point).
    let (gpos, att) = {
        let g = &sim.drones.grenades[0];
        (g.pos, scene.attachments.iter().position(|a| a.projectile == Some((0, stun))).unwrap())
    };
    let fs = scene.frame_state(None, 0.0, nf_view::scene::ViewOpts::default());
    let m = fs.attach[att];
    let near = (m[3][0] - gpos[0]).abs() + (m[3][1] - gpos[1]).abs() + (m[3][2] - gpos[2]).abs();
    assert!(near < 1.0, "the grenade mesh is drawn at the grenade ({m:?} vs {gpos:?})");
    // Sister: while NO grenade is live the slot collapses to the FAR point (nothing drawn).
    // Step the sim past the fuse (210 ticks) and check the attachment hides.
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(nf_game::player::Input::default(), false, false, false); }
    let live: Vec<_> = sim.drones.grenades.iter().filter(|g| g.state < 2).collect();
    if live.is_empty() {
        let fs = scene.frame_state(None, 0.0, nf_view::scene::ViewOpts::default());
        let m = fs.attach[att];
        assert!(m[3][1] < -1000.0, "the mesh slot hides when no grenade flies ({m:?})");
    }
}

/// The session object table (the merge in `nf_view::load_impl`): Tower-2A's guards (07000011) carry
/// weapons 0x10/0x1c/0x1d/0x1a whose muzzle-flash meshes 0x02000261/0x0200044a are on NO archive from
/// 07000011 on [D scan] — the game resolves them through the object hashtable the earlier story
/// archives filled. Before the merge the port drew 32 armed guards with no flash at all.
#[test]
fn tower_2_guards_get_their_muzzle_flashes_from_the_session() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000011.bin", false).expect("load 07000011");
    // The merged map objects landed in the sim's model table (`pickup::models` walks every map).
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.models.contains_key(&0x0200_0261), "Pistol_Muz (0x02000261) resolves on 07000011 via the session");
    assert!(sim.models.contains_key(&0x0200_044a), "weapon 0x1a's flash mesh resolves via the session");
    // The armed drones now carry muzzle-flash attachments (pre-fix: none of the 32).
    let flashes = scene.attachments.iter().filter(|a| a.flash.is_some()).count();
    assert!(flashes >= 32, "the armed guards' flashes resolve (got {flashes})");
    // The held guns of 07000016's SMG guard (0x0200025c, the same session mechanism) merge too.
    let scene = nf_view::load(&iso, "07000016.bin", false).expect("load 07000016");
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.models.contains_key(&0x0200_025c), "the SMG's held mesh resolves on 07000016 via the session");
    // Sister: the local resolutions are untouched — the first map/object indices still resolve to the
    // same objects (the merge only appends maps and fills by_hash gaps).
    let stats = scene.stats.clone();
    // The held-weapon count (the number before " weapons held"): a plain `contains("0 weapons held")` also
    // matched counts ending in 0 (3470 after fix-gadgets-live's AIMS-20 light attachments).
    let held: usize = stats.split(" weapons held").next().and_then(|h| h.rsplit(|c: char| !c.is_ascii_digit()).next())
        .and_then(|n| n.parse().ok()).unwrap_or(0);
    assert!(held > 0, "the scene still holds its weapons ({held}): {stats}");
}

/// Folded 190-fix-dev2: with `--dev`, EVERY selectable weapon has a first-person view model on
/// 07000005 (the missing FP skins merge from their holding archives, the way the game's session
/// tables accumulate). Normal play must not change: no `--dev`, no merged skins, and the weapons
/// beyond the level's own stay view-model-less (the cycle skips them, D-003).
#[test]
fn dev_gives_every_selectable_weapon_a_view_model_on_07000005() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // Normal play: only the level's own skins build view models.
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let normal: Vec<u8> = (1..0x72u8).filter(|w| scene.view_models.contains_key(w)).collect();
    eprintln!("[normal] {} view models", normal.len());
    // Dev: every selectable weapon (the cycle's members) has a view model.
    let scene = nf_view::load_dev(&iso, "07000005.bin", false).expect("dev load 07000005");
    let sim = scene.sim.as_ref().unwrap();
    let mut missing: Vec<u8> = Vec::new();
    for w in 1..0x72u8 {
        if !sim.weapons.selectable(w) { continue; }
        if sim.weapons.first_person(w as usize) == 0 { continue; }
        if !scene.view_models.contains_key(&w) { missing.push(w); }
    }
    assert!(missing.is_empty(), "selectable weapons without a dev view model: {:02x?}", missing);
    // The GPU skinning layout covers them (each view model is a character with a palette slot).
    let (layout, attrs) = scene.skin_layout();
    assert_eq!(layout.char_base.len(), scene.characters.len(), "every character has a palette slot");
    assert!(!attrs.is_empty(), "the skin attributes are built");
    // The panel catalog then reports a first-person model for every selectable weapon.
    let cat = nf_view::panel::catalog(&scene);
    let no_model = cat.matches("no 1st-person model here").count();
    assert_eq!(no_model, 0, "the catalog shows {no_model} weapons without a first-person model");
    // Sister: normal play stays unchanged — the dev-only weapons have no view model there.
    let l96 = 0x24u8; // the L96A1 (its skin is not in 07000005's archive [D])
    if !normal.contains(&l96) {
        let scene = nf_view::load(&iso, "07000005.bin", false).expect("reload");
        assert!(!scene.view_models.contains_key(&l96), "no --dev: the L96A1 has no view model on 07000005");
    }
}

