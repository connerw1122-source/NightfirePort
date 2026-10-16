//! fix-striped-boxes (102k): the red/yellow diagonal-striped box drawn on 07000005 (trench right,
//! against the castle wall; driver shot `--bond 16.67,30.04,58.60,1.755,-0.001 --frame 30` with
//! NF_NOPORTAL=1) is an editor/marker brush the game never draws.
//!
//! The game's rule [M]: a world object is submitted to the draw lists only when its glist-flags
//! copy (`obj+0xF0`, `Control_SetGList` 0x134960 copying `celglist+0x44`) passes
//! `(flags & 0x8010) == 0` — `View_AddObjects` 0x1e62d8:33 (disasm 0x1e6344-4c) — and the cel pass
//! refuses the same bit straight off the glist: `View_AddCels` 0x1e6ba0 L29/L46 `*(glist+0x44) & 0x10`.
//! Bit 0x10 = collision-only (blocking polys, skies, trigger volumes, touch plates, hurt volumes);
//! `Collide_Pick` does NOT skip it, so the brush stays solid but invisible.
//!
//! The striped instances on 07000005 all carry flags bit 0x10 (probe_striped_box.rs dump): the
//! kind-0x19 "blocker1"/"Blocking_*"/"BLOCKINGPOLY_*" cel placements (already skipped by the
//! port's static loop via `skipped_by_cel_pass`), "TouchPlate_GrappleLedge" (kind 0xfa →
//! `ThirdIcon_Create` 0x197070, live obj+0xF0 = 0x1050 [L ee_start_05.bin 0x016d43e0]) and
//! "WallHug_Blocker1_Short"/"WallHug_Blocker2_Windows" (kind 0x3b → `DynamicObject_Create`
//! 0x18dbf8, +0xF0 = *(glist+0x44) = 0x10). The port's models-misc sweep drew the 0xfa/0x3b ones
//! because it lacked the gate — the user's box.
//!
//! The test drives the real input path (`nf_view::load`) and reads the built draw list:
//! `Material.lflags` is the low 16 bits of the source object's glist flags word and
//! `Material.key` the `(map, texture)`, so "a striped-marker brush was drawn" is observable as a
//! World-layer material with lflags bit 0x10 / with the bold 32×32 red+yellow diagonal-stripe
//! texture. Opt-in via NF_ISO.

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_view::scene::{Layer, Scene};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

/// Fraction of pixels that are "red" (r high, g low, b low) and "yellow" (r and g high, b low).
fn stripe_frac(rgba: &[u8]) -> (f32, f32) {
    let n = (rgba.len() / 4) as f32;
    let (mut r, mut y) = (0usize, 0usize);
    for px in rgba.chunks_exact(4) {
        if px[3] < 8 { continue; }
        if px[0] >= 170 && px[1] < 90 && px[2] < 90 { r += 1; }
        else if px[0] >= 170 && px[1] >= 150 && px[2] < 90 { y += 1; }
    }
    (r as f32 / n, y as f32 / n)
}

/// The bold 32×32 diagonal red/yellow marker texture (07000005 map 50 idx 15, 0700000c map 9 idx 0
/// and map 45 idx 31 — identical pixels, red 63.9% / yellow 33.7% [D probe_striped_box.rs]).
/// Identified per level by the game's own rule: a 32×32 red+yellow texture EVERY of whose
/// referencing objects carries the no-draw bit 0x10 (the game refuses them all, so nothing ever
/// draws the texture). Legit red/yellow world textures referenced by drawable objects are not
/// markers (07000015's club decor draws in the game too).
fn marker_texture_keys(level: &Level) -> Vec<(usize, u32)> {
    let mut keys = Vec::new();
    for (mi, m) in level.maps.iter().enumerate() {
        'tex: for (ti, t) in m.textures.iter().enumerate() {
            if t.width != 32 || t.height != 32 { continue; }
            let px = t.palette_rgba8();
            let (r, y) = stripe_frac_by(&t.indices, &px);
            if r < 0.5 || y < 0.25 { continue; }
            let mut any_ref = false;
            for o in m.objects.iter() {
                if o.gfx.leaves.iter().flat_map(|l| l.batches.iter()).any(|b| b.texture == Some(ti as u32)) {
                    any_ref = true;
                    if o.flags & 0x10 == 0 { continue 'tex; }
                }
            }
            let _ = any_ref;
            keys.push((mi, ti as u32));
        }
    }
    keys
}

