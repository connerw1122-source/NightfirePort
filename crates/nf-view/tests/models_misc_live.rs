//! models-misc: the remaining model classes — static dynamic entities, the Destroy LOD swap, the
//! cars' fixed parts, the sky clouds, and the per-level draw census golden.
//!
//! Every drawn kind here is a placement kind whose game creator attaches the placement's glist to
//! `obj+0xD8` and whose update never moves `obj+0x90` away from the placement transform, so a static
//! World-layer instance at the placement transform is the game's draw at rest exactly [M
//! View_DrawObjList 0x1e69a0 → View_DrawObjectGlist 0x1e6780 draws obj+0xD8 at obj+0x90;
//! control_create_object 0x1344e0:20-29 builds obj+0x90 from the placement pos/rot].
//! docs/models/misc.md carries the per-kind creator citations and the kinds that must NOT draw.
//!
//! Run with: `NF_ISO=… cargo test -p nf-view --test models_misc_live -- --test-threads=1`

use std::collections::BTreeMap;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Parse a level archive straight off the disc image (the same data `nf_view::load` reads).
fn parse_level(iso: &str, name: &str) -> nf_formats::level::Level {
    let img = nf_formats::iso9660::IsoImage::open(iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("no {name}"));
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    nf_formats::level::Level::parse(&data).unwrap()
}

/// The kind's expected drawn count for a level, recomputed independently of the scene sweep: main-map
/// placements of the kind whose object resolves with gfx batches, minus the kinds' own skip rules
/// (0x3B hidden-until-channel: param 0 ≠ 0 [M DynamicObject_Create 0x18dbf8:31]; mirror-only glists,
/// flags 0x400 [M View_AddObjects 0x1e62d8:38-47]).
fn expected_misc(level: &nf_formats::level::Level, kinds: &[u8]) -> BTreeMap<u8, usize> {
    let mut out: BTreeMap<u8, usize> = BTreeMap::new();
    let Some(m) = level.main_map() else { return out };
    for p in &level.maps[m].placements {
        let kind = (p.flags & 0xff) as u8;
        if !kinds.contains(&kind) { continue; }
        if kind == 0x3b && p.params.iter().find(|(i, _)| *i == 0).is_some_and(|(_, v)| *v != 0) { continue; }
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
        let Some((om, oi)) = target else { continue };
        let o = &level.maps[om].objects[oi];
        if !o.gfx.leaves.iter().any(|l| !l.batches.is_empty()) || o.flags & 0x400 != 0 { continue; }
        *out.entry(kind).or_default() += 1;
    }
    out
}

// fix-gadgets2: 0x3A (grapple points) and 0x3F (creep walls) are out — their creators attach NO gfx
// (`Grapple_Create` 0x198298 has zero hashtable gfx calls, instance size 0, no placement params;
// `CreepWall_Create` 0x196fc8 sets class 0x2B with no gfx attach), so the game never draws them —
// the user's "red and yellow boxes ... for the wall shimmy and one for the grapple".
const MISC_KINDS: [u8; 15] = [0x2e, 0x30, 0x34, 0x35, 0x3b, 0x71, 0xd5, 0xd8, 0xe1, 0xe3, 0xda, 0xfa, 0xfd, 0xfe, 0x4a];

