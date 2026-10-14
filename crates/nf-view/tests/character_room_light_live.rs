//! Headless NF_ISO tests for `character-room-light`: which room-cel ambient a character uses in the
//! game vs the port's lookup.
//!
//! Game [M]: every `obj+0xF0 & 0x800` object keeps a **linked cel** (`obj+0x20`,
//! `control_link_object_to_cel` 0x1346b8) — linked once at create (`Control_CreateObjEx` 0x1343c8:39 →
//! `build_LinkToRoom` 0x1c9680 → `build_FindCel` 0x1c8cb0 at the world sphere), re-linked per tick only
//! when the object leaves its cel (`control_handle_cel_change` 0x133288: `Cel_ObjectLeftCel` 0x1e0d58
//! walks the current cel's PORTALS and returns the neighbour across the crossed one; a miss keeps the
//! cel while `build_point_in_bounding_box` 0x1c9500 still contains the point; a full re-find only
//! otherwise), and objects with a proxy (`obj+0x1C`) are re-linked to the PROXY's cel each tick
//! (`control_movement_object_handler` 0x1336f8 relink pass). `Lights_CalcAmbientLight` 0x1cb178 then
//! ramps `obj+0x100..0x102` toward `cel + switch_channels[cel+0xa2]`, bytes +0x9a/0x9c/0x9e, through
//! `ClrStep(…, 0.1)` — and the link snaps the sentinel once.
//!
//! The port instead re-derives the cel from the position every tick with a global smallest-volume
//! bbox pick (`Sim::ambient_at` → `find_cel`). On the party level 07000007 the user still sees the
//! guests lit wrongly ("characters lit by the room": no). These tests pin the port to the game's
//! observable model as far as the disc data allows, and dump the diagnostics the fix needs.
//!
//! Opt-in via `NF_ISO`.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Step the sim `n` ticks with no input (the viewer's per-frame `step_bond`).
fn step(scene: &scene::Scene, n: usize) {
    let sim = scene.sim.as_ref().unwrap() as *const scene::Sim as *mut scene::Sim;
    for _ in 0..n {
        unsafe { (*sim).step_bond(Default::default(), false, false, false) };
    }
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1.0 / 255.0)
}

/// The multiplier floats `(tint × ambient) >> 8` of `View_SetupRenderModes_Tweak` (0x1e5888) for the
/// white tint, on the game's integer path.
fn expect_bytes(a: [u8; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| ((0xffu32 * a[k] as u32) >> 8) as f32 / 255.0)
}

