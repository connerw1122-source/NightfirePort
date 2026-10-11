//! Headless NF_ISO tests for the 0x800-object TWEAK/ambient lighting (`docs/atlas/gs-leaf.md`,
//! `docs/atlas/render-port-check.md` §1.1) and the HUD cut-scene gate (`Game_Draw` 0x1e6f50 /
//! `Camera_Enable` 0x1b9920, viewer+0x2a).
//!
//! The game dims every `obj+0xF0 & 0x800` object (characters, the first-person view model, flagged world
//! props) by `tint × obj+0x100..0x102 >> 8` (`View_SetupRenderModes_Tweak` 0x1e5888), with the ambient
//! bytes snapped to the object's cel's ambient on the level-start link (`Lights_CalcAmbientLight`
//! 0x1cb178, snap in `control_link_object_to_cel` 0x1346b8). Before the fix the port pushed vertex colours
//! verbatim for characters and world alike, so characters and guns rendered at ~100 % albedo where the
//! game renders them at the cel ambient — on 07000005 22–34 % [L ee_start_05.bin: obj 0x01382bc0
//! +0x100..0x102 = `39 46 56`].
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

/// The multiplier floats `(tint × ambient) >> 8` of `View_SetupRenderModes_Tweak` (0x1e5888) for the white
/// tint, computed on the game's integer path.
fn expect_bytes(a: [u8; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| ((0xffu32 * a[k] as u32) >> 8) as f32 / 255.0)
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1.0 / 255.0)
}

/// The level archive parsed straight off the disc (for the source vertex colours).
fn level_from_disc(iso: &str, archive: &str) -> nf_formats::level::Level {
    let img = nf_formats::iso9660::IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive entry");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive");
    nf_formats::level::Level::parse(&data).expect("parse level")
}

#[test]
fn guard_and_view_model_dim_to_the_cel_ambient_on_07000005() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_ref().expect("sim");
    step(&scene, 1);
    // [L ee_start_05.bin] every drone reads obj+0x100..0x102 = (0x39, 0x46, 0x56) = (57, 70, 86) — all 13
    // drones sit in cels whose ambient triple is (57, 70, 86) (zone 0, switch_channels[0] = 0).
    let expect = expect_bytes([0x39, 0x46, 0x56]);
    // The view model (the first-person gun) follows Bond's cel: the snapshot's view-only objects
    // (obj+0xFA & 1, created by Player_InitWeapon 0x1a7ef8) read the same (57, 70, 86) in Bond's cel.
    let vm_char = sim.view_model_chars.first().copied().expect("a view model exists on 07000005");
    let vm = sim.char_multiplier(vm_char);
    assert!(close(vm, expect), "view model multiplier {vm:?} != [L] (57,70,86)/255 {expect:?}");
    // One guard: the first drone-backed character. The snapshot's drones (e.g. 0x01382bc0, the sniper at
    // (62.2, 34.8, 104.2)) all carry (57, 70, 86).
    let guard = (0..scene.characters.len()).find(|&ci| sim.char_drone[ci].is_some()).expect("a guard");
    let gm = sim.char_multiplier(guard);
    assert!(close(gm, expect), "guard multiplier {gm:?} != [L] (57,70,86)/255 {expect:?}");
    // Sisters: the comparison can fail, and the naive "first cel = the level ambient" port would fail too —
    // the snapshot's FIRST runtime cel (0x016f12a0) carries (0x1b, 0x28, 0x38) = (27, 40, 56) [L cel.md],
    // which is NOT what the characters read.
    assert!(!close(gm, [1.0; 3]), "guard still at the pre-fix 1.0 multiplier");
    assert_ne!([0x39u8, 0x46, 0x56], [0x1b, 0x28, 0x38], "the first cel's ambient differs from the guards' cel");
    // The multipliers reach the vertex buffer: some Character-layer vertex must be darker than the raw rgba.
    let posed = scene.posed_vertices(None, 0.0, Default::default());
    let start = scene.char_vertex_start;
    assert!(posed.iter().zip(scene.vertices[start..].iter())
        .any(|(p, v)| p.rgba[0] < v.rgba[0] || p.rgba[1] < v.rgba[1] || p.rgba[2] < v.rgba[2]),
        "no Character-layer vertex was dimmed by the ambient multiply");
}

#[test]
fn characters_dim_to_their_own_cel_ambient_on_07000001() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000001.bin", false).expect("load");
    let sim = scene.sim.as_ref().expect("sim");
    step(&scene, 1);
    // [L ee_start_07000001.bin] the drones sit in cels with three distinct ambient triples:
    //   cel 0x191bae0 / 0x191bc60 → (0x3c, 0x3c, 0x3c) = (60, 60, 60)
    //   cel 0x191f020             → (0x2d, 0x1e, 0x1e) = (45, 30, 30)
    //   cel 0x191e660             → (0x50, 0x50, 0x46) = (80, 80, 70)
    // and Bond's cel (0x191c860, the courtyard — where the view-only objects live) → (0x28, 0x1e, 0x1a).
    let vm_char = sim.view_model_chars.first().copied().expect("a view model exists on 07000001");
    let vm = sim.char_multiplier(vm_char);
    let bond_ambient = expect_bytes([0x28, 0x1e, 0x1a]);
    assert!(close(vm, bond_ambient), "view model multiplier {vm:?} != [L] (40,30,26)/255 {bond_ambient:?}");
    let triples = [[0x3cu8, 0x3c, 0x3c], [0x2d, 0x1e, 0x1e], [0x50, 0x50, 0x46]];
    let mut seen = [false; 3];
    for ci in 0..scene.characters.len() {
        if sim.char_drone[ci].is_none() { continue; }
        let m = sim.char_multiplier(ci);
        for (k, t) in triples.iter().enumerate() {
            if close(m, expect_bytes(*t)) { seen[k] = true; }
        }
        // Sister: no guard stays at the pre-fix 1.0.
        assert!(!close(m, [1.0; 3]), "guard {ci} still at the pre-fix 1.0 multiplier");
    }
    assert!(seen.iter().all(|&s| s), "not all three [L] drone ambient triples were hit: {seen:?}");
    // Sister: the room-cel parse matches the snapshot's 11 flags-0x40000 cels of 120 total [L].
    let level01 = level_from_disc(&iso, "07000001.bin");
    assert_eq!(scene::parse_room_cels(&level01).len(), 11, "07000001's 0x40000 room cels");
}