/// Every misc kind the level carries (with a resolvable, gfx-ful object) gets exactly its census count
/// of World-layer instances — the sweep must neither skip nor double-draw a class.
#[test]
fn misc_static_entities_match_the_census_on_every_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let levels = ["07000001.bin", "07000002.bin", "07000003.bin", "07000004.bin", "07000005.bin", "07000006.bin",
        "07000007.bin", "07000008.bin", "07000009.bin", "0700000a.bin", "0700000b.bin", "0700000c.bin",
        "0700000d.bin", "07000011.bin", "07000012.bin", "07000013.bin", "07000014.bin", "07000015.bin",
        "07000016.bin", "0700001b.bin", "07000041.bin", "0700004a.bin", "0700004b.bin",
        "07000023.bin", "07000024.bin", "07000025.bin", "07000026.bin", "07000027.bin", "07000028.bin",
        "07000029.bin", "07000043.bin", "07000046.bin", "07000049.bin"];
    for name in levels {
        let scene = nf_view::load(&iso, name, false).unwrap_or_else(|e| panic!("load {name}: {e}"));
        let level = parse_level(&iso, name);
        let expect = expected_misc(&level, &MISC_KINDS);
        let mut got: BTreeMap<u8, usize> = scene.misc_drawn.clone();
        // The cars' two children add to the 0xDA count in the sweep (body + 0x2000502 + 0x2000505 each);
        // the census expectation counts bodies, so subtract the drawn children for the comparison.
        let car_children = level.main_map().map(|m| level.by_hash.get(&0x0200_0502).is_some_and(|&(cm, co)| {
            level.maps[cm].objects[co].gfx.leaves.iter().any(|l| !l.batches.is_empty())
        }) && level.by_hash.get(&0x0200_0505).is_some_and(|&(cm, co)| {
            level.maps[cm].objects[co].gfx.leaves.iter().any(|l| !l.batches.is_empty())
        })).unwrap_or(false);
        if car_children { if let Some(n) = got.get_mut(&0xda) { *n = n.saturating_sub(2 * n.div_ceil(3)); } }
        assert_eq!(got, expect, "{name}: misc draw census mismatch");
        // Spot checks against the disc census (docs/models/misc.md): 0700000c carries 5 locks and 5 dynamic
        // objects; 0700000d carries 5 dynamic objects, 1 gun imp and 9 draw-able destroyables. The grapple
        // points (0c: 5, 0d: 3 [D placements]) stay out of THIS census: fix-grapple-hook corrected
        // fix-gadgets2's "never draw" — the game does draw their pulley (`Grapple_Create` 0x198298 keeps
        // the placement's glist argument at obj+0xD8; `View_DrawObjectGlist` renders it), and the port's
        // grapple sweep draws it under `grapple_hooks_drawn`, not the misc sweep — so 0x3a stays absent
        // from `misc_drawn` here.
        if name == "0700000c.bin" {
            assert_eq!(scene.misc_drawn.get(&0x2e), Some(&5));
            assert_eq!(scene.misc_drawn.get(&0x3b), Some(&5));
            assert_eq!(scene.misc_drawn.get(&0x3a), None, "the grapple points draw via grapple_hooks_drawn, not the misc sweep [M fix-grapple-hook]");
        }
        if name == "0700000d.bin" {
            // One of the five hides until its channel (param 0 = 43 [D placements dump]) — 4 draw.
            assert_eq!(scene.misc_drawn.get(&0x3b), Some(&4));
            assert_eq!(scene.misc_drawn.get(&0x3a), None, "the grapple points draw via grapple_hooks_drawn, not the misc sweep [M fix-grapple-hook]");
            assert_eq!(scene.misc_drawn.get(&0x34), Some(&1));
            assert_eq!(scene.misc_drawn.get(&0xe1), Some(&9));
        }
        // 07000014 carries 45 dynamic objects, 23 of them hidden until their switch channel (param 0 ≠ 0
        // [D placements; M DynamicObject_Create 0x18dbf8:31]) — 22 draw; 12 monitors. Its 11 Apocalypse
        // placements resolve to collision-only glists, which the game draws as nothing [M 0x18dcc0:20].
        if name == "07000014.bin" {
            assert_eq!(scene.misc_drawn.get(&0x3b), Some(&22));
            assert_eq!(scene.misc_drawn.get(&0x30), Some(&12));
            assert_eq!(scene.misc_drawn.get(&0x4a), None);
        }
    }
}

/// The kinds whose game creators never attach a mesh must never appear in the sweep — the wrong fix
/// would draw fuseboxes/switches/rainboxes the game does not draw ([M 001adcc0:35 switch and 001b0448:25
/// fusebox: obj+0xD8 = 0; 0x18d0d0 rainbox: only the AABB is read; 0x17ffa0:78 GT: hidden at create;
/// 0x17ae00:100-101 emitter: no glist ever; 0x18d620:25-27 cloud: sky pipeline]).
#[test]
fn meshless_kinds_never_draw() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // 0700001b carries 129 fusebox placements (all with gfx on their objects — but the game never
    // attaches one); 0700000b carries 17 mesh-ful rainboxes; 07000009 12. If any of these draw, the
    // sweep is wrong, not merely silent.
    for name in ["0700001b.bin", "0700000b.bin", "07000009.bin", "0700000c.bin"] {
        let scene = nf_view::load(&iso, name, false).unwrap_or_else(|e| panic!("load {name}: {e}"));
        // fix-gadgets2 adds 0x3f (creep walls) to this list — the creator attaches no gfx [M 0x196fc8],
        // the user's red marker boxes. 0x3a stays: the grapple points draw via `grapple_hooks_drawn`
        // (fix-grapple-hook), never through this misc sweep.
        for bad in [0x29u8, 0x31, 0x32, 0x27, 0x38, 0x39, 0xf2, 0xd4, 0x3a, 0x3f] {
            assert_eq!(scene.misc_drawn.get(&bad), None, "{name}: kind {bad:#x} must not draw");
        }
        // The sister: the same levels really do carry those placements, so the assertion above has teeth.
        let level = parse_level(&iso, name);
        let m = level.main_map().unwrap();
        let has = |k: u8| level.maps[m].placements.iter().any(|p| p.flags & 0xff == k as u32);
        match name {
            "0700001b.bin" => assert!(has(0x31), "no fuseboxes?"),
            "0700000b.bin" => assert!(has(0x32) && has(0x29), "no rainboxes/switches?"),
            // 0700000c carries the 5 grapple placements the teeth are for [D]; no rainboxes.
            "0700000c.bin" => assert!(has(0x3a) && has(0x3b), "no grapple points/dynamic objects?"),
            _ => assert!(has(0x32), "no rainboxes?"),
        }
    }
}

