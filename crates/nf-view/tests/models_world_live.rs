//! models-world (task 041): every level's world geometry and placed static objects.
//!
//! The inventory lives in `docs/models/world.md`; the golden test below pins the per-level counts
//! (numbers and hashes only — no game bytes). Run the survey with
//! `NF_SURVEY=1 NF_ISO=<iso> cargo test -p nf-view --release models_survey -- --ignored --nocapture`.
//!
//! Evidence base (the game addresses the checks implement):
//! - `View_AddCels` 0x1e6ba0 skips a glist (celglist) whose flags word has bit 0x10 — the port's
//!   `MapObject::skipped_by_cel_pass` (level.rs).
//! - `parsemap_block_map_data_static` 0x1d1998 places only the placements with flag 0x8000
//!   (CREATE_CEL); the dynamic creators in `parsemap_create_dynamic_objects` 0x1d04f0 handle the rest
//!   (docs/atlas/object.md §Placement → creator dispatch).
//! - `psiCreateEntityGfx` 0x1171b0 resolves each texture REF's address word (the map texture index on
//!   disc) to a texture — an out-of-range index can never resolve (the port draws white).
//! - Per-batch GS state: the packet's `DIRECT` A+D writes apply in DMA order (G-018); the batch
//!   snapshots are the ones `Draw::from_gs` (scene.rs) maps to the port's draw state.

use nf_formats::disc;
use nf_formats::golden::diff_tsv;
use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;

fn iso_path() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The story level codes (docs/systems/levels.md §1 — the case order of `ResetMap_LevelCode2Img`
/// 0x1d5258; the driving codes 07000023–29 and the packs 07000500/07000999 are out of scope).
const STORY_LEVELS: [&str; 27] = [
    "07000001", "07000002", "07000003", "07000004", "07000005", "07000006", "07000007", "07000008",
    "07000009", "0700000a", "0700000b", "0700000c", "0700000d", "07000011", "07000012", "07000013",
    "07000014", "07000015", "07000016", "0700001b", "07000041", "07000043", "07000046", "07000048",
    "07000049", "0700004a", "0700004b",
];

/// One level's model inventory row (TSV; numbers only).
struct LevelRow {
    name: String,
    maps: usize,
    objects: usize,
    textures: usize,
    animated: usize,
    placements: usize,
    resolved: usize,
    unresolved: usize,
    static_kept: usize,
    cel_skipped: usize,
    /// Placement types the port resolves to a meshed object but draws through no pass (completeness gaps;
    /// the other model lanes' material — listed in docs/models/world.md).
    undrawn: String,
    leaves: usize,
    batches: usize,
    verts: usize,
    tris: usize,
    tex_oob: usize,
    uv_nonfinite: usize,
    alpha_zero: usize,
    alpha_off: usize,
    box_mismatch: usize,
    vc_mismatch: usize,
    skinned: usize,
}

fn inventory_row(lvl_id: u32, lvl: &Level) -> LevelRow {
    let mut r = LevelRow {
        name: format!("{lvl_id:08x}"), maps: lvl.maps.len(),
        objects: lvl.maps.iter().map(|m| m.objects.len()).sum(),
        textures: lvl.maps.iter().map(|m| m.textures.len()).sum(),
        animated: lvl.maps.iter().flat_map(|m| &m.textures).filter(|t| t.frames > 1).count(),
        placements: 0, resolved: 0, unresolved: 0, static_kept: 0, cel_skipped: 0,
        undrawn: String::new(), leaves: 0, batches: 0, verts: 0, tris: 0,
        tex_oob: 0, uv_nonfinite: 0, alpha_zero: 0, alpha_off: 0, box_mismatch: 0, vc_mismatch: 0,
        skinned: 0,
    };
    // Types the port draws through some pass (docs/models/world.md §placement types):
    // 0x0f drones (characters), 0x20 breakables, 0x22 ladders, 0x2a sky, 0xdb doors,
    // 0xde searchlights, 0xe2 SimpleScripts, 0xe8 exit triggers (visible mesh), 0xf0 pickups,
    // 0x3e wires, 0xd9 script players, 0xfc coronas, and everything with the 0x8000 CREATE_CEL flag
    // (statics). The MP-gated creators (0x24/0x25/0x26) don't create their objects in single-player.
    const DRAWN: &[u32] = &[0x0f, 0x20, 0x22, 0x2a, 0xdb, 0xde, 0xe2, 0xe8, 0xf0, 0x3e, 0xd9, 0xfc];
    let Some(m) = lvl.main_map() else { return r };
    let map = &lvl.maps[m];
    r.placements = map.placements.len();
    let mut undrawn: std::collections::BTreeMap<u32, usize> = Default::default();
    for p in &map.placements {
        let target = if p.by_hash() { lvl.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < map.objects.len() { Some((m, p.index as usize)) } else { None };
        let Some((om, oi)) = target else { r.unresolved += 1; continue };
        r.resolved += 1;
        let obj = &lvl.maps[om].objects[oi];
        let meshed = obj.gfx.leaves.iter().any(|l| !l.batches.is_empty());
        let ptype = p.flags & 0xff;
        let drawn = p.creates_cel() || DRAWN.contains(&ptype);
        if !drawn && meshed { *undrawn.entry(ptype).or_default() += 1; }
        if p.creates_cel() {
            if obj.skipped_by_cel_pass() { r.cel_skipped += 1; } else if meshed { r.static_kept += 1; }
        }
        if obj.gfx.is_skinned() && p.creates_cel() && ptype != 0x0f { r.skinned += 1; }
    }
    r.undrawn = undrawn.iter().map(|(k, v)| format!("{k:#04x}:{v}")).collect::<Vec<_>>().join(",");
    // Geometry totals and defect counts over every map's objects (disc data: each map is parsed
    // fresh, so the per-level numbers are the bytes this level loads).
    for map in &lvl.maps {
        let n_tex = map.textures.len();
        for obj in &map.objects {
            for leaf in &obj.gfx.leaves {
                r.leaves += 1;
                let mut lo = [f32::INFINITY; 3]; let mut hi = [f32::NEG_INFINITY; 3];
                let mut any = false;
                for b in &leaf.batches {
                    r.batches += 1;
                    r.verts += b.verts.len();
                    r.tris += b.triangles().len();
                    if let Some(t) = b.texture { if t as usize >= n_tex { r.tex_oob += 1; } }
                    for v in &b.verts {
                        if v.uv.iter().any(|c| !c.is_finite()) { r.uv_nonfinite += 1; }
                        match v.rgba[3] { 0 => r.alpha_zero += 1, 0x80 => {} _ => r.alpha_off += 1 }
                        for k in 0..3 {
                            lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]);
                        }
                        any = true;
                    }
                }
                let bx = &obj.gfx.boxes[leaf.box_index];
                if any {
                    // bit-exact bounds check (ps2gfx.md: min/max equal the decoded vertex bounds)
                    if lo != bx.a || hi != bx.b { r.box_mismatch += 1; }
                    // +0x30 field vs the decoded triangle count (U-020's per-level view)
                    if bx.vertex_count as usize != leaf.batches.iter().map(|b| b.triangles().len()).sum::<usize>() { r.vc_mismatch += 1; }
                }
            }
        }
    }
    r
}