/// World-layer materials drawn from an object whose glist flags carry the no-draw bit 0x10, and
/// World-layer materials keyed on the striped marker texture. `lflags` is `obj.flags & 0xffff`
/// (scene.rs's add_instance), `key` the `(map, texture)` of the batch's texture.
fn marker_draws(sc: &Scene, level: &Level) -> (usize, usize, Vec<String>) {
    let keys = marker_texture_keys(level);
    let mut no_draw_bit = 0usize;
    let mut striped = 0usize;
    let mut spots: Vec<String> = Vec::new();
    for m in &sc.materials {
        if m.layer != Layer::World { continue; }
        let mut hit: Vec<String> = Vec::new();
        if m.lflags & 0x10 != 0 { no_draw_bit += 1; hit.push(format!("0x10-flags lflags={:#x}", m.lflags)); }
        if let Some(k) = m.key {
            if keys.contains(&k) { striped += 1; hit.push(format!("striped-texture {k:?}")); }
        }
        if !hit.is_empty() { spots.push(format!("{} at {:?}", hit.join(" + "), m.centre)); }
    }
    (no_draw_bit, striped, spots)
}

/// The game-drawn exception the census must allow for: LOAD_LEVEL triggers (kind 0xe8).
/// `Trigger_LoadLevelCreate` 0x1b2950 forces type 10 and `Trigger_Create` 0x1b1720 clears the
/// no-draw bit for exactly type 10 (0x1b18c8-d4 `obj+0xF0 &= 0xffffffef`) — the game draws these
/// meshes whatever their glist flags. Count of 0xe8 placements resolving to gfx-true 0x10-flagged
/// objects = the only no-draw-bit draws a faithful port may carry.
fn game_drawn_0x10_exceptions(level: &Level) -> usize {
    let mm = level.main_map();
    let mut n = 0;
    for (mi, m) in level.maps.iter().enumerate() {
        for p in m.placements.iter() {
            if (p.flags & 0xff) != 0xe8 { continue; }
            let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < m.objects.len() { Some((mi, p.index as usize)) } else { None };
            let Some((tmap, tobj)) = target else { continue };
            let o = &level.maps[tmap].objects[tobj];
            if o.flags & 0x10 != 0 && o.gfx.leaves.iter().any(|l| !l.batches.is_empty()) { n += 1; }
        }
    }
    let _ = mm;
    n
}

/// The user's symptom: on 07000005 the port draws the striped marker brushes (the trench-right box
/// is "WallHug_Blocker1_Short", kind 0x3b, at the placement (24.75, 32.17, 65.60) right of the
/// camera axis; "TouchPlate_GrappleLedge", kind 0xfa, world-baked at (−18.46, 37.43, −72.49)).
/// Post-fix: zero World draws carry the game's no-draw bit or the marker texture.
#[test]
fn striped_marker_brushes_not_drawn_on_07000005() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let level = Level::parse(&read_archive(&iso, "07000005.bin")).expect("parse");
    let (no_draw_bit, striped, spots) = marker_draws(&sc, &level);
    for s in spots.iter().take(12) { println!("marker draw: {s}"); }
    assert_eq!(no_draw_bit, 0, "07000005 draws World objects whose glist flags carry the game's no-draw bit 0x10 (View_AddObjects 0x1e62d8's 0x8010 gate)");
    assert_eq!(striped, 0, "07000005 draws World objects textured with the red/yellow diagonal marker texture");
    // Sister (over-hide): the 07000005 castle gate door (Door_Gate8, an 0xE8 LOAD_LEVEL placement
    // whose object carries no 0x10) must still draw — a World material within its extent.
    let mm = level.main_map().expect("main map");
    let gate = level.maps[mm].placements.iter().enumerate()
        .filter(|(_, p)| (p.flags & 0xff) == 0xe8)
        .find_map(|(_, p)| {
            let t = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < level.maps[mm].objects.len() { Some((mm, p.index as usize)) } else { None };
            t.map(|(tm, to)| (p, tm, to)).filter(|(_, tm, to)| level.maps[*tm].objects[*to].name == "Door_Gate8")
        });
    if let Some((p, tm, to)) = gate {
        let o = &level.maps[tm].objects[to];
        assert_eq!(o.flags & 0x10, 0, "Door_Gate8 must not carry the no-draw bit");
        let approx = [p.translation[0] + o.centre[0], p.translation[1] + o.centre[1], p.translation[2] + o.centre[2]];
        let near = sc.materials.iter().any(|m| m.layer == Layer::World
            && (m.centre[0] - approx[0]).abs() < o.radius + 2.0
            && (m.centre[1] - approx[1]).abs() < o.radius + 2.0
            && (m.centre[2] - approx[2]).abs() < o.radius + 2.0);
        assert!(near, "the castle gate Door_Gate8 must still draw post-fix (centre ~{approx:?})");
    } else {
        panic!("07000005 must carry the Door_Gate8 LOAD_LEVEL placement (the castle gate)");
    }
    // Sister: the grapple-point pulleys are REAL draws (class-0x51 objects hold the graple_pulley
    // glist, +0xF0 = 0x4000, +0xF8 = 0x1f [L ee_start_05.bin 0x16d80a0/0x16d81c0]; the user's
    // playtest sees them). The new gate must not eat them.
    assert_eq!(sc.grapple_hooks_drawn, 2, "the two 07000005 grapple pulleys must keep drawing");
}