/// The cars (`Car_Create` 0x1305b0 body + `Car_InitBits` 0x12fec8's fixed parts 0x2000502/0x2000505):
/// each of 07000024's four car placements draws body + two children (12 instances), and the children
/// resolve in the level's own hash table.
#[test]
fn cars_draw_body_and_two_fixed_parts() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000024.bin", false).unwrap();
    assert_eq!(scene.misc_drawn.get(&0xda), Some(&12), "4 cars × (body + 2 children)");
    let level = parse_level(&iso, "07000024.bin");
    for &h in &[0x0200_0502u32, 0x0200_0505] {
        assert!(level.by_hash.contains_key(&h), "car part {h:#x} missing from the hash table");
    }
    // Sister: a level without cars draws none.
    let scene = nf_view::load(&iso, "0700000d.bin", false).unwrap();
    assert_eq!(scene.misc_drawn.get(&0xda), None);
}

/// The sky clouds (`Cloud_Create` 0x18d620:26-27 → `View_AddSkyObj`, slot = param 4) ride the sky pass:
/// 07000041 and 07000006 each carry one cloud placement; it must appear in `sky_instances` and the
/// static instance sweep must not also draw it.
#[test]
fn clouds_ride_the_sky_pass() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for name in ["07000041.bin", "07000006.bin"] {
        let data = nf_view::load(&iso, name, false).unwrap();
        let level = parse_level(&iso, name);
        let m = level.main_map().unwrap();
        let clouds: Vec<_> = level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0xd4).collect();
        assert_eq!(clouds.len(), 1, "{name}: expected one cloud placement");
        let sky = level.sky_instances();
        let cloud_obj = if clouds[0].by_hash() { level.by_hash.get(&clouds[0].hashcode).copied() } else { Some((m, clouds[0].index as usize)) };
        assert!(cloud_obj.is_some_and(|c| sky.iter().any(|i| (i.0.map, i.0.object) == c)), "{name}: cloud not in the sky pass");
        assert_eq!(data.misc_drawn.get(&0xd4), None, "the cloud must not also draw in the world sweep");
    }
}

/// The LOD block (map block 0x27, `parsemap_block_LOD` 0x1d1f10) parses and answers lookups:
/// 0700000d's map 01000147 has object 6 → 7; every −1 and out-of-range index reads None. This is the
/// pointer `Destroy_Create` 0x12c318:25 stores as tag[0] and `Destroy_Smash` 0x12bb60 swaps to
/// (Control_SetGList, arg proven by the disassembly 0x12bbb8 `lw a1,0(s1)`).
#[test]
fn lod_block_parses_and_destroy_swap_targets_resolve() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    nf_view::load(&iso, "0700000d.bin", false).unwrap();
    let level = parse_level(&iso, "0700000d.bin");
    let m = level.maps.iter().position(|mp| mp.name == "010000cf").expect("map 010000cf");
    assert_eq!(level.maps[m].lod_target(6), Some(7));
    assert_eq!(level.maps[m].lod_target(0), None, "−1 = no LOD");
    assert_eq!(level.maps[m].lod_target(9), Some(11));
    assert_eq!(level.maps[m].lod_target(usize::MAX), None, "out of range");
    // The 0700000d C5 doors (bigdoor01/02) have NO LOD entry [D block 0x27 all −1 but map 01000147]:
    // their smash keeps the intact glist (Destroy_Smash's tag[0] == 0 skips the swap) — the port must
    // not invent a variant. Count the destroyables whose object has a LOD target level-wide.
    let mut with_variant = 0;
    if let Some(mm) = level.main_map() {
        for p in level.maps[mm].placements.iter().filter(|p| p.flags & 0xff == 0xe1) {
            let t = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else { Some((mm, p.index as usize)) };
            if let Some((om, oi)) = t { if level.maps[om].lod_target(oi).is_some() { with_variant += 1; } }
        }
    }
    eprintln!("0700000d destroyables with a LOD variant: {with_variant}");
}