/// Golden: every story level's model inventory matches the pinned counts, and every placement of
/// every story level resolves (the golden's `static_kept + cel_skipped + undrawn` set comes from the
/// same walk). The golden holds numbers only.
#[test]
fn story_levels_match_the_model_inventory_golden() {
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;
    let mut actual = String::from("level\tmaps\tplacements\tstatic_kept\tcel_skipped\tleaves\tbatches\tverts\ttris\ttex_oob\tuv_nonfinite\n");
    let mut total_unresolved = 0usize;
    for code in STORY_LEVELS {
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&format!("{code}.bin"))) else { panic!("no {code}.bin in FILES.BIN") };
        let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).expect("read level");
        let lvl = Level::parse(&data).unwrap_or_else(|e| panic!("{code}: parse failed: {e}"));
        let r = inventory_row(u32::from_str_radix(code, 16).unwrap(), &lvl);
        total_unresolved += r.unresolved;
        actual += &format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            r.name, r.maps, r.placements, r.static_kept, r.cel_skipped, r.leaves, r.batches, r.verts, r.tris, r.tex_oob, r.uv_nonfinite);
    }
    assert_eq!(total_unresolved, 0, "every placement of every story level must resolve to an object");
    let golden = include_str!("../golden/models_world.tsv");
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} golden differences:\n{}", d.len(), d.join("\n"));
    // Sister: a one-character change to the actual table must be caught by the same comparison.
    let mut lines: Vec<String> = actual.lines().map(String::from).collect();
    let row = &mut lines[1];
    let last = row.pop().unwrap();
    row.push(if last == '0' { '1' } else { '0' });
    assert_eq!(diff_tsv(golden, &(lines.join("\n") + "\n")).len(), 1, "sister: a one-character change must be caught");
}