/// Sister: 0700000c — the user's check found no striped boxes there (after fix-gadgets2 removed
/// the roof markers nothing references the stripe texture any more: map 9 carries it unused [D
/// probe_striped_box.rs]), and "laptops" — an object whose own 16×16 hazard texture looks
/// striped but whose glist flags are 0 (the game draws it) — must keep drawing.
#[test]
fn striped_marker_brushes_not_drawn_on_0700000c() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    // The level parse: the bold 32×32 stripe texture exists here (map 9 idx 0 / map 45 idx 31) but
    // no object references it, and "laptops" (hazard texture, flags 0x0) resolves through the
    // static cel path — the game draws it, so the port must too.
    let level = Level::parse(&read_archive(&iso, "0700000c.bin")).expect("parse");
    let (no_draw_bit, striped, spots) = marker_draws(&sc, &level);
    for s in spots.iter().take(12) { println!("marker draw: {s}"); }
    assert_eq!(no_draw_bit, 0, "0700000c draws World objects whose glist flags carry the game's no-draw bit 0x10");
    assert_eq!(striped, 0, "0700000c draws World objects textured with the red/yellow diagonal marker texture");
    let mm = level.main_map().expect("main map");
    let laptops = level.maps.iter().enumerate()
        .flat_map(|(mi, m)| m.objects.iter().enumerate().map(move |(oi, o)| (mi, oi, o)))
        .find(|(_, _, o)| o.name == "laptops").expect("laptops object");
    assert_eq!(laptops.2.flags & 0x10, 0, "laptops must not carry the no-draw bit (the game draws it)");
    assert!(laptops.2.gfx.leaves.iter().any(|l| !l.batches.is_empty()), "laptops has a mesh");
    let placed = level.maps[mm].placements.iter().enumerate()
        .filter(|(_, p)| p.creates_cel())
        .filter(|(_, p)| if p.by_hash() { level.by_hash.get(&p.hashcode).copied() == Some((laptops.0, laptops.1)) }
            else { p.index as usize == laptops.1 && laptops.0 == mm })
        .count();
    assert!(placed >= 1, "laptops has a static-cel placement (it draws through the port's static loop)");
}