#[test]
fn ballroom_guests_lit_by_their_own_room_on_07000007() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let sim = scene.sim.as_ref().expect("sim");
    step(&scene, 5);
    let cels = &sim.cels;
    eprintln!("07000007: {} room cels, {} characters", cels.len(), scene.characters.len());
    for (i, c) in cels.iter().enumerate() {
        eprintln!("  cel {i}: min {:?} max {:?} ambient {:?} zone {}", c.min, c.max, c.ambient, c.zone);
    }
    let mut unresolved = 0usize;
    let mut sentinel = 0usize;
    let mut distinct_mults: Vec<[f32; 3]> = Vec::new();
    for (ci, ch) in scene.characters.iter().enumerate() {
        let pos = [ch.matrix[3][0], ch.matrix[3][1], ch.matrix[3][2]];
        let di = sim.char_drone[ci];
        let amb = sim.char_ambient[ci];
        let mult = sim.char_multiplier(ci);
        // Every cel bbox containing the position (the port's tie-break input).
        let mut containing: Vec<usize> = Vec::new();
        for (i, c) in cels.iter().enumerate() {
            let inside = c.min[0] <= pos[0] && pos[0] <= c.max[0] && c.min[1] <= pos[1] && pos[1] <= c.max[1] && c.min[2] <= pos[2] && pos[2] <= c.max[2];
            if inside { containing.push(i); }
        }
        let picked = sim.cel_at(pos);
        let floor_y = ch.floor.map(|f| format!("{:.2}", f)).unwrap_or("?".into());
        eprintln!("char {ci}: skin {:08x} role {:?} drone {:?} pos ({:.2},{:.2},{:.2}) floor {} -> cels {:?} picked {:?} amb {:?} mult {:?}",
            ch.skin.hashcode, ch.role, di, pos[0], pos[1], pos[2], floor_y, containing, picked, amb, mult);
        if containing.is_empty() { unresolved += 1; }
        if amb == [0xff; 3] { sentinel += 1; }
        if !distinct_mults.iter().any(|m| close(*m, mult)) { distinct_mults.push(mult); }
        // The game never leaves a linked 0x800 object at the 0xff sentinel: the create-time link
        // (Control_CreateObjEx 0x1343c8:39 → control_link_object_to_cel 0x1346b8:27-31) snaps it.
        if di.is_some() {
            assert!(!close(mult, [1.0; 3]),
                "char {ci} (drone-backed) still renders at the pre-link 1.0 multiplier — the guest is full-bright");
        }
    }
    eprintln!("summary: unresolved {unresolved}, sentinel {sentinel}, distinct multipliers {}", distinct_mults.len());
    // The dynamic lights: which exist on this level, and what the character draws pick up
    // (`Lights_CalcClosestLights` 0x1cb2e0 with the skin flags 0x8a0 — bit 0x80 admits kinds 0/1/2).
    let lights = &sim.drones.lights;
    eprintln!("lights: {} in the pool", lights.lights.len());
    for (i, l) in lights.lights.iter().enumerate().take(24) {
        eprintln!("  light {i}: kind {} pos {:?} r {:.2} inten {:.2} rgb {:?} life {} active {}",
            l.kind, l.pos, l.radius, l.intensity, l.rgb, l.life, l.active as u8);
    }
    let kind0: Vec<_> = lights.lights.iter().filter(|l| l.kind == 0 && l.active).collect();
    eprintln!("kind-0 active: {}", kind0.len());
    // pick2 at three placed guests (chars 9, 30, 38 — different rooms): the frame-resolved light set
    // (`Scene::frame_lights`), what gpu.rs feeds each draw (`lights.pick2(centre, dc.lflags)`).
    let resolved = sim.drones.lights.resolve(&|i: u32| sim.drones.searchlights.parts.get(i as usize).map(|p| p.pos).unwrap_or([0.0; 3]));
    for ci in [9usize, 30, 38] {
        let ch = &scene.characters[ci];
        let p4 = [ch.matrix[3][0], ch.matrix[3][1] + 1.0, ch.matrix[3][2], 2.0];
        let (lp, lc) = resolved.pick2(p4, 0x8a0);
        eprintln!("guest {ci} at {:?}: lights {:?} colours {:?}", [p4[0], p4[1], p4[2]], lp, lc);
    }
    assert_eq!(sentinel, 0, "{sentinel} characters never snapped off the 0xff ambient sentinel");
    assert_eq!(unresolved, 0, "{unresolved} characters' positions are inside no room-cel bbox");
}