/// The port's per-batch draw state must equal the decoded GS A+D writes (`Draw::from_gs` over every
/// batch's snapshot) — G-018's audit, on the level that carries every state: 07000041 has the env-map
/// overlays (ALPHA 0x58) and the FIX=0 mod batches (0x64); 07000005 exercises the normal world.
#[test]
fn draw_state_matches_the_decoded_gs_writes() {
    use nf_view::scene::Draw;
    // The six A/B/C/D shapes on the disc (models_world_live survey histograms [D]) and the mapping
    // they must produce (ps2tek GS ALPHA/TEST tables, *sourced*).
    let cases: [(u64, u64, u64, u8, u8, bool); 7] = [
        (0x44, 0x5001b, 0x3100_0000, 0, 1, true),          // normal, depth write, As >= 1
        (0x0000_007f_0000_0048, 0x5001b, 0x3100_0000, 1, 1, true),  // additive
        (0x0000_007f_0000_0042, 0x5001b, 0x3100_0000, 2, 1, true),  // subtractive
        (0x58, 0x5001b, 0x1_3100_0000, 4, 1, false),         // additive by destination alpha (env maps)
        (0x64, 0x5001b, 0x3100_0000, 3, 1, true),          // FIX=0 keep-destination, still depth-writing
        (0x44, 0x5000b, 0x3100_0000, 0, 0, true),          // GEQUAL 0 = always passes: no alpha test
        (0x44, 0x507fd, 0x1_3100_0000, 0, 2, false),         // GREATER 0x7f: the cutout test, no depth write
    ];
    for (alpha, test, zbuf, blend, atest, zwrite) in cases {
        let d = Draw::from_gs(1, 1, alpha, test, zbuf);
        assert_eq!((d.blend, d.atest, d.zwrite), (blend, atest, zwrite),
            "alpha {alpha:#x} test {test:#x} zbuf {zbuf:#x}");
    }
    // Sister: the old mapping (0x58/0x64 → normal) would fail these two.
    assert_ne!(Draw::from_gs(1, 1, 0x58, 0x5001b, 0x1_3100_0000).blend, 0, "0x58 is not the normal blend");
    assert_ne!(Draw::from_gs(1, 1, 0x64, 0x5001b, 0x3100_0000).blend, 0, "0x64 is not the normal blend");
    // And on the disc: every referenced object's batches map to a Draw the renderer can express, with
    // 0x58/0x64 batches present on 07000041 to prove the path runs through real data.
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;
    let mut env_batches = 0usize;
    for code in ["07000005", "07000041"] {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&format!("{code}.bin"))).unwrap();
        let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).unwrap();
        let lvl = Level::parse(&data).unwrap();
        for map in &lvl.maps {
            for obj in &map.objects {
                for leaf in &obj.gfx.leaves {
                    for b in &leaf.batches {
                        let d = Draw::from_gs(1, 1, b.alpha, b.test, b.zbuf);
                        assert!(d.blend <= 4 && d.atest <= 2, "unrepresentable draw state {d:?} on {code}");
                        if b.alpha & 0xff == 0x58 { env_batches += 1; assert_eq!(d.blend, 4); }
                        if b.alpha & 0xff == 0x64 { assert_eq!(d.blend, 3); }
                    }
                }
            }
        }
    }
    assert!(env_batches > 0, "07000041 must carry the env-map (0x58) batches on the disc");
}