/// The census: every on-foot level of FILES.BIN (docs/systems/levels.md §7 — the driving missions
/// 07000023-29 and the shared packs 07000500/07000999 are out of scope) must draw zero instances
/// using the marker texture or carrying the no-draw bit. Levels are prefiltered by a cheap parse:
/// only archives that actually carry the marker texture or a 0x10-flagged object of a kind the
/// port's sweeps reach get the full `nf_view::load`.
#[test]
fn census_no_marker_brush_drawn_on_any_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = IsoImage::open(&iso).expect("open iso");
    let (entries, _bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let mut archives: Vec<String> = entries.iter().filter_map(|e| {
        let n = e.name.to_ascii_uppercase();
        n.starts_with("0700").then(|| e.name.clone())
    }).collect();
    archives.sort();
    let mut checked = 0usize;
    let mut skipped_no_markers = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for a in &archives {
        let code = u32::from_str_radix(a.trim_end_matches(".BIN"), 16).unwrap_or(0);
        if (0x0700_0023..=0x0700_0029).contains(&code) || code == 0x0700_0500 || code == 0x0700_0999 { continue; }
    let Ok(level) = Level::parse(&read_archive(&iso, a)) else { continue };
        if level.main_map().is_none() { continue; }
        // Cheap prefilter: does the archive carry (a) a bold 32×32 red+yellow diagonal texture, or
        // (b) any object with the no-draw bit whose kind reaches one of the port's sweeps?
        let has_marker_texture = level.maps.iter().any(|m| m.textures.iter().any(|t| {
            t.width == 32 && t.height == 32 && {
                let px = t.palette_rgba8();
                let (r, y) = stripe_frac_by(&t.indices, &px);
                r >= 0.5 && y >= 0.25
            }
        }));
        const SWEEP_KINDS: [u8; 16] = [0x2e, 0x30, 0x34, 0x35, 0x3b, 0x71, 0xd5, 0xd8, 0xe1, 0xe3, 0xda, 0xfa, 0xfd, 0xfe, 0x4a, 0x3a];
        let has_0x10_sweep_object = level.maps.iter().any(|m| m.objects.iter().any(|o| {
            o.flags & 0x10 != 0 && o.gfx.leaves.iter().any(|l| !l.batches.is_empty())
        })) && level.maps.iter().any(|m| m.placements.iter().any(|p| SWEEP_KINDS.contains(&((p.flags & 0xff) as u8)) || p.creates_cel()));
        if !has_marker_texture && !has_0x10_sweep_object { skipped_no_markers += 1; continue; }
        checked += 1;
        let sc = match nf_view::load(&iso, a, false) { Ok(sc) => sc, Err(e) => { failures.push(format!("{a}: load failed: {e}")); continue; } };
        let (no_draw_bit, striped, spots) = marker_draws(&sc, &level);
        // The equality pins BOTH directions: no 0x10 object draws except the game-drawn LOAD_LEVEL
        // meshes, and every one of THOSE still draws (an over-hide gate would drop the count below
        // the expectation — the audit's Hole A).
        let expect_0x10 = game_drawn_0x10_exceptions(&level);
        if no_draw_bit != expect_0x10 || striped != 0 {
            failures.push(format!("{a}: {no_draw_bit} no-draw-bit draws (expect {expect_0x10} = the game-drawn 0xe8 LOAD_LEVEL meshes), {striped} striped-texture draws (first: {:?})", spots.first()));
        }
        if has_marker_texture {
            println!("{a}: marker texture present, {no_draw_bit} no-draw-bit draws, {striped} striped draws post-fix");
        }
    }
    println!("census: {checked} levels fully checked, {skipped_no_markers} skipped (no marker texture and no 0x10 sweep object)");
    assert!(!failures.is_empty() == false, "marker-brush draws remain: {failures:?}");
    assert!(checked >= 2, "the census must have checked the archives carrying the markers (05 and 0c), got {checked}");
}