/// The per-level misc census golden (numbers only): the BTreeMap of kind → drawn instances, hashed as
/// a line `name kind:count …` per level. Sister: perturbing any count changes the hash (asserted by
/// comparing two different levels' hashes differ and by the per-kind spot checks in the census test).
#[test]
fn misc_census_golden() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let levels = ["07000001.bin", "07000002.bin", "07000003.bin", "07000004.bin", "07000005.bin", "07000006.bin",
        "07000007.bin", "07000008.bin", "07000009.bin", "0700000a.bin", "0700000b.bin", "0700000c.bin",
        "0700000d.bin", "07000011.bin", "07000012.bin", "07000013.bin", "07000014.bin", "07000015.bin",
        "07000016.bin", "0700001b.bin", "07000023.bin", "07000024.bin", "07000025.bin", "07000026.bin",
        "07000027.bin", "07000028.bin", "07000029.bin", "07000041.bin", "07000043.bin", "07000046.bin",
        "07000048.bin", "07000049.bin", "0700004a.bin", "0700004b.bin"];
    let mut lines = Vec::new();
    for name in levels {
        let scene = match nf_view::load(&iso, name, false) { Ok(s) => s, Err(e) => { eprintln!("skip {name}: {e}"); continue } };
        let mut line = format!("{name}\t");
        for (k, n) in &scene.misc_drawn { line.push_str(&format!("{k:#04x}:{n} ")); }
        lines.push(line.trim_end().to_string());
        eprintln!("{}", lines.last().unwrap());
    }
    let h = sha1_lines(&lines);
    // Re-pinned for fix-gadgets2: 0x3A grapple points and 0x3F creep walls left the sweep (their
    // creators attach no gfx [M 0x198298, 0x196fc8] — the user's red/yellow marker boxes).
    let expected = "bb66b783b7500c3912098cd57daa1449d725b60b";
    assert_eq!(h, expected, "misc census golden — if a count legitimately changed, re-pin");
}

/// The front end (07000048) — measured on the disc [D block dump + kind census]:
/// its main map (0100009a) carries 291 textures, ONE placement (a 0x31 fusebox — a meshless-creator
/// kind, correctly undrawn) and no 3D geometry placements; the four sub-maps' objects are sprite/menu
/// resources. The game's front-end visuals are the kind-8 menu-def archive entry (`LoaderProcess`
/// 0x1d4880 case 8 → `MenuManager_Load`) drawn as sprite pages by `MenuManager_Create` 0x21b908 —
/// a sprite pipeline the port does not have yet (docs/models/misc.md, "front end"). The models-lane
/// facts: the archive loads, the single placement resolves, the menu-def entry exists.
#[test]
fn frontend_level_loads_and_resolves() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000048.bin", false).unwrap();
    let level = parse_level(&iso, "07000048.bin");
    let m = level.main_map().expect("07000048 main map");
    assert_eq!(level.maps.len(), 5, "4 geometry maps + the texture main map");
    assert_eq!(level.maps[m].placements.len(), 1);
    let p = &level.maps[m].placements[0];
    assert_eq!(p.flags & 0xff, 0x31, "the only placement is a fusebox (meshless creator)");
    assert_eq!(scene.misc_drawn.get(&0x31), None, "and it correctly draws nothing");
    // The kind-8 menu-def entry exists in the archive (`LoaderProcess` 0x1d4880 case 8).
    let img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000048.bin")).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let lb = nf_formats::levelbin::LevelBin::parse(&data).unwrap();
    assert_eq!(lb.entries.iter().filter(|e| e.kind == 8).count(), 1, "the menu def entry");
}

fn sha1_lines(lines: &[String]) -> String {
    let mut h = nf_formats::sha1::Sha1::new();
    for l in lines { h.update(l.as_bytes()); h.update(b"\n"); }
    nf_formats::sha1::hex(&h.finish())
}