/// The list-0 order (`View_AddCels` 0x1e6ba0 + `View_DrawObjList` 0x1e69a0): the statics draw first
/// in placement-parse order (the cel primaries' synthetic keys), the dynamics after them by distance.
/// On 07000007 this is the R2 hole fix: every baked shadow decal draws after the opaque static whose
/// geometry contains it. Without the parse-order key the front-to-back sort can put the decal first.
#[test]
fn statics_draw_in_parse_order_and_decals_after_their_walls() {
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&path, "07000007.bin", false).expect("load");
    // Replicate the renderer's list-0 sort (gpu.rs): key = −90000 + 10·chain for the statics (u32::MAX
    // chain = a distance-keyed object entry), then the squared distance; ties by material index.
    let world: Vec<(usize, &nf_view::scene::Material)> = scene.materials.iter().enumerate()
        .filter(|(_, m)| m.layer == nf_view::scene::Layer::World && m.draw.pass == 1).collect();
    assert!(world.iter().any(|(_, m)| m.chain != u32::MAX), "the level must have static World draws");
    let mut order: Vec<(f32, usize)> = world.iter().map(|(k, m)| {
        let d2 = m.centre.iter().map(|c| c * c).sum::<f32>();
        (if m.chain != u32::MAX { -90_000.0 + 10.0 * m.chain as f32 } else { d2 }, *k)
    }).collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    // 1. Every static draws before every dynamic World entry in pass 1.
    let n_statics = world.iter().filter(|(_, m)| m.chain != u32::MAX).count();
    assert!(order[..n_statics].iter().all(|(key, _)| *key < 0.0) && order[n_statics..].iter().all(|(key, _)| *key >= 0.0),
        "the statics (synthetic keys) must sort before the distance-keyed dynamics");
    // 2. The statics draw in placement-parse order (their chain ordinals ascend through the order).
    let chains: Vec<u32> = order[..n_statics].iter()
        .map(|(_, k)| world.iter().find(|(i, _)| i == k).unwrap().1.chain).collect();
    let mut sorted = chains.clone();
    sorted.sort_unstable();
    assert_eq!(chains, sorted, "statics must draw in placement-parse order");
    // 3. The statics' glist flag 0x1 no longer moves them to the alpha list (the cel primaries go to
    // list 0 unconditionally — `View_AddCels` 0x1e6ba0's push has no list choice).
    for (_, m) in world.iter().filter(|(_, m)| m.chain != u32::MAX) {
        assert_eq!(m.draw.pass, 1, "a static cel primary must draw in list 0 (pass 1), not the alpha list");
    }
    // 4. Decals: every alpha-shaped static texture's material draws after (larger chain than) the
    // opaque statics whose bounds overlap it — the R2 regression on 07000007's
    // `hall_shadows`/`banistershadows` decals [D fix-render4].
    let decal: Vec<_> = world.iter().filter(|(_, m)| {
        m.chain != u32::MAX && m.key.is_some_and(|k| scene.textures.iter().find(|t| t.key == k).is_some_and(|t| nf_view::gpu::alpha_shaped(&t.rgba)))
    }).collect();
    assert!(!decal.is_empty(), "07000007 must have alpha-shaped (decal) statics");
    for (_, d) in &decal {
        let wall_before = world.iter().any(|(_, m)| {
            m.chain != u32::MAX && m.chain < d.chain && m.key.is_some_and(|k| !scene.textures.iter().find(|t| t.key == k).is_some_and(|t| nf_view::gpu::alpha_shaped(&t.rgba)))
                && (0..3).all(|ax| (m.centre[ax] - d.centre[ax]).abs() < m.radius + d.radius)
        });
        assert!(wall_before, "the decal at {:?} must draw after an opaque static overlapping it", d.centre);
    }
}

/// LOD blocks (map block 0x1f, `parsemap_block_LOD` 0x1d1f10) and map-header entity counts: the block
/// holds 8-byte pairs (u32 record index, u32 ?) per entity — record i's pair names the pCelList record
/// whose glist replaces record i's on the cel's switch-channel change (`control_movement_object_handler`
/// 0x1336f8 swaps `cel+0x38` for 0xc047 cels). Measure per level: how many links, and whether each link
/// target is itself placed (model completeness).
#[test]
#[ignore]
fn probe_lod_blocks() {
    use nf_formats::map::{id, MapFile};
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;
    for fe in &entries {
        if !fe.name.starts_with("0700") || !fe.name.ends_with(".bin") { continue; }
        let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).expect("read level");
        let Ok(lvl) = Level::parse(&data) else { continue };
        for (mi, map) in lvl.maps.iter().enumerate() {
            // Re-derive the MapFile for the raw blocks (Level::parse drops them).
            let Ok(mf) = MapFile::parse(map_entry_bytes(&data, map)) else { continue };
            let entities = mf.first(id::MAP_HEADER).map(|b| {
                let p = b.payload(map_entry_bytes(&data, map));
                u32::from_le_bytes(p.get(4..8).unwrap_or(&[0; 4]).try_into().unwrap())
            });
            let lod_block = mf.first(id::LOD);
            if lod_block.is_none() { continue; }
            let payload = lod_block.unwrap().payload(map_entry_bytes(&data, map));
            let n = entities.unwrap_or(0) as usize;
            let mut links = Vec::new();
            for i in 0..n.min(payload.len() / 8) {
                let target = u32::from_le_bytes(payload[8 * i..8 * i + 4].try_into().unwrap());
                let spare = u32::from_le_bytes(payload[8 * i + 4..8 * i + 8].try_into().unwrap());
                if target != u32::MAX {
                    // Is the target object placed anywhere in the level (by index on its own map, or by
                    // hashcode anywhere)?
                    let placed = lvl.maps.iter().any(|m| m.placements.iter().any(|p| !p.by_hash() && p.index as u32 == target)
                        || lvl.by_hash.get(&lvl.maps[mi].objects.get(target as usize).map_or(0, |o| o.hashcode)).is_some_and(|&(hm, _)| lvl.maps[hm].objects.get(hm as usize).is_some()));
                    links.push(format!("{i}->{target}{}(spare {spare:#x})", if placed { "+" } else { "!" }));
                }
            }
            eprintln!("{} map {mi:#x} \"{}\": entities {n:?}, objects {}, LOD links [{}]",
                &fe.name[..8], map.name, map.objects.len(), links.join(" "));
        }
    }
}