/// fix-striped-boxes2 — the gate's per-kind verdicts. Every FLAGS_COPYING-kind object the main
/// agent's disc probe listed carries the no-draw bit (its creator copies `celglist+0x44` into
/// obj+0xF0 — every creator verified at the scene.rs gate comment — and nothing clears the bit), so
/// the game never draws it: drawn must equal the UNFLAGGED count for each listed pair. The sisters
/// pin the same kinds on objects whose glist lacks the bit (0a's monitor + 3 PCQ worms, 41's two
/// third icons, 12's monitor, 0b/46's PCQ worms) — they must keep drawing, so an over-broad
/// kind-level hide fails here too. Fail-first: with the gate disabled the flagged objects draw and
/// every pair with a flagged object fails.
#[test]
fn probe_gate_verdicts_per_kind() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // (level, kind, the probe's flagged-object count) — name / kind / flags from the driver's probe.
    const PROBE: &[(&str, u8, usize)] = &[
        ("07000008.bin", 0xe1, 1),   // SmallExplosionBox (flags 0x30)
        ("0700000d.bin", 0x3b, 1),   // WallHug_Blocker1_Long (flags 0x10, 0/1 striped — hidden regardless of texture)
        ("07000011.bin", 0xfa, 2),   // 2x 3RD_Person_Icon (0x50)
        ("07000012.bin", 0x35, 1),   // "Computer to hack" (0x50) — the invisible use volume; the laptop is baked geometry
        ("07000012.bin", 0xe1, 1),   // ExplosionBox (0x30)
        ("07000012.bin", 0xfa, 1),   // 3RD_Person_Icon
        ("07000014.bin", 0x30, 12),  // 12x SWITCH_Consoles_Evilbase (0x50) — the visible consoles are baked geometry
        ("07000014.bin", 0xe1, 19),  // 19x DESTROY_BoxSatellite (0x30) — Destroy_Create 0x12c318:57-59 zeroes their smash target
        ("07000014.bin", 0xfa, 3),   // 3x 3RD_Person_Icon
        ("07000015.bin", 0xe1, 3),   // 3x DESTROY_BoxSatellite
        ("0700001b.bin", 0x4a, 8),   // 8x Apocalypse (0x30)
        ("0700001b.bin", 0xe1, 3),   // DESTROY_BoxMissile/BoxSatellite/BoxSmall
    ];
    // Sisters: same-kind objects WITHOUT the bit keep drawing — the gate is per-object flags.
    const SISTERS: &[(&str, u8, usize)] = &[
        ("0700000a.bin", 0x30, 1),   // the 0a monitor draws (no bit) while 14's 12 do not
        ("0700000a.bin", 0x35, 3),   // the 0a PCQ worms draw while 12's "Computer to hack" does not
        ("07000041.bin", 0xfa, 2),   // 41's two third icons draw while 05/11/12/14's do not
        ("07000012.bin", 0x30, 1),   // 12's own monitor draws
        ("0700000b.bin", 0x35, 1),   // 0b's PCQ worm draws
        ("07000046.bin", 0x35, 1),   // 46's PCQ worm draws
    ];
    // faster-live-tests: group the checks per archive — the two loops loaded 07000012 and
    // 07000014 three times each and re-parsed every archive per entry (18 loads / 18 parses for
    // 11 distinct archives). Same assertions per (name, kind), one load and one parse each; the
    // suite runs --test-threads=1 and nothing here mutates.
    let mut per_archive: std::collections::BTreeMap<&str, Vec<(u8, usize, bool)>> = std::collections::BTreeMap::new();
    for (name, kind, probe_bit) in PROBE { per_archive.entry(name).or_default().push((*kind, *probe_bit, true)); }
    for (name, kind, expect) in SISTERS { per_archive.entry(name).or_default().push((*kind, *expect, false)); }
    for (name, checks) in &per_archive {
        let sc = nf_view::load(&iso, name, false).unwrap_or_else(|e| panic!("load {name}: {e}"));
        let level = Level::parse(&read_archive(&iso, name)).unwrap_or_else(|e| panic!("parse {name}: {e}"));
        for (kind, want, is_probe) in checks {
            let (bit, nobit) = kind_flag_counts(&level, *kind);
            if *is_probe {
                assert_eq!(bit, *want, "{name} kind {kind:#x}: the probe's flagged count changed — re-check the gate scope");
                let drawn = sc.misc_drawn.get(kind).copied().unwrap_or(0);
                assert_eq!(drawn, nobit, "{name} kind {kind:#x}: drawn must equal the unflagged count (the probe's {bit} flagged objects must not draw)");
            } else {
                assert_eq!(bit, 0, "{name} kind {kind:#x}: the sister level must carry no flagged object");
                assert_eq!(nobit, *want, "{name} kind {kind:#x}: expectation must match the placements");
                assert_eq!(sc.misc_drawn.get(kind).copied().unwrap_or(0), *want, "{name} kind {kind:#x}: the unflagged peers must keep drawing");
            }
        }
    }
}

/// Placements of `kind` resolving to a mesh-ful, non-mirror object, split by the no-draw bit 0x10.
/// Applies the sweep's own 0x3b hidden-until-channel rule (param 0 ≠ 0 [M DynamicObject_Create
/// 0x18dbf8:31]) so the channel-hidden objects count in neither bucket.
fn kind_flag_counts(level: &Level, kind: u8) -> (usize, usize) {
    let Some(m) = level.main_map() else { return (0, 0) };
    let mut bit = 0;
    let mut nobit = 0;
    for p in &level.maps[m].placements {
        if (p.flags & 0xff) as u8 != kind { continue; }
        if kind == 0x3b && p.params.iter().find(|(i, _)| *i == 0).is_some_and(|(_, v)| *v != 0) { continue; }
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
        let Some((om, oi)) = target else { continue };
        let o = &level.maps[om].objects[oi];
        if !o.gfx.leaves.iter().any(|l| !l.batches.is_empty()) || o.flags & 0x400 != 0 { continue; }
        if o.flags & 0x10 != 0 { bit += 1 } else { nobit += 1 }
    }
    (bit, nobit)
}

fn stripe_frac_by(indices: &[u8], px: &[[u8; 4]]) -> (f32, f32) {
    let n = indices.len() as f32;
    let (mut r, mut y) = (0usize, 0usize);
    for &i in indices {
        let c = px[i as usize];
        if c[3] < 8 { continue; }
        if c[0] >= 170 && c[1] < 90 && c[2] < 90 { r += 1; }
        else if c[0] >= 170 && c[1] >= 150 && c[2] < 90 { y += 1; }
    }
    (r as f32 / n, y as f32 / n)
}