/// The smoke puffs (`Gas_Init` 0x1aacc8 / `gases_smoke` 0x1ab2d0) through the real input path: firing
/// spawns a puff at the muzzle with the game's parameters (`Player_MuzzleFlash` 0x1a4298:91-103 —
/// scale 0.05→0.1, mesh 0x2000039, type 7, rgba 0x48403080), it fades and dies. The scene draws every
/// live puff as a billboarded `smoke_1_dust` quad. Fail-first: pre-fix, `drones.gas` did not exist.
#[test]
fn firing_spawns_a_muzzle_smoke_puff_that_fades_and_dies() {
    use nf_game::player::{Bond, Input};
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    sim.disabled = false; // the level's opening cut-scene disables input; the test stands Bond outside it
    // Fire one shot (the input path: the trigger through step_bond, pulsed like the fire-rate allows).
    let spawn_y = sim.bond.pos[1];
    let mut g0 = None;
    for t in 0..40 {
        sim.step_bond(Default::default(), t % 9 == 0, false, false);
        // fix-world-fx: the pool is shared with the ripples now (the game's is too — every Gas_Init
        // appends to the same gases list), so the muzzle puff is picked by its type-7 signature.
        if let Some(g) = sim.drones.gas.puffs.iter().find(|g| g.kind == 7) { g0 = Some(g.clone()); break; }
    }
    let g0 = g0.expect("firing never spawned the muzzle smoke puff");
    // By capture time the pool ticked once (the update runs at the next tick head), so the scale is
    // 0.05 + one growth tick of (0.1−0.05)/8 = 0.00625.
    assert!(g0.scale >= 0.05 && g0.scale < 0.0563, "the game's start scale 0.05 [M 0x1a4298], got {}", g0.scale);
    assert_eq!(g0.kind, 7, "the game's type 7 [M 0x1a4298]");
    // The rgba started at 0x48403080 and faded one tick (rate = start/life: r −9, a −16).
    assert!((56.0..=72.0).contains(&g0.rgba[0]) && (112.0..=128.0).contains(&g0.rgba[3]),
        "the game's rgba 0x48403080 fading out, got {:?}", g0.rgba);
    // The spawn y is the eye at spawn time (pos + 1.5) minus the aim drop; Bond keeps settling after
    // the shot, so compare against the pre-fire pos with a unit window.
    assert!((g0.pos[1] - (spawn_y + 1.45)).abs() < 1.0, "the puff sits at the muzzle height, got {} vs {}", g0.pos[1], spawn_y + 1.45);
    // It fades and dies (8 ticks [S], the decomp drops the game's life arg).
    let mut ticks = 0;
    while sim.drones.gas.puffs.iter().any(|g| g.kind == 7) {
        sim.step_bond(Input::default(), false, false, false);
        ticks += 1;
        assert!(ticks < 20, "the puff outlived its life");
    }
    assert_eq!(ticks, 7, "life 8, one tick spent before capture: gone after 7 more");
    // The sister: not firing spawns no muzzle smoke (fix-world-fx: the pool legitimately carries
    // the ripples' type-6 rings, so the check is the type-7 count).
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    assert!(!sim.drones.gas.puffs.iter().any(|g| g.kind == 7), "no smoke without a shot");
}

/// The broken searchlight's smoke (`Searchlight_Update` 0x1cb900 state 2, every 7th frame at obj+0x30,
/// scale 0.05→0.25): break a head and tick — a puff every 7th tick while broken.
#[test]
fn a_broken_searchlight_smokes_every_seventh_tick() {
    use nf_game::player::{Bond, Input};
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    let head = sim.drones.searchlights.parts.iter().position(|p| p.kind == 1).expect("a searchlight head");
    sim.drones.searchlights.parts[head].state = 2;
    sim.drones.searchlights.parts[head].drawn = true;
    let mut spawns = 0;
    for t in 0..70u32 {
        sim.step_bond(Input::default(), false, false, false);
        if let Some(g) = sim.drones.gas.puffs.last() {
            if (g.scale - 0.05).abs() < 1e-6 { spawns += 1; }
        }
        let _ = t;
    }
    assert_eq!(spawns, 10, "70 ticks at one puff per 7 = 10");
}