/// The map entry's raw bytes for a parsed `LevelMap` (by entry index — the archive order is the
/// entry order of the level bin, and `LevelMap.entry` records it).
fn map_entry_bytes<'a>(data: &'a [u8], map: &nf_formats::level::LevelMap) -> &'a [u8] {
    let lb = nf_formats::levelbin::LevelBin::parse(data).expect("level bin");
    lb.data(data, &lb.entries[map.entry])
}

/// The two data defects the survey found: (1) 0700000d has one batch whose texture REF is out of the
/// map's texture range; (2) 0700004a has 243 boxes whose +0x30 field is larger than the decoded
/// triangle count (U-020's spike). Name them so the disc reading can be pinned [D].
#[test]
#[ignore]
fn probe_data_defects() {
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;
    for name in ["0700000d.bin", "0700004a.bin"] {
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)) else { continue };
        let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).expect("read level");
        let lvl = Level::parse(&data).expect("parse");
        for (mi, map) in lvl.maps.iter().enumerate() {
            let n_tex = map.textures.len();
            for (oi, obj) in map.objects.iter().enumerate() {
                for (li, leaf) in obj.gfx.leaves.iter().enumerate() {
                    for (bi, b) in leaf.batches.iter().enumerate() {
                        if let Some(t) = b.texture {
                            if t as usize >= n_tex {
                                eprintln!("TEX-OOB {name} map {mi:#x} obj {oi} \"{}\" hash {:08x} leaf {li} batch {bi}: texture {t} >= {n_tex} textures; nloop {} state alpha {:#x} test {:#x}",
                                    obj.name.trim(), obj.hashcode, b.nloop, b.alpha, b.test);
                            }
                        }
                    }
                    let bx = &obj.gfx.boxes[leaf.box_index];
                    if bx.packet != 0 {
                        let tris: usize = leaf.batches.iter().map(|b| b.triangles().len()).sum();
                        let verts: usize = leaf.batches.iter().map(|b| b.verts.len()).sum();
                        let kicked: usize = leaf.batches.iter().map(|b| b.triangles().len() + 2).sum::<usize>().min(verts);
                        if bx.vertex_count as usize != tris {
                            eprintln!("VC {name} map {mi:#x} obj {oi} \"{}\" leaf {li}: +0x30 {} vs tris {tris} verts {verts} kicked+2 {kicked} batches {}", obj.name.trim(), bx.vertex_count, leaf.batches.len());
                        }
                    }
                }
            }
        }
    }
}

#[test]
#[ignore]
fn probe_unusual_alpha_states() {
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;
    for fe in &entries {
        if !fe.name.starts_with("0700") || !fe.name.ends_with(".bin") { continue; }
        let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).expect("read level");
        let Ok(lvl) = Level::parse(&data) else { continue };
        for (mi, map) in lvl.maps.iter().enumerate() {
            for (oi, obj) in map.objects.iter().enumerate() {
                for (li, leaf) in obj.gfx.leaves.iter().enumerate() {
                    if leaf.batches.iter().any(|b| matches!(b.alpha & 0xff, 0x58 | 0x64)) {
                        eprintln!("{} map {mi:#x} obj {oi} \"{}\" hash {:08x} leaf {li}: {} batches, glist flags {:#x}",
                            &fe.name[..8], obj.name.trim(), obj.hashcode, leaf.batches.len(), obj.flags);
                        for (bi, b) in leaf.batches.iter().enumerate() {
                            eprintln!("  batch {bi}: alpha {:#x} test {:#x} zbuf {:#x} tex {:?} nloop {} verts {} tris {}",
                                b.alpha, b.test, b.zbuf, b.texture, b.nloop, b.verts.len(), b.triangles().len());
                        }
                    }
                }
            }
        }
    }
}