#[test]
fn bathhouse_civilian_skin_texels_are_not_white() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    // G-042: the 07000002 bathhouse civilian's skin texels. The civilian = the towel skin — find the
    // skins the level's characters actually use and histogram every skin texture (the skin mesh's
    // object batches resolve through `level.by_hash` into the owning map's texture list).
    let sim = scene.sim.as_ref().expect("sim");
    // The level archive parsed straight off the disc (the skins' meshes live in its maps).
    let img = nf_formats::iso9660::IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000002.bin")).expect("archive");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    let level = nf_formats::level::Level::parse(&data).expect("parse level");
    let mut checked = 0usize;
    for (skin_hash, skin) in level.skins.iter() {
        // The skin's own meshes only (main list), not the shared rigid extras.
        for mh in skin.main.iter().flatten() {
            let Some(&(m, o)) = level.by_hash.get(mh) else { continue };
            for leaf in &level.maps[m].objects[o].gfx.leaves {
                for b in &leaf.batches {
                    let Some(t) = b.texture else { continue };
                    if t as usize >= level.maps[m].textures.len() { continue; }
                    let tex = &level.maps[m].textures[t as usize];
                    let rgba = tex.frame(0).iter().flat_map(|&i| tex.palette_rgba8()[i as usize]).collect::<Vec<u8>>();
                    let n = rgba.len() / 4;
                    if n == 0 { continue; }
                    let white = rgba.chunks(4).filter(|px| px[0] >= 240 && px[1] >= 240 && px[2] >= 240).count();
                    if white * 10 > n * 9 {
                        panic!("skin {:08x} mesh {:08x} texture {} is {:.0}% white texels ({} of {}) — G-042's white-skin symptom",
                            skin_hash, mh, t, 100.0 * white as f32 / n as f32, white, n);
                    }
                    checked += 1;
                }
            }
        }
    }
    eprintln!("G-042: {checked} skin textures checked on 07000002 — none is ≥90% white");
    let _ = sim;
}

/// G-042's investigate branch: the white texture — which skin/mesh, its size and alpha, which
/// characters place it, and what the batches' UVs sample from it.
#[test]
fn g042_white_skin_texture_investigation() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000002.bin")).expect("archive");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    let level = nf_formats::level::Level::parse(&data).expect("parse level");
    let skin = level.skins.get(&0x05000038).expect("the 0x05000038 skin exists");
    eprintln!("skin 05000038: skeleton {} main meshes {}", skin.skeleton, skin.main.len());
    for mh in skin.main.iter().flatten() {
        let Some(&(m, o)) = level.by_hash.get(mh) else { eprintln!("  mesh {mh:08x}: unresolved"); continue };
        let obj = &level.maps[m].objects[o];
        eprintln!("  mesh {mh:08x} (map {m} object {o} '{}')", obj.name.trim());
        for (li, leaf) in obj.gfx.leaves.iter().enumerate() {
            for (bi, b) in leaf.batches.iter().enumerate() {
                let Some(t) = b.texture else { eprintln!("    leaf {li} batch {bi}: untextured ({} verts)", b.verts.len()); continue };
                let tex = &level.maps[m].textures[t as usize];
                let rgba = tex.frame(0).iter().flat_map(|&i| tex.palette_rgba8()[i as usize]).collect::<Vec<u8>>();
                let n = rgba.len() / 4;
                let (mut white, mut opaque) = (0usize, 0usize);
                for px in rgba.chunks(4) {
                    if px[0] >= 240 && px[1] >= 240 && px[2] >= 240 { white += 1; }
                    if px[3] >= 128 { opaque += 1; }
                }
                let uv: Vec<[f32; 2]> = b.verts.iter().map(|v| v.uv).collect();
                let (mut umin, mut umax, mut vmin, mut vmax) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
                for u in &uv { umin = umin.min(u[0]); umax = umax.max(u[0]); vmin = vmin.min(u[1]); vmax = vmax.max(u[1]); }
                eprintln!("    leaf {li} batch {bi}: tex {t} {}x{} frames {} — {white}/{n} white rgb, {opaque}/{n} opaque alpha, uv u {umin:.3}..{umax:.3} v {vmin:.3}..{vmax:.3} ({} verts)",
                    tex.width, tex.height, tex.frames, b.verts.len());
            }
        }
    }
    // Who places this skin?
    for inst in level.character_instances().0.iter() {
        let sh = inst.placement.params.iter().find(|(i, _)| *i == 0).map(|&(_, v)| v).unwrap_or(0);
        if sh == 0x05000038 {
            eprintln!("placement at {:?} flags {:04x} draws skin 05000038", inst.placement.translation, inst.placement.flags & 0xffff);
        }
    }
}