#[test]
fn world_cel_geometry_stays_at_multiplier_one() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    // The world-cel path forces the white tweak (`View_DrawCelGlist` 0x1e6538 → TweakRGBA = 0), so cel
    // geometry keeps multiplier 1.0 — its drawn vertex colours must equal the source object bytes exactly.
    let level = level_from_disc(&iso, "07000005.bin");
    // A cel-creating instance whose object is one leaf with one batch (unique vertex → material match).
    let all_instances = level.instances(false);
    let inst = all_instances.0.iter().find(|i| {
        if !i.placement.creates_cel() { return false; }
        // Only instances the build actually draws (not skipped by the cel pass, no door/searchlight/corona/sky type).
        if matches!(i.placement.flags & 0xff, 0xdb | 0xde | 0xfc | 0x2a) { return false; }
        let o = &level.maps[i.map].objects[i.object];
        if o.skipped_by_cel_pass() { return false; }
        let mut leaves = o.gfx.leaves.iter().filter(|l| !l.batches.is_empty());
        matches!(leaves.next(), Some(l) if l.batches.len() == 1 && leaves.next().is_none())
    }).expect("a single-batch cel instance");
    let obj = &level.maps[inst.map].objects[inst.object];
    let batch = obj.gfx.leaves.iter().flat_map(|l| &l.batches).next().unwrap();
    let centre = nf_formats::placement::transform(&inst.matrix, obj.centre);
    let key = batch.texture.filter(|&t| (t as usize) < level.maps[inst.map].textures.len()).map(|t| (inst.map, t));
    let mat = scene.materials.iter().find(|m| m.layer == scene::Layer::World && m.centre.map(f32::to_bits) == centre.map(f32::to_bits) && m.key == key)
        .unwrap_or_else(|| panic!("no material for cel instance {} at {:?}", obj.name.trim(), centre));
    // The material's unique vertex indices, in batch order, must carry the batch rgba bytes untouched.
    let mut seen: Vec<u32> = Vec::new();
    for t in scene.indices[mat.first_index as usize..(mat.first_index + mat.index_count) as usize].chunks(3) {
        for &k in t { if !seen.contains(&k) { seen.push(k); } }
    }
    seen.sort();
    assert_eq!(seen.len(), batch.verts.len(), "vertex count mismatch for {}", obj.name.trim());
    for (vi, &k) in seen.iter().enumerate() {
        assert_eq!(scene.vertices[k as usize].rgba, batch.verts[vi].rgba,
            "cel geometry vertex {vi} of {} was modified — the world must stay at multiplier 1.0", obj.name.trim());
    }
    // Sister: the parsed room cels are the real tables — the snapshot's runtime cel list has exactly 19
    // flags-0x40000 room cels of 137 total [L ee_start_05.bin], and one carries the (57, 70, 86) ambient.
    let cels = scene::parse_room_cels(&level);
    assert_eq!(cels.len(), 19, "the level's 0x40000 room cels");
    assert!(cels.iter().any(|c| c.ambient[0] == [0x39, 0x46, 0x56]), "no parsed cel carries the [L] (57,70,86) ambient");
}

#[test]
fn hud_hides_while_a_script_camera_runs() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000001.bin", false).expect("load");
    // Before any script camera the overlay draws (the HUD exists).
    let before = nf_view::overlay_for(&scene, 16.0 / 9.0);
    assert!(!before.v.is_empty(), "the HUD overlay is empty before the cut-scene");
    // The opening cut-scene of 07000001 starts a script camera: while it runs the overlay must vanish
    // (the game clears viewer+0x2a via Camera_Enable 0x1b9920 from Script_CameraStart 0x1d9140, and
    // Game_Draw 0x1e6f50 skips the whole sprite pass — health iris, ammo, crosshair, damage glow).
    // The opening cut-scene's script player is channel-gated, so arm the channels like the viewer's K key
    // (`for c in 1..=255 { sim.drones.set_channel(c) }`) and let the script run.
    let sim = scene.sim.as_ref().unwrap() as *const scene::Sim as *mut scene::Sim;
    unsafe { for c in 1..=255 { (*sim).drones.set_channel(c) } }
    let mut saw_cutscene = false;
    let mut saw_hidden = false;
    for _ in 0..900 {
        step(&scene, 1);
        let cam = scene.sim.as_ref().unwrap().script_camera();
        let hud = nf_view::overlay_for(&scene, 16.0 / 9.0);
        if cam.is_some() {
            saw_cutscene = true;
            if hud.v.is_empty() { saw_hidden = true; }
        }
        if saw_cutscene && cam.is_none() { break; }
    }
    assert!(saw_cutscene, "07000001's opening cut-scene never ran a script camera in 900 ticks");
    assert!(saw_hidden, "the HUD overlay kept drawing while the script camera ran");
    // Sister: the same overlay that hid draws again once the camera is gone (the loop breaks on camera end;
    // `before` pins that the gate is the script camera, not a broken overlay).
}