#[test]
#[ignore]
fn models_survey() {
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    if std::env::var_os("NF_SURVEY").is_none() { eprintln!("NF_SURVEY not set — skipping"); return }
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;

    // Global GS A+D write histogram: register → count, and value histograms for the state registers
    // the port consumes (ALPHA_1 0x42, TEST_1 0x47, ZBUF_1 0x4e) plus the ones G-018 names but the
    // port ignores (CLAMP_1 0x08, TEX1_1 0x14, TEX0_1 0x06).
    let mut reg_hist: std::collections::BTreeMap<u8, usize> = Default::default();
    let mut val_hist: std::collections::BTreeMap<u8, std::collections::BTreeMap<u64, usize>> = Default::default();
    // Per-batch (alpha, test, zbuf) draw-state triples.
    let mut triples: std::collections::BTreeMap<(u64, u64, u64), usize> = Default::default();

    let mut rows: Vec<LevelRow> = Vec::new();
    for fe in &entries {
        if !fe.name.starts_with("0700") || !fe.name.ends_with(".bin") { continue; }
        let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).expect("read level");
        let lvl_id = u32::from_str_radix(&fe.name[..8], 16).unwrap_or(0);
        let lvl = match Level::parse(&data) {
            Ok(l) => l,
            Err(e) => { eprintln!("{lvl_id:08x}: PARSE ERR {e}"); continue; }
        };
        let mut r = LevelRow {
            name: format!("{lvl_id:08x}"), maps: lvl.maps.len(),
            objects: lvl.maps.iter().map(|m| m.objects.len()).sum(),
            textures: lvl.maps.iter().map(|m| m.textures.len()).sum(),
            animated: lvl.maps.iter().flat_map(|m| &m.textures).filter(|t| t.frames > 1).count(),
            placements: 0, resolved: 0, unresolved: 0, static_kept: 0, cel_skipped: 0,
            undrawn: String::new(), leaves: 0, batches: 0, verts: 0, tris: 0,
            tex_oob: 0, uv_nonfinite: 0, alpha_zero: 0, alpha_off: 0, box_mismatch: 0, vc_mismatch: 0,
            skinned: 0,
        };
        // Types the port draws through some pass (docs/models/world.md §placement types):
        // 0x0f drones (characters), 0x20 breakables, 0x22 ladders, 0x2a sky, 0xdb doors,
        // 0xde searchlights, 0xe2 SimpleScripts, 0xe8 exit triggers (visible mesh), 0xf0 pickups,
        // 0x3e wires, 0xd9 script players, and everything with the 0x8000 CREATE_CEL flag (statics).
        const DRAWN: &[u32] = &[0x0f, 0x20, 0x22, 0x2a, 0xdb, 0xde, 0xe2, 0xe8, 0xf0, 0x3e, 0xd9, 0xfc];
        let Some(m) = lvl.main_map() else { rows.push(r); continue };
        let map = &lvl.maps[m];
        r.placements = map.placements.len();
        let mut undrawn: std::collections::BTreeMap<u32, usize> = Default::default();
        for p in &map.placements {
            let target = if p.by_hash() { lvl.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < map.objects.len() { Some((m, p.index as usize)) } else { None };
            let Some((om, oi)) = target else { r.unresolved += 1; continue };
            r.resolved += 1;
            let obj = &lvl.maps[om].objects[oi];
            let meshed = obj.gfx.leaves.iter().any(|l| !l.batches.is_empty());
            let ptype = p.flags & 0xff;
            let drawn = p.creates_cel() || DRAWN.contains(&ptype);
            if !drawn && meshed { *undrawn.entry(ptype).or_default() += 1; }
            if p.creates_cel() {
                if obj.skipped_by_cel_pass() { r.cel_skipped += 1; } else if meshed { r.static_kept += 1; }
            }
            if obj.gfx.is_skinned() && p.creates_cel() && ptype != 0x0f { r.skinned += 1; }
        }
        r.undrawn = undrawn.iter().map(|(k, v)| format!("{k:#04x}:{v}")).collect::<Vec<_>>().join(",");
        // Geometry totals and defect counts over every map's objects (disc data: each map is parsed
        // fresh, so the per-level numbers are the bytes this level loads).
        for map in &lvl.maps {
            let n_tex = map.textures.len();
            for obj in &map.objects {
                for leaf in &obj.gfx.leaves {
                    r.leaves += 1;
                    let mut lo = [f32::INFINITY; 3]; let mut hi = [f32::NEG_INFINITY; 3];
                    let mut any = false;
                    for b in &leaf.batches {
                        r.batches += 1;
                        r.verts += b.verts.len();
                        r.tris += b.triangles().len();
                        *triples.entry((b.alpha, b.test, b.zbuf)).or_default() += 1;
                        if let Some(t) = b.texture { if t as usize >= n_tex { r.tex_oob += 1; } }
                        for v in &b.verts {
                            if v.uv.iter().any(|c| !c.is_finite()) { r.uv_nonfinite += 1; }
                            match v.rgba[3] { 0 => r.alpha_zero += 1, 0x80 => {} _ => r.alpha_off += 1 }
                            for k in 0..3 {
                                lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]);
                            }
                            any = true;
                        }
                        for (reg, val, _) in &leaf.gs {
                            *reg_hist.entry(*reg).or_default() += 1;
                            if matches!(reg, 0x06 | 0x08 | 0x14 | 0x42 | 0x47 | 0x4e) {
                                *val_hist.entry(*reg).or_default().entry(*val).or_default() += 1;
                            }
                        }
                    }
                    let bx = &obj.gfx.boxes[leaf.box_index];
                    if any {
                        // bit-exact bounds check (ps2gfx.md: min/max equal the decoded vertex bounds)
                        if lo != bx.a || hi != bx.b { r.box_mismatch += 1; }
                        // +0x30 field vs the decoded triangle count (U-020's per-level view)
                        if bx.vertex_count as usize != leaf.batches.iter().map(|b| b.triangles().len()).sum::<usize>() { r.vc_mismatch += 1; }
                    }
                }
            }
        }
        rows.push(r);
    }
    println!("level\tmaps\tobjects\ttextures\tanimated\tplacements\tresolved\tunresolved\tstatic_kept\tcel_skipped\tundrawn_meshed\tleaves\tbatches\tverts\ttris\ttex_oob\tuv_nonfinite\talpha_zero\talpha_off\tbox_mismatch\tvc_mismatch\tstatic_skinned");
    for r in &rows {
        println!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            r.name, r.maps, r.objects, r.textures, r.animated, r.placements, r.resolved, r.unresolved,
            r.static_kept, r.cel_skipped, r.undrawn, r.leaves, r.batches, r.verts, r.tris,
            r.tex_oob, r.uv_nonfinite, r.alpha_zero, r.alpha_off, r.box_mismatch, r.vc_mismatch, r.skinned);
    }
    println!("\n== GS DIRECT A+D register histogram (all levels) ==");
    for (reg, n) in &reg_hist { println!("reg {reg:#04x}: {n}"); }
    println!("\n== value histograms for the state registers ==");
    for (reg, vals) in &val_hist {
        println!("reg {reg:#04x}:");
        for (v, n) in vals { println!("  {v:#018x}: {n}"); }
    }
    println!("\n== per-batch (ALPHA_1, TEST_1, ZBUF_1) triples ==");
    for ((a, t, z), n) in &triples { println!("alpha {a:#018x} test {t:#018x} zbuf {z:#018x}: {n}"); }
}