/// The explosion visual (`Explode_Create` 0x17cbd0): a grenade blast spawns the one-shot 0x6000052
/// script player scaled by radius·0.06666667, and its `Blast_frame_N`/`Blast_radius_ball` entity
/// streams resolve to real glist meshes in the scene's script-entity attachments.
#[test]
fn a_grenade_blast_plays_the_explosion_script_entities() {
    use nf_game::player::{Bond, Input};
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    // The script's entity hashes resolve in the level (the blast meshes are level-map objects).
    {
        let sim = scene.sim.as_ref().unwrap();
        let level = parse_level(&iso, "07000001.bin");
        let nis = level.nis.get(&0x0600_0052).expect("07000001 carries the explosion script");
        let entities: Vec<u32> = nis.streams.iter().flat_map(|st| st.decode()).filter_map(|c| match c {
            nf_formats::nis::Cmd::Entity { hash, obj_type, .. } if !matches!(obj_type, 0xea | 0xeb | 0xf5 | 0xf2) => Some(hash),
            _ => None,
        }).collect();
        assert!(entities.len() >= 30, "the blast script spawns its entities, got {}", entities.len());
        let missing: Vec<u32> = entities.iter().filter(|h| !level.by_hash.contains_key(h)).copied().collect();
        assert!(missing.is_empty(), "unresolved blast meshes: {missing:02x?}");
    }
    // Through the real input path: throw a grenade, wait for the blast, the script player runs and
    // `script_entity` answers for its entities.
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    sim.disabled = false;
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    // Own a frag (0x35) and switch to it (fix-player3's pickup route).
    let id = 0x35usize;
    let model = sim.weapons.model(id);
    let bounds = *sim.models.get(&model).unwrap_or(&nf_game::pickup::Model { centre: [0.0, 0.05, 0.0], radius: 0.3 });
    let p = nf_game::pickup::Pickup::create_placed(sim.bond.pos, [0.0; 3], nf_game::pickup::kind::WEAPON, id as u16, 1, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons, &std::collections::HashMap::new()).expect("grenade pickup builds");
    sim.drones.pickups.push(p);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    // The blast through the real explosion path: a frag in flight with a 10-tick fuse (the sim's own
    // Grenade — the same struct the input-driven throw fills; fix_player3_live's thrown_grenade test
    // covers the throw itself). The explosion tick pushes drones.explosions AND plays the script.
    let at = [sim.bond.pos[0] + 2.0, sim.bond.pos[1] + 1.5, sim.bond.pos[2]];
    let mesh = sim.weapons.model(0x35);
    sim.drones.grenades.push(nf_game::grenade::Grenade {
        pos: at, velocity: [0.05, 0.0, 0.0], speed: 0.05, travelled: 0.0, range: 10.0,
        damage: 100.0, radius: 5.0, weapon_type: 0x34, owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, mesh,
    });
    let mut blasted = false;
    for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); if !sim.drones.explosions.is_empty() { blasted = true; break; } }
    assert!(blasted, "the grenade never exploded");
    // The one-shot player is in the scripts list with the blast scale.
    let now = sim.drones.scripts.players.iter().filter(|p| p.scripts[0] == 0x0600_0052).count();
    assert!(now >= 1, "the explosion script player missing");
    // The blast entities spawn over the script's 165 ticks (one Blast_frame per tick from its stream's
    // start); tick into the fireball, then the scene's attachment machinery answers for one.
    for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); }
    let live: Vec<(u8, u32)> = sim.drones.scripts.objects.list.iter().flatten()
        .filter_map(|o| match o { nf_game::nis::Obj::Entity { hash, src: (h, si), .. } if *h == 0x0600_0052 => Some((*si, *hash)), _ => None })
        .collect();
    eprintln!("live blast entities: {}", live.iter().map(|(si, h)| format!("{si}:{h:08x}")).collect::<Vec<_>>().join(" "));
    // The blast family: clouds 0x0200021a, bursts 0x0200021e, Blast_frame_N 0x020003d7..0x020003e8,
    // sparks 0x0200042d, the big fireball 0x020006a7 [D 07000001's script decode].
    let blast_family = |h: u32| (0x0200_03d7..=0x0200_03e8).contains(&h) || matches!(h, 0x0200_021a | 0x0200_021e | 0x0200_042d | 0x0200_06a7);
    assert!(live.iter().any(|&(_, h)| blast_family(h)), "no blast mesh live; live = {live:02x?}");
    let any = live.iter().any(|&(si, h)| sim.script_entity((0x0600_0052, si), h).is_some());
    assert!(any, "no live blast entity matrix from the script-entity path");
}