/// `WallHug_Blocker1_Long` on 0700000d map 0x31 carries the out-of-range texture REF (index 185 with
/// 185 textures). Check its glist flags and whether any placement draws it — the cel-pass skip (flag
/// 0x10) would mean the game never draws the batch either [D].
#[test]
#[ignore]
fn probe_blocker_object() {
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("0700000d.bin")).unwrap();
    let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).unwrap();
    let lvl = Level::parse(&data).unwrap();
    let map = &lvl.maps[0x31];
    for (oi, obj) in map.objects.iter().enumerate() {
        if obj.name.contains("WallHug") {
            let placements: Vec<usize> = (0..map.placements.len())
                .filter(|&pi| map.placements[pi].by_hash() && lvl.by_hash.get(&map.placements[pi].hashcode) == Some(&(0x31, oi))
                    || (!map.placements[pi].by_hash() && map.placements[pi].index as usize == oi))
                .collect();
            eprintln!("obj {oi} \"{}\" hash {:08x} flags {:#x} creates_cel placements {:?} leaves {} collision {}",
                obj.name.trim(), obj.hashcode, obj.flags, placements, obj.gfx.leaves.len(), obj.collision.is_some());
        }
    }
}

/// `--bond x,y,z,yaw,pitch` spots for the visual check (docs/models/world.md): for every class of
/// world model, one spot per level where it appears — a door, a breakable, a SimpleScript prop, a
/// baked shadow decal, an env-map object — computed from the real placements so the main agent can
/// render and eyeball each one [D].
#[test]
#[ignore]
fn bond_spots_for_every_model_class() {
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;
    println!("level\tclass\tx\ty\tz\tyaw\tpitch\tobject");
    for code in STORY_LEVELS {
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&format!("{code}.bin"))) else { continue };
        let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).expect("read level");
        let Ok(lvl) = Level::parse(&data) else { continue };
        let Some(m) = lvl.main_map() else { continue };
        let map = &lvl.maps[m];
        let mut done = std::collections::BTreeSet::<&str>::new();
        fn spot(code: &str, class: &'static str, done: &mut std::collections::BTreeSet<&'static str>, p: &nf_formats::placement::Placement, obj: &nf_formats::level::MapObject) {
            if !done.insert(class) { return; }
            if p.translation.iter().all(|c| c.abs() < 0.5) { println!("{code}\t{class}\t(at origin — script pose)\t0\t0\t0\t0\t-0.2\t{}", obj.name.trim()); return; }
            // Stand 3 units back along the placement's yaw and look at it slightly down.
            let yaw = p.rotation[1];
            let (sy, cy) = yaw.sin_cos();
            let (x, z) = (p.translation[0] - sy * 3.0, p.translation[2] - cy * 3.0);
            let y = p.translation[1] + 1.6;
            println!("{code}\t{class}\t{x:.2}\t{y:.2}\t{z:.2}\t{:.3}\t-0.2\t{}", yaw, obj.name.trim());
        }
        for p in &map.placements {
            let target = if p.by_hash() { lvl.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < map.objects.len() { Some((m, p.index as usize)) } else { None };
            let Some((om, oi)) = target else { continue };
            let obj = &lvl.maps[om].objects[oi];
            if !obj.gfx.leaves.iter().any(|l| !l.batches.is_empty()) { continue; }
            match p.flags & 0xff {
                0xdb => spot(code, "door", &mut done, p, obj),
                0x20 => spot(code, "breakable", &mut done, p, obj),
                0xe2 => spot(code, "simplescript", &mut done, p, obj),
                0x2a => spot(code, "sky", &mut done, p, obj),
                _ if p.creates_cel() && !obj.skipped_by_cel_pass() && obj.gfx.leaves.iter().any(|l| l.batches.iter().any(|b| b.texture.is_none())) => spot(code, "prop", &mut done, p, obj),
                _ => {}
            }
        }
        // The baked shadow decals: statics whose texture is alpha-shaped (the R2 class — drawn in
        // parse order by the fix; `gpu::alpha_shaped` is the classifier the decal_order test pins).
        for p in &map.placements {
            if !p.creates_cel() { continue; }
            let target = if p.by_hash() { lvl.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < map.objects.len() { Some((m, p.index as usize)) } else { None };
            let Some((om, oi)) = target else { continue };
            let obj = &lvl.maps[om].objects[oi];
            let Some(b) = obj.gfx.leaves.iter().flat_map(|l| &l.batches).find(|b| b.texture.is_some()) else { continue };
            let Some(t) = map.textures.get(b.texture.unwrap() as usize) else { continue };
            let rgba: Vec<u8> = t.frame(0).iter().flat_map(|&i| t.palette_rgba8()[i as usize]).collect();
            if nf_view::gpu::alpha_shaped(&rgba) {
                spot(code, "decal", &mut done, p, obj);
            }
        }
        // The env-map objects (07000041 only).
        for (oi, obj) in map.objects.iter().enumerate() {
            if !obj.name.contains("EnvMod") { continue; }
            if let Some(p) = map.placements.iter().filter(|p| (p.by_hash() && lvl.by_hash.get(&p.hashcode) == Some(&(m, oi))) || (!p.by_hash() && p.index as usize == oi))
                .max_by(|a, b| a.translation.iter().map(|c| c.abs()).sum::<f32>().total_cmp(&b.translation.iter().map(|c| c.abs()).sum::<f32>())) {
                spot(code, "envmap", &mut done, p, obj);
            }
        }
    }
}

/// G-014's disc-side measurement: the port reads the second stream as f32 UVs. If the game used
/// ST·Q (unbounded projective ST) the distribution would have a huge tail; UV semantics wrap in
/// [0,1]. Print the range histogram over every batch of the story levels [D].
#[test]
#[ignore]
fn uv_range_histogram() {
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;
    let (mut in01, mut out01, mut max) = (0usize, 0usize, 0.0f32);
    for code in STORY_LEVELS {
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&format!("{code}.bin"))) else { continue };
        let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).expect("read level");
        let Ok(lvl) = Level::parse(&data) else { continue };
        for map in &lvl.maps {
            for obj in &map.objects {
                for leaf in &obj.gfx.leaves {
                    for b in &leaf.batches {
                        for v in &b.verts {
                            for c in v.uv {
                                if !c.is_finite() { continue; }
                                max = max.max(c.abs());
                                if (-1024.0..=1024.0).contains(&c) { in01 += 1; } else { out01 += 1; }
                            }
                        }
                    }
                }
            }
        }
    }
    println!("uv coords within ±1024: {in01}, outside: {out01}, max |uv| {max}");
    assert!(in01 > out01 * 1000, "UVs must overwhelmingly stay in the UV range");
    // The 303 outliers are U-022's garbage floats around the known NaN vertex block
    // (`Room-L_atrium_bottom`, 0700000a/12); everything else stays in UV range.
    assert!(out01 < 400, "the garbage tail is the known U-022 family, nothing more");
}
